use super::roots::{
    attestation_root, block_checks_root, miner_root, receipt_root, reward_root,
    selected_receipt_root,
};
use super::*;
use crate::jobs::{
    LinearTrainingStepJob, LinearTrainingStepReceipt, LinearTrainingStepSpec, MatmulJob,
    TensorOpReceipt,
};
use crate::scheduler::JobScheduler;
use crate::tensor::{DType, Tensor};
use crate::types::{address, hash_bytes, sign};
use crate::verify::{
    AttestationStatement, FreivaldsParams, ValidatorAttestation, VerificationResult,
    verify_tensor_op,
};
use std::collections::BTreeSet;

mod blocks;
mod boundaries;
mod commands;
mod settlement;

#[test]
fn model_transition_enforces_single_sequential_weight_root() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let model_id = hash_bytes(b"test", &[b"model"]);
    let architecture = hash_bytes(b"test", &[b"architecture"]);
    let config = hash_bytes(b"test", &[b"config"]);
    let before = hash_bytes(b"test", &[b"weights-before"]);
    let after = hash_bytes(b"test", &[b"weights-after"]);
    let conflicting = hash_bytes(b"test", &[b"conflicting"]);

    chain
        .register_model(model_id, architecture, before, config)
        .unwrap();
    let before_optimizer_root = chain.state_root();
    chain
        .state
        .model_states
        .get_mut(&model_id)
        .unwrap()
        .optimizer_state_root = Some(hash_bytes(b"test", &[b"optimizer"]));
    assert_ne!(before_optimizer_root, chain.state_root());
    chain
        .apply_model_transition(&model_id, 0, &before, after)
        .unwrap();
    assert_eq!(chain.state.model_states.get(&model_id).unwrap().step, 1);
    let transitioned_model = chain.state.model_states.get(&model_id).unwrap().clone();
    assert_eq!(
        chain.register_model(model_id, architecture, before, config),
        Err(TvmError::InvalidReceipt("duplicate model"))
    );
    assert_eq!(
        chain.state.model_states.get(&model_id),
        Some(&transitioned_model)
    );
    assert_eq!(
        chain.apply_model_transition(&model_id, 0, &before, conflicting),
        Err(TvmError::InvalidReceipt("model step mismatch"))
    );
    assert_eq!(
        chain.apply_model_transition(&model_id, 1, &before, conflicting),
        Err(TvmError::InvalidReceipt("model weight root mismatch"))
    );
}

#[test]
fn challenge_outcome_slashes_miner_and_credits_treasury() {
    let beacon = hash_bytes(b"test", &[b"beacon"]);
    let mut chain = Chain::new(beacon);
    let miner = address(b"miner");
    chain.register_miner(miner, 100).unwrap();
    assert_eq!(
        chain
            .apply_command(ChainCommand::ApplyChallengeOutcome(
                ChallengeOutcome::ProvenInvalid {
                    dishonest_party: miner,
                    slash_amount: 25,
                    reason: "invalid receipt".to_owned(),
                },
            ))
            .unwrap(),
        vec![ChainEvent::ChallengeProvenInvalid {
            dishonest_party: miner,
            slash_amount: 25,
            reason: "invalid receipt".to_owned(),
        }]
    );
    chain
        .apply_challenge_outcome(ChallengeOutcome::ProvenInvalid {
            dishonest_party: miner,
            slash_amount: 5,
            reason: "invalid receipt again".to_owned(),
        })
        .unwrap();
    assert_eq!(chain.state.miners.get(&miner).unwrap().stake, 70);
    assert_eq!(chain.state.miners.get(&miner).unwrap().reputation, -20);
    assert_eq!(chain.state.rewards.treasury(), 30);
}
