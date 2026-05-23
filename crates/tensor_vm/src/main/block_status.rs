use super::status::hex_hash_list;
use std::collections::BTreeSet;
use tensor_vm::{Chain, NodeStore, PrimitiveType, hash::hex};

pub(super) fn service_block_status(
    data_dir: &str,
    height: u64,
) -> std::result::Result<String, String> {
    let store = NodeStore::open(data_dir);
    let chain = store
        .load_chain()
        .map_err(|error| format!("failed to load node store {data_dir}: {error}"))?;
    let Some(block) = chain.blocks().iter().find(|block| block.height == height) else {
        return Err(format!(
            "block height {height} is not in node store {data_dir}"
        ));
    };
    let block_hash = block.hash();
    let selected_receipt_ids = chain.selected_receipts_for_block(block);
    let blockspace_caps = chain.blockspace_caps();
    let selected_receipt_twu = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.tensor_work_units())
        .sum::<u64>();
    let selected_receipt_bytes = selected_receipt_ids
        .iter()
        .filter_map(|receipt_id| chain.state().receipts().get(receipt_id))
        .map(|receipt| receipt.estimated_block_bytes())
        .sum::<u64>();
    let block_valid = chain.validate_block(block).is_ok();
    let proposer_registered = chain.state().validators().contains_key(&block.proposer);
    let pow_hash = block.pow_hash();
    let pow_header_hash = block.pow_header_hash();
    let block_votes = chain
        .state()
        .block_votes()
        .get(&block_hash)
        .cloned()
        .unwrap_or_default();
    let total_validator_stake = chain
        .state()
        .validators()
        .values()
        .map(|validator| validator.stake)
        .sum::<u64>();
    let finality_threshold_stake = finality_threshold_stake(&chain, total_validator_stake);
    let mut seen_vote_validators = BTreeSet::new();
    let mut valid_vote_validators = Vec::new();
    let mut valid_vote_stake = 0_u64;
    for vote in &block_votes {
        let Some(validator) = chain.state().validators().get(&vote.validator) else {
            continue;
        };
        if validator.stake != vote.stake || !vote.verify_signature() {
            continue;
        }
        if seen_vote_validators.insert(vote.validator) {
            valid_vote_validators.push(vote.validator);
            valid_vote_stake = valid_vote_stake.saturating_add(vote.stake);
        }
    }
    let mut receipt_ids = Vec::new();
    let mut tensor_op_receipt_ids = Vec::new();
    let mut linear_training_receipt_ids = Vec::new();
    let mut settled_receipt_ids = Vec::new();
    for receipt in chain
        .state()
        .receipts()
        .values()
        .filter(|receipt| receipt.submitted_at_block() == height)
    {
        let receipt_id = receipt.receipt_id();
        receipt_ids.push(receipt_id);
        if chain.state().settled_receipts().contains(&receipt_id) {
            settled_receipt_ids.push(receipt_id);
        }
        match receipt.primitive_type() {
            PrimitiveType::TensorOp => tensor_op_receipt_ids.push(receipt_id),
            PrimitiveType::LinearTrainingStep => linear_training_receipt_ids.push(receipt_id),
        }
    }
    Ok(format!(
        "command=service_block\ndata_dir={data_dir}\nheight={height}\nblock_hash={}\nblock_validation=useful_verification_pow\nparent_hash={}\nproposer={}\nproposer_role=validator\nproposer_registered={}\ntensorwork_proposer_selection=false\nstate_root={}\nepoch={}\nlatest_height={}\nfinalized={}\nsettled_receipt_set_root={}\nselected_receipt_ids={}\nselected_receipt_count={}\nselected_receipt_twu={}\nselected_receipt_bytes={}\nblock_twu_cap={}\nblock_byte_cap={}\nblock_receipt_cap={}\nchecks_root={}\ncheck_leaf_count={}\nchecks_root_recomputed={}\ndifficulty_target={}\nnonce={}\npow_header_hash={}\npow_hash={}\npow_valid={}\ncanonical_blockspace_valid={}\nblock_vote_count={}\nblock_vote_validators={}\nblock_vote_stake={}\nfinality_threshold_stake={}\nfinality_validated_block={}\nreceipt_count={}\nreceipt_ids={}\ntensor_op_receipt_count={}\ntensor_op_receipt_ids={}\nlinear_training_receipt_count={}\nlinear_training_receipt_ids={}\nsettled_receipt_count={}\nsettled_receipt_ids={}\nstatus_source=node_store",
        hex(&block_hash),
        hex(&block.parent_hash),
        hex(&block.proposer),
        proposer_registered,
        hex(&block.state_root),
        block.epoch,
        chain.state().height(),
        chain.is_block_finalized(&block_hash),
        hex(&block.settled_receipt_set_root),
        hex_hash_list(&selected_receipt_ids),
        selected_receipt_ids.len(),
        selected_receipt_twu,
        selected_receipt_bytes,
        blockspace_caps.max_tensor_work_units,
        blockspace_caps.max_bytes,
        blockspace_caps.max_receipts,
        hex(&block.checks_root),
        selected_receipt_ids.len(),
        block_valid,
        hex(&block.difficulty_target),
        block.nonce,
        hex(&pow_header_hash),
        hex(&pow_hash),
        block.pow_valid(),
        block_valid,
        valid_vote_validators.len(),
        hex_hash_list(&valid_vote_validators),
        valid_vote_stake,
        finality_threshold_stake,
        chain.is_block_finalized(&block_hash) && block_valid,
        receipt_ids.len(),
        hex_hash_list(&receipt_ids),
        tensor_op_receipt_ids.len(),
        hex_hash_list(&tensor_op_receipt_ids),
        linear_training_receipt_ids.len(),
        hex_hash_list(&linear_training_receipt_ids),
        settled_receipt_ids.len(),
        hex_hash_list(&settled_receipt_ids),
    ))
}

fn finality_threshold_stake(chain: &Chain, total_validator_stake: u64) -> u64 {
    let numerator = chain.params().finality_stake_numerator;
    let denominator = chain.params().finality_stake_denominator.max(1);
    total_validator_stake
        .saturating_mul(numerator)
        .saturating_add(denominator.saturating_sub(1))
        / denominator
}
