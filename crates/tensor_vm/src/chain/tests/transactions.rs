use super::*;

#[test]
fn chain_applies_register_transfer_and_claim_reward_transactions() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner-tx");
    let validator = address(b"validator-tx");
    let receiver = address(b"receiver");
    assert_eq!(
        chain
            .apply_transaction(None, Transaction::RegisterMiner(miner))
            .unwrap(),
        vec![ChainEvent::MinerRegistered(miner)]
    );
    assert_eq!(
        chain
            .apply_transaction(None, Transaction::RegisterValidator(validator))
            .unwrap(),
        vec![ChainEvent::ValidatorRegistered(validator)]
    );
    assert!(chain.state().miners().contains_key(&miner));
    assert!(chain.state().validators().contains_key(&validator));

    chain.credit_account(miner, 500);
    assert_eq!(
        chain
            .apply_transaction(
                Some(miner),
                Transaction::Transfer {
                    to: receiver,
                    amount: 125,
                },
            )
            .unwrap(),
        vec![ChainEvent::AccountTransferred {
            from: miner,
            to: receiver,
            amount: 125,
        }]
    );
    assert_eq!(
        chain.state().accounts().get(&receiver).unwrap().balance,
        125
    );

    chain.credit_reward_for_testing(miner, 42);
    assert_eq!(
        chain
            .apply_transaction(None, Transaction::ClaimReward(miner))
            .unwrap(),
        vec![ChainEvent::RewardClaimed {
            address: miner,
            amount: 42,
        }]
    );
    assert_eq!(chain.state().rewards().balance(&miner), 0);
    assert_eq!(chain.state().accounts().get(&miner).unwrap().balance, 417);
}

#[test]
fn reference_submission_transactions_are_txpool_only() {
    let beacon = hash_bytes(b"test", &[b"reference-submission-txpool-only"]);
    let mut chain = Chain::new(beacon);
    for tx in [
        Transaction::SubmitTensorOpReceipt(hash_bytes(b"test", &[b"queued-tensor-receipt"])),
        Transaction::SubmitLinearTrainingStepReceipt(hash_bytes(
            b"test",
            &[b"queued-linear-receipt"],
        )),
        Transaction::SubmitAttestation(hash_bytes(b"test", &[b"queued-attestation"])),
    ] {
        assert!(tx.is_reference_submission());
        assert_eq!(
            chain.apply_transaction(None, tx),
            Err(TvmError::InvalidReceipt(
                "reference submissions must enter the transaction pool"
            ))
        );
    }
}
