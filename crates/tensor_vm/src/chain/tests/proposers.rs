use super::*;

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
