use crate::hash::hex;
use crate::types::Hash;

pub(super) fn identity_report(identity_seed: Option<Hash>) -> String {
    match identity_seed {
        Some(seed) => format!("p2p_identity_seeded=true\np2p_identity_seed={}", hex(&seed)),
        None => "p2p_identity_seeded=false".to_owned(),
    }
}
