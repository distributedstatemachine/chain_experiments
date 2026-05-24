use super::*;

fn resign_test_block(block: &mut TensorBlock) {
    let block_hash = block.hash();
    block.proposer_signature = sign(&block.proposer, &block_hash);
    block.validator_signature_aggregate =
        hash_bytes(b"tensor-vm-validator-aggregate", &[&block_hash]);
}

fn mine_test_block(block: &mut TensorBlock) {
    while !block.pow_valid() {
        block.nonce = block.nonce.saturating_add(1);
    }
    resign_test_block(block);
}

#[test]
fn reward_allocation_matches_mvp_split_and_credits_proposer_and_treasury() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"reward-proposer");
    chain
        .register_validator(proposer, chain.params.validator_min_stake)
        .unwrap();

    let allocation = chain.params.reward_allocation(10_000);
    assert_eq!(
        allocation,
        RewardAllocation {
            miner_reward_pool: 7_000,
            validator_reward_pool: 2_000,
            proposer_reward: 500,
            treasury_reward: 500,
        }
    );

    let block = chain
        .produce_block_with_rewards(proposer, 1_000, 400, 100)
        .unwrap();
    assert_eq!(chain.state.rewards.balance(&proposer), 500);
    assert_eq!(block.reward_root, reward_root(&chain.state.rewards));

    chain.settle_epoch_rewards(allocation, proposer);
    assert_eq!(chain.state.rewards.balance(&proposer), 1_000);
    assert_eq!(chain.state.rewards.treasury(), 500);
}

#[test]
fn reward_block_production_failure_does_not_credit_proposer() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"unknown-reward-proposer");
    let rewards_before = chain.state.rewards.clone();

    assert_eq!(
        chain.produce_block_with_rewards(proposer, 1_000, 400, 100),
        Err(TvmError::UnknownValidator)
    );
    assert_eq!(chain.state.rewards, rewards_before);
    assert!(chain.blocks.is_empty());
}

#[test]
fn validation_seed_is_bound_to_finalized_randomness_and_receipt() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let chain = Chain::new(beacon);
    let receipt_a = hash_bytes(b"test", &[b"receipt-a"]);
    let receipt_b = hash_bytes(b"test", &[b"receipt-b"]);
    assert_ne!(
        chain.validation_seed(&receipt_a),
        chain.validation_seed(&receipt_b)
    );

    let other_chain = Chain::new(hash_bytes(b"test", &[b"other-beacon"]));
    assert_ne!(
        chain.validation_seed(&receipt_a),
        other_chain.validation_seed(&receipt_a)
    );
}

#[test]
fn proposer_selection_uses_validator_stake() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"validator");
    chain.register_validator(validator, 10_000).unwrap();
    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn fallback_proposer_handles_zero_stake_validator_records() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"zero-stake-validator");
    chain.register_validator(validator, 10_000).unwrap();
    chain.state.validators.get_mut(&validator).unwrap().stake = 0;

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
}

#[test]
fn proposer_selection_ignores_tensorwork() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"settled-miner");
    let validator = address(b"validator-proposer");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();
    chain
        .state
        .miners
        .get_mut(&miner)
        .unwrap()
        .settled_tensor_work = 1_000_000;
    chain
        .state
        .miners
        .get_mut(&miner)
        .unwrap()
        .pending_tensor_work = 1_000_000;

    assert_eq!(chain.proposer_for_next_epoch(&beacon), Some(validator));
    assert_eq!(
        chain.produce_block(miner, 1_000),
        Err(TvmError::UnknownValidator)
    );
}

#[test]
fn blocks_advance_height_and_commit_state() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let proposer = address(b"proposer");
    chain.register_validator(proposer, 10_000).unwrap();
    let block = chain.produce_block(proposer, 1_000).unwrap();
    assert_eq!(block.height, 0);
    assert_eq!(chain.state.height, 1);
    assert_eq!(chain.blocks.len(), 1);
}

