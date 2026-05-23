use super::{Chain, ModelState};
use crate::error::{Result, TvmError};
use crate::types::Hash;

pub fn register(
    chain: &mut Chain,
    model_id: Hash,
    architecture_hash: Hash,
    weight_root: Hash,
    config_hash: Hash,
) {
    chain.state.model_states.insert(
        model_id,
        ModelState {
            model_id,
            architecture_hash,
            weight_root,
            optimizer_state_root: None,
            step: 0,
            config_hash,
        },
    );
}

pub fn apply_transition(
    chain: &mut Chain,
    model_id: &Hash,
    step: u64,
    weight_root_before: &Hash,
    weight_root_after: Hash,
) -> Result<()> {
    let model = chain
        .state
        .model_states
        .get_mut(model_id)
        .ok_or(TvmError::InvalidReceipt("unknown model"))?;
    if model.step != step {
        return Err(TvmError::InvalidReceipt("model step mismatch"));
    }
    if &model.weight_root != weight_root_before {
        return Err(TvmError::InvalidReceipt("model weight root mismatch"));
    }
    model.weight_root = weight_root_after;
    model.step += 1;
    Ok(())
}
