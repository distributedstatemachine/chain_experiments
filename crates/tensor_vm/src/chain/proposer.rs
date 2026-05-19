use super::ChainState;
use crate::types::{Address, Hash, hash_to_u128};

pub(super) fn for_next_epoch(state: &ChainState, beacon: &Hash) -> Option<Address> {
    let total_work: u64 = state
        .miners
        .values()
        .map(|miner| miner.settled_tensor_work)
        .sum();
    if total_work == 0 {
        return fallback(state, beacon);
    }

    let mut draw = (hash_to_u128(beacon) % total_work as u128) as u64;
    let mut selected = None;
    for miner in state.miners.values() {
        if miner.settled_tensor_work == 0 {
            continue;
        }
        selected = Some(miner.address);
        if draw < miner.settled_tensor_work {
            break;
        }
        draw -= miner.settled_tensor_work;
    }
    selected
}

fn fallback(state: &ChainState, beacon: &Hash) -> Option<Address> {
    if state.validators.is_empty() {
        return state.miners.keys().next().copied();
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
