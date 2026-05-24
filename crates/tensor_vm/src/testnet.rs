use crate::ExplorerSummary;
use crate::chain::{
    BlockVote, Chain, ChainCommand, ChainEngine, ChainParams, JobState, ReceiptState, TensorBlock,
    Transaction,
};
use crate::faucet::Faucet;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
use crate::miner::MinerNode;
use crate::profile::ChainProfile;
use crate::runtime::CpuReferenceBackend;
use crate::scheduler::JobScheduler;
use crate::telemetry::TelemetrySnapshot;
use crate::tensor::{DType, Tensor};
use crate::tensor_server::TensorServer;
use crate::txpool::TxPool;
use crate::types::{Address, Hash, Signature, address, hash_bytes, sign, verify_signature};
use crate::validator::ValidatorNode;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

mod public_evidence_bundle;
mod public_evidence_crypto;
mod public_evidence_manifest;
mod public_manifest_fields;
mod public_operators;
mod public_preflight_manifest;
mod public_run_evidence;
mod public_urls;

#[cfg(test)]
use public_evidence_crypto::deterministic_public_network_peer_id;
#[cfg(test)]
use public_evidence_crypto::public_evidence_supporting_artifact_uri;
pub use public_evidence_crypto::{
    PublicEvidenceRecordKind, sign_public_evidence_artifact, sign_public_evidence_record,
    sign_public_run_window,
};
use public_evidence_crypto::{
    PublicNetworkRuntimeObservationDetails, public_evidence_artifact_message,
    public_evidence_auditor_message, public_evidence_manifest_message,
    public_network_runtime_observation_root, public_network_runtime_observation_signature,
    public_node_heartbeat_message, public_operator_identity_message,
    public_service_content_message, public_service_health_message,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use public_evidence_crypto::{
    aggregate_public_evidence_record_roots, public_network_runtime_observations_for_run,
};
pub use public_evidence_manifest::parse_public_testnet_evidence_manifest;
#[cfg(test)]
use public_manifest_fields::parse_hash_hex;
#[cfg(test)]
use public_operators::match_public_operator_address;
pub use public_preflight_manifest::parse_public_testnet_preflight_manifest;
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

impl PublicDeploymentServicePlan {
    pub fn is_public_https_endpoint(&self) -> bool {
        let Some(host) = public_https_host(&self.public_url) else {
            return false;
        };
        public_host_is_external(host)
    }

    pub fn has_public_content_surface(&self) -> bool {
        let Some(host) = public_https_host(&self.content_url) else {
            return false;
        };
        public_host_is_external(host)
            && public_https_authorities_match(&self.public_url, &self.content_url)
            && self.content_path == self.kind.content_path()
            && public_https_path(&self.content_url) == Some(self.kind.content_path())
    }

    pub fn is_ready_for_public_run(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.is_public_https_endpoint()
            && self.health_path.starts_with('/')
            && self.health_path.len() > 1
            && public_https_path(&self.public_url) == Some(self.health_path.as_str())
            && self.has_public_content_surface()
            && self.auth_enabled
            && self.rate_limit_enabled
    }
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

impl PublicTestnetPreflightPlan {
    pub fn evaluate(&self, block_time_seconds: u64) -> PublicTestnetPreflightReport {
        let required_blocks =
            required_blocks_for_days(self.criteria.duration_days, block_time_seconds.max(1));
        let has_required_miners = self.config.miner_count >= self.criteria.min_miners;
        let has_required_validators = self.config.validator_count >= self.criteria.min_validators;
        let has_positive_stakes = self.config.miner_stake > 0 && self.config.validator_stake > 0;
        let has_funded_faucet =
            self.config.faucet_drip > 0 && self.config.faucet_balance >= self.config.faucet_drip;
        let has_cuda_ready_miners = self.cuda_kernels_available
            && self.config.miner_count > 0
            && self.cuda_ready_miner_count == self.config.miner_count;
        let planned_node_count = self
            .config
            .miner_count
            .saturating_add(self.config.validator_count);
        let has_libp2p_ready_nodes =
            planned_node_count > 0 && self.libp2p_ready_node_count == planned_node_count;
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_rpc_service_plan = self.has_ready_service_plan(PublicServiceKind::Rpc);
        let has_explorer_service_plan = self.has_ready_service_plan(PublicServiceKind::Explorer);
        let has_faucet_service_plan = self.has_ready_service_plan(PublicServiceKind::Faucet);
        let has_telemetry_service_plan = self.has_ready_service_plan(PublicServiceKind::Telemetry);
        let has_public_service_content_plan = self
            .has_ready_service_content_plan(PublicServiceKind::Rpc)
            && self.has_ready_service_content_plan(PublicServiceKind::Explorer)
            && self.has_ready_service_content_plan(PublicServiceKind::Faucet)
            && self.has_ready_service_content_plan(PublicServiceKind::Telemetry);
        let has_public_service_plan = has_rpc_service_plan
            && has_explorer_service_plan
            && has_faucet_service_plan
            && has_telemetry_service_plan
            && has_public_service_content_plan
            && self.has_exact_ready_service_plans()
            && self.has_distinct_ready_service_endpoint_ids();
        let local_shape_ready = has_required_miners
            && has_required_validators
            && has_positive_stakes
            && has_funded_faucet
            && required_blocks > 0;
        let deployment_plan_ready = has_cuda_ready_miners
            && has_libp2p_ready_nodes
            && has_production_libp2p_runtime
            && has_public_service_plan;
        PublicTestnetPreflightReport {
            miner_count: self.config.miner_count,
            validator_count: self.config.validator_count,
            required_blocks,
            has_required_miners,
            has_required_validators,
            has_positive_stakes,
            has_funded_faucet,
            has_cuda_kernels_available: self.cuda_kernels_available,
            cuda_ready_miner_count: self.cuda_ready_miner_count,
            has_cuda_ready_miners,
            libp2p_ready_node_count: self.libp2p_ready_node_count,
            has_libp2p_ready_nodes,
            has_production_libp2p_runtime,
            has_rpc_service_plan,
            has_explorer_service_plan,
            has_faucet_service_plan,
            has_telemetry_service_plan,
            has_public_service_content_plan,
            has_public_service_plan,
            local_shape_ready,
            deployment_plan_ready,
            can_start_public_run: local_shape_ready && deployment_plan_ready,
        }
    }

    fn has_ready_service_plan(&self, kind: PublicServiceKind) -> bool {
        self.services
            .iter()
            .any(|service| service.kind == kind && service.is_ready_for_public_run())
    }

    fn has_ready_service_content_plan(&self, kind: PublicServiceKind) -> bool {
        self.services
            .iter()
            .any(|service| service.kind == kind && service.has_public_content_surface())
    }

    fn has_distinct_ready_service_endpoint_ids(&self) -> bool {
        let mut endpoint_ids = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(service) = self
                .services
                .iter()
                .find(|service| service.kind == kind && service.is_ready_for_public_run())
            else {
                return false;
            };
            if !endpoint_ids.insert(service.endpoint_id) {
                return false;
            }
        }
        true
    }

    fn has_exact_ready_service_plans(&self) -> bool {
        self.services.len() == public_service_kinds().len()
            && public_service_kinds().iter().all(|kind| {
                self.services
                    .iter()
                    .filter(|service| service.kind == *kind && service.is_ready_for_public_run())
                    .count()
                    == 1
            })
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicServiceKind {
    Rpc,
    Explorer,
    Faucet,
    Telemetry,
}

impl PublicServiceKind {
    fn evidence_tag(self) -> &'static [u8] {
        match self {
            Self::Rpc => b"rpc",
            Self::Explorer => b"explorer",
            Self::Faucet => b"faucet",
            Self::Telemetry => b"telemetry",
        }
    }

    fn content_path(self) -> &'static str {
        match self {
            Self::Rpc => "/chain/head",
            Self::Explorer => "/explorer",
            Self::Faucet => "/faucet/page",
            Self::Telemetry => "/telemetry/dashboard",
        }
    }
}

fn public_service_kinds() -> [PublicServiceKind; 4] {
    [
        PublicServiceKind::Rpc,
        PublicServiceKind::Explorer,
        PublicServiceKind::Faucet,
        PublicServiceKind::Telemetry,
    ]
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceEndpoint {
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
}

impl PublicServiceEndpoint {
    pub fn new(
        endpoint_id: Hash,
        public_url: impl Into<String>,
        health_path: impl Into<String>,
    ) -> Self {
        Self {
            endpoint_id,
            public_url: public_url.into(),
            health_path: health_path.into(),
        }
    }

    fn has_external_health_url(&self) -> bool {
        public_https_host(&self.public_url).is_some_and(public_host_is_external)
            && self.health_path.starts_with('/')
            && self.health_path.len() > 1
            && public_https_path(&self.public_url) == Some(self.health_path.as_str())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicNetworkRuntimeEvidence {
    pub libp2p_runtime_used: bool,
    pub peer_discovery_observed: bool,
    pub gossip_propagation_observed: bool,
    pub request_response_observed: bool,
    pub dos_controls_enabled: bool,
}

impl PublicNetworkRuntimeEvidence {
    pub fn has_production_libp2p_runtime(&self) -> bool {
        self.libp2p_runtime_used
            && self.peer_discovery_observed
            && self.gossip_propagation_observed
            && self.request_response_observed
            && self.dos_controls_enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicNetworkRuntimeObservation {
    pub operator_id: Hash,
    pub peer_id: String,
    pub listen_address: String,
    pub observed_at_unix_seconds: u64,
    pub gossip_topic_count: u64,
    pub request_response_protocol_count: u64,
    pub bootstrap_peer_count: u64,
    pub max_transmit_bytes: u64,
    pub request_timeout_seconds: u64,
    pub max_concurrent_streams: u64,
    pub idle_connection_timeout_seconds: u64,
    pub record_root: Hash,
    pub observation_signature: Signature,
}

impl PublicNetworkRuntimeObservation {
    fn new(details: PublicNetworkRuntimeObservationDetails) -> Self {
        let record_root = public_network_runtime_observation_root(&details);
        let observation_signature =
            public_network_runtime_observation_signature(&details.operator_id, &record_root);
        Self {
            operator_id: details.operator_id,
            peer_id: details.peer_id,
            listen_address: details.listen_address,
            observed_at_unix_seconds: details.observed_at_unix_seconds,
            gossip_topic_count: details.gossip_topic_count,
            request_response_protocol_count: details.request_response_protocol_count,
            bootstrap_peer_count: details.bootstrap_peer_count,
            max_transmit_bytes: details.max_transmit_bytes,
            request_timeout_seconds: details.request_timeout_seconds,
            max_concurrent_streams: details.max_concurrent_streams,
            idle_connection_timeout_seconds: details.idle_connection_timeout_seconds,
            record_root,
            observation_signature,
        }
    }

    fn details(&self) -> PublicNetworkRuntimeObservationDetails {
        PublicNetworkRuntimeObservationDetails {
            operator_id: self.operator_id,
            peer_id: self.peer_id.clone(),
            listen_address: self.listen_address.clone(),
            observed_at_unix_seconds: self.observed_at_unix_seconds,
            gossip_topic_count: self.gossip_topic_count,
            request_response_protocol_count: self.request_response_protocol_count,
            bootstrap_peer_count: self.bootstrap_peer_count,
            max_transmit_bytes: self.max_transmit_bytes,
            request_timeout_seconds: self.request_timeout_seconds,
            max_concurrent_streams: self.max_concurrent_streams,
            idle_connection_timeout_seconds: self.idle_connection_timeout_seconds,
        }
    }

    fn has_public_network_observation_proof(&self) -> bool {
        let details = self.details();
        self.operator_id != [0; 32]
            && self.record_root != [0; 32]
            && self.observed_at_unix_seconds > 0
            && self.gossip_topic_count > 0
            && self.request_response_protocol_count > 0
            && self.bootstrap_peer_count > 0
            && self.max_transmit_bytes > 0
            && self.request_timeout_seconds > 0
            && self.max_concurrent_streams > 0
            && self.idle_connection_timeout_seconds > 0
            && self.peer_id.parse::<PeerId>().is_ok()
            && self
                .listen_address
                .parse::<Multiaddr>()
                .is_ok_and(|address| public_network_runtime_multiaddr_is_external(&address))
            && self.record_root == public_network_runtime_observation_root(&details)
            && self.observation_signature
                == public_network_runtime_observation_signature(
                    &self.operator_id,
                    &self.record_root,
                )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub health_path: String,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub reachable_observation_count: u64,
    pub signed_health_check_count: u64,
    pub health_check_signature: Signature,
}

impl PublicServiceEvidence {
    pub fn new(
        kind: PublicServiceKind,
        endpoint: PublicServiceEndpoint,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    ) -> Self {
        let message = public_service_health_message(
            kind,
            &endpoint,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        );
        let endpoint_id = endpoint.endpoint_id;
        Self {
            kind,
            endpoint_id,
            public_url: endpoint.public_url,
            health_path: endpoint.health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
            health_check_signature: sign(&endpoint_id, &message),
        }
    }

    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn signed_health_check_valid(&self) -> bool {
        verify_signature(
            &self.endpoint_id,
            &public_service_health_message(
                self.kind,
                &self.endpoint(),
                self.first_seen_block,
                self.last_seen_block,
                self.reachable_observation_count,
                self.signed_health_check_count,
            ),
            &self.health_check_signature,
        )
    }

    pub fn has_reachable_endpoint_proof(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.endpoint().has_external_health_url()
            && self.last_seen_block >= self.first_seen_block
            && self.reachable_observation_count > 0
            && self.signed_health_check_count > 0
            && self.reachable_observation_count <= self.signed_health_check_count
            && self.signed_health_check_valid()
    }

    pub fn is_reachable_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks)
            && self.has_reachable_endpoint_proof()
            && self.has_run_health_coverage(observed_blocks)
    }

    fn endpoint(&self) -> PublicServiceEndpoint {
        PublicServiceEndpoint {
            endpoint_id: self.endpoint_id,
            public_url: self.public_url.clone(),
            health_path: self.health_path.clone(),
        }
    }

    fn has_run_health_coverage(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.reachable_observation_count >= observed_blocks
                && self.signed_health_check_count >= observed_blocks)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicServiceContentEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub public_url: String,
    pub content_path: String,
    pub content_root: Hash,
    pub observed_at_unix_seconds: u64,
    pub min_content_bytes: u64,
    pub content_signature: Signature,
}

impl PublicServiceContentEvidence {
    pub fn new(
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: impl Into<String>,
        content_path: impl Into<String>,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    ) -> Self {
        let public_url = public_url.into();
        let content_path = content_path.into();
        let message = public_service_content_message(
            kind,
            &endpoint_id,
            &public_url,
            &content_path,
            &content_root,
            observed_at_unix_seconds,
            min_content_bytes,
        );
        Self {
            kind,
            endpoint_id,
            public_url,
            content_path,
            content_root,
            observed_at_unix_seconds,
            min_content_bytes,
            content_signature: sign(&endpoint_id, &message),
        }
    }

    pub fn content_signature_valid(&self) -> bool {
        verify_signature(
            &self.endpoint_id,
            &public_service_content_message(
                self.kind,
                &self.endpoint_id,
                &self.public_url,
                &self.content_path,
                &self.content_root,
                self.observed_at_unix_seconds,
                self.min_content_bytes,
            ),
            &self.content_signature,
        )
    }

    pub fn has_external_content_proof(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.content_root != [0; 32]
            && self.observed_at_unix_seconds > 0
            && self.min_content_bytes >= PUBLIC_SERVICE_MIN_CONTENT_BYTES
            && public_https_host(&self.public_url).is_some_and(public_host_is_external)
            && self.content_path == self.kind.content_path()
            && public_https_path(&self.public_url) == Some(self.kind.content_path())
            && self.content_signature_valid()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidencePublication {
    pub bundle_id: Hash,
    pub public_uri: String,
    pub manifest_signer: Address,
    pub manifest_signature: Signature,
    pub manifest_signature_count: u64,
    pub independent_auditor_count: u64,
}

impl PublicEvidencePublication {
    pub fn new(
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    ) -> Self {
        let message = public_evidence_manifest_message(
            &bundle_id,
            &public_uri,
            manifest_signature_count,
            independent_auditor_count,
        );
        Self {
            bundle_id,
            public_uri,
            manifest_signer,
            manifest_signature: sign(&manifest_signer, &message),
            manifest_signature_count,
            independent_auditor_count,
        }
    }

    pub fn manifest_signature_valid(&self) -> bool {
        verify_signature(
            &self.manifest_signer,
            &public_evidence_manifest_message(
                &self.bundle_id,
                &self.public_uri,
                self.manifest_signature_count,
                self.independent_auditor_count,
            ),
            &self.manifest_signature,
        )
    }

    pub fn is_published_and_independently_checkable(&self) -> bool {
        self.bundle_id != [0; 32]
            && public_evidence_uri_is_external(&self.public_uri)
            && self.manifest_signer != [0; 32]
            && self.manifest_signature_count == 1
            && self.manifest_signature_valid()
            && self.independent_auditor_count > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceAuditorRecord {
    pub auditor_id: Address,
    pub audit_uri: String,
    pub observed_at_unix_seconds: u64,
    pub auditor_signature: Signature,
}

impl PublicEvidenceAuditorRecord {
    pub fn new(
        bundle_id: &Hash,
        public_uri: &str,
        auditor_id: Address,
        audit_uri: impl Into<String>,
        observed_at_unix_seconds: u64,
    ) -> Self {
        let audit_uri = audit_uri.into();
        let message = public_evidence_auditor_message(
            bundle_id,
            public_uri,
            &auditor_id,
            &audit_uri,
            observed_at_unix_seconds,
        );
        Self {
            auditor_id,
            audit_uri,
            observed_at_unix_seconds,
            auditor_signature: sign(&auditor_id, &message),
        }
    }

    pub fn auditor_signature_valid(&self, bundle_id: &Hash, public_uri: &str) -> bool {
        verify_signature(
            &self.auditor_id,
            &public_evidence_auditor_message(
                bundle_id,
                public_uri,
                &self.auditor_id,
                &self.audit_uri,
                self.observed_at_unix_seconds,
            ),
            &self.auditor_signature,
        )
    }

    pub fn has_external_auditor_proof(&self, bundle_id: &Hash, public_uri: &str) -> bool {
        self.auditor_id != [0; 32]
            && *bundle_id != [0; 32]
            && self.observed_at_unix_seconds > 0
            && public_evidence_uri_is_external(public_uri)
            && public_evidence_uri_is_external(&self.audit_uri)
            && self.auditor_signature_valid(bundle_id, public_uri)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidenceSupportingArtifact {
    pub kind: PublicEvidenceRecordKind,
    pub artifact_uri: String,
    pub record_root: Hash,
    pub record_count: u64,
    pub artifact_signature: Signature,
}

impl PublicEvidenceSupportingArtifact {
    pub fn new(
        bundle_id: &Hash,
        manifest_signer: &Address,
        kind: PublicEvidenceRecordKind,
        artifact_uri: impl Into<String>,
        record_root: Hash,
        record_count: u64,
    ) -> Self {
        let artifact_uri = artifact_uri.into();
        let message = public_evidence_artifact_message(
            bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            record_count,
        );
        Self {
            kind,
            artifact_uri,
            record_root,
            record_count,
            artifact_signature: sign(manifest_signer, &message),
        }
    }

    pub fn artifact_signature_valid(&self, bundle_id: &Hash, manifest_signer: &Address) -> bool {
        verify_signature(
            manifest_signer,
            &public_evidence_artifact_message(
                bundle_id,
                self.kind,
                &self.artifact_uri,
                &self.record_root,
                self.record_count,
            ),
            &self.artifact_signature,
        )
    }

    pub fn is_public_and_signed(&self, bundle_id: &Hash, manifest_signer: &Address) -> bool {
        self.record_root != [0; 32]
            && self.record_count > 0
            && public_evidence_uri_is_external(&self.artifact_uri)
            && self.artifact_signature_valid(bundle_id, manifest_signer)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PublicNodeRole {
    Miner,
    Validator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicNodeEvidence {
    pub address: Address,
    pub operator_id: Hash,
    pub role: PublicNodeRole,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub signed_heartbeat_count: u64,
    pub heartbeat_signature: Signature,
}

impl PublicNodeEvidence {
    pub fn miner(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Miner,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn validator(
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        Self::new(
            address,
            operator_id,
            PublicNodeRole::Validator,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        )
    }

    pub fn covers_run(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0
            || (self.first_seen_block == 0
                && self.last_seen_block.saturating_add(1) >= observed_blocks)
    }

    pub fn has_external_operator_proof(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && self.last_seen_block >= self.first_seen_block
            && self.signed_heartbeat_count > 0
            && self.heartbeat_signature_valid()
    }

    pub fn is_live_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks)
            && self.has_external_operator_proof()
            && self.has_run_heartbeat_coverage(observed_blocks)
    }

    pub fn heartbeat_signature_valid(&self) -> bool {
        verify_signature(
            &self.address,
            &public_node_heartbeat_message(
                &self.address,
                &self.operator_id,
                self.role,
                self.first_seen_block,
                self.last_seen_block,
                self.signed_heartbeat_count,
            ),
            &self.heartbeat_signature,
        )
    }

    fn new(
        address: Address,
        operator_id: Hash,
        role: PublicNodeRole,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    ) -> Self {
        let message = public_node_heartbeat_message(
            &address,
            &operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        );
        Self {
            address,
            operator_id,
            role,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
            heartbeat_signature: sign(&address, &message),
        }
    }

    fn has_run_heartbeat_coverage(&self, observed_blocks: u64) -> bool {
        observed_blocks == 0 || self.signed_heartbeat_count >= observed_blocks
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicOperatorIdentityAttestation {
    pub role: PublicNodeRole,
    pub address: Address,
    pub operator_id: Hash,
    pub identity_uri: String,
    pub observed_at_unix_seconds: u64,
    pub operator_signature: Signature,
}

impl PublicOperatorIdentityAttestation {
    pub fn new(
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: impl Into<String>,
        observed_at_unix_seconds: u64,
    ) -> Self {
        let identity_uri = identity_uri.into();
        let message = public_operator_identity_message(
            role,
            &address,
            &operator_id,
            &identity_uri,
            observed_at_unix_seconds,
        );
        Self {
            role,
            address,
            operator_id,
            identity_uri,
            observed_at_unix_seconds,
            operator_signature: sign(&operator_id, &message),
        }
    }

    pub fn operator_signature_valid(&self) -> bool {
        verify_signature(
            &self.operator_id,
            &public_operator_identity_message(
                self.role,
                &self.address,
                &self.operator_id,
                &self.identity_uri,
                self.observed_at_unix_seconds,
            ),
            &self.operator_signature,
        )
    }

    pub fn has_external_identity_proof(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && self.observed_at_unix_seconds > 0
            && public_evidence_uri_is_external(&self.identity_uri)
            && self.operator_signature_valid()
    }
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

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: Chain,
    pub faucet: Faucet,
    pub miners: Vec<Address>,
    pub validators: Vec<Address>,
    pub participant_endpoints: Vec<LocalParticipantEndpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalParticipantEndpoint {
    pub role: PublicNodeRole,
    pub address: Address,
    pub operator_id: Hash,
    pub node_endpoint: String,
}

impl LocalParticipantEndpoint {
    pub fn has_mandatory_libp2p_node_path(&self) -> bool {
        self.address != [0; 32]
            && self.operator_id != [0; 32]
            && local_libp2p_multiaddr_has_tcp_node_path(&self.node_endpoint)
    }
}

fn local_participant_tcp_port(base: u16, index: usize) -> u16 {
    base.saturating_add(u16::try_from(index).unwrap_or(u16::MAX.saturating_sub(base)))
}

fn local_libp2p_multiaddr_has_tcp_node_path(endpoint: &str) -> bool {
    let Ok(address) = endpoint.parse::<Multiaddr>() else {
        return false;
    };
    let mut has_node_address = false;
    let mut has_tcp_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(_)
            | Protocol::Ip6(_)
            | Protocol::Dns(_)
            | Protocol::Dns4(_)
            | Protocol::Dns6(_) => has_node_address = true,
            Protocol::Tcp(port) if port != 0 => has_tcp_port = true,
            Protocol::Tcp(_) => return false,
            _ => {}
        }
    }
    has_node_address && has_tcp_port
}

impl LocalTestnet {
    pub fn new(config: TestnetConfig, finalized_randomness: Hash) -> Self {
        Self::with_chain_params(config, ChainParams::default(), finalized_randomness)
    }

    pub fn from_profile(profile: &ChainProfile, finalized_randomness: Hash) -> Self {
        Self::with_chain_params(
            TestnetConfig::from_profile(profile),
            profile.chain_params.clone(),
            finalized_randomness,
        )
    }

    pub fn with_chain_params(
        config: TestnetConfig,
        params: ChainParams,
        finalized_randomness: Hash,
    ) -> Self {
        let mut chain = Chain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        let mut participant_endpoints =
            Vec::with_capacity(config.miner_count + config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain
                .apply_command(ChainCommand::RegisterMiner {
                    address: miner,
                    stake: config.miner_stake,
                })
                .unwrap();
            miners.push(miner);
            let index = (i as u64).to_le_bytes();
            participant_endpoints.push(LocalParticipantEndpoint {
                role: PublicNodeRole::Miner,
                address: miner,
                operator_id: hash_bytes(b"tensor-vm-local-operator-v1", &[b"miner", &index]),
                node_endpoint: format!(
                    "/ip4/127.0.0.1/tcp/{}",
                    local_participant_tcp_port(4_001, i)
                ),
            });
        }
        for i in 0..config.validator_count {
            let validator = address(format!("testnet-validator-{i}").as_bytes());
            chain
                .apply_command(ChainCommand::RegisterValidator {
                    address: validator,
                    stake: config.validator_stake,
                })
                .unwrap();
            validators.push(validator);
            let index = (i as u64).to_le_bytes();
            participant_endpoints.push(LocalParticipantEndpoint {
                role: PublicNodeRole::Validator,
                address: validator,
                operator_id: hash_bytes(b"tensor-vm-local-operator-v1", &[b"validator", &index]),
                node_endpoint: format!(
                    "/ip4/127.0.0.1/tcp/{}",
                    local_participant_tcp_port(5_001, i)
                ),
            });
        }
        Self {
            chain,
            faucet: Faucet::new(config.faucet_balance, config.faucet_drip),
            miners,
            validators,
            participant_endpoints,
        }
    }

    pub fn has_mandatory_libp2p_participant_paths(&self) -> bool {
        if self.participant_endpoints.len() != self.miners.len() + self.validators.len() {
            return false;
        }
        let mut node_endpoints = BTreeSet::new();
        let mut operator_ids = BTreeSet::new();
        let mut miner_endpoints = 0;
        let mut validator_endpoints = 0;
        for participant in &self.participant_endpoints {
            if !participant.has_mandatory_libp2p_node_path()
                || !node_endpoints.insert(participant.node_endpoint.clone())
                || !operator_ids.insert(participant.operator_id)
            {
                return false;
            }
            match participant.role {
                PublicNodeRole::Miner => miner_endpoints += 1,
                PublicNodeRole::Validator => validator_endpoints += 1,
            }
        }
        miner_endpoints == self.miners.len() && validator_endpoints == self.validators.len()
    }

    pub fn run_blocks(&mut self, count: u64) {
        for i in 0..count {
            let beacon = self.chain.state().finalized_randomness();
            let proposer = self
                .chain
                .proposer_for_next_epoch(&beacon)
                .or_else(|| self.validators.first().copied())
                .unwrap_or([0; 32]);
            let timestamp = i.saturating_mul(self.chain.params.block_time_seconds);
            let block = self.produce_block_with_command(proposer, timestamp);
            self.finalize_block(&block);
        }
    }

    pub fn run_matmul_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state().finalized_randomness();
        let job = scheduler.generate_small_matmul(
            self.chain.state().epoch(),
            self.chain.state().height(),
            &beacon,
            self.chain.state().height() + self.chain.params.receipt_submission_window,
        );
        let mut txpool = TxPool::default();
        self.chain
            .apply_command(ChainCommand::SubmitJob(JobState::TensorOp(job.clone())))
            .expect("generated tensor job should be accepted");
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, _a, _b, _c) = miner
                .solve_matmul_job(&job, self.chain.state().height(), 1 + index as u64)
                .expect("reference miner should solve generated job");
            assert!(txpool.submit(Transaction::SubmitTensorOpReceipt(receipt.receipt_id)));
            self.chain
                .apply_command(ChainCommand::SubmitReceipt(ReceiptState::TensorOp(
                    receipt.clone(),
                )))
                .expect("registered miner receipt should be accepted");
            receipts.push((receipt, miner.tensor_server.clone()));
        }

        self.attest_matmul_receipts(scheduler, &job, &receipts, &beacon, &mut txpool);

        self.chain
            .apply_command(ChainCommand::SettleEpoch {
                miner_reward_pool: 1_000,
                validator_reward_pool: 500,
            })
            .expect("verified receipts should settle");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.validators[0]);
        let block = self.produce_block_with_command(
            proposer,
            self.chain.state().height() * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn run_linear_training_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state().finalized_randomness();
        let model_id = hash_bytes(b"tensor-vm-testnet-model-v1", &[&beacon]);
        let architecture = hash_bytes(b"tensor-vm-testnet-architecture-v1", &[]);
        let config = hash_bytes(b"tensor-vm-testnet-config-v1", &[]);
        let weights = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static weights should be valid");
        self.chain
            .apply_command(ChainCommand::RegisterModel {
                model_id,
                architecture_hash: architecture,
                weight_root: weights.commitment_root(),
                config_hash: config,
            })
            .expect("testnet linear model should be registered");
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-testnet-batch-v1", &[&beacon]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: self.chain.state().height()
                + self.chain.params.receipt_submission_window,
        });
        let mut txpool = TxPool::default();
        self.chain
            .apply_command(ChainCommand::SubmitJob(JobState::LinearTrainingStep(
                job.clone(),
            )))
            .expect("generated linear training job should be accepted");
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, output) = miner
                .solve_linear_training_step(
                    &job,
                    &weights,
                    self.chain.state().height(),
                    1 + index as u64,
                )
                .expect("reference miner should solve generated training step");
            assert!(txpool.submit(Transaction::SubmitLinearTrainingStepReceipt(
                receipt.receipt_id
            )));
            self.chain
                .apply_command(ChainCommand::SubmitReceipt(
                    ReceiptState::LinearTrainingStep(receipt.clone()),
                ))
                .expect("registered miner linear receipt should be accepted");
            receipts.push((receipt, output));
        }

        for (receipt, output) in &receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, &beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state()
                    .validators()
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_linear_training_step(
                        &job,
                        receipt,
                        &weights,
                        output,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated training step");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .apply_command(ChainCommand::SubmitAttestation(attestation))
                    .expect("registered validator attestation should be accepted");
            }
        }

        let canonical_receipt = &receipts[0].0;
        assert!(
            self.chain
                .has_attestation_quorum(&canonical_receipt.receipt_id)
        );
        assert!(
            self.chain
                .has_redundant_agreement(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_command(ChainCommand::SettleEpoch {
                miner_reward_pool: 1_000,
                validator_reward_pool: 500,
            })
            .expect("verified linear receipts should settle");
        assert!(
            self.chain
                .state()
                .settled_receipts()
                .contains(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_command(ChainCommand::ApplyModelTransition {
                model_id,
                step: 0,
                weight_root_before: weights.commitment_root(),
                weight_root_after: canonical_receipt.weight_root_after,
            })
            .expect("verified training receipt should advance model state");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.validators[0]);
        let block = self.produce_block_with_command(
            proposer,
            self.chain.state().height() * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn expected_blocks_for_days(&self, days: u64) -> u64 {
        required_blocks_for_days(days, self.chain.params.block_time_seconds.max(1))
    }

    pub fn telemetry(&self) -> TelemetrySnapshot {
        TelemetrySnapshot::from_chain(&self.chain)
    }

    pub fn explorer_summary(&self) -> ExplorerSummary {
        let state = self.chain.state();
        ExplorerSummary {
            height: state.height(),
            epoch: state.epoch(),
            block_count: self.chain.blocks.len(),
            miner_count: state.miners().len(),
            validator_count: state.validators().len(),
            job_count: state.jobs().len(),
            model_count: state.model_states().len(),
            attestation_count: state.attestations().values().map(Vec::len).sum(),
            receipt_count: state.receipts().len(),
            settled_receipt_count: state.settled_receipts().len(),
            finalized_block_count: state.finalized_blocks().len(),
            treasury_balance: state.rewards().treasury(),
            total_reward_balance: state.rewards().total_balance(),
        }
    }

    pub fn public_testnet_evidence(
        &self,
        criteria: &PublicTestnetCriteria,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let telemetry = self.telemetry();
        let required_blocks = self.expected_blocks_for_days(criteria.duration_days);
        let required_duration_seconds = required_duration_seconds_for_days(criteria.duration_days);
        let observed_blocks = self.chain.blocks.len() as u64;
        let run_started_at_unix_seconds = self
            .chain
            .blocks
            .first()
            .map(|block| block.timestamp)
            .unwrap_or(0);
        let run_ended_at_unix_seconds = self
            .chain
            .blocks
            .last()
            .map(|block| {
                block
                    .timestamp
                    .saturating_add(self.chain.params.block_time_seconds)
            })
            .unwrap_or(run_started_at_unix_seconds);
        let observed_duration_seconds =
            run_ended_at_unix_seconds.saturating_sub(run_started_at_unix_seconds);
        let finality_rate_bps = ratio_to_bps(telemetry.block_finality_rate);
        let data_availability_bps = ratio_to_bps(telemetry.data_availability_rate);
        let invalid_receipts_submitted = telemetry.invalid_receipts_submitted as u64;
        let invalid_receipts_rejected =
            invalid_receipts_submitted.saturating_sub(telemetry.invalid_receipts_accepted);
        let invalid_work_rejection_rate_bps =
            ratio_parts_to_bps(invalid_receipts_rejected, invalid_receipts_submitted);
        let reward_settlement_records = telemetry.settled_receipt_count as u64;
        let has_required_miners = self.miners.len() >= criteria.min_miners;
        let has_required_validators = self.validators.len() >= criteria.min_validators;
        let has_required_run_duration = observed_duration_seconds >= required_duration_seconds;
        let has_required_block_count = observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && invalid_receipts_rejected <= invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            reward_settlement_records >= criteria.min_reward_settlement_records;
        let has_production_libp2p_runtime = false;
        let has_deployed_rpc_service = false;
        let has_deployed_explorer_service = false;
        let has_deployed_faucet_service = false;
        let has_deployed_telemetry_service = false;
        let has_deployed_public_service_content = false;
        let has_deployed_public_services = false;
        let public_criterion_met = false;
        PublicTestnetEvidence {
            miner_count: self.miners.len(),
            validator_count: self.validators.len(),
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_duration_seconds,
            required_duration_seconds,
            observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted,
            invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records,
            external_operator_evidence,
            has_production_libp2p_runtime,
            has_deployed_rpc_service,
            has_deployed_explorer_service,
            has_deployed_faucet_service,
            has_deployed_telemetry_service,
            has_deployed_public_service_content,
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_run_duration,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            public_criterion_met,
        }
    }

    fn attest_matmul_receipts(
        &mut self,
        scheduler: &JobScheduler,
        job: &crate::jobs::MatmulJob,
        receipts: &[(crate::jobs::TensorOpReceipt, TensorServer)],
        beacon: &Hash,
        txpool: &mut TxPool,
    ) {
        for (receipt, tensor_server) in receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state()
                    .validators()
                    .get(&validator_address)
                    .map(|validator| validator.stake)
                    .unwrap_or_default();
                let validator = ValidatorNode::new(validator_address, stake);
                let attestation = validator
                    .verify_matmul_from_server(
                        job,
                        receipt,
                        tensor_server,
                        &validation_seed,
                        &self.chain.params.freivalds,
                    )
                    .expect("reference validator should verify generated job");
                assert!(txpool.submit(Transaction::SubmitAttestation(attestation.receipt_id)));
                self.chain
                    .apply_command(ChainCommand::SubmitAttestation(attestation))
                    .expect("registered validator attestation should be accepted");
            }
        }
    }

    fn produce_block_with_command(&mut self, proposer: Address, timestamp: u64) -> TensorBlock {
        self.chain
            .apply_command(ChainCommand::ProduceBlock {
                proposer,
                timestamp,
            })
            .expect("registered validator should produce a useful-verification block");
        self.chain
            .blocks()
            .last()
            .cloned()
            .expect("producing a block should append to the chain")
    }

    fn finalize_block(&mut self, block: &TensorBlock) {
        for validator in self.validators.clone() {
            let stake = self
                .chain
                .state()
                .validators()
                .get(&validator)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            self.chain
                .apply_command(ChainCommand::SubmitBlockVote(BlockVote::new(
                    validator, stake, block,
                )))
                .expect("registered validator vote should finalize local block");
            if self.chain.is_block_finalized(&block.hash()) {
                break;
            }
        }
    }
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
    mod network_runtime;
    mod preflight_manifest;
    mod run_evidence;
    mod run_services;

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

    fn manifest_hash(domain: &[u8], label: &[u8]) -> String {
        hex(&hash_bytes(domain, &[label]))
    }

    fn manifest_address(label: &[u8]) -> String {
        hex(&address(label))
    }

    fn manifest_publication_signature() -> String {
        let publication = PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        );
        hex(&publication.manifest_signature)
    }

    fn manifest_auditor_uri() -> String {
        format!(
            "https://auditors.tensorvm.net/{}/0",
            manifest_hash(b"test", b"public-evidence-bundle")
        )
    }

    fn manifest_auditor_signature() -> String {
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let record = PublicEvidenceAuditorRecord::new(
            &bundle_id,
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            1_700_000_060,
        );
        hex(&record.auditor_signature)
    }

    fn manifest_bundle() -> PublicTestnetEvidenceBundle {
        complete_public_evidence_bundle()
    }

    fn manifest_node_signature(
        role: PublicNodeRole,
        address_label: &[u8],
        operator_label: &[u8],
    ) -> String {
        let node_address = address(address_label);
        let operator_id = hash_bytes(b"test", &[operator_label]);
        let node = match role {
            PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
            PublicNodeRole::Validator => {
                PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
            }
        };
        hex(&node.heartbeat_signature)
    }

    fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
        format!("https://operators.tensorvm.net/{}", hex(operator_id))
    }

    fn manifest_operator_signature(
        role: PublicNodeRole,
        address_label: &[u8],
        operator_label: &[u8],
    ) -> String {
        let node_address = address(address_label);
        let operator_id = hash_bytes(b"test", &[operator_label]);
        let attestation = PublicOperatorIdentityAttestation::new(
            role,
            node_address,
            operator_id,
            manifest_operator_identity_uri(&operator_id),
            1_700_000_000,
        );
        hex(&attestation.operator_signature)
    }

    fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
        hex(&public_service(kind, label, 0, 9).health_check_signature)
    }

    fn manifest_artifact_line(
        kind: PublicEvidenceRecordKind,
        root_label: &[u8],
        record_count: u64,
    ) -> String {
        manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
    }

    fn manifest_artifact_line_for_root(
        kind: PublicEvidenceRecordKind,
        record_root: Hash,
        record_count: u64,
    ) -> String {
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let artifact_uri = public_evidence_supporting_artifact_uri(&bundle_id, kind);
        let signature = sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            record_count,
        );
        format!(
            "record_artifact={},{},{},{},{}",
            kind.manifest_tag(),
            artifact_uri,
            hex(&record_root),
            record_count,
            hex(&signature)
        )
    }

    fn manifest_network_observation_lines() -> String {
        public_network_runtime_observations_for_run(&complete_public_run_evidence())
            .iter()
            .map(|observation| {
                format!(
                    "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                    hex(&observation.operator_id),
                    observation.peer_id,
                    observation.listen_address,
                    observation.observed_at_unix_seconds,
                    observation.gossip_topic_count,
                    observation.request_response_protocol_count,
                    observation.bootstrap_peer_count,
                    observation.max_transmit_bytes,
                    observation.request_timeout_seconds,
                    observation.max_concurrent_streams,
                    observation.idle_connection_timeout_seconds,
                    hex(&observation.record_root),
                    hex(&observation.observation_signature)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn resign_record_summary_and_artifact(
        bundle: &mut PublicTestnetEvidenceBundle,
        kind: PublicEvidenceRecordKind,
        record_root: Hash,
        record_count: u64,
    ) {
        let bundle_id = bundle.publication.bundle_id;
        let signer = bundle.publication.manifest_signer;
        let summary_signature =
            sign_public_evidence_record(&signer, &bundle_id, kind, &record_root, record_count);
        match kind {
            PublicEvidenceRecordKind::BlockHistory => {
                bundle.block_history_records = record_count;
                bundle.block_history_root = record_root;
                bundle.block_history_signature = summary_signature;
            }
            PublicEvidenceRecordKind::FinalityHistory => {
                bundle.finality_history_records = record_count;
                bundle.finality_history_root = record_root;
                bundle.finality_history_signature = summary_signature;
            }
            PublicEvidenceRecordKind::NetworkRuntimeObservations => {
                bundle.network_runtime_observation_records = record_count;
                bundle.network_runtime_observation_root = record_root;
                bundle.network_runtime_observation_signature = summary_signature;
            }
            PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
                bundle.data_availability_measurement_records = record_count;
                bundle.data_availability_measurement_root = record_root;
                bundle.data_availability_measurement_signature = summary_signature;
            }
            PublicEvidenceRecordKind::InvalidWorkRejections => {
                bundle.invalid_work_rejection_records = record_count;
                bundle.invalid_work_rejection_root = record_root;
                bundle.invalid_work_rejection_signature = summary_signature;
            }
            PublicEvidenceRecordKind::RewardSettlements => {
                bundle.reward_settlement_root = record_root;
                bundle.reward_settlement_signature = summary_signature;
            }
        }
        if let Some(artifact) = bundle
            .supporting_artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == kind)
        {
            artifact.record_root = record_root;
            artifact.record_count = record_count;
            let artifact_uri = artifact.artifact_uri.clone();
            artifact.artifact_signature = sign_public_evidence_artifact(
                &signer,
                &bundle_id,
                kind,
                &artifact_uri,
                &record_root,
                record_count,
            );
        }
    }

    fn complete_public_evidence_manifest_text() -> String {
        format!(
            "\
# TensorVM external public evidence manifest
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}

bundle_id=0x{}
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
auditor={},{},1700000060,{}
{}
{}
{}
{}
{}
{}
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
operator=miner,{},{},{},1700000000,{}
operator=miner,{},{},{},1700000000,{}
operator=validator,{},{},{},1700000000,{}
{}
network_runtime_observation_records=3
network_runtime_observation_root={}
network_runtime_observation_signature={}
data_availability_measurement_records=20
data_availability_measurement_root={}
data_availability_measurement_signature={}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
run_window_signature={}
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root={}
invalid_work_rejection_signature={}
reward_settlement_records=1
reward_settlement_root={}
reward_settlement_signature={}
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.net/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.net/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.net/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,0,9,10,10,{}
{}
{}
{}
{}
",
            manifest_hash(b"test", b"public-evidence-bundle"),
            manifest_address(b"public-evidence-publisher"),
            manifest_publication_signature(),
            manifest_address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            manifest_auditor_signature(),
            manifest_artifact_line(
                PublicEvidenceRecordKind::BlockHistory,
                b"block-history-root",
                10
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::FinalityHistory,
                b"finality-history-root",
                10
            ),
            manifest_artifact_line_for_root(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                manifest_bundle().network_runtime_observation_root,
                3
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                b"data-availability-root",
                20
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::InvalidWorkRejections,
                b"invalid-work-root",
                1
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::RewardSettlements,
                b"reward-settlement-root",
                1
            ),
            manifest_hash(b"test", b"block-history-root"),
            hex(&manifest_bundle().block_history_signature),
            manifest_hash(b"test", b"finality-history-root"),
            hex(&manifest_bundle().finality_history_signature),
            manifest_address(b"miner-a"),
            manifest_hash(b"test", b"miner-a-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            manifest_address(b"miner-b"),
            manifest_hash(b"test", b"miner-b-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
            manifest_address(b"validator-a"),
            manifest_hash(b"test", b"validator-a-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
            manifest_operator_signature(
                PublicNodeRole::Validator,
                b"validator-a",
                b"validator-a-operator"
            ),
            manifest_network_observation_lines(),
            hex(&manifest_bundle().network_runtime_observation_root),
            hex(&manifest_bundle().network_runtime_observation_signature),
            manifest_hash(b"test", b"data-availability-root"),
            hex(&manifest_bundle().data_availability_measurement_signature),
            hex(&manifest_bundle().run_window_signature),
            manifest_hash(b"test", b"invalid-work-root"),
            hex(&manifest_bundle().invalid_work_rejection_signature),
            manifest_hash(b"test", b"reward-settlement-root"),
            hex(&manifest_bundle().reward_settlement_signature),
            manifest_address(b"miner-a"),
            manifest_hash(b"test", b"miner-a-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            manifest_address(b"miner-b"),
            manifest_hash(b"test", b"miner-b-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
            manifest_address(b"validator-a"),
            manifest_hash(b"test", b"validator-a-operator"),
            manifest_node_signature(
                PublicNodeRole::Validator,
                b"validator-a",
                b"validator-a-operator"
            ),
            manifest_hash(b"test", b"rpc-service"),
            manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
            manifest_hash(b"test", b"explorer-service"),
            manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
            manifest_hash(b"test", b"faucet-service"),
            manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
            manifest_hash(b"test", b"telemetry-service"),
            manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
            manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
            manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
            manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
            manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
        )
    }

    fn complete_public_preflight_manifest_text() -> String {
        format!(
            "\
# TensorVM public testnet launch preflight manifest
version={PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION}
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,{},https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,{},https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
",
            manifest_hash(b"test", b"rpc-service"),
            manifest_hash(b"test", b"explorer-service"),
            manifest_hash(b"test", b"faucet-service"),
            manifest_hash(b"test", b"telemetry-service"),
        )
    }

    fn manifest_without_line(manifest: &str, prefix: &str) -> String {
        manifest
            .lines()
            .filter(|line| !line.starts_with(prefix))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
