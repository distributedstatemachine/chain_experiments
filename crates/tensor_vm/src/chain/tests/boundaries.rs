use super::*;

#[test]
fn chain_rejects_boundary_registration_receipt_vote_and_challenge_errors() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"boundary-miner");
    let validator = address(b"boundary-validator");
    let receiver = address(b"boundary-receiver");

    assert_eq!(chain.proposer_for_next_epoch(&beacon), None);
    assert_eq!(
        chain.register_miner(miner, chain.params.miner_min_stake - 1),
        Err(TvmError::InsufficientStake)
    );
    assert_eq!(
        chain.register_miner_with_profile(
            miner,
            chain.params.miner_min_stake,
            HardwareClass::ConsumerGpu,
            10_001,
        ),
        Err(TvmError::InvalidReceipt("gpu utilization exceeds 100%"))
    );
    assert_eq!(
        chain.register_miner_with_profile(
            miner,
            chain.params.miner_min_stake,
            HardwareClass::Other,
            1,
        ),
        Err(TvmError::InvalidReceipt(
            "non-gpu miner cannot report gpu utilization"
        ))
    );
    chain
        .register_miner_with_profile(
            miner,
            chain.params.miner_min_stake,
            HardwareClass::DatacenterGpu,
            9_000,
        )
        .unwrap();
    let registered_miner = chain.state.miners.get(&miner).unwrap();
    assert_eq!(registered_miner.operator_id, miner);
    assert_eq!(
        registered_miner.hardware_class,
        HardwareClass::DatacenterGpu
    );
    let explicit_operator = address(b"boundary-operator");
    let explicit_miner = address(b"boundary-explicit-miner");
    chain
        .register_miner_with_operator(
            explicit_miner,
            chain.params.miner_min_stake,
            explicit_operator,
        )
        .unwrap();
    assert_eq!(
        chain.state.miners.get(&explicit_miner).unwrap().operator_id,
        explicit_operator
    );
    assert_ne!(miner_root(&chain.state.miners), [0; 32]);
    assert_eq!(
        [HardwareClass::Cpu.tag(), HardwareClass::ConsumerGpu.tag()],
        [1, 2]
    );
    assert_eq!(HardwareClass::Other.tag(), 4);
    assert!(HardwareClass::DatacenterGpu.is_gpu());

    assert_eq!(
        chain.register_validator(validator, chain.params.validator_min_stake - 1),
        Err(TvmError::InsufficientStake)
    );
    chain
        .register_validator(validator, chain.params.validator_min_stake)
        .unwrap();

    assert_eq!(
        chain.transfer(miner, receiver, 1),
        Err(TvmError::InvalidReceipt("insufficient account balance"))
    );
    assert_eq!(
        chain.apply_transaction(
            None,
            Transaction::Transfer {
                to: receiver,
                amount: 1,
            },
        ),
        Err(TvmError::InvalidReceipt("missing sender"))
    );
    assert_eq!(
        chain.apply_transaction(None, Transaction::ClaimReward(miner)),
        Err(TvmError::InvalidReceipt("no reward to claim"))
    );

    let job = MatmulJob::synthetic(0, 77, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let mut unknown_miner_receipt = receipt.clone();
    unknown_miner_receipt.miner = address(b"missing-miner");
    assert_eq!(
        chain.submit_tensor_op_receipt(unknown_miner_receipt),
        Err(TvmError::UnknownMiner)
    );
    assert_eq!(
        chain.submit_tensor_op_receipt(receipt.clone()),
        Err(TvmError::InvalidReceipt("unknown job"))
    );

    let weights = Tensor::from_vec(vec![2, 2], DType::FieldElement, vec![1, 2, 3, 4]).unwrap();
    let linear_job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
        model_id: hash_bytes(b"test", &[b"boundary-model"]),
        step: 0,
        batch_seed: hash_bytes(b"test", &[b"boundary-batch"]),
        weight_root_before: weights.commitment_root(),
        input_shape: vec![2, 2],
        weight_shape: vec![2, 2],
        target_shape: vec![2, 2],
        lr: 1,
        deadline_block: 10,
    });
    let (linear_receipt, _output) =
        LinearTrainingStepReceipt::from_job(&linear_job, miner, &weights, 1, 5).unwrap();
    assert_eq!(
        chain.submit_linear_receipt(linear_receipt.clone()),
        Err(TvmError::InvalidReceipt("unknown job"))
    );
    let mut unknown_linear_miner = linear_receipt.clone();
    unknown_linear_miner.miner = address(b"missing-linear-miner");
    assert_eq!(
        chain.submit_linear_receipt(unknown_linear_miner),
        Err(TvmError::UnknownMiner)
    );
    chain.submit_job(JobState::LinearTrainingStep(linear_job.clone()));
    assert_eq!(chain.job(&linear_job.job_id).unwrap().deadline_block(), 10);
    chain.submit_linear_receipt(linear_receipt.clone()).unwrap();
    assert!(!receipts_agree(
        &ReceiptState::TensorOp(receipt.clone()),
        &ReceiptState::LinearTrainingStep(linear_receipt.clone())
    ));
    assert_eq!(
        chain
            .state
            .receipts
            .get(&linear_receipt.receipt_id)
            .unwrap()
            .receipt_id(),
        linear_receipt.receipt_id
    );
    assert_eq!(
        chain.submit_linear_receipt(linear_receipt.clone()),
        Err(TvmError::InvalidReceipt("duplicate receipt"))
    );

    chain.submit_job(JobState::TensorOp(job.clone()));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    let statement = AttestationStatement {
        receipt_id: receipt.receipt_id,
        job_id: receipt.job_id,
        primitive_type: PrimitiveType::TensorOp,
        result: VerificationResult::Valid,
        checks_root: hash_bytes(b"test", &[b"checks"]),
        data_availability_passed: true,
    };
    assert_eq!(
        chain.submit_attestation(ValidatorAttestation::new(
            address(b"unknown-validator"),
            chain.params.validator_min_stake,
            statement.clone(),
        )),
        Err(TvmError::UnknownValidator)
    );
    let mut bad_signature =
        ValidatorAttestation::new(validator, chain.params.validator_min_stake, statement);
    bad_signature.signature = [9; 32];
    assert_eq!(
        chain.submit_attestation(bad_signature),
        Err(TvmError::InvalidReceipt("bad attestation signature"))
    );
    assert_eq!(
        chain.submit_attestation(ValidatorAttestation::new(
            validator,
            chain.params.validator_min_stake,
            AttestationStatement {
                receipt_id: hash_bytes(b"test", &[b"unknown-receipt"]),
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: VerificationResult::Valid,
                checks_root: hash_bytes(b"test", &[b"checks"]),
                data_availability_passed: true,
            },
        )),
        Err(TvmError::UnknownReceipt)
    );

    let block = chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(
            address(b"unknown-vote-validator"),
            1,
            &block
        )),
        Err(TvmError::UnknownValidator)
    );
    let mut bad_vote = BlockVote::new(validator, chain.params.validator_min_stake, &block);
    bad_vote.signature = [7; 32];
    assert_eq!(
        chain.submit_block_vote(bad_vote),
        Err(TvmError::InvalidReceipt("bad block vote signature"))
    );
    let mut orphan = block.clone();
    orphan.height = 999;
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(
            validator,
            chain.params.validator_min_stake,
            &orphan,
        )),
        Err(TvmError::InvalidReceipt("unknown block"))
    );

    let model = hash_bytes(b"test", &[b"missing-model"]);
    assert_eq!(
        chain.apply_model_transition(&model, 0, &weights.commitment_root(), [1; 32]),
        Err(TvmError::InvalidReceipt("unknown model"))
    );
    assert_eq!(
        chain.apply_challenge_outcome(ChallengeOutcome::Rejected {
            reason: "honest".to_owned(),
        }),
        Ok(())
    );
    assert_eq!(
        chain.apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
            dishonest_party: address(b"unknown-dishonest-party"),
            slash_amount: 1,
            reason: "invalid".to_owned(),
        }),
        Err(TvmError::InvalidReceipt("unknown dishonest party"))
    );
    assert_eq!(
        chain.apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
            dishonest_party: validator,
            slash_amount: 100,
            reason: "bad attestation".to_owned(),
        }),
        Ok(())
    );
    assert_eq!(
        chain.state.validators.get(&validator).unwrap().stake,
        chain.params.validator_min_stake - 100
    );
}
