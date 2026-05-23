use std::collections::BTreeSet;

use super::{
    network::{chain_announcement_checkpoint, publish_new_chain_announcements},
    runtime_config::{ServiceRuntimeConfig, runtime_role_wallet_registration},
    validator_fetch::fetch_validator_role_missing_tensors,
};
use tensor_vm::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobScheduler, NodeRuntimeState, NodeStore,
    ReceiptState, RpcHttpServer, RpcNode, SyntheticLocalJobSource, TensorVmLibp2pService,
    hash::hex,
    jobs::LinearTrainingStepOutput,
    roles::{ReferenceValidatorRole, RoleReceiptArtifacts, RoleReceiptBundle},
    types::{Address, Hash},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ValidatorRoleWorkObservation {
    pub(super) assigned_receipts: BTreeSet<Hash>,
    pub(super) unattested_receipts: BTreeSet<Hash>,
    pub(super) artifact_ready_receipts: BTreeSet<Hash>,
    pub(super) artifact_missing_receipts: BTreeSet<Hash>,
}

pub(super) fn validator_role_work_observation(
    node: &RpcNode,
    validator: Address,
) -> ValidatorRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = node.chain.state().finalized_randomness();
    let mut observation = ValidatorRoleWorkObservation::default();
    for (receipt_id, receipt) in node.chain.state().receipts() {
        let assignment = scheduler.assign_validators(&node.chain, *receipt_id, &assignment_seed);
        if !assignment.validators.contains(&validator) {
            continue;
        }
        observation.assigned_receipts.insert(*receipt_id);
        if validator_has_attested_for_receipt(&node.chain, validator, *receipt_id) {
            continue;
        }
        observation.unattested_receipts.insert(*receipt_id);
        if role_receipt_bundle_from_local_tensors(node, receipt).is_some() {
            observation.artifact_ready_receipts.insert(*receipt_id);
        } else {
            observation.artifact_missing_receipts.insert(*receipt_id);
        }
    }
    observation
}

fn validator_has_attested_for_receipt(chain: &Chain, validator: Address, receipt_id: Hash) -> bool {
    chain
        .state()
        .attestations()
        .get(&receipt_id)
        .is_some_and(|attestations| {
            attestations
                .iter()
                .any(|attestation| attestation.validator == validator)
        })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ValidatorRoleAttestationSubmission {
    pub(super) attestations_submitted: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ValidatorRoleBlockVoteSubmission {
    pub(super) block_votes_submitted: usize,
}

pub(super) fn submit_validator_role_attestation(
    node: &mut RpcNode,
    validator: Address,
    receipt_id: Hash,
) -> std::result::Result<Option<ValidatorRoleAttestationSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment = scheduler.assign_validators(
        &node.chain,
        receipt_id,
        &node.chain.state().finalized_randomness(),
    );
    if !assignment.validators.contains(&validator)
        || validator_has_attested_for_receipt(&node.chain, validator, receipt_id)
    {
        return Ok(None);
    }
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(None);
    };
    let Some(job) = node.chain.state().jobs().get(&receipt.job_id()).cloned() else {
        return Ok(None);
    };
    let Some(bundle) = role_receipt_bundle_from_local_tensors(node, &receipt) else {
        return Ok(None);
    };
    let validation_seed = node.chain.validation_seed(&receipt_id);
    let attestation = ReferenceValidatorRole::new(validator, validator_stake)
        .verify_receipt(
            &job,
            &bundle,
            &validation_seed,
            &node.chain.params().freivalds,
        )
        .map_err(|error| {
            format!(
                "validator role failed to verify receipt {}: {error}",
                hex(&receipt_id)
            )
        })?;
    if attestation.receipt_id != receipt_id || attestation.validator != validator {
        return Err(
            "validator role produced attestation for the wrong receipt or validator".to_owned(),
        );
    }
    node.chain
        .apply_command(ChainCommand::SubmitAttestation(attestation))
        .map_err(|error| {
            format!(
                "validator role failed to submit attestation {}: {error}",
                hex(&receipt_id)
            )
        })?;
    Ok(Some(ValidatorRoleAttestationSubmission {
        attestations_submitted: 1,
    }))
}

pub(super) fn submit_validator_role_block_vote(
    node: &mut RpcNode,
    validator: Address,
) -> std::result::Result<Option<ValidatorRoleBlockVoteSubmission>, String> {
    let Some(validator_state) = node.chain.state().validators().get(&validator) else {
        return Ok(None);
    };
    let validator_stake = validator_state.stake;
    let Some(block) = node
        .chain
        .blocks()
        .iter()
        .rev()
        .find(|block| {
            let block_hash = block.hash();
            !node.chain.is_block_finalized(&block_hash)
                && !validator_has_block_vote(&node.chain, validator, block_hash)
                && node.chain.validate_block(block).is_ok()
        })
        .cloned()
    else {
        return Ok(None);
    };
    let vote = BlockVote::new(validator, validator_stake, &block);
    node.chain
        .apply_command(ChainCommand::SubmitBlockVote(vote))
        .map_err(|error| {
            format!(
                "validator role failed to submit block vote {}: {error}",
                hex(&block.hash())
            )
        })?;
    Ok(Some(ValidatorRoleBlockVoteSubmission {
        block_votes_submitted: 1,
    }))
}

fn validator_has_block_vote(chain: &Chain, validator: Address, block_hash: Hash) -> bool {
    chain
        .state()
        .block_votes()
        .get(&block_hash)
        .is_some_and(|votes| votes.iter().any(|vote| vote.validator == validator))
}

