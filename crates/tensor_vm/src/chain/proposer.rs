use super::ChainState;
use crate::types::{Address, Hash, hash_to_u128};

pub(super) fn for_next_epoch(state: &ChainState, beacon: &Hash) -> Option<Address> {
    if state.validators.is_empty() {
        return None;
    }

    let total_stake: u64 = state
        .validators
        .values()
        .map(|validator| validator.stake)
        .sum();
    let mut draw = if total_stake == 0 {
        0
    } else {
        (hash_to_u128(beacon) % total_stake as u128) as u64
    };
    for validator in state.validators.values() {
        if draw < validator.stake {
            return Some(validator.address);
        }
        draw -= validator.stake;
    }
    state.validators.keys().next().copied()
}
