use crate::profile::ChainProfile;
#[cfg(test)]
use crate::types::address;
use crate::types::{Hash, Signature};
#[cfg(test)]
use crate::{ChainParams, JobScheduler};
#[cfg(test)]
use libp2p::Multiaddr;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

mod local_harness;
mod public_evidence_bundle;
mod public_evidence_crypto;
mod public_evidence_manifest;
mod public_evidence_publication;
mod public_manifest_fields;
mod public_network_runtime;
mod public_node_evidence;
mod public_operators;
mod public_preflight_manifest;
mod public_preflight_plan;
mod public_run_evidence;
mod public_services;
mod public_urls;

#[cfg(test)]
use local_harness::local_libp2p_multiaddr_has_tcp_node_path;
pub use local_harness::{LocalParticipantEndpoint, LocalTestnet};
#[cfg(test)]
use public_evidence_crypto::PublicNetworkRuntimeObservationDetails;
#[cfg(test)]
use public_evidence_crypto::deterministic_public_network_peer_id;
#[cfg(test)]
use public_evidence_crypto::public_evidence_supporting_artifact_uri;
pub use public_evidence_crypto::{
    PublicEvidenceRecordKind, sign_public_evidence_artifact, sign_public_evidence_record,
    sign_public_run_window,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use public_evidence_crypto::{
    aggregate_public_evidence_record_roots, public_network_runtime_observations_for_run,
};
pub use public_evidence_manifest::parse_public_testnet_evidence_manifest;
pub use public_evidence_publication::{
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceSupportingArtifact,
};
#[cfg(test)]
use public_manifest_fields::parse_hash_hex;
pub use public_network_runtime::{PublicNetworkRuntimeEvidence, PublicNetworkRuntimeObservation};
pub use public_node_evidence::{
    PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation,
};
#[cfg(test)]
use public_operators::match_public_operator_address;
pub use public_preflight_manifest::parse_public_testnet_preflight_manifest;
use public_services::public_service_kinds;
pub use public_services::{
    PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind,
};
#[cfg(test)]
use public_urls::{
    public_evidence_uri_is_external, public_host_is_external, public_https_authorities_match,
    public_https_host, public_https_path, public_network_runtime_multiaddr_is_external,
};

pub const PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-evidence-v1";
pub const PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-preflight-v1";
pub const PUBLIC_SERVICE_MIN_CONTENT_BYTES: u64 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestnetConfig {
    pub miner_count: usize,
    pub validator_count: usize,
    pub miner_stake: u64,
    pub validator_stake: u64,
    pub faucet_balance: u64,
    pub faucet_drip: u64,
}

impl Default for TestnetConfig {
    fn default() -> Self {
        Self {
            miner_count: 10,
            validator_count: 5,
            miner_stake: 100,
            validator_stake: 10_000,
            faucet_balance: 1_000_000,
            faucet_drip: 100,
        }
    }
}

impl TestnetConfig {
    pub fn from_profile(profile: &ChainProfile) -> Self {
        Self {
            miner_count: profile.miner_count,
            validator_count: profile.validator_count,
            miner_stake: profile.miner_stake,
            validator_stake: profile.validator_stake,
            faucet_balance: profile.faucet_balance,
            faucet_drip: profile.faucet_drip,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetCriteria {
    pub min_miners: usize,
    pub min_validators: usize,
    pub duration_days: u64,
    pub min_finality_rate_bps: u64,
    pub min_data_availability_bps: u64,
    pub min_invalid_work_rejections: u64,
    pub min_reward_settlement_records: u64,
}

impl Default for PublicTestnetCriteria {
    fn default() -> Self {
        Self {
            min_miners: 10,
            min_validators: 5,
            duration_days: 7,
            min_finality_rate_bps: 10_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicDeploymentServicePlan {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
    pub content_url: String,
    pub content_path: String,
    pub auth_enabled: bool,
    pub rate_limit_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetPreflightPlan {
    pub config: TestnetConfig,
    pub criteria: PublicTestnetCriteria,
    pub cuda_kernels_available: bool,
    pub cuda_ready_miner_count: usize,
    pub libp2p_ready_node_count: usize,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicDeploymentServicePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetPreflightReport {
    pub miner_count: usize,
    pub validator_count: usize,
    pub required_blocks: u64,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_positive_stakes: bool,
    pub has_funded_faucet: bool,
    pub has_cuda_kernels_available: bool,
    pub cuda_ready_miner_count: usize,
    pub has_cuda_ready_miners: bool,
    pub libp2p_ready_node_count: usize,
    pub has_libp2p_ready_nodes: bool,
    pub has_production_libp2p_runtime: bool,
    pub has_rpc_service_plan: bool,
    pub has_explorer_service_plan: bool,
    pub has_faucet_service_plan: bool,
    pub has_telemetry_service_plan: bool,
    pub has_public_service_content_plan: bool,
    pub has_public_service_plan: bool,
    pub local_shape_ready: bool,
    pub deployment_plan_ready: bool,
    pub can_start_public_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidence {
    pub miner_count: usize,
    pub validator_count: usize,
    pub run_started_at_unix_seconds: u64,
    pub run_ended_at_unix_seconds: u64,
    pub observed_duration_seconds: u64,
    pub required_duration_seconds: u64,
    pub observed_blocks: u64,
    pub required_blocks: u64,
    pub finality_rate_bps: u64,
    pub data_availability_bps: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub invalid_work_rejection_rate_bps: u64,
    pub reward_settlement_records: u64,
    pub external_operator_evidence: bool,
    pub has_production_libp2p_runtime: bool,
    pub has_deployed_rpc_service: bool,
    pub has_deployed_explorer_service: bool,
    pub has_deployed_faucet_service: bool,
    pub has_deployed_telemetry_service: bool,
    pub has_deployed_public_service_content: bool,
    pub has_deployed_public_services: bool,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
    pub has_required_run_duration: bool,
    pub has_required_block_count: bool,
    pub has_required_finality: bool,
    pub has_required_data_availability: bool,
    pub has_invalid_work_rejection_evidence: bool,
    pub has_reward_settlement_records: bool,
    pub public_criterion_met: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetRunEvidence {
    pub nodes: Vec<PublicNodeEvidence>,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicServiceEvidence>,
    pub service_content: Vec<PublicServiceContentEvidence>,
    pub run_started_at_unix_seconds: u64,
    pub run_ended_at_unix_seconds: u64,
    pub observed_blocks: u64,
    pub finalized_blocks: u64,
    pub checked_receipts: u64,
    pub available_receipts: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub reward_settlement_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceRecordSummaries {
    pub block_history_records: u64,
    pub block_history_root: Hash,
    pub finality_history_records: u64,
    pub finality_history_root: Hash,
    pub operator_identity_attestation_records: u64,
    pub network_runtime_observation_records: u64,
    pub network_runtime_observation_root: Hash,
    pub data_availability_measurement_records: u64,
    pub data_availability_measurement_root: Hash,
    pub invalid_work_rejection_records: u64,
    pub invalid_work_rejection_root: Hash,
    pub reward_settlement_root: Hash,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundle {
    pub run: PublicTestnetRunEvidence,
    pub publication: PublicEvidencePublication,
    pub auditor_records: Vec<PublicEvidenceAuditorRecord>,
    pub supporting_artifacts: Vec<PublicEvidenceSupportingArtifact>,
    pub run_window_signature: Signature,
    pub block_history_records: u64,
    pub block_history_root: Hash,
    pub block_history_signature: Signature,
    pub finality_history_records: u64,
    pub finality_history_root: Hash,
    pub finality_history_signature: Signature,
    pub operator_identity_attestation_records: u64,
    pub operator_identity_attestations: Vec<PublicOperatorIdentityAttestation>,
    pub network_runtime_observations: Vec<PublicNetworkRuntimeObservation>,
    pub network_runtime_observation_records: u64,
    pub network_runtime_observation_root: Hash,
    pub network_runtime_observation_signature: Signature,
    pub data_availability_measurement_records: u64,
    pub data_availability_measurement_root: Hash,
    pub data_availability_measurement_signature: Signature,
    pub invalid_work_rejection_records: u64,
    pub invalid_work_rejection_root: Hash,
    pub invalid_work_rejection_signature: Signature,
    pub reward_settlement_root: Hash,
    pub reward_settlement_signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundleReport {
    pub run_evidence: PublicTestnetEvidence,
    pub has_published_evidence_bundle: bool,
    pub has_independent_auditor_records: bool,
    pub has_signed_run_window: bool,
    pub has_block_history: bool,
    pub has_finality_history: bool,
    pub has_operator_identity_attestations: bool,
    pub has_network_runtime_observations: bool,
    pub has_data_availability_measurements: bool,
    pub has_invalid_work_rejection_records: bool,
    pub has_reward_settlement_record_summary: bool,
    pub has_public_supporting_record_artifacts: bool,
    pub independently_checkable: bool,
    pub full_spec_evidence_met: bool,
}

fn ratio_to_bps(value: f64) -> u64 {
    (value.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn ratio_parts_to_bps(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    let numerator = u128::from(numerator.min(denominator));
    let denominator = u128::from(denominator);
    (((numerator * 10_000) + (denominator / 2)) / denominator) as u64
}

fn required_blocks_for_days(days: u64, block_time_seconds: u64) -> u64 {
    required_duration_seconds_for_days(days) / block_time_seconds.max(1)
}

fn required_duration_seconds_for_days(days: u64) -> u64 {
    days.saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
}

fn public_testnet_criteria_are_full_spec(criteria: &PublicTestnetCriteria) -> bool {
    let full_spec = PublicTestnetCriteria::default();
    criteria.min_miners >= full_spec.min_miners
        && criteria.min_validators >= full_spec.min_validators
        && criteria.duration_days >= full_spec.duration_days
        && criteria.min_finality_rate_bps >= full_spec.min_finality_rate_bps
        && criteria.min_data_availability_bps >= full_spec.min_data_availability_bps
        && criteria.min_invalid_work_rejections >= full_spec.min_invalid_work_rejections
        && criteria.min_reward_settlement_records >= full_spec.min_reward_settlement_records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::types::hash_bytes;

    mod deployment_docs;
    mod evidence_bundle;
    mod evidence_manifest;
    mod local_harness;
    mod manifest_fixtures;
    mod network_runtime;
    mod preflight_manifest;
    mod run_evidence;
    mod run_services;
    use manifest_fixtures::*;

    fn production_runtime_evidence() -> PublicNetworkRuntimeEvidence {
        PublicNetworkRuntimeEvidence {
            libp2p_runtime_used: true,
            peer_discovery_observed: true,
            gossip_propagation_observed: true,
            request_response_observed: true,
            dos_controls_enabled: true,
        }
    }

    fn public_service(
        kind: PublicServiceKind,
        label: &[u8],
        first_seen_block: u64,
        last_seen_block: u64,
    ) -> PublicServiceEvidence {
        public_service_with_observations(kind, label, first_seen_block, last_seen_block, 10)
    }

    fn public_service_with_observations(
        kind: PublicServiceKind,
        label: &[u8],
        first_seen_block: u64,
        last_seen_block: u64,
        observation_count: u64,
    ) -> PublicServiceEvidence {
        PublicServiceEvidence::new(
            kind,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[label]),
                public_service_url(kind),
                "/health",
            ),
            first_seen_block,
            last_seen_block,
            observation_count,
            observation_count,
        )
    }

    fn public_service_url(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
            PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
            PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
            PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
        }
    }

    fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
            PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
            PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
            PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
        }
    }

    fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "/chain/head",
            PublicServiceKind::Explorer => "/explorer",
            PublicServiceKind::Faucet => "/faucet/page",
            PublicServiceKind::Telemetry => "/telemetry/dashboard",
        }
    }

    fn public_service_content(
        kind: PublicServiceKind,
        label: &[u8],
    ) -> PublicServiceContentEvidence {
        PublicServiceContentEvidence::new(
            kind,
            hash_bytes(b"test", &[label]),
            public_service_content_url(kind),
            public_service_content_path(kind),
            hash_bytes(b"test", &[label, b"content-root"]),
            1_700_000_000,
            64,
        )
    }

    fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
        let content = public_service_content(kind, label);
        format!(
            "service_content={},{},{},{},{},{},{},{}",
            service_kind_tag(kind),
            hex(&content.endpoint_id),
            content.public_url,
            content.content_path,
            hex(&content.content_root),
            content.observed_at_unix_seconds,
            content.min_content_bytes,
            hex(&content.content_signature)
        )
    }

    fn service_kind_tag(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "rpc",
            PublicServiceKind::Explorer => "explorer",
            PublicServiceKind::Faucet => "faucet",
            PublicServiceKind::Telemetry => "telemetry",
        }
    }

    fn deployed_public_services(last_seen_block: u64) -> Vec<PublicServiceEvidence> {
        vec![
            public_service(PublicServiceKind::Rpc, b"rpc-service", 0, last_seen_block),
            public_service(
                PublicServiceKind::Explorer,
                b"explorer-service",
                0,
                last_seen_block,
            ),
            public_service(
                PublicServiceKind::Faucet,
                b"faucet-service",
                0,
                last_seen_block,
            ),
            public_service(
                PublicServiceKind::Telemetry,
                b"telemetry-service",
                0,
                last_seen_block,
            ),
        ]
    }

    fn deployed_public_service_content() -> Vec<PublicServiceContentEvidence> {
        vec![
            public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
            public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
            public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
            public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
        ]
    }

    fn complete_public_run_evidence() -> PublicTestnetRunEvidence {
        PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"miner-a"),
                    hash_bytes(b"test", &[b"miner-a-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::miner(
                    address(b"miner-b"),
                    hash_bytes(b"test", &[b"miner-b-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    hash_bytes(b"test", &[b"validator-a-operator"]),
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: production_runtime_evidence(),
            services: deployed_public_services(9),
            service_content: deployed_public_service_content(),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
            invalid_receipts_submitted: 1,
            invalid_receipts_rejected: 1,
            reward_settlement_records: 1,
        }
    }

    fn complete_public_evidence_bundle() -> PublicTestnetEvidenceBundle {
        let run = complete_public_run_evidence();
        let network_runtime_observation_root = network_runtime_root_for_run(&run);
        PublicTestnetEvidenceBundle::new(
            run,
            PublicEvidencePublication::new(
                hash_bytes(b"test", &[b"public-evidence-bundle"]),
                String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
                address(b"public-evidence-publisher"),
                1,
                1,
            ),
            PublicEvidenceRecordSummaries {
                block_history_records: 10,
                block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
                finality_history_records: 10,
                finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
                operator_identity_attestation_records: 3,
                network_runtime_observation_records: 3,
                network_runtime_observation_root,
                data_availability_measurement_records: 20,
                data_availability_measurement_root: hash_bytes(
                    b"test",
                    &[b"data-availability-root"],
                ),
                invalid_work_rejection_records: 1,
                invalid_work_rejection_root: hash_bytes(b"test", &[b"invalid-work-root"]),
                reward_settlement_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
            },
        )
    }

    fn full_spec_public_evidence_bundle(block_time_seconds: u64) -> PublicTestnetEvidenceBundle {
        let criteria = PublicTestnetCriteria::default();
        let observed_blocks =
            required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
        let last_seen_block = observed_blocks.saturating_sub(1);
        let run_started_at_unix_seconds = 1_700_000_000;
        let run_ended_at_unix_seconds = run_started_at_unix_seconds
            + required_duration_seconds_for_days(criteria.duration_days);
        let mut nodes = Vec::new();
        for index in 0..criteria.min_miners {
            nodes.push(PublicNodeEvidence::miner(
                address(format!("full-spec-miner-{index}").as_bytes()),
                hash_bytes(
                    b"test",
                    &[format!("full-spec-miner-{index}-operator").as_bytes()],
                ),
                0,
                last_seen_block,
                observed_blocks,
            ));
        }
        for index in 0..criteria.min_validators {
            nodes.push(PublicNodeEvidence::validator(
                address(format!("full-spec-validator-{index}").as_bytes()),
                hash_bytes(
                    b"test",
                    &[format!("full-spec-validator-{index}-operator").as_bytes()],
                ),
                0,
                last_seen_block,
                observed_blocks,
            ));
        }
        let operator_records = nodes.len() as u64;
        let checked_receipts = observed_blocks;
        let run = PublicTestnetRunEvidence {
            nodes,
            network_runtime: production_runtime_evidence(),
            services: vec![
                public_service_with_observations(
                    PublicServiceKind::Rpc,
                    b"rpc-service",
                    0,
                    last_seen_block,
                    observed_blocks,
                ),
                public_service_with_observations(
                    PublicServiceKind::Explorer,
                    b"explorer-service",
                    0,
                    last_seen_block,
                    observed_blocks,
                ),
                public_service_with_observations(
                    PublicServiceKind::Faucet,
                    b"faucet-service",
                    0,
                    last_seen_block,
                    observed_blocks,
                ),
                public_service_with_observations(
                    PublicServiceKind::Telemetry,
                    b"telemetry-service",
                    0,
                    last_seen_block,
                    observed_blocks,
                ),
            ],
            service_content: deployed_public_service_content(),
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
            finalized_blocks: observed_blocks,
            checked_receipts,
            available_receipts: checked_receipts,
            invalid_receipts_submitted: 1,
            invalid_receipts_rejected: 1,
            reward_settlement_records: 1,
        };
        let network_runtime_observation_root = network_runtime_root_for_run(&run);
        PublicTestnetEvidenceBundle::new(
            run,
            PublicEvidencePublication::new(
                hash_bytes(b"test", &[b"full-spec-public-evidence-bundle"]),
                String::from("https://tensorvm.net/tensorvm/full-spec-public-evidence.json"),
                address(b"full-spec-public-evidence-publisher"),
                1,
                1,
            ),
            PublicEvidenceRecordSummaries {
                block_history_records: observed_blocks,
                block_history_root: hash_bytes(b"test", &[b"full-spec-block-history-root"]),
                finality_history_records: observed_blocks,
                finality_history_root: hash_bytes(b"test", &[b"full-spec-finality-history-root"]),
                operator_identity_attestation_records: operator_records,
                network_runtime_observation_records: operator_records,
                network_runtime_observation_root,
                data_availability_measurement_records: checked_receipts,
                data_availability_measurement_root: hash_bytes(
                    b"test",
                    &[b"full-spec-data-availability-root"],
                ),
                invalid_work_rejection_records: 1,
                invalid_work_rejection_root: hash_bytes(b"test", &[b"full-spec-invalid-work-root"]),
                reward_settlement_root: hash_bytes(b"test", &[b"full-spec-reward-settlement-root"]),
            },
        )
    }

    fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
        let record_roots = public_network_runtime_observations_for_run(run)
            .iter()
            .map(|observation| observation.record_root)
            .collect::<Vec<_>>();
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_roots,
        )
        .expect("generated network observation roots should aggregate")
    }

    fn public_network_runtime_observation(
        operator_id: Hash,
        node_index: usize,
        observed_at_unix_seconds: u64,
    ) -> PublicNetworkRuntimeObservation {
        PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
            operator_id,
            peer_id: deterministic_public_network_peer_id(&operator_id),
            listen_address: format!(
                "/dns/role-order-node-{node_index}.tensorvm.net/tcp/{}",
                4_101 + node_index
            ),
            observed_at_unix_seconds,
            gossip_topic_count: 5,
            request_response_protocol_count: 3,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        })
    }
}
