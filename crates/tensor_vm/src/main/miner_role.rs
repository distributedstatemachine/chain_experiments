use std::collections::BTreeSet;
use tensor_vm::{
    Chain, ChainCommand, ChainEngine, JobScheduler, RpcNode, Tensor,
    hash::hex,
    roles::CpuReferenceMinerRole,
    types::{Address, Hash},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MinerRoleWorkObservation {
    pub(super) assigned_jobs: BTreeSet<Hash>,
    pub(super) unreceipted_jobs: BTreeSet<Hash>,
}

pub(super) fn miner_role_work_observation(
    chain: &Chain,
    miner: Address,
) -> MinerRoleWorkObservation {
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment_seed = chain.state().finalized_randomness();
    let mut observation = MinerRoleWorkObservation::default();
    for job_id in chain.state().jobs().keys() {
        let assignment = scheduler.assign_miners(chain, *job_id, &assignment_seed);
        if !assignment.miners.contains(&miner) {
            continue;
        }
        observation.assigned_jobs.insert(*job_id);
        if !miner_has_receipt_for_job(chain, miner, *job_id) {
            observation.unreceipted_jobs.insert(*job_id);
        }
    }
    observation
}

fn miner_has_receipt_for_job(chain: &Chain, miner: Address, job_id: Hash) -> bool {
    chain
        .state()
        .receipts()
        .values()
        .any(|receipt| receipt.job_id() == job_id && receipt.miner() == miner)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MinerRoleReceiptSubmission {
    pub(super) receipts_submitted: usize,
    pub(super) tensors_inserted: usize,
    pub(super) served_tensors: Vec<Tensor>,
}

pub(super) fn submit_miner_role_receipt(
    node: &mut RpcNode,
    miner: Address,
    job_id: Hash,
) -> std::result::Result<Option<MinerRoleReceiptSubmission>, String> {
    if !node.chain.state().miners().contains_key(&miner) {
        return Ok(None);
    }
    let scheduler = JobScheduler::with_small_shape((8, 8, 8));
    let assignment = scheduler.assign_miners(
        &node.chain,
        job_id,
        &node.chain.state().finalized_randomness(),
    );
    if !assignment.miners.contains(&miner) || miner_has_receipt_for_job(&node.chain, miner, job_id)
    {
        return Ok(None);
    }
    let Some(job) = node.chain.state().jobs().get(&job_id).cloned() else {
        return Ok(None);
    };
    let bundle = CpuReferenceMinerRole::new(miner)
        .execute_job(&job, node.chain.state().height(), 1)
        .map_err(|error| format!("miner role failed to execute job {}: {error}", hex(&job_id)))?;
    if bundle.receipt.job_id() != job_id || bundle.receipt.miner() != miner {
        return Err("miner role produced receipt for the wrong job or miner".to_owned());
    }
    let served_tensors = bundle.served_tensors();
    node.chain
        .apply_command(ChainCommand::SubmitReceipt(bundle.receipt))
        .map_err(|error| {
            format!(
                "miner role failed to submit receipt {}: {error}",
                hex(&job_id)
            )
        })?;
    let mut tensors_inserted = 0usize;
    for tensor in &served_tensors {
        node.insert_tensor(tensor.clone());
        tensors_inserted = tensors_inserted.saturating_add(1);
    }
    Ok(Some(MinerRoleReceiptSubmission {
        receipts_submitted: 1,
        tensors_inserted,
        served_tensors,
    }))
}