#[test]
fn block_finality_requires_two_thirds_validator_stake() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("finality-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let block = chain.produce_block(validators[0], 1_000).unwrap();
    let block_hash = block.hash();

    assert!(!chain.has_block_finality(&block_hash));
    chain
        .submit_block_vote(BlockVote::new(validators[0], 10_000, &block))
        .unwrap();
    assert!(!chain.has_block_finality(&block_hash));
    chain
        .submit_block_vote(BlockVote::new(validators[1], 10_000, &block))
        .unwrap();

    assert!(chain.has_block_finality(&block_hash));
    assert!(chain.is_block_finalized(&block_hash));
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validators[1], 10_000, &block)),
        Err(TvmError::InvalidReceipt("duplicate block vote"))
    );
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validators[2], 1, &block)),
        Err(TvmError::InvalidReceipt("block vote stake mismatch"))
    );
}

#[test]
fn block_finality_ignores_invalid_direct_vote_records() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    assert!(!Chain::new(beacon).has_block_finality(&hash_bytes(b"test", &[b"no-stake"])));

    let mut chain = Chain::new(beacon);
    let validators: Vec<_> = (0..3)
        .map(|i| address(format!("invalid-finality-validator-{i}").as_bytes()))
        .collect();
    for validator in &validators {
        chain.register_validator(*validator, 10_000).unwrap();
    }
    let block = chain.produce_block(validators[0], 1_000).unwrap();
    let block_hash = block.hash();

    let unknown = BlockVote::new(address(b"unknown-direct-validator"), 10_000, &block);
    let wrong_stake = BlockVote::new(validators[0], 1, &block);
    let valid = BlockVote::new(validators[0], 10_000, &block);
    let duplicate = BlockVote::new(validators[0], 10_000, &block);
    let mut bad_signature = BlockVote::new(validators[1], 10_000, &block);
    bad_signature.signature = [9; 32];
    chain.state.block_votes.insert(
        block_hash,
        vec![unknown, wrong_stake, valid, duplicate, bad_signature],
    );

    assert!(!chain.has_block_finality(&block_hash));
    assert!(!chain.is_block_finalized(&block_hash));
}

#[test]
fn block_votes_reject_invalid_useful_pow_and_checks_root() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let validator = address(b"block-validity-validator");
    chain.register_validator(validator, 10_000).unwrap();
    let block = chain.produce_block(validator, 1_000).unwrap();

    let mut bad_target = block.clone();
    bad_target.difficulty_target = [0; 32];
    resign_test_block(&mut bad_target);
    chain.blocks.push(bad_target.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_target)),
        Err(TvmError::InvalidReceipt("block difficulty target mismatch"))
    );
    chain.blocks.pop();

    let mut bad_checks = block.clone();
    bad_checks.checks_root = hash_bytes(b"test", &[b"bad-block-checks"]);
    mine_test_block(&mut bad_checks);
    chain.blocks.push(bad_checks.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_checks)),
        Err(TvmError::InvalidReceipt("block checks root mismatch"))
    );
    chain.blocks.pop();

    let mut bad_state_root = block.clone();
    bad_state_root.state_root = hash_bytes(b"test", &[b"bad-block-state-root"]);
    mine_test_block(&mut bad_state_root);
    chain.blocks.push(bad_state_root.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_state_root)),
        Err(TvmError::InvalidReceipt("block state root mismatch"))
    );
    chain.blocks.pop();

    let mut bad_receipts = block.clone();
    bad_receipts.settled_receipt_set_root = hash_bytes(b"test", &[b"bad-receipt-set"]);
    mine_test_block(&mut bad_receipts);
    chain.blocks.push(bad_receipts.clone());
    assert_eq!(
        chain.submit_block_vote(BlockVote::new(validator, 10_000, &bad_receipts)),
        Err(TvmError::InvalidReceipt("noncanonical settled receipt set"))
    );
}

