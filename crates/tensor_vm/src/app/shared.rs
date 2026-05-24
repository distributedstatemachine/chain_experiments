use super::KeyValueReportWriter;
use crate::{hash::hex, types::hash_bytes};

pub fn local_cpu_seed_beacon() -> [u8; 32] {
    hash_bytes(b"tensor-vm-local-cpu-compose-seed", &[b"shared-chain-base"])
}

pub fn p2p_identity_report(identity_seed: Option<[u8; 32]>) -> String {
    let mut report = KeyValueReportWriter::new();
    report.field("p2p_identity_seeded", identity_seed.is_some());
    if let Some(seed) = identity_seed {
        report.field("p2p_identity_seed", hex(&seed));
    }
    report.finish()
}