pub(super) fn tick_validator_role_work_once(
    config: &ServiceRuntimeConfig,
    store: &NodeStore,
    server: &mut RpcHttpServer,
    p2p_service: &TensorVmLibp2pService,
    runtime_state: &mut NodeRuntimeState,
) -> std::result::Result<bool, String> {
    let Some(validator) = config.role_wallet_address else {
        return Ok(false);
    };
    if runtime_role_wallet_registration(
        config.role,
        config.role_wallet_address,
        &server.gateway().node.chain,
    ) != "validator"
    {
        return Ok(false);
    }
    let observation = validator_role_work_observation(&server.gateway().node, validator);
    let receipt_to_fetch = observation.artifact_missing_receipts.iter().next().copied();
    let mut receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
    let mut status_changed = false;
    if runtime_state.record_validator_work_observation(
        observation.assigned_receipts,
        observation.unattested_receipts,
        observation.artifact_ready_receipts,
        observation.artifact_missing_receipts,
    ) {
        status_changed = true;
    }
    if receipt_to_submit.is_none()
        && let Some(receipt_id) = receipt_to_fetch
    {
        let fetch_report = fetch_validator_role_missing_tensors(
            &mut server.gateway_mut().node,
            p2p_service,
            receipt_id,
        )?;
        if fetch_report.attempts > 0
            || fetch_report.successes > 0
            || fetch_report.failures > 0
            || fetch_report.tensors_inserted > 0
        {
            runtime_state.record_validator_remote_tensor_fetch(
                fetch_report.attempts,
                fetch_report.successes,
                fetch_report.failures,
                fetch_report.bytes,
                fetch_report.tensors_inserted,
            );
            let observation = validator_role_work_observation(&server.gateway().node, validator);
            receipt_to_submit = observation.artifact_ready_receipts.iter().next().copied();
            runtime_state.record_validator_work_observation(
                observation.assigned_receipts,
                observation.unattested_receipts,
                observation.artifact_ready_receipts,
                observation.artifact_missing_receipts,
            );
            status_changed = true;
        }
    }
    if let Some(receipt_id) = receipt_to_submit {
        let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
        if let Some(submission) = submit_validator_role_attestation(
            &mut server.gateway_mut().node,
            validator,
            receipt_id,
        )? {
            publish_new_chain_announcements(
                p2p_service,
                &announcement_checkpoint,
                &server.gateway().node.chain,
            )?;
            store
                .persist_chain(&server.gateway().node.chain)
                .map_err(|error| {
                    format!("failed to persist validator attestation state: {error}")
                })?;
            runtime_state
                .record_validator_attestation_submission(submission.attestations_submitted);
            let observation = validator_role_work_observation(&server.gateway().node, validator);
            runtime_state.record_validator_work_observation(
                observation.assigned_receipts,
                observation.unattested_receipts,
                observation.artifact_ready_receipts,
                observation.artifact_missing_receipts,
            );
            status_changed = true;
        }
    }
    let announcement_checkpoint = chain_announcement_checkpoint(&server.gateway().node.chain);
    if let Some(submission) =
        submit_validator_role_block_vote(&mut server.gateway_mut().node, validator)?
    {
        publish_new_chain_announcements(
            p2p_service,
            &announcement_checkpoint,
            &server.gateway().node.chain,
        )?;
        store
            .persist_chain(&server.gateway().node.chain)
            .map_err(|error| format!("failed to persist validator block vote state: {error}"))?;
        runtime_state.record_validator_block_vote_submission(submission.block_votes_submitted);
        status_changed = true;
    }
    Ok(status_changed)
}

fn role_receipt_bundle_from_local_tensors(
    node: &RpcNode,
    receipt: &ReceiptState,
) -> Option<RoleReceiptBundle> {
    let job = node.chain.state().jobs().get(&receipt.job_id())?;
    match (job, receipt) {
        (tensor_vm::JobState::TensorOp(_), ReceiptState::TensorOp(receipt)) => {
            let a = node
                .tensor_by_commitment_root(receipt.input_roots.first()?)?
                .clone();
            let b = node
                .tensor_by_commitment_root(receipt.input_roots.get(1)?)?
                .clone();
            let c = node
                .tensor_by_commitment_root(receipt.output_roots.first()?)?
                .clone();
            Some(RoleReceiptBundle {
                receipt: ReceiptState::TensorOp(receipt.clone()),
                artifacts: RoleReceiptArtifacts::TensorOp { a, b, c },
            })
        }
        (
            tensor_vm::JobState::LinearTrainingStep(job),
            ReceiptState::LinearTrainingStep(receipt),
        ) => {
            let weights_before = SyntheticLocalJobSource::linear_training_weights();
            if weights_before.commitment_root() != job.weight_root_before
                || receipt.weight_root_before != job.weight_root_before
            {
                return None;
            }
            let (x, target) = job.batch_tensors().ok()?;
            let y = node.tensor_by_commitment_root(&receipt.y_root)?.clone();
            let grad_w = node
                .tensor_by_commitment_root(&receipt.grad_w_root)?
                .clone();
            let weight_after = node
                .tensor_by_commitment_root(&receipt.weight_root_after)?
                .clone();
            let dy = y.sub(&target).ok()?;
            Some(RoleReceiptBundle {
                receipt: ReceiptState::LinearTrainingStep(receipt.clone()),
                artifacts: RoleReceiptArtifacts::LinearTrainingStep {
                    weights_before,
                    output: Box::new(LinearTrainingStepOutput {
                        x,
                        target,
                        y,
                        dy,
                        grad_w,
                        weight_after,
                        loss_commitment: receipt.loss_commitment,
                    }),
                },
            })
        }
        _ => None,
    }
}