#[test]
fn produced_blocks_mark_selected_settled_receipts_included_once() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"included-receipt-miner");
    let validator = address(b"included-receipt-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 2, 2, 2, &beacon, 10);
    let (receipt, _a, _b, _c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    chain
        .state
        .receipts
        .insert(receipt.receipt_id, ReceiptState::TensorOp(receipt.clone()));
    chain.state.settled_receipts.insert(receipt.receipt_id);

    let first = chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        chain.selected_receipts_for_block(&first),
        vec![receipt.receipt_id]
    );
    assert!(chain.state.included_receipts.contains(&receipt.receipt_id));

    let second = chain.produce_block(validator, 2_000).unwrap();
    assert!(chain.selected_receipts_for_block(&second).is_empty());
    assert_eq!(
        second.settled_receipt_set_root,
        selected_receipt_root(&BTreeSet::new())
    );
}

#[test]
fn block_roots_commit_to_canonical_receipts_checks_attestations_and_state_values() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"root-miner");
    let validator = address(b"root-validator");
    chain.register_miner(miner, 100).unwrap();
    chain.register_validator(validator, 10_000).unwrap();

    let job = MatmulJob::synthetic(0, 0, 4, 4, 4, &beacon, 10);
    let (receipt, a, b, c) = TensorOpReceipt::from_job(&job, miner, 1, 5).unwrap();
    let report = verify_tensor_op(
        &job,
        &receipt,
        &a,
        &b,
        &c,
        &hash_bytes(b"test", &[b"validation"]),
        &chain.params.freivalds,
    )
    .unwrap();
    chain.submit_job(JobState::TensorOp(job.clone()));
    chain.submit_tensor_op_receipt(receipt.clone()).unwrap();
    chain
        .submit_attestation(ValidatorAttestation::new(
            validator,
            10_000,
            AttestationStatement {
                receipt_id: receipt.receipt_id,
                job_id: receipt.job_id,
                primitive_type: PrimitiveType::TensorOp,
                result: report.result,
                checks_root: report.checks_root,
                data_availability_passed: report.data_availability_passed,
            },
        ))
        .unwrap();

    chain.state.settled_receipts.insert(receipt.receipt_id);
    let parent_hash = chain
        .blocks
        .last()
        .map(TensorBlock::hash)
        .unwrap_or([0; 32]);
    let expected_selection =
        chain.canonical_blockspace(&parent_hash, &chain.state.finalized_randomness);
    let expected_settled_receipt_set_root =
        selected_receipt_root(&expected_selection.receipt_set());
    let expected_checks_root =
        block_checks_root(&expected_selection.receipt_ids, &chain.state.attestations);
    let expected_attestation_root = attestation_root(&chain.state.attestations);
    let expected_state_root = chain.state_root();
    let block = chain.produce_block(validator, 1_000).unwrap();
    assert_eq!(
        block.settled_receipt_set_root,
        expected_settled_receipt_set_root
    );
    assert_eq!(block.checks_root, expected_checks_root);
    assert_eq!(block.attestation_root, expected_attestation_root);
    assert_eq!(block.state_root, expected_state_root);
    assert!(block.pow_valid());

    let mut altered_miners = chain.state.miners.clone();
    altered_miners.get_mut(&miner).unwrap().stake += 1;
    assert_ne!(miner_root(&chain.state.miners), miner_root(&altered_miners));

    let mut altered_receipts = chain.state.receipts.clone();
    match altered_receipts.get_mut(&receipt.receipt_id).unwrap() {
        ReceiptState::TensorOp(receipt) => receipt.execution_time_ms += 1,
        ReceiptState::LinearTrainingStep(_) => unreachable!("test inserts tensor op receipt"),
    }
    assert_ne!(
        receipt_root(&chain.state.receipts),
        receipt_root(&altered_receipts)
    );
}
