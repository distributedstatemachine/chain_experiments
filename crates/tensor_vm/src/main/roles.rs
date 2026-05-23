use std::{collections::BTreeSet, time::Duration};

use tensor_vm::{
    BlockVote, Chain, ChainCommand, ChainEngine, JobScheduler, ReceiptState, RpcNode,
    SyntheticLocalJobSource, Tensor, TensorVmLibp2pService,
    api::P2pMessage,
    decode_tensor_payload,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ValidatorRemoteTensorFetchReport {
    pub(super) attempts: usize,
    pub(super) successes: usize,
    pub(super) failures: usize,
    pub(super) bytes: usize,
    pub(super) tensors_inserted: usize,
}

pub(super) fn fetch_validator_role_missing_tensors(
    node: &mut RpcNode,
    p2p_service: &TensorVmLibp2pService,
    receipt_id: Hash,
) -> std::result::Result<ValidatorRemoteTensorFetchReport, String> {
    let Some(receipt) = node.chain.state().receipts().get(&receipt_id).cloned() else {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    };
    let missing_roots = validator_receipt_required_remote_roots(node, &receipt);
    if missing_roots.is_empty() {
        return Ok(ValidatorRemoteTensorFetchReport::default());
    }
    let peers = p2p_service.connected_peer_ids();
    let mut report = ValidatorRemoteTensorFetchReport::default();
    if peers.is_empty() {
        report.failures = missing_roots.len();
        return Ok(report);
    }
    for root in missing_roots {
        let mut fetched = false;
        let mut failed_response_recorded = false;
        for peer in &peers {
            report.attempts = report.attempts.saturating_add(1);
            let response = p2p_service.request_response(
                *peer,
                P2pMessage::RequestTensorByCommitmentRoot {
                    commitment_root: root,
                },
                Duration::from_secs(2),
            );
            let Ok(response) = response else {
                continue;
            };
            match validator_remote_tensor_response(root, response) {
                ValidatorRemoteTensorResponse::Found { tensor, bytes } => {
                    node.insert_tensor(tensor.clone());
                    p2p_service.register_tensor(tensor);
                    report.bytes = report.bytes.saturating_add(bytes);
                    report.successes = report.successes.saturating_add(1);
                    report.tensors_inserted = report.tensors_inserted.saturating_add(1);
                    fetched = true;
                    break;
                }
                ValidatorRemoteTensorResponse::Missing => {}
                ValidatorRemoteTensorResponse::Invalid => {
                    record_validator_remote_fetch_failure(
                        &mut report,
                        &mut failed_response_recorded,
                    );
                }
            }
        }
        if !fetched && !failed_response_recorded {
            report.failures = report.failures.saturating_add(1);
        }
    }
    Ok(report)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ValidatorRemoteTensorResponse {
    Found { tensor: Tensor, bytes: usize },
    Missing,
    Invalid,
}

pub(super) fn validator_remote_tensor_response(
    requested_root: Hash,
    response: P2pMessage,
) -> ValidatorRemoteTensorResponse {
    let P2pMessage::TensorByCommitmentRootResponse {
        commitment_root,
        payload,
    } = response
    else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    if commitment_root != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    let Some(payload) = payload else {
        return ValidatorRemoteTensorResponse::Missing;
    };
    let bytes = payload.len();
    let Ok(tensor) = decode_tensor_payload(&payload) else {
        return ValidatorRemoteTensorResponse::Invalid;
    };
    if tensor.commitment_root() != requested_root {
        return ValidatorRemoteTensorResponse::Invalid;
    }
    ValidatorRemoteTensorResponse::Found { tensor, bytes }
}

fn record_validator_remote_fetch_failure(
    report: &mut ValidatorRemoteTensorFetchReport,
    recorded_for_root: &mut bool,
) {
    if !*recorded_for_root {
        report.failures = report.failures.saturating_add(1);
        *recorded_for_root = true;
    }
}

fn validator_receipt_required_remote_roots(node: &RpcNode, receipt: &ReceiptState) -> Vec<Hash> {
    let mut roots = Vec::new();
    match receipt {
        ReceiptState::TensorOp(receipt) => {
            roots.extend(receipt.input_roots.iter().copied());
            roots.extend(receipt.output_roots.iter().copied());
        }
        ReceiptState::LinearTrainingStep(receipt) => {
            roots.push(receipt.y_root);
            roots.push(receipt.grad_w_root);
            roots.push(receipt.weight_root_after);
        }
    }
    roots.sort();
    roots.dedup();
    roots
        .into_iter()
        .filter(|root| !node.contains_tensor_commitment_root(root))
        .collect()
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
