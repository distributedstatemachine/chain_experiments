use crate::hash::hex;
use crate::types::Hash;

pub(super) fn identity_description(identity_seed: Option<Hash>) -> String {
    identity_seed
        .map(|seed| format!(" identity_seed={}", hex(&seed)))
        .unwrap_or_default()
}
