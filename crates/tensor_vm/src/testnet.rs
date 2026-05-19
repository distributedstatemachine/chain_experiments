use crate::chain::{BlockVote, ChainParams, JobState, LocalChain, TensorBlock, Transaction};
use crate::error::{Result, TvmError};
use crate::explorer::ExplorerSummary;
use crate::faucet::Faucet;
use crate::hash::hex;
use crate::jobs::{LinearTrainingStepJob, LinearTrainingStepSpec};
use crate::miner::MinerNode;
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
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct PublicNetworkRuntimeObservationDetails {
    operator_id: Hash,
    peer_id: String,
    listen_address: String,
    observed_at_unix_seconds: u64,
    gossip_topic_count: u64,
    request_response_protocol_count: u64,
    bootstrap_peer_count: u64,
    max_transmit_bytes: u64,
    request_timeout_seconds: u64,
    max_concurrent_streams: u64,
    idle_connection_timeout_seconds: u64,
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

pub fn parse_public_testnet_evidence_manifest(input: &str) -> Result<PublicTestnetEvidenceBundle> {
    let mut builder = PublicEvidenceManifestBuilder::default();
    let mut scalar_fields = BTreeSet::new();
    for raw_line in input.lines() {
        let line = raw_line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = raw_line
            .split_once('=')
            .ok_or(TvmError::InvalidReceipt("malformed evidence manifest line"))?;
        reject_manifest_key_whitespace(key)?;
        let key = key.trim();
        if !public_evidence_manifest_field_allows_repeated(key)
            && !scalar_fields.insert(key.to_owned())
        {
            return Err(TvmError::InvalidReceipt(
                "duplicate evidence manifest field",
            ));
        }
        builder.set(key, value)?;
    }
    builder.finish()
}

pub fn parse_public_testnet_preflight_manifest(input: &str) -> Result<PublicTestnetPreflightPlan> {
    let mut builder = PublicTestnetPreflightManifestBuilder::default();
    let mut scalar_fields = BTreeSet::new();
    for raw_line in input.lines() {
        let line = raw_line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = raw_line.split_once('=').ok_or(TvmError::InvalidReceipt(
            "malformed preflight manifest line",
        ))?;
        reject_manifest_key_whitespace(key)?;
        let key = key.trim();
        if key != "service" && !scalar_fields.insert(key.to_owned()) {
            return Err(TvmError::InvalidReceipt(
                "duplicate preflight manifest field",
            ));
        }
        builder.set(key, value)?;
    }
    builder.finish()
}

fn reject_manifest_key_whitespace(key: &str) -> Result<()> {
    if key.trim() != key {
        return Err(TvmError::InvalidReceipt("malformed manifest field key"));
    }
    Ok(())
}

fn public_evidence_manifest_field_allows_repeated(key: &str) -> bool {
    matches!(
        key,
        "auditor"
            | "record_artifact"
            | "operator"
            | "network_runtime_observation"
            | "node"
            | "service"
            | "service_content"
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

    fn matches_node(&self, node: &PublicNodeEvidence) -> bool {
        self.role == node.role
            && self.address == node.address
            && self.operator_id == node.operator_id
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

#[derive(Default)]
struct PublicEvidenceManifestBuilder {
    version_seen: bool,
    bundle_id: Option<Hash>,
    public_uri: Option<String>,
    manifest_signer: Option<Address>,
    manifest_signature: Option<Signature>,
    manifest_signature_count: Option<u64>,
    independent_auditor_count: Option<u64>,
    auditor_records: Vec<PublicEvidenceAuditorRecord>,
    supporting_artifacts: Vec<PublicEvidenceSupportingArtifact>,
    block_history_records: Option<u64>,
    block_history_root: Option<Hash>,
    block_history_signature: Option<Signature>,
    finality_history_records: Option<u64>,
    finality_history_root: Option<Hash>,
    finality_history_signature: Option<Signature>,
    operator_identity_attestation_records: Option<u64>,
    operator_identity_attestations: Vec<PublicOperatorIdentityAttestation>,
    network_runtime_observations: Vec<PublicNetworkRuntimeObservation>,
    network_runtime_observation_records: Option<u64>,
    network_runtime_observation_root: Option<Hash>,
    network_runtime_observation_signature: Option<Signature>,
    data_availability_measurement_records: Option<u64>,
    data_availability_measurement_root: Option<Hash>,
    data_availability_measurement_signature: Option<Signature>,
    invalid_work_rejection_records: Option<u64>,
    invalid_work_rejection_root: Option<Hash>,
    invalid_work_rejection_signature: Option<Signature>,
    reward_settlement_root: Option<Hash>,
    reward_settlement_signature: Option<Signature>,
    run_started_at_unix_seconds: Option<u64>,
    run_ended_at_unix_seconds: Option<u64>,
    run_window_signature: Option<Signature>,
    libp2p_runtime_used: Option<bool>,
    peer_discovery_observed: Option<bool>,
    gossip_propagation_observed: Option<bool>,
    request_response_observed: Option<bool>,
    dos_controls_enabled: Option<bool>,
    nodes: Vec<PublicNodeEvidence>,
    services: Vec<PublicServiceEvidence>,
    service_content: Vec<PublicServiceContentEvidence>,
    observed_blocks: Option<u64>,
    finalized_blocks: Option<u64>,
    checked_receipts: Option<u64>,
    available_receipts: Option<u64>,
    invalid_receipts_submitted: Option<u64>,
    invalid_receipts_rejected: Option<u64>,
    reward_settlement_records: Option<u64>,
}

#[derive(Default)]
struct PublicTestnetPreflightManifestBuilder {
    version_seen: bool,
    miner_count: Option<usize>,
    validator_count: Option<usize>,
    miner_stake: Option<u64>,
    validator_stake: Option<u64>,
    faucet_balance: Option<u64>,
    faucet_drip: Option<u64>,
    cuda_kernels_available: Option<bool>,
    cuda_ready_miner_count: Option<usize>,
    libp2p_ready_node_count: Option<usize>,
    libp2p_runtime_used: Option<bool>,
    peer_discovery_observed: Option<bool>,
    gossip_propagation_observed: Option<bool>,
    request_response_observed: Option<bool>,
    dos_controls_enabled: Option<bool>,
    services: Vec<PublicDeploymentServicePlan>,
}

impl PublicTestnetPreflightManifestBuilder {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let scalar = value.trim();
        match key {
            "version" => {
                if scalar != PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION {
                    return Err(TvmError::InvalidReceipt(
                        "unsupported preflight manifest version",
                    ));
                }
                self.version_seen = true;
            }
            "miner_count" => self.miner_count = Some(parse_manifest_usize(scalar)?),
            "validator_count" => self.validator_count = Some(parse_manifest_usize(scalar)?),
            "miner_stake" => self.miner_stake = Some(parse_manifest_u64(scalar)?),
            "validator_stake" => self.validator_stake = Some(parse_manifest_u64(scalar)?),
            "faucet_balance" => self.faucet_balance = Some(parse_manifest_u64(scalar)?),
            "faucet_drip" => self.faucet_drip = Some(parse_manifest_u64(scalar)?),
            "cuda_kernels_available" => {
                self.cuda_kernels_available = Some(parse_manifest_bool(scalar)?);
            }
            "cuda_ready_miner_count" => {
                self.cuda_ready_miner_count = Some(parse_manifest_usize(scalar)?);
            }
            "libp2p_ready_node_count" => {
                self.libp2p_ready_node_count = Some(parse_manifest_usize(scalar)?);
            }
            "libp2p_runtime_used" => self.libp2p_runtime_used = Some(parse_manifest_bool(scalar)?),
            "peer_discovery_observed" => {
                self.peer_discovery_observed = Some(parse_manifest_bool(scalar)?);
            }
            "gossip_propagation_observed" => {
                self.gossip_propagation_observed = Some(parse_manifest_bool(scalar)?);
            }
            "request_response_observed" => {
                self.request_response_observed = Some(parse_manifest_bool(scalar)?);
            }
            "dos_controls_enabled" => {
                self.dos_controls_enabled = Some(parse_manifest_bool(scalar)?)
            }
            "service" => self.services.push(parse_preflight_service_plan(value)?),
            _ => return Err(TvmError::InvalidReceipt("unknown preflight manifest field")),
        }
        Ok(())
    }

    fn finish(self) -> Result<PublicTestnetPreflightPlan> {
        if !self.version_seen {
            return Err(TvmError::InvalidReceipt(
                "missing preflight manifest version",
            ));
        }
        Ok(PublicTestnetPreflightPlan {
            config: TestnetConfig {
                miner_count: required_usize(self.miner_count)?,
                validator_count: required_usize(self.validator_count)?,
                miner_stake: required_u64(self.miner_stake)?,
                validator_stake: required_u64(self.validator_stake)?,
                faucet_balance: required_u64(self.faucet_balance)?,
                faucet_drip: required_u64(self.faucet_drip)?,
            },
            criteria: PublicTestnetCriteria::default(),
            cuda_kernels_available: required_bool(self.cuda_kernels_available)?,
            cuda_ready_miner_count: required_usize(self.cuda_ready_miner_count)?,
            libp2p_ready_node_count: required_usize(self.libp2p_ready_node_count)?,
            network_runtime: PublicNetworkRuntimeEvidence {
                libp2p_runtime_used: required_bool(self.libp2p_runtime_used)?,
                peer_discovery_observed: required_bool(self.peer_discovery_observed)?,
                gossip_propagation_observed: required_bool(self.gossip_propagation_observed)?,
                request_response_observed: required_bool(self.request_response_observed)?,
                dos_controls_enabled: required_bool(self.dos_controls_enabled)?,
            },
            services: self.services,
        })
    }
}

fn parse_preflight_service_plan(value: &str) -> Result<PublicDeploymentServicePlan> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 8 {
        return Err(TvmError::InvalidReceipt("malformed preflight service plan"));
    }
    Ok(PublicDeploymentServicePlan {
        kind: parse_service_kind(fields[0].trim())?,
        endpoint_id: parse_hash_hex(fields[1].trim())?,
        public_url: fields[2].to_owned(),
        health_path: fields[3].to_owned(),
        content_url: fields[4].to_owned(),
        content_path: fields[5].to_owned(),
        auth_enabled: parse_manifest_bool(fields[6].trim())?,
        rate_limit_enabled: parse_manifest_bool(fields[7].trim())?,
    })
}

impl PublicEvidenceManifestBuilder {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let scalar = value.trim();
        match key {
            "version" => {
                if scalar != PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION {
                    return Err(TvmError::InvalidReceipt(
                        "unsupported evidence manifest version",
                    ));
                }
                self.version_seen = true;
            }
            "bundle_id" => self.bundle_id = Some(parse_hash_hex(scalar)?),
            "public_uri" => self.public_uri = Some(value.to_owned()),
            "manifest_signer" => self.manifest_signer = Some(parse_hash_hex(scalar)?),
            "manifest_signature" => self.manifest_signature = Some(parse_hash_hex(scalar)?),
            "manifest_signature_count" => {
                self.manifest_signature_count = Some(parse_manifest_u64(scalar)?);
            }
            "independent_auditor_count" => {
                self.independent_auditor_count = Some(parse_manifest_u64(scalar)?);
            }
            "auditor" => self
                .auditor_records
                .push(parse_manifest_auditor_record(value)?),
            "record_artifact" => self
                .supporting_artifacts
                .push(parse_manifest_supporting_artifact(value)?),
            "block_history_records" => {
                self.block_history_records = Some(parse_manifest_u64(scalar)?);
            }
            "block_history_root" => self.block_history_root = Some(parse_hash_hex(scalar)?),
            "block_history_signature" => {
                self.block_history_signature = Some(parse_hash_hex(scalar)?);
            }
            "finality_history_records" => {
                self.finality_history_records = Some(parse_manifest_u64(scalar)?);
            }
            "finality_history_root" => self.finality_history_root = Some(parse_hash_hex(scalar)?),
            "finality_history_signature" => {
                self.finality_history_signature = Some(parse_hash_hex(scalar)?);
            }
            "operator_identity_attestation_records" => {
                self.operator_identity_attestation_records = Some(parse_manifest_u64(scalar)?);
            }
            "operator" => self
                .operator_identity_attestations
                .push(parse_manifest_operator_identity_attestation(value)?),
            "network_runtime_observation" => self
                .network_runtime_observations
                .push(parse_manifest_network_runtime_observation(value)?),
            "network_runtime_observation_records" => {
                self.network_runtime_observation_records = Some(parse_manifest_u64(scalar)?);
            }
            "network_runtime_observation_root" => {
                self.network_runtime_observation_root = Some(parse_hash_hex(scalar)?);
            }
            "network_runtime_observation_signature" => {
                self.network_runtime_observation_signature = Some(parse_hash_hex(scalar)?);
            }
            "data_availability_measurement_records" => {
                self.data_availability_measurement_records = Some(parse_manifest_u64(scalar)?);
            }
            "data_availability_measurement_root" => {
                self.data_availability_measurement_root = Some(parse_hash_hex(scalar)?);
            }
            "data_availability_measurement_signature" => {
                self.data_availability_measurement_signature = Some(parse_hash_hex(scalar)?);
            }
            "invalid_work_rejection_records" => {
                self.invalid_work_rejection_records = Some(parse_manifest_u64(scalar)?);
            }
            "invalid_work_rejection_root" => {
                self.invalid_work_rejection_root = Some(parse_hash_hex(scalar)?);
            }
            "invalid_work_rejection_signature" => {
                self.invalid_work_rejection_signature = Some(parse_hash_hex(scalar)?);
            }
            "reward_settlement_root" => self.reward_settlement_root = Some(parse_hash_hex(scalar)?),
            "reward_settlement_signature" => {
                self.reward_settlement_signature = Some(parse_hash_hex(scalar)?);
            }
            "run_started_at_unix_seconds" => {
                self.run_started_at_unix_seconds = Some(parse_manifest_u64(scalar)?);
            }
            "run_ended_at_unix_seconds" => {
                self.run_ended_at_unix_seconds = Some(parse_manifest_u64(scalar)?);
            }
            "run_window_signature" => self.run_window_signature = Some(parse_hash_hex(scalar)?),
            "libp2p_runtime_used" => self.libp2p_runtime_used = Some(parse_manifest_bool(scalar)?),
            "peer_discovery_observed" => {
                self.peer_discovery_observed = Some(parse_manifest_bool(scalar)?);
            }
            "gossip_propagation_observed" => {
                self.gossip_propagation_observed = Some(parse_manifest_bool(scalar)?);
            }
            "request_response_observed" => {
                self.request_response_observed = Some(parse_manifest_bool(scalar)?);
            }
            "dos_controls_enabled" => {
                self.dos_controls_enabled = Some(parse_manifest_bool(scalar)?)
            }
            "node" => self.nodes.push(parse_manifest_node(value)?),
            "service" => self.services.push(parse_manifest_service(value)?),
            "service_content" => self
                .service_content
                .push(parse_manifest_service_content(value)?),
            "observed_blocks" => self.observed_blocks = Some(parse_manifest_u64(scalar)?),
            "finalized_blocks" => self.finalized_blocks = Some(parse_manifest_u64(scalar)?),
            "checked_receipts" => self.checked_receipts = Some(parse_manifest_u64(scalar)?),
            "available_receipts" => self.available_receipts = Some(parse_manifest_u64(scalar)?),
            "invalid_receipts_submitted" => {
                self.invalid_receipts_submitted = Some(parse_manifest_u64(scalar)?);
            }
            "invalid_receipts_rejected" => {
                self.invalid_receipts_rejected = Some(parse_manifest_u64(scalar)?);
            }
            "reward_settlement_records" => {
                self.reward_settlement_records = Some(parse_manifest_u64(scalar)?);
            }
            _ => return Err(TvmError::InvalidReceipt("unknown evidence manifest field")),
        }
        Ok(())
    }

    fn finish(self) -> Result<PublicTestnetEvidenceBundle> {
        if !self.version_seen {
            return Err(TvmError::InvalidReceipt(
                "missing evidence manifest version",
            ));
        }
        Ok(PublicTestnetEvidenceBundle {
            run: PublicTestnetRunEvidence {
                nodes: self.nodes,
                network_runtime: PublicNetworkRuntimeEvidence {
                    libp2p_runtime_used: required_bool(self.libp2p_runtime_used)?,
                    peer_discovery_observed: required_bool(self.peer_discovery_observed)?,
                    gossip_propagation_observed: required_bool(self.gossip_propagation_observed)?,
                    request_response_observed: required_bool(self.request_response_observed)?,
                    dos_controls_enabled: required_bool(self.dos_controls_enabled)?,
                },
                services: self.services,
                service_content: self.service_content,
                run_started_at_unix_seconds: required_u64(self.run_started_at_unix_seconds)?,
                run_ended_at_unix_seconds: required_u64(self.run_ended_at_unix_seconds)?,
                observed_blocks: required_u64(self.observed_blocks)?,
                finalized_blocks: required_u64(self.finalized_blocks)?,
                checked_receipts: required_u64(self.checked_receipts)?,
                available_receipts: required_u64(self.available_receipts)?,
                invalid_receipts_submitted: required_u64(self.invalid_receipts_submitted)?,
                invalid_receipts_rejected: required_u64(self.invalid_receipts_rejected)?,
                reward_settlement_records: required_u64(self.reward_settlement_records)?,
            },
            publication: {
                let mut publication = PublicEvidencePublication::new(
                    required_hash(self.bundle_id)?,
                    required_string(self.public_uri)?,
                    required_hash(self.manifest_signer)?,
                    required_u64(self.manifest_signature_count)?,
                    required_u64(self.independent_auditor_count)?,
                );
                publication.manifest_signature = required_hash(self.manifest_signature)?;
                publication
            },
            auditor_records: self.auditor_records,
            supporting_artifacts: self.supporting_artifacts,
            run_window_signature: required_hash(self.run_window_signature)?,
            block_history_records: required_u64(self.block_history_records)?,
            block_history_root: required_hash(self.block_history_root)?,
            block_history_signature: required_hash(self.block_history_signature)?,
            finality_history_records: required_u64(self.finality_history_records)?,
            finality_history_root: required_hash(self.finality_history_root)?,
            finality_history_signature: required_hash(self.finality_history_signature)?,
            operator_identity_attestation_records: required_u64(
                self.operator_identity_attestation_records,
            )?,
            operator_identity_attestations: self.operator_identity_attestations,
            network_runtime_observations: self.network_runtime_observations,
            network_runtime_observation_records: required_u64(
                self.network_runtime_observation_records,
            )?,
            network_runtime_observation_root: required_hash(self.network_runtime_observation_root)?,
            network_runtime_observation_signature: required_hash(
                self.network_runtime_observation_signature,
            )?,
            data_availability_measurement_records: required_u64(
                self.data_availability_measurement_records,
            )?,
            data_availability_measurement_root: required_hash(
                self.data_availability_measurement_root,
            )?,
            data_availability_measurement_signature: required_hash(
                self.data_availability_measurement_signature,
            )?,
            invalid_work_rejection_records: required_u64(self.invalid_work_rejection_records)?,
            invalid_work_rejection_root: required_hash(self.invalid_work_rejection_root)?,
            invalid_work_rejection_signature: required_hash(self.invalid_work_rejection_signature)?,
            reward_settlement_root: required_hash(self.reward_settlement_root)?,
            reward_settlement_signature: required_hash(self.reward_settlement_signature)?,
        })
    }
}

fn parse_manifest_supporting_artifact(value: &str) -> Result<PublicEvidenceSupportingArtifact> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 5 {
        return Err(TvmError::InvalidReceipt(
            "malformed supporting evidence artifact",
        ));
    }
    Ok(PublicEvidenceSupportingArtifact {
        kind: parse_public_evidence_record_kind_tag(fields[0].trim())?,
        artifact_uri: fields[1].to_owned(),
        record_root: parse_hash_hex(fields[2].trim())?,
        record_count: parse_manifest_u64(fields[3].trim())?,
        artifact_signature: parse_hash_hex(fields[4].trim())?,
    })
}

fn parse_manifest_node(value: &str) -> Result<PublicNodeEvidence> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() != 7 {
        return Err(TvmError::InvalidReceipt("malformed node evidence"));
    }
    let address = parse_hash_hex(fields[1])?;
    let operator_id = parse_hash_hex(fields[2])?;
    let first_seen_block = parse_manifest_u64(fields[3])?;
    let last_seen_block = parse_manifest_u64(fields[4])?;
    let signed_heartbeat_count = parse_manifest_u64(fields[5])?;
    let heartbeat_signature = parse_hash_hex(fields[6])?;
    let mut evidence = match fields[0] {
        "miner" => PublicNodeEvidence::miner(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        "validator" => PublicNodeEvidence::validator(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        _ => return Err(TvmError::InvalidReceipt("unknown node evidence role")),
    };
    evidence.heartbeat_signature = heartbeat_signature;
    Ok(evidence)
}

fn parse_manifest_operator_identity_attestation(
    value: &str,
) -> Result<PublicOperatorIdentityAttestation> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 6 {
        return Err(TvmError::InvalidReceipt(
            "malformed operator identity attestation",
        ));
    }
    let role = match fields[0].trim() {
        "miner" => PublicNodeRole::Miner,
        "validator" => PublicNodeRole::Validator,
        _ => {
            return Err(TvmError::InvalidReceipt(
                "unknown operator attestation role",
            ));
        }
    };
    let mut attestation = PublicOperatorIdentityAttestation::new(
        role,
        parse_hash_hex(fields[1].trim())?,
        parse_hash_hex(fields[2].trim())?,
        fields[3].to_owned(),
        parse_manifest_u64(fields[4].trim())?,
    );
    attestation.operator_signature = parse_hash_hex(fields[5].trim())?;
    Ok(attestation)
}

fn parse_manifest_network_runtime_observation(
    value: &str,
) -> Result<PublicNetworkRuntimeObservation> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 13 {
        return Err(TvmError::InvalidReceipt(
            "malformed network runtime observation",
        ));
    }
    let mut observation =
        PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
            operator_id: parse_hash_hex(fields[0].trim())?,
            peer_id: fields[1].to_owned(),
            listen_address: fields[2].to_owned(),
            observed_at_unix_seconds: parse_manifest_u64(fields[3].trim())?,
            gossip_topic_count: parse_manifest_u64(fields[4].trim())?,
            request_response_protocol_count: parse_manifest_u64(fields[5].trim())?,
            bootstrap_peer_count: parse_manifest_u64(fields[6].trim())?,
            max_transmit_bytes: parse_manifest_u64(fields[7].trim())?,
            request_timeout_seconds: parse_manifest_u64(fields[8].trim())?,
            max_concurrent_streams: parse_manifest_u64(fields[9].trim())?,
            idle_connection_timeout_seconds: parse_manifest_u64(fields[10].trim())?,
        });
    observation.record_root = parse_hash_hex(fields[11].trim())?;
    observation.observation_signature = parse_hash_hex(fields[12].trim())?;
    Ok(observation)
}

fn parse_manifest_auditor_record(value: &str) -> Result<PublicEvidenceAuditorRecord> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 4 {
        return Err(TvmError::InvalidReceipt("malformed auditor record"));
    }
    Ok(PublicEvidenceAuditorRecord {
        auditor_id: parse_hash_hex(fields[0].trim())?,
        audit_uri: fields[1].to_owned(),
        observed_at_unix_seconds: parse_manifest_u64(fields[2].trim())?,
        auditor_signature: parse_hash_hex(fields[3].trim())?,
    })
}

fn parse_manifest_service(value: &str) -> Result<PublicServiceEvidence> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 9 {
        return Err(TvmError::InvalidReceipt("malformed service evidence"));
    }
    let kind = parse_service_kind(fields[0].trim())?;
    let endpoint_id = parse_hash_hex(fields[1].trim())?;
    let public_url = fields[2].to_owned();
    let health_path = fields[3].to_owned();
    let first_seen_block = parse_manifest_u64(fields[4].trim())?;
    let last_seen_block = parse_manifest_u64(fields[5].trim())?;
    let reachable_observation_count = parse_manifest_u64(fields[6].trim())?;
    let signed_health_check_count = parse_manifest_u64(fields[7].trim())?;
    let mut evidence = PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(endpoint_id, public_url, health_path),
        first_seen_block,
        last_seen_block,
        reachable_observation_count,
        signed_health_check_count,
    );
    evidence.health_check_signature = parse_hash_hex(fields[8].trim())?;
    Ok(evidence)
}

fn parse_manifest_service_content(value: &str) -> Result<PublicServiceContentEvidence> {
    let fields: Vec<&str> = value.split(',').collect();
    if fields.len() != 8 {
        return Err(TvmError::InvalidReceipt(
            "malformed service content evidence",
        ));
    }
    let mut evidence = PublicServiceContentEvidence::new(
        parse_service_kind(fields[0].trim())?,
        parse_hash_hex(fields[1].trim())?,
        fields[2].to_owned(),
        fields[3].to_owned(),
        parse_hash_hex(fields[4].trim())?,
        parse_manifest_u64(fields[5].trim())?,
        parse_manifest_u64(fields[6].trim())?,
    );
    evidence.content_signature = parse_hash_hex(fields[7].trim())?;
    Ok(evidence)
}

fn parse_service_kind(value: &str) -> Result<PublicServiceKind> {
    match value {
        "rpc" => Ok(PublicServiceKind::Rpc),
        "explorer" => Ok(PublicServiceKind::Explorer),
        "faucet" => Ok(PublicServiceKind::Faucet),
        "telemetry" => Ok(PublicServiceKind::Telemetry),
        _ => Err(TvmError::InvalidReceipt("unknown service evidence kind")),
    }
}

fn parse_hash_hex(value: &str) -> Result<Hash> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid evidence hash length"));
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let high = parse_hex_nibble(value.as_bytes()[index * 2])?;
        let low = parse_hex_nibble(value.as_bytes()[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(out)
}

fn parse_hex_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid evidence hash hex")),
    }
}

fn parse_manifest_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid evidence manifest number"))
}

fn parse_manifest_usize(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid evidence manifest number"))
}

fn parse_manifest_bool(value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(TvmError::InvalidReceipt(
            "invalid evidence manifest boolean",
        )),
    }
}

fn required_u64(value: Option<u64>) -> Result<u64> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest number"))
}

fn required_usize(value: Option<usize>) -> Result<usize> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest number"))
}

fn required_bool(value: Option<bool>) -> Result<bool> {
    value.ok_or(TvmError::InvalidReceipt(
        "missing evidence manifest boolean",
    ))
}

fn required_hash(value: Option<Hash>) -> Result<Hash> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest hash"))
}

fn required_string(value: Option<String>) -> Result<String> {
    value.ok_or(TvmError::InvalidReceipt("missing evidence manifest string"))
}

fn public_https_host(url: &str) -> Option<&str> {
    let rest = public_https_url_rest(url)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    public_https_authority_host(&rest[..authority_end])
}

fn public_https_authority(url: &str) -> Option<(&str, Option<u16>)> {
    let rest = public_https_url_rest(url)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    public_https_authority_parts(&rest[..authority_end])
}

fn public_https_url_rest(url: &str) -> Option<&str> {
    if url
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return None;
    }
    url.strip_prefix("https://")
}

fn public_https_authority_host(authority: &str) -> Option<&str> {
    public_https_authority_parts(authority).map(|(host, _port)| host)
}

fn public_https_authority_parts(authority: &str) -> Option<(&str, Option<u16>)> {
    if authority.is_empty()
        || authority.contains('@')
        || authority.contains(['/', '?', '#', '\\'])
        || authority
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return None;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        let host = &bracketed[..end];
        let suffix = &bracketed[end + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            Some(parse_public_https_port(suffix.strip_prefix(':')?)?)
        };
        if host.is_empty() || host.parse::<Ipv6Addr>().is_err() {
            return None;
        }
        Some((host, port))
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(host, port)| (host, Some(port)));
        if host.is_empty()
            || host.contains(['[', ']', ':'])
            || port.is_some_and(|port| port.contains(':'))
        {
            return None;
        }
        let port = match port {
            Some(port) => Some(parse_public_https_port(port)?),
            None => None,
        };
        if host.parse::<Ipv4Addr>().is_err() && !public_dns_host_is_well_formed(host) {
            return None;
        }
        Some((host, port))
    }
}

fn parse_public_https_port(port: &str) -> Option<u16> {
    if port.is_empty() || port.len() > 5 || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    port.parse::<u16>().ok().filter(|parsed| *parsed != 0)
}

fn public_https_authorities_match(left: &str, right: &str) -> bool {
    let Some((left_host, left_port)) = public_https_authority(left) else {
        return false;
    };
    let Some((right_host, right_port)) = public_https_authority(right) else {
        return false;
    };
    public_authority_host_key(left_host) == public_authority_host_key(right_host)
        && left_port.unwrap_or(443) == right_port.unwrap_or(443)
}

fn public_authority_host_key(host: &str) -> String {
    match host.parse::<IpAddr>() {
        Ok(ip) => ip.to_string(),
        Err(_) => host.trim_end_matches('.').to_ascii_lowercase(),
    }
}

fn public_https_path(url: &str) -> Option<&str> {
    let rest = public_https_url_rest(url)?;
    let path_start = rest.find('/')?;
    let path = &rest[path_start..];
    if path.contains(['?', '#']) {
        return None;
    }
    (!path.is_empty()).then_some(path)
}

fn public_host_is_external(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let lowercase_host = host.to_ascii_lowercase();
    if lowercase_host == "localhost"
        || lowercase_host.ends_with(".local")
        || special_use_dns_name(&lowercase_host)
    {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => public_ipv4_is_external(ip),
        Ok(IpAddr::V6(ip)) => public_ipv6_is_external(ip),
        Err(_) => public_dns_host_is_well_formed(host),
    }
}

fn special_use_dns_name(host: &str) -> bool {
    host == "local"
        || host == "test"
        || host == "example"
        || host == "invalid"
        || host == "example.com"
        || host == "example.net"
        || host == "example.org"
        || host.ends_with(".example.com")
        || host.ends_with(".example.net")
        || host.ends_with(".example.org")
        || host.ends_with(".localhost")
        || host.ends_with(".test")
        || host.ends_with(".example")
        || host.ends_with(".invalid")
}

fn public_dns_host_is_well_formed(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let mut label_count = 0;
    let mut labels = host.split('.').peekable();
    while let Some(label) = labels.next() {
        label_count += 1;
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let bytes = label.as_bytes();
        if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
            return false;
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return false;
        }
        if labels.peek().is_none() && !bytes.iter().any(|byte| byte.is_ascii_alphabetic()) {
            return false;
        }
    }
    label_count >= 2
}

fn public_ipv4_is_external(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();
    let is_shared_address_space = a == 100 && (64..=127).contains(&b);
    let is_protocol_assignment = a == 192 && b == 0 && c == 0;
    let is_documentation = (a == 192 && b == 0 && c == 2)
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113);
    let is_benchmarking = a == 198 && (b == 18 || b == 19);
    let is_multicast = (224..=239).contains(&a);
    let is_reserved_or_broadcast = (240..=255).contains(&a);
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_private()
        || ip.is_link_local()
        || is_shared_address_space
        || is_protocol_assignment
        || is_documentation
        || is_benchmarking
        || is_multicast
        || is_reserved_or_broadcast)
}

fn public_ipv6_is_external(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.is_multicast()
        || is_documentation)
}

fn public_evidence_uri_is_external(uri: &str) -> bool {
    if let Some(host) = public_https_host(uri) {
        return public_host_is_external(host)
            && public_https_path(uri).is_some_and(|path| path.len() > 1);
    }
    content_addressed_uri_has_identifier(uri, "ipfs://")
        || content_addressed_uri_has_identifier(uri, "ar://")
}

fn public_evidence_manifest_message(
    bundle_id: &Hash,
    public_uri: &str,
    manifest_signature_count: u64,
    independent_auditor_count: u64,
) -> Hash {
    let signature_count = manifest_signature_count.to_le_bytes();
    let auditor_count = independent_auditor_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-manifest-v1",
        &[
            bundle_id,
            public_uri.as_bytes(),
            &signature_count,
            &auditor_count,
        ],
    )
}

fn public_evidence_auditor_message(
    bundle_id: &Hash,
    public_uri: &str,
    auditor_id: &Address,
    audit_uri: &str,
    observed_at_unix_seconds: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-auditor-v1",
        &[
            bundle_id,
            public_uri.as_bytes(),
            auditor_id,
            audit_uri.as_bytes(),
            &observed_at,
        ],
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicEvidenceRecordKind {
    BlockHistory,
    FinalityHistory,
    NetworkRuntimeObservations,
    DataAvailabilityMeasurements,
    InvalidWorkRejections,
    RewardSettlements,
}

impl PublicEvidenceRecordKind {
    fn tag(self) -> &'static [u8] {
        match self {
            Self::BlockHistory => b"block-history",
            Self::FinalityHistory => b"finality-history",
            Self::NetworkRuntimeObservations => b"network-runtime-observations",
            Self::DataAvailabilityMeasurements => b"data-availability-measurements",
            Self::InvalidWorkRejections => b"invalid-work-rejections",
            Self::RewardSettlements => b"reward-settlements",
        }
    }

    pub fn manifest_tag(self) -> &'static str {
        match self {
            Self::BlockHistory => "block-history",
            Self::FinalityHistory => "finality-history",
            Self::NetworkRuntimeObservations => "network-runtime",
            Self::DataAvailabilityMeasurements => "data-availability",
            Self::InvalidWorkRejections => "invalid-work",
            Self::RewardSettlements => "reward-settlement",
        }
    }
}

fn parse_public_evidence_record_kind_tag(value: &str) -> Result<PublicEvidenceRecordKind> {
    match value {
        "block-history" => Ok(PublicEvidenceRecordKind::BlockHistory),
        "finality-history" => Ok(PublicEvidenceRecordKind::FinalityHistory),
        "network-runtime" => Ok(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        "data-availability" => Ok(PublicEvidenceRecordKind::DataAvailabilityMeasurements),
        "invalid-work" => Ok(PublicEvidenceRecordKind::InvalidWorkRejections),
        "reward-settlement" => Ok(PublicEvidenceRecordKind::RewardSettlements),
        _ => Err(TvmError::InvalidReceipt(
            "invalid public evidence record kind",
        )),
    }
}

fn public_evidence_record_message(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    record_root: &Hash,
    record_count: u64,
) -> Hash {
    let count = record_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-record-v1",
        &[bundle_id, kind.tag(), record_root, &count],
    )
}

fn public_evidence_artifact_message(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    artifact_uri: &str,
    record_root: &Hash,
    record_count: u64,
) -> Hash {
    let count = record_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-evidence-artifact-v1",
        &[
            bundle_id,
            kind.tag(),
            artifact_uri.as_bytes(),
            record_root,
            &count,
        ],
    )
}

pub fn sign_public_evidence_record(
    signer: &Address,
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    record_root: &Hash,
    record_count: u64,
) -> Signature {
    sign(
        signer,
        &public_evidence_record_message(bundle_id, kind, record_root, record_count),
    )
}

pub fn sign_public_evidence_artifact(
    signer: &Address,
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
    artifact_uri: &str,
    record_root: &Hash,
    record_count: u64,
) -> Signature {
    sign(
        signer,
        &public_evidence_artifact_message(bundle_id, kind, artifact_uri, record_root, record_count),
    )
}

fn public_evidence_supporting_artifact_uri(
    bundle_id: &Hash,
    kind: PublicEvidenceRecordKind,
) -> String {
    format!(
        "https://evidence.tensorvm.net/{}/{}.json",
        hex(bundle_id),
        kind.manifest_tag()
    )
}

fn public_run_window_message(
    bundle_id: &Hash,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Hash {
    let started = run_started_at_unix_seconds.to_le_bytes();
    let ended = run_ended_at_unix_seconds.to_le_bytes();
    let blocks = observed_blocks.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-run-window-v1",
        &[bundle_id, &started, &ended, &blocks],
    )
}

pub fn sign_public_run_window(
    signer: &Address,
    bundle_id: &Hash,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Signature {
    sign(
        signer,
        &public_run_window_message(
            bundle_id,
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
        ),
    )
}

fn content_addressed_uri_has_identifier(uri: &str, scheme: &str) -> bool {
    if uri
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        || uri.contains(['?', '#', '\\'])
    {
        return false;
    }
    let Some(rest) = uri.strip_prefix(scheme) else {
        return false;
    };
    let (identifier, path) = rest
        .split_once('/')
        .map_or((rest, ""), |(identifier, path)| (identifier, path));
    content_addressed_identifier_is_well_formed(identifier)
        && (path.is_empty()
            || path
                .split('/')
                .all(content_addressed_path_segment_is_well_formed))
}

fn content_addressed_identifier_is_well_formed(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier != "."
        && identifier != ".."
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn content_addressed_path_segment_is_well_formed(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
}

fn public_node_role_tag(role: PublicNodeRole) -> &'static [u8] {
    match role {
        PublicNodeRole::Miner => b"miner",
        PublicNodeRole::Validator => b"validator",
    }
}

fn public_node_heartbeat_message(
    address: &Address,
    operator_id: &Hash,
    role: PublicNodeRole,
    first_seen_block: u64,
    last_seen_block: u64,
    signed_heartbeat_count: u64,
) -> Hash {
    let first_seen = first_seen_block.to_le_bytes();
    let last_seen = last_seen_block.to_le_bytes();
    let heartbeat_count = signed_heartbeat_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-node-heartbeat-v1",
        &[
            address,
            operator_id,
            public_node_role_tag(role),
            &first_seen,
            &last_seen,
            &heartbeat_count,
        ],
    )
}

fn public_operator_identity_message(
    role: PublicNodeRole,
    address: &Address,
    operator_id: &Hash,
    identity_uri: &str,
    observed_at_unix_seconds: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-operator-identity-v1",
        &[
            public_node_role_tag(role),
            address,
            operator_id,
            identity_uri.as_bytes(),
            &observed_at,
        ],
    )
}

fn public_service_health_message(
    kind: PublicServiceKind,
    endpoint: &PublicServiceEndpoint,
    first_seen_block: u64,
    last_seen_block: u64,
    reachable_observation_count: u64,
    signed_health_check_count: u64,
) -> Hash {
    let first_seen = first_seen_block.to_le_bytes();
    let last_seen = last_seen_block.to_le_bytes();
    let reachable_count = reachable_observation_count.to_le_bytes();
    let signed_count = signed_health_check_count.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-service-health-v1",
        &[
            kind.evidence_tag(),
            &endpoint.endpoint_id,
            endpoint.public_url.as_bytes(),
            endpoint.health_path.as_bytes(),
            &first_seen,
            &last_seen,
            &reachable_count,
            &signed_count,
        ],
    )
}

fn public_service_content_message(
    kind: PublicServiceKind,
    endpoint_id: &Hash,
    public_url: &str,
    content_path: &str,
    content_root: &Hash,
    observed_at_unix_seconds: u64,
    min_content_bytes: u64,
) -> Hash {
    let observed_at = observed_at_unix_seconds.to_le_bytes();
    let min_bytes = min_content_bytes.to_le_bytes();
    hash_bytes(
        b"tensor-vm-public-service-content-v1",
        &[
            kind.evidence_tag(),
            endpoint_id,
            public_url.as_bytes(),
            content_path.as_bytes(),
            content_root,
            &observed_at,
            &min_bytes,
        ],
    )
}

fn public_network_runtime_observation_root(
    details: &PublicNetworkRuntimeObservationDetails,
) -> Hash {
    let observed_at = details.observed_at_unix_seconds.to_le_bytes();
    let gossip_topics = details.gossip_topic_count.to_le_bytes();
    let request_response_protocols = details.request_response_protocol_count.to_le_bytes();
    let bootstrap_peers = details.bootstrap_peer_count.to_le_bytes();
    let max_transmit = details.max_transmit_bytes.to_le_bytes();
    let request_timeout = details.request_timeout_seconds.to_le_bytes();
    let max_streams = details.max_concurrent_streams.to_le_bytes();
    let idle_timeout = details.idle_connection_timeout_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-network-runtime-observation-v1",
        &[
            &details.operator_id,
            details.peer_id.as_bytes(),
            details.listen_address.as_bytes(),
            &observed_at,
            &gossip_topics,
            &request_response_protocols,
            &bootstrap_peers,
            &max_transmit,
            &request_timeout,
            &max_streams,
            &idle_timeout,
        ],
    )
}

fn public_network_runtime_observation_signature(
    operator_id: &Hash,
    record_root: &Hash,
) -> Signature {
    hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[operator_id, record_root],
    )
}

pub(crate) fn aggregate_public_evidence_record_roots(
    kind: PublicEvidenceRecordKind,
    record_roots: &[Hash],
) -> Result<Hash> {
    if record_roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record roots argument is empty"));
    }
    if record_roots.contains(&[0; 32]) {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    let mut unique_roots = BTreeSet::new();
    if record_roots.iter().any(|root| !unique_roots.insert(*root)) {
        return Err(TvmError::InvalidReceipt("duplicate record root argument"));
    }
    let record_count = (record_roots.len() as u64).to_le_bytes();
    let mut encoded_roots = Vec::with_capacity(record_roots.len() * 32);
    for root in record_roots {
        encoded_roots.extend_from_slice(root);
    }
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-record-root-aggregation-v1",
        &[
            kind.manifest_tag().as_bytes(),
            &record_count,
            &encoded_roots,
        ],
    ))
}

fn public_network_runtime_multiaddr_is_external(address: &Multiaddr) -> bool {
    let mut saw_public_address = false;
    let mut saw_tcp_listen_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(ip) => {
                if !public_host_is_external(&ip.to_string()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Ip6(ip) => {
                if !public_host_is_external(&ip.to_string()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Dns(host) | Protocol::Dns4(host) | Protocol::Dns6(host) => {
                if !public_host_is_external(host.as_ref()) {
                    return false;
                }
                saw_public_address = true;
            }
            Protocol::Tcp(port) if port != 0 => saw_tcp_listen_port = true,
            Protocol::Tcp(_) => return false,
            _ => {}
        }
    }
    saw_public_address && saw_tcp_listen_port
}

fn deterministic_public_network_peer_id(operator_id: &Hash) -> String {
    let seed = hash_bytes(
        b"tensor-vm-public-network-observation-peer-id-v1",
        &[operator_id],
    );
    let keypair = libp2p::identity::Keypair::ed25519_from_bytes(seed)
        .expect("hashed operator id should form an ed25519 secret key");
    PeerId::from(keypair.public()).to_string()
}

pub(crate) fn public_network_runtime_observations_for_run(
    run: &PublicTestnetRunEvidence,
) -> Vec<PublicNetworkRuntimeObservation> {
    run.nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            PublicNetworkRuntimeObservation::new(PublicNetworkRuntimeObservationDetails {
                operator_id: node.operator_id,
                peer_id: deterministic_public_network_peer_id(&node.operator_id),
                listen_address: format!("/dns/node-{index}.tensorvm.net/tcp/{}", 4_001 + index),
                observed_at_unix_seconds: run.run_started_at_unix_seconds,
                gossip_topic_count: 5,
                request_response_protocol_count: 3,
                bootstrap_peer_count: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            })
        })
        .collect()
}

impl PublicTestnetEvidenceBundle {
    pub fn new(
        run: PublicTestnetRunEvidence,
        publication: PublicEvidencePublication,
        record_summaries: PublicEvidenceRecordSummaries,
    ) -> Self {
        let signer = publication.manifest_signer;
        let bundle_id = publication.bundle_id;
        let public_uri = publication.public_uri.clone();
        let auditor_records = (0..publication.independent_auditor_count)
            .map(|index| {
                let auditor_label = format!("public-evidence-auditor-{index}");
                PublicEvidenceAuditorRecord::new(
                    &bundle_id,
                    &public_uri,
                    address(auditor_label.as_bytes()),
                    format!(
                        "https://auditors.tensorvm.net/{}/{}",
                        hex(&bundle_id),
                        index
                    ),
                    run.run_ended_at_unix_seconds,
                )
            })
            .collect();
        let operator_identity_attestations = run
            .nodes
            .iter()
            .map(|node| {
                PublicOperatorIdentityAttestation::new(
                    node.role,
                    node.address,
                    node.operator_id,
                    format!("https://operators.tensorvm.net/{}", hex(&node.operator_id)),
                    run.run_started_at_unix_seconds,
                )
            })
            .collect();
        let network_runtime_observations = public_network_runtime_observations_for_run(&run);
        let run_window_signature = sign_public_run_window(
            &signer,
            &bundle_id,
            run.run_started_at_unix_seconds,
            run.run_ended_at_unix_seconds,
            run.observed_blocks,
        );
        let reward_settlement_records = run.reward_settlement_records;
        let supporting_artifacts = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                record_summaries.block_history_root,
                record_summaries.block_history_records,
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                record_summaries.finality_history_root,
                record_summaries.finality_history_records,
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                record_summaries.network_runtime_observation_root,
                record_summaries.network_runtime_observation_records,
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                record_summaries.data_availability_measurement_root,
                record_summaries.data_availability_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                record_summaries.invalid_work_rejection_root,
                record_summaries.invalid_work_rejection_records,
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                record_summaries.reward_settlement_root,
                reward_settlement_records,
            ),
        ]
        .into_iter()
        .map(|(kind, record_root, record_count)| {
            PublicEvidenceSupportingArtifact::new(
                &bundle_id,
                &signer,
                kind,
                public_evidence_supporting_artifact_uri(&bundle_id, kind),
                record_root,
                record_count,
            )
        })
        .collect();
        Self {
            run,
            publication,
            auditor_records,
            supporting_artifacts,
            run_window_signature,
            block_history_records: record_summaries.block_history_records,
            block_history_root: record_summaries.block_history_root,
            block_history_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::BlockHistory,
                &record_summaries.block_history_root,
                record_summaries.block_history_records,
            ),
            finality_history_records: record_summaries.finality_history_records,
            finality_history_root: record_summaries.finality_history_root,
            finality_history_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::FinalityHistory,
                &record_summaries.finality_history_root,
                record_summaries.finality_history_records,
            ),
            operator_identity_attestation_records: record_summaries
                .operator_identity_attestation_records,
            operator_identity_attestations,
            network_runtime_observations,
            network_runtime_observation_records: record_summaries
                .network_runtime_observation_records,
            network_runtime_observation_root: record_summaries.network_runtime_observation_root,
            network_runtime_observation_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &record_summaries.network_runtime_observation_root,
                record_summaries.network_runtime_observation_records,
            ),
            data_availability_measurement_records: record_summaries
                .data_availability_measurement_records,
            data_availability_measurement_root: record_summaries.data_availability_measurement_root,
            data_availability_measurement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &record_summaries.data_availability_measurement_root,
                record_summaries.data_availability_measurement_records,
            ),
            invalid_work_rejection_records: record_summaries.invalid_work_rejection_records,
            invalid_work_rejection_root: record_summaries.invalid_work_rejection_root,
            invalid_work_rejection_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &record_summaries.invalid_work_rejection_root,
                record_summaries.invalid_work_rejection_records,
            ),
            reward_settlement_root: record_summaries.reward_settlement_root,
            reward_settlement_signature: sign_public_evidence_record(
                &signer,
                &bundle_id,
                PublicEvidenceRecordKind::RewardSettlements,
                &record_summaries.reward_settlement_root,
                reward_settlement_records,
            ),
        }
    }

    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
    ) -> PublicTestnetEvidenceBundleReport {
        let has_published_evidence_bundle =
            self.publication.is_published_and_independently_checkable();
        let valid_auditor_record_count = self.valid_auditor_record_count() as u64;
        let has_independent_auditor_records = self.publication.independent_auditor_count > 0
            && self.auditor_records.len() as u64 == self.publication.independent_auditor_count
            && valid_auditor_record_count == self.publication.independent_auditor_count;
        let has_signed_run_window = self.public_run_window_signature_valid();
        let has_block_history = self.run.observed_blocks > 0
            && self.block_history_records == self.run.observed_blocks
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::BlockHistory,
                &self.block_history_root,
                self.block_history_records,
                &self.block_history_signature,
            );
        let has_finality_history = self.run.observed_blocks > 0
            && self.finality_history_records == self.run.observed_blocks
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::FinalityHistory,
                &self.finality_history_root,
                self.finality_history_records,
                &self.finality_history_signature,
            );
        let (miner_count, validator_count) = self.run.independent_operator_counts();
        let required_operator_attestations = (miner_count + validator_count) as u64;
        let valid_operator_attestation_count =
            self.valid_operator_identity_attestation_count() as u64;
        let has_operator_identity_attestations = required_operator_attestations > 0
            && self.operator_identity_attestation_records >= required_operator_attestations
            && self.operator_identity_attestation_records <= valid_operator_attestation_count
            && valid_operator_attestation_count >= required_operator_attestations;
        let run_evidence = self.run.evaluate(
            criteria,
            block_time_seconds,
            has_operator_identity_attestations,
        );
        let required_network_runtime_observation_count = miner_count + validator_count;
        let required_network_runtime_observations =
            required_network_runtime_observation_count as u64;
        let has_network_runtime_observations =
            self.run.network_runtime.has_production_libp2p_runtime()
                && required_network_runtime_observations > 0
                && self.network_runtime_observation_records
                    == required_network_runtime_observations
                && self.has_network_runtime_observation_records_for_public_operators(
                    required_network_runtime_observation_count,
                )
                && self.public_record_signature_valid(
                    PublicEvidenceRecordKind::NetworkRuntimeObservations,
                    &self.network_runtime_observation_root,
                    self.network_runtime_observation_records,
                    &self.network_runtime_observation_signature,
                );
        let has_data_availability_measurements = self.run.checked_receipts > 0
            && self.data_availability_measurement_records == self.run.checked_receipts
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &self.data_availability_measurement_root,
                self.data_availability_measurement_records,
                &self.data_availability_measurement_signature,
            );
        let has_invalid_work_rejection_records = run_evidence.has_invalid_work_rejection_evidence
            && self.invalid_work_rejection_records == self.run.invalid_receipts_submitted
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &self.invalid_work_rejection_root,
                self.invalid_work_rejection_records,
                &self.invalid_work_rejection_signature,
            );
        let has_reward_settlement_record_summary = run_evidence.has_reward_settlement_records
            && self.public_record_signature_valid(
                PublicEvidenceRecordKind::RewardSettlements,
                &self.reward_settlement_root,
                self.run.reward_settlement_records,
                &self.reward_settlement_signature,
            );
        let required_supporting_artifacts = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                &self.block_history_root,
                self.block_history_records,
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                &self.finality_history_root,
                self.finality_history_records,
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &self.network_runtime_observation_root,
                self.network_runtime_observation_records,
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                &self.data_availability_measurement_root,
                self.data_availability_measurement_records,
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                &self.invalid_work_rejection_root,
                self.invalid_work_rejection_records,
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                &self.reward_settlement_root,
                self.run.reward_settlement_records,
            ),
        ];
        let has_public_supporting_record_artifacts = self.supporting_artifacts.len()
            == required_supporting_artifacts.len()
            && required_supporting_artifacts
                .iter()
                .all(|(kind, record_root, record_count)| {
                    self.has_exact_public_supporting_record_artifact(
                        *kind,
                        record_root,
                        *record_count,
                    )
                });
        let independently_checkable = has_published_evidence_bundle
            && has_independent_auditor_records
            && has_signed_run_window
            && has_block_history
            && has_finality_history
            && has_operator_identity_attestations
            && has_network_runtime_observations
            && has_data_availability_measurements
            && has_invalid_work_rejection_records
            && has_reward_settlement_record_summary
            && has_public_supporting_record_artifacts;
        let full_spec_evidence_met = public_testnet_criteria_are_full_spec(criteria)
            && run_evidence.public_criterion_met
            && independently_checkable;
        PublicTestnetEvidenceBundleReport {
            run_evidence,
            has_published_evidence_bundle,
            has_independent_auditor_records,
            has_signed_run_window,
            has_block_history,
            has_finality_history,
            has_operator_identity_attestations,
            has_network_runtime_observations,
            has_data_availability_measurements,
            has_invalid_work_rejection_records,
            has_reward_settlement_record_summary,
            has_public_supporting_record_artifacts,
            independently_checkable,
            full_spec_evidence_met,
        }
    }

    fn has_exact_public_supporting_record_artifact(
        &self,
        kind: PublicEvidenceRecordKind,
        record_root: &Hash,
        record_count: u64,
    ) -> bool {
        self.supporting_artifacts
            .iter()
            .filter(|artifact| {
                artifact.kind == kind
                    && artifact.record_root == *record_root
                    && artifact.record_count == record_count
                    && artifact.is_public_and_signed(
                        &self.publication.bundle_id,
                        &self.publication.manifest_signer,
                    )
            })
            .take(2)
            .count()
            == 1
    }

    fn public_record_signature_valid(
        &self,
        kind: PublicEvidenceRecordKind,
        record_root: &Hash,
        record_count: u64,
        signature: &Signature,
    ) -> bool {
        self.publication.manifest_signer != [0; 32]
            && self.publication.bundle_id != [0; 32]
            && *record_root != [0; 32]
            && verify_signature(
                &self.publication.manifest_signer,
                &public_evidence_record_message(
                    &self.publication.bundle_id,
                    kind,
                    record_root,
                    record_count,
                ),
                signature,
            )
    }

    fn public_run_window_signature_valid(&self) -> bool {
        self.publication.manifest_signer != [0; 32]
            && self.publication.bundle_id != [0; 32]
            && self.run.run_ended_at_unix_seconds >= self.run.run_started_at_unix_seconds
            && verify_signature(
                &self.publication.manifest_signer,
                &public_run_window_message(
                    &self.publication.bundle_id,
                    self.run.run_started_at_unix_seconds,
                    self.run.run_ended_at_unix_seconds,
                    self.run.observed_blocks,
                ),
                &self.run_window_signature,
            )
    }

    fn valid_auditor_record_count(&self) -> usize {
        let mut valid_auditors = BTreeSet::new();
        for auditor in &self.auditor_records {
            if auditor.auditor_id == self.publication.manifest_signer {
                continue;
            }
            if auditor.observed_at_unix_seconds < self.run.run_ended_at_unix_seconds {
                continue;
            }
            if auditor.has_external_auditor_proof(
                &self.publication.bundle_id,
                &self.publication.public_uri,
            ) {
                valid_auditors.insert(auditor.auditor_id);
            }
        }
        valid_auditors.len()
    }

    fn valid_operator_identity_attestation_count(&self) -> usize {
        let mut valid_attestations = BTreeSet::new();
        for attestation in &self.operator_identity_attestations {
            if !attestation.has_external_identity_proof() {
                continue;
            }
            if !self
                .run
                .observation_is_within_run(attestation.observed_at_unix_seconds)
            {
                continue;
            }
            let matches_public_node = self.run.nodes.iter().any(|node| {
                node.is_live_for_run(self.run.observed_blocks) && attestation.matches_node(node)
            });
            if matches_public_node {
                valid_attestations.insert(hash_bytes(
                    b"tensor-vm-public-operator-attestation-key-v1",
                    &[
                        public_node_role_tag(attestation.role),
                        &attestation.address,
                        &attestation.operator_id,
                    ],
                ));
            }
        }
        valid_attestations.len()
    }

    fn live_public_operator_ids(&self) -> BTreeSet<Hash> {
        let mut miner_operator_ids = BTreeSet::new();
        let mut validator_operator_ids = BTreeSet::new();
        for node in &self.run.nodes {
            if !node.is_live_for_run(self.run.observed_blocks) {
                continue;
            }
            match node.role {
                PublicNodeRole::Miner => {
                    miner_operator_ids.insert(node.operator_id);
                }
                PublicNodeRole::Validator => {
                    validator_operator_ids.insert(node.operator_id);
                }
            }
        }
        validator_operator_ids.retain(|operator_id| !miner_operator_ids.contains(operator_id));
        miner_operator_ids.extend(validator_operator_ids);
        miner_operator_ids
    }

    fn has_network_runtime_observation_records_for_public_operators(
        &self,
        required_count: usize,
    ) -> bool {
        if self.network_runtime_observations.len() != required_count {
            return false;
        }
        let expected_operator_ids = self.live_public_operator_ids();
        if expected_operator_ids.len() != required_count {
            return false;
        }
        let mut observed_operator_ids = BTreeSet::new();
        let mut record_roots = Vec::with_capacity(required_count);
        for observation in &self.network_runtime_observations {
            if !expected_operator_ids.contains(&observation.operator_id)
                || !self
                    .run
                    .observation_is_within_run(observation.observed_at_unix_seconds)
                || !observation.has_public_network_observation_proof()
                || !observed_operator_ids.insert(observation.operator_id)
            {
                return false;
            }
            record_roots.push(observation.record_root);
        }
        observed_operator_ids == expected_operator_ids
            && aggregate_public_evidence_record_roots(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &record_roots,
            )
            .is_ok_and(|record_root| record_root == self.network_runtime_observation_root)
    }
}

impl PublicTestnetRunEvidence {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidence {
        let (miner_count, validator_count) = self.independent_operator_counts();
        let required_blocks =
            required_blocks_for_days(criteria.duration_days, block_time_seconds.max(1));
        let required_duration_seconds = required_duration_seconds_for_days(criteria.duration_days);
        let has_valid_run_window =
            self.run_ended_at_unix_seconds >= self.run_started_at_unix_seconds;
        let observed_duration_seconds = if has_valid_run_window {
            self.run_ended_at_unix_seconds
                .saturating_sub(self.run_started_at_unix_seconds)
        } else {
            0
        };
        let finality_rate_bps = ratio_parts_to_bps(self.finalized_blocks, self.observed_blocks);
        let data_availability_bps =
            ratio_parts_to_bps(self.available_receipts, self.checked_receipts);
        let has_consistent_finality_counts = self.finalized_blocks <= self.observed_blocks;
        let has_consistent_data_availability_counts =
            self.available_receipts <= self.checked_receipts;
        let invalid_work_rejection_rate_bps = ratio_parts_to_bps(
            self.invalid_receipts_rejected,
            self.invalid_receipts_submitted,
        );
        let has_required_miners = miner_count >= criteria.min_miners;
        let has_required_validators = validator_count >= criteria.min_validators;
        let has_required_run_duration =
            has_valid_run_window && observed_duration_seconds >= required_duration_seconds;
        let has_required_block_count = self.observed_blocks >= required_blocks;
        let has_required_finality =
            has_consistent_finality_counts && finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability = has_consistent_data_availability_counts
            && data_availability_bps >= criteria.min_data_availability_bps;
        let has_invalid_work_rejection_evidence = self.invalid_receipts_submitted
            >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected >= criteria.min_invalid_work_rejections
            && self.invalid_receipts_rejected <= self.invalid_receipts_submitted
            && invalid_work_rejection_rate_bps == 10_000;
        let has_reward_settlement_records =
            self.reward_settlement_records >= criteria.min_reward_settlement_records;
        let external_operator_evidence =
            external_operator_evidence && miner_count > 0 && validator_count > 0;
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_rpc_content =
            self.has_service_content_for_reachable_endpoint(PublicServiceKind::Rpc);
        let has_explorer_content =
            self.has_service_content_for_reachable_endpoint(PublicServiceKind::Explorer);
        let has_faucet_content =
            self.has_service_content_for_reachable_endpoint(PublicServiceKind::Faucet);
        let has_telemetry_content =
            self.has_service_content_for_reachable_endpoint(PublicServiceKind::Telemetry);
        let has_distinct_deployed_service_endpoint_ids =
            self.has_distinct_deployed_service_endpoint_ids();
        let has_distinct_deployed_service_content_roots =
            self.has_distinct_deployed_service_content_roots();
        let has_deployed_public_service_content = has_rpc_content
            && has_explorer_content
            && has_faucet_content
            && has_telemetry_content
            && has_distinct_deployed_service_content_roots;
        let has_deployed_rpc_service = has_rpc_content;
        let has_deployed_explorer_service = has_explorer_content;
        let has_deployed_faucet_service = has_faucet_content;
        let has_deployed_telemetry_service = has_telemetry_content;
        let has_deployed_public_services = has_deployed_rpc_service
            && has_deployed_explorer_service
            && has_deployed_faucet_service
            && has_deployed_telemetry_service
            && has_deployed_public_service_content
            && has_distinct_deployed_service_endpoint_ids;
        let public_criterion_met = has_required_miners
            && has_required_validators
            && has_required_run_duration
            && has_required_block_count
            && has_required_finality
            && has_required_data_availability
            && has_invalid_work_rejection_evidence
            && has_reward_settlement_records
            && has_production_libp2p_runtime
            && has_deployed_public_services
            && external_operator_evidence;
        PublicTestnetEvidence {
            miner_count,
            validator_count,
            run_started_at_unix_seconds: self.run_started_at_unix_seconds,
            run_ended_at_unix_seconds: self.run_ended_at_unix_seconds,
            observed_duration_seconds,
            required_duration_seconds,
            observed_blocks: self.observed_blocks,
            required_blocks,
            finality_rate_bps,
            data_availability_bps,
            invalid_receipts_submitted: self.invalid_receipts_submitted,
            invalid_receipts_rejected: self.invalid_receipts_rejected,
            invalid_work_rejection_rate_bps,
            reward_settlement_records: self.reward_settlement_records,
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

    fn has_service_content_for_reachable_endpoint(&self, kind: PublicServiceKind) -> bool {
        self.deployed_service_content_root(kind).is_some()
    }

    fn deployed_service_content_root(&self, kind: PublicServiceKind) -> Option<Hash> {
        self.services
            .iter()
            .filter(|service| {
                service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
            })
            .find_map(|service| {
                self.service_content.iter().find_map(|content| {
                    let matches_content = content.kind == kind
                        && content.endpoint_id == service.endpoint_id
                        && content.has_external_content_proof()
                        && public_https_authorities_match(&service.public_url, &content.public_url)
                        && self.observation_is_within_run(content.observed_at_unix_seconds);
                    matches_content.then_some(content.content_root)
                })
            })
    }

    fn deployed_service_endpoint_id(&self, kind: PublicServiceKind) -> Option<Hash> {
        self.services
            .iter()
            .filter(|service| {
                service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
            })
            .find_map(|service| {
                self.service_content
                    .iter()
                    .any(|content| {
                        content.kind == kind
                            && content.endpoint_id == service.endpoint_id
                            && content.has_external_content_proof()
                            && public_https_authorities_match(
                                &service.public_url,
                                &content.public_url,
                            )
                            && self.observation_is_within_run(content.observed_at_unix_seconds)
                    })
                    .then_some(service.endpoint_id)
            })
    }

    fn has_distinct_deployed_service_endpoint_ids(&self) -> bool {
        let mut endpoint_ids = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(endpoint_id) = self.deployed_service_endpoint_id(kind) else {
                return false;
            };
            if !endpoint_ids.insert(endpoint_id) {
                return false;
            }
        }
        true
    }

    fn has_distinct_deployed_service_content_roots(&self) -> bool {
        let mut content_roots = BTreeSet::new();
        for kind in public_service_kinds() {
            let Some(content_root) = self.deployed_service_content_root(kind) else {
                return false;
            };
            if !content_roots.insert(content_root) {
                return false;
            }
        }
        true
    }

    fn observation_is_within_run(&self, observed_at_unix_seconds: u64) -> bool {
        self.run_started_at_unix_seconds <= observed_at_unix_seconds
            && observed_at_unix_seconds <= self.run_ended_at_unix_seconds
    }

    fn independent_operator_counts(&self) -> (usize, usize) {
        let mut miner_operator_ids = BTreeSet::new();
        let mut miner_addresses = BTreeSet::new();
        let mut validator_operator_ids = BTreeSet::new();
        let mut validator_addresses = BTreeSet::new();
        for node in &self.nodes {
            if !node.is_live_for_run(self.observed_blocks) {
                continue;
            }
            match node.role {
                PublicNodeRole::Miner => {
                    miner_operator_ids.insert(node.operator_id);
                    miner_addresses.insert(node.address);
                }
                PublicNodeRole::Validator => {
                    validator_operator_ids.insert(node.operator_id);
                    validator_addresses.insert(node.address);
                }
            }
        }
        validator_operator_ids.retain(|operator_id| !miner_operator_ids.contains(operator_id));
        validator_addresses.retain(|address| !miner_addresses.contains(address));
        (
            miner_operator_ids.len().min(miner_addresses.len()),
            validator_operator_ids.len().min(validator_addresses.len()),
        )
    }
}

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: LocalChain,
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
        let params = ChainParams::default();
        let mut chain = LocalChain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        let mut participant_endpoints =
            Vec::with_capacity(config.miner_count + config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain.register_miner(miner, config.miner_stake).unwrap();
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
                .register_validator(validator, config.validator_stake)
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
            let beacon = self.chain.state.finalized_randomness;
            let proposer = self
                .chain
                .proposer_for_next_epoch(&beacon)
                .or_else(|| self.miners.first().copied())
                .or_else(|| self.validators.first().copied())
                .unwrap_or([0; 32]);
            let timestamp = i.saturating_mul(self.chain.params.block_time_seconds);
            let block = self.chain.produce_block(proposer, timestamp);
            self.finalize_block(&block);
        }
    }

    pub fn run_matmul_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let job = scheduler.generate_small_matmul(
            self.chain.state.epoch,
            self.chain.state.height,
            &beacon,
            self.chain.state.height + self.chain.params.receipt_submission_window,
        );
        let mut txpool = TxPool::default();
        self.chain.submit_job(JobState::TensorOp(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, _a, _b, _c) = miner
                .solve_matmul_job(&job, self.chain.state.height, 1 + index as u64)
                .expect("reference miner should solve generated job");
            assert!(txpool.submit(Transaction::SubmitTensorOpReceipt(receipt.receipt_id)));
            self.chain
                .submit_tensor_op_receipt(receipt.clone())
                .expect("registered miner receipt should be accepted");
            receipts.push((receipt, miner.tensor_server.clone()));
        }

        self.attest_matmul_receipts(scheduler, &job, &receipts, &beacon, &mut txpool);

        self.chain.settle_epoch(1_000, 500);
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
        );
        self.finalize_block(&block);
    }

    pub fn run_linear_training_round(&mut self, scheduler: &JobScheduler) {
        let beacon = self.chain.state.finalized_randomness;
        let model_id = hash_bytes(b"tensor-vm-testnet-model-v1", &[&beacon]);
        let architecture = hash_bytes(b"tensor-vm-testnet-architecture-v1", &[]);
        let config = hash_bytes(b"tensor-vm-testnet-config-v1", &[]);
        let weights = Tensor::from_vec(vec![3, 2], DType::FieldElement, vec![1, 2, 3, 4, 5, 6])
            .expect("static weights should be valid");
        self.chain
            .register_model(model_id, architecture, weights.commitment_root(), config);
        let job = LinearTrainingStepJob::from_spec(LinearTrainingStepSpec {
            model_id,
            step: 0,
            batch_seed: hash_bytes(b"tensor-vm-testnet-batch-v1", &[&beacon]),
            weight_root_before: weights.commitment_root(),
            input_shape: vec![4, 3],
            weight_shape: vec![3, 2],
            target_shape: vec![4, 2],
            lr: 2,
            deadline_block: self.chain.state.height + self.chain.params.receipt_submission_window,
        });
        let mut txpool = TxPool::default();
        self.chain
            .submit_job(JobState::LinearTrainingStep(job.clone()));
        let miner_assignment = scheduler.assign_miners(&self.chain, job.job_id, &beacon);
        let mut receipts = Vec::new();
        for (index, miner_address) in miner_assignment.miners.iter().copied().enumerate() {
            let mut miner = MinerNode::new(miner_address, CpuReferenceBackend);
            let (receipt, output) = miner
                .solve_linear_training_step(
                    &job,
                    &weights,
                    self.chain.state.height,
                    1 + index as u64,
                )
                .expect("reference miner should solve generated training step");
            assert!(txpool.submit(Transaction::SubmitLinearTrainingStepReceipt(
                receipt.receipt_id
            )));
            self.chain
                .submit_linear_receipt(receipt.clone())
                .expect("registered miner linear receipt should be accepted");
            receipts.push((receipt, output));
        }

        for (receipt, output) in &receipts {
            let validation_seed = self.chain.validation_seed(&receipt.receipt_id);
            let assignment = scheduler.assign_validators(&self.chain, receipt.receipt_id, &beacon);
            for validator_address in assignment.validators {
                let stake = self
                    .chain
                    .state
                    .validators
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
                    .submit_attestation(attestation)
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
        self.chain.settle_epoch(1_000, 500);
        assert!(
            self.chain
                .state
                .settled_receipts
                .contains(&canonical_receipt.receipt_id)
        );
        self.chain
            .apply_model_transition(
                &model_id,
                0,
                &weights.commitment_root(),
                canonical_receipt.weight_root_after,
            )
            .expect("verified training receipt should advance model state");
        let proposer = self
            .chain
            .proposer_for_next_epoch(&beacon)
            .unwrap_or_else(|| self.miners[0]);
        let block = self.chain.produce_block(
            proposer,
            self.chain.state.height * self.chain.params.block_time_seconds,
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
        ExplorerSummary::from_chain(&self.chain)
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
                    .state
                    .validators
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
                    .submit_attestation(attestation)
                    .expect("registered validator attestation should be accepted");
            }
        }
    }

    fn finalize_block(&mut self, block: &TensorBlock) {
        for validator in self.validators.clone() {
            let stake = self
                .chain
                .state
                .validators
                .get(&validator)
                .map(|validator| validator.stake)
                .unwrap_or_default();
            self.chain
                .submit_block_vote(BlockVote::new(validator, stake, block))
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

    #[test]
    fn network_runtime_observation_helpers_reject_bad_roots_addresses_and_counts() {
        let root_a = hash_bytes(b"test", &[b"network-observation-a"]);
        assert!(
            aggregate_public_evidence_record_roots(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &[]
            )
            .is_err()
        );
        assert!(
            aggregate_public_evidence_record_roots(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &[[0; 32]]
            )
            .is_err()
        );
        assert!(
            aggregate_public_evidence_record_roots(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &[root_a, root_a]
            )
            .is_err()
        );

        let public_ipv4: Multiaddr = "/ip4/8.8.8.8/tcp/4001".parse().unwrap();
        let private_ipv4: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let public_ipv6: Multiaddr = "/ip6/2606:4700:4700::1111/tcp/4001".parse().unwrap();
        let local_ipv6: Multiaddr = "/ip6/::1/tcp/4001".parse().unwrap();
        let special_dns: Multiaddr = "/dns/example.test/tcp/4001".parse().unwrap();
        let zero_tcp_port: Multiaddr = "/ip4/8.8.8.8/tcp/0".parse().unwrap();
        let ignored_protocol: Multiaddr = "/ip4/8.8.8.8/udp/4001/tcp/4001".parse().unwrap();
        assert!(public_network_runtime_multiaddr_is_external(&public_ipv4));
        assert!(!public_network_runtime_multiaddr_is_external(&private_ipv4));
        assert!(public_network_runtime_multiaddr_is_external(&public_ipv6));
        assert!(!public_network_runtime_multiaddr_is_external(&local_ipv6));
        assert!(!public_network_runtime_multiaddr_is_external(&special_dns));
        assert!(!public_network_runtime_multiaddr_is_external(
            &zero_tcp_port
        ));
        assert!(public_network_runtime_multiaddr_is_external(
            &ignored_protocol
        ));

        let mut bundle = complete_public_evidence_bundle();
        bundle.run.nodes[0].signed_heartbeat_count = 0;
        assert!(!bundle.has_network_runtime_observation_records_for_public_operators(3));
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

    #[test]
    fn local_testnet_bootstraps_required_public_shape() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        assert_eq!(testnet.miners.len(), 10);
        assert_eq!(testnet.validators.len(), 5);
        assert_eq!(testnet.participant_endpoints.len(), 15);
        assert!(testnet.has_mandatory_libp2p_participant_paths());
        assert!(
            testnet
                .participant_endpoints
                .iter()
                .all(LocalParticipantEndpoint::has_mandatory_libp2p_node_path)
        );
        assert!(!local_libp2p_multiaddr_has_tcp_node_path("not-a-multiaddr"));
        assert!(!local_libp2p_multiaddr_has_tcp_node_path(
            "/ip4/127.0.0.1/tcp/0"
        ));
        let gate0_peer = libp2p::PeerId::random();
        assert!(local_libp2p_multiaddr_has_tcp_node_path(&format!(
            "/ip4/127.0.0.1/tcp/4001/p2p/{gate0_peer}"
        )));
        let mut missing_endpoint = testnet.clone();
        missing_endpoint.participant_endpoints.pop();
        assert!(!missing_endpoint.has_mandatory_libp2p_participant_paths());
        let mut duplicate_endpoint = testnet.clone();
        duplicate_endpoint.participant_endpoints[0].node_endpoint = duplicate_endpoint
            .participant_endpoints[1]
            .node_endpoint
            .clone();
        assert!(!duplicate_endpoint.has_mandatory_libp2p_participant_paths());
        let libp2p_service =
            crate::p2p::spawn_libp2p_service(crate::p2p::Libp2pControlPlaneConfig {
                listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".to_owned()],
                ..crate::p2p::Libp2pControlPlaneConfig::default()
            })
            .expect("Gate 0 must construct the mandatory libp2p control-plane runtime");
        assert_eq!(libp2p_service.info().subscribed_topics.len(), 5);
        assert_eq!(libp2p_service.info().request_response_protocols.len(), 3);
        assert!(
            libp2p_service
                .info()
                .identify_protocol
                .starts_with(crate::p2p::LIBP2P_PROTOCOL_PREFIX)
        );
        testnet.run_blocks(12);
        let summary = testnet.explorer_summary();
        assert_eq!(summary.block_count, 12);
        assert_eq!(testnet.expected_blocks_for_days(7), 100_800);
        assert_eq!(testnet.telemetry().block_finality_rate, 1.0);
        let public_evidence =
            testnet.public_testnet_evidence(&PublicTestnetCriteria::default(), false);
        assert_eq!(public_evidence.required_blocks, 100_800);
        assert!(public_evidence.has_required_miners);
        assert!(public_evidence.has_required_validators);
        assert!(!public_evidence.has_required_block_count);
        assert!(!public_evidence.external_operator_evidence);
        assert!(!public_evidence.has_production_libp2p_runtime);
        assert!(!public_evidence.has_deployed_public_service_content);
        assert!(!public_evidence.has_deployed_public_services);
        assert!(!public_evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_full_matmul_receipt_attestation_settlement_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_matmul_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert!(testnet.telemetry().total_tensor_work > 0);
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);

        let evidence = testnet.public_testnet_evidence(
            &PublicTestnetCriteria {
                duration_days: 0,
                min_finality_rate_bps: 10_000,
                min_data_availability_bps: 9_500,
                ..PublicTestnetCriteria::default()
            },
            true,
        );
        assert_eq!(evidence.observed_blocks, 1);
        assert_eq!(evidence.required_blocks, 0);
        assert_eq!(evidence.finality_rate_bps, 10_000);
        assert_eq!(evidence.data_availability_bps, 10_000);
        assert!(evidence.has_reward_settlement_records);
        assert!(!evidence.has_invalid_work_rejection_evidence);
        assert!(!evidence.public_criterion_met);

        let invalid_receipt_id = hash_bytes(b"test", &[b"public-invalid-receipt"]);
        let invalid_statement = crate::verify::AttestationStatement {
            receipt_id: invalid_receipt_id,
            job_id: hash_bytes(b"test", &[b"public-invalid-job"]),
            primitive_type: crate::jobs::PrimitiveType::TensorOp,
            result: crate::verify::VerificationResult::Invalid,
            checks_root: hash_bytes(b"test", &[b"public-invalid-checks"]),
            data_availability_passed: true,
        };
        let invalid_validator = testnet.validators[0];
        let invalid_stake = testnet
            .chain
            .state
            .validators
            .get(&invalid_validator)
            .unwrap()
            .stake;
        testnet
            .chain
            .state
            .attestations
            .entry(invalid_receipt_id)
            .or_default()
            .push(crate::verify::ValidatorAttestation::new(
                invalid_validator,
                invalid_stake,
                invalid_statement,
            ));

        let complete_local_evidence = testnet.public_testnet_evidence(
            &PublicTestnetCriteria {
                duration_days: 0,
                min_finality_rate_bps: 10_000,
                min_data_availability_bps: 9_500,
                ..PublicTestnetCriteria::default()
            },
            true,
        );
        assert_eq!(complete_local_evidence.invalid_receipts_submitted, 1);
        assert_eq!(complete_local_evidence.invalid_receipts_rejected, 1);
        assert_eq!(
            complete_local_evidence.invalid_work_rejection_rate_bps,
            10_000
        );
        assert!(complete_local_evidence.has_invalid_work_rejection_evidence);
        assert!(complete_local_evidence.has_reward_settlement_records);
        assert!(!complete_local_evidence.has_production_libp2p_runtime);
        assert!(!complete_local_evidence.has_deployed_rpc_service);
        assert!(!complete_local_evidence.has_deployed_explorer_service);
        assert!(!complete_local_evidence.has_deployed_faucet_service);
        assert!(!complete_local_evidence.has_deployed_telemetry_service);
        assert!(!complete_local_evidence.has_deployed_public_services);
        assert!(!complete_local_evidence.public_criterion_met);
    }

    #[test]
    fn local_testnet_runs_linear_training_receipt_state_transition_round() {
        let mut testnet =
            LocalTestnet::new(TestnetConfig::default(), hash_bytes(b"test", &[b"beacon"]));
        let scheduler = JobScheduler::with_small_shape((8, 8, 8));
        testnet.run_linear_training_round(&scheduler);

        assert_eq!(
            testnet.chain.state.receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(
            testnet.chain.state.settled_receipts.len(),
            testnet.chain.params.replication_factor
        );
        assert_eq!(testnet.chain.blocks.len(), 1);
        assert_eq!(testnet.chain.state.model_states.len(), 1);
        assert_eq!(
            testnet
                .chain
                .state
                .model_states
                .values()
                .next()
                .unwrap()
                .step,
            1
        );
        let rewarded_miners = testnet
            .miners
            .iter()
            .filter(|miner| testnet.chain.state.rewards.balance(miner) > 0)
            .count();
        assert!(rewarded_miners >= testnet.chain.params.agreement_quorum);
    }

    #[test]
    fn public_testnet_run_evidence_requires_independent_external_operators() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 2,
            min_reward_settlement_records: 3,
        };
        let shared_operator = hash_bytes(b"test", &[b"shared-operator"]);
        let validator_operator = hash_bytes(b"test", &[b"validator-operator"]);
        let mut run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(address(b"miner-a"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::miner(address(b"miner-b"), shared_operator, 0, 9, 10),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    validator_operator,
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
            invalid_receipts_submitted: 2,
            invalid_receipts_rejected: 2,
            reward_settlement_records: 3,
        };

        let insufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(insufficient.miner_count, 1);
        assert_eq!(insufficient.validator_count, 1);
        assert_eq!(insufficient.required_blocks, 0);
        assert_eq!(insufficient.finality_rate_bps, 10_000);
        assert_eq!(insufficient.data_availability_bps, 9_500);
        assert_eq!(insufficient.invalid_work_rejection_rate_bps, 10_000);
        assert!(insufficient.external_operator_evidence);
        assert!(insufficient.has_production_libp2p_runtime);
        assert!(insufficient.has_deployed_public_services);
        assert!(!insufficient.has_required_miners);
        assert!(insufficient.has_invalid_work_rejection_evidence);
        assert!(insufficient.has_reward_settlement_records);
        assert!(!insufficient.public_criterion_met);

        run.nodes[1] = PublicNodeEvidence::miner(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-b-operator"]),
            0,
            9,
            10,
        );
        let shared_node_address = run.evaluate(&criteria, 6, true);
        assert_eq!(shared_node_address.miner_count, 1);
        assert!(!shared_node_address.has_required_miners);
        assert!(!shared_node_address.public_criterion_met);

        run.nodes[1] = PublicNodeEvidence::miner(
            address(b"miner-b"),
            hash_bytes(b"test", &[b"miner-b-operator"]),
            0,
            9,
            10,
        );
        let no_external_flag = run.evaluate(&criteria, 6, false);
        assert!(!no_external_flag.external_operator_evidence);
        assert!(!no_external_flag.public_criterion_met);

        let sufficient = run.evaluate(&criteria, 6, true);
        assert_eq!(sufficient.miner_count, 2);
        assert!(sufficient.has_required_miners);
        assert!(sufficient.has_required_validators);
        assert!(sufficient.has_required_run_duration);
        assert!(sufficient.has_required_block_count);
        assert!(sufficient.has_required_finality);
        assert!(sufficient.has_required_data_availability);
        assert!(sufficient.has_invalid_work_rejection_evidence);
        assert!(sufficient.has_reward_settlement_records);
        assert!(sufficient.has_production_libp2p_runtime);
        assert!(sufficient.has_deployed_public_services);
        assert!(sufficient.public_criterion_met);

        let mut over_finalized = run.clone();
        over_finalized.finalized_blocks = over_finalized.observed_blocks + 1;
        let over_finalized = over_finalized.evaluate(&criteria, 6, true);
        assert_eq!(over_finalized.finality_rate_bps, 10_000);
        assert!(!over_finalized.has_required_finality);
        assert!(!over_finalized.public_criterion_met);

        let mut over_available = run.clone();
        over_available.available_receipts = over_available.checked_receipts + 1;
        let over_available = over_available.evaluate(&criteria, 6, true);
        assert_eq!(over_available.data_availability_bps, 10_000);
        assert!(!over_available.has_required_data_availability);
        assert!(!over_available.public_criterion_met);

        let mut shared_cross_role_operator = run.clone();
        shared_cross_role_operator.nodes[2] =
            PublicNodeEvidence::validator(address(b"validator-a"), shared_operator, 0, 9, 10);
        let shared_cross_role_operator = shared_cross_role_operator.evaluate(&criteria, 6, true);
        assert_eq!(shared_cross_role_operator.miner_count, 2);
        assert_eq!(shared_cross_role_operator.validator_count, 0);
        assert!(!shared_cross_role_operator.has_required_validators);
        assert!(!shared_cross_role_operator.public_criterion_met);

        let mut sparse_heartbeat = run.clone();
        sparse_heartbeat.nodes[0] = PublicNodeEvidence::miner(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            9,
        );
        let sparse_heartbeat = sparse_heartbeat.evaluate(&criteria, 6, true);
        assert_eq!(sparse_heartbeat.miner_count, 1);
        assert!(!sparse_heartbeat.has_required_miners);
        assert!(!sparse_heartbeat.public_criterion_met);

        let mut zero_address = run.clone();
        zero_address.nodes[0] = PublicNodeEvidence::miner(
            [0; 32],
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            10,
        );
        let zero_address = zero_address.evaluate(&criteria, 6, true);
        assert_eq!(zero_address.miner_count, 1);
        assert!(!zero_address.public_criterion_met);

        let one_day_criteria = PublicTestnetCriteria {
            duration_days: 1,
            ..criteria.clone()
        };
        let short_window = run.evaluate(&one_day_criteria, 8_640, true);
        assert!(short_window.has_required_block_count);
        assert!(!short_window.has_required_run_duration);
        assert!(!short_window.public_criterion_met);

        let mut full_window = run.clone();
        full_window.run_ended_at_unix_seconds = full_window.run_started_at_unix_seconds + 86_400;
        let full_window = full_window.evaluate(&one_day_criteria, 8_640, true);
        assert!(full_window.has_required_run_duration);
        assert!(full_window.public_criterion_met);

        let mut tampered_heartbeat = run.clone();
        tampered_heartbeat.nodes[0].heartbeat_signature = [7; 32];
        let tampered_heartbeat = tampered_heartbeat.evaluate(&criteria, 6, true);
        assert_eq!(tampered_heartbeat.miner_count, 1);
        assert!(!tampered_heartbeat.has_required_miners);
        assert!(!tampered_heartbeat.public_criterion_met);

        run.invalid_receipts_rejected = 1;
        let accepted_invalid_work = run.evaluate(&criteria, 6, true);
        assert_eq!(accepted_invalid_work.invalid_work_rejection_rate_bps, 5_000);
        assert!(!accepted_invalid_work.has_invalid_work_rejection_evidence);
        assert!(!accepted_invalid_work.public_criterion_met);
    }

    #[test]
    fn public_testnet_run_evidence_requires_production_runtime_and_reachable_services() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let mut run = PublicTestnetRunEvidence {
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
        };

        assert!(run.services[0].covers_run(0));
        let complete = run.evaluate(&criteria, 6, true);
        assert!(complete.has_production_libp2p_runtime);
        assert!(complete.has_deployed_rpc_service);
        assert!(complete.has_deployed_explorer_service);
        assert!(complete.has_deployed_faucet_service);
        assert!(complete.has_deployed_telemetry_service);
        assert!(complete.has_deployed_public_service_content);
        assert!(complete.has_deployed_public_services);
        assert!(complete.public_criterion_met);

        run.services[1] = PublicServiceEvidence::new(
            PublicServiceKind::Explorer,
            PublicServiceEndpoint::new(
                run.services[0].endpoint_id,
                public_service_url(PublicServiceKind::Explorer),
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        run.service_content[1] = PublicServiceContentEvidence::new(
            PublicServiceKind::Explorer,
            run.services[0].endpoint_id,
            public_service_content_url(PublicServiceKind::Explorer),
            public_service_content_path(PublicServiceKind::Explorer),
            hash_bytes(b"test", &[b"explorer-service", b"content-root"]),
            1_700_000_000,
            64,
        );
        let duplicate_service_endpoint = run.evaluate(&criteria, 6, true);
        assert!(duplicate_service_endpoint.has_deployed_explorer_service);
        assert!(duplicate_service_endpoint.has_deployed_public_service_content);
        assert!(!duplicate_service_endpoint.has_deployed_public_services);
        assert!(!duplicate_service_endpoint.public_criterion_met);
        run.services = deployed_public_services(9);
        run.service_content = deployed_public_service_content();

        run.service_content[1] = PublicServiceContentEvidence::new(
            PublicServiceKind::Explorer,
            hash_bytes(b"test", &[b"explorer-service"]),
            public_service_content_url(PublicServiceKind::Explorer),
            public_service_content_path(PublicServiceKind::Explorer),
            run.service_content[0].content_root,
            1_700_000_000,
            64,
        );
        let duplicate_service_content_root = run.evaluate(&criteria, 6, true);
        assert!(duplicate_service_content_root.has_deployed_explorer_service);
        assert!(!duplicate_service_content_root.has_deployed_public_service_content);
        assert!(!duplicate_service_content_root.has_deployed_public_services);
        assert!(!duplicate_service_content_root.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0].content_signature = [8; 32];
        let tampered_rpc_content = run.evaluate(&criteria, 6, true);
        assert!(!tampered_rpc_content.has_deployed_rpc_service);
        assert!(!tampered_rpc_content.has_deployed_public_service_content);
        assert!(!tampered_rpc_content.has_deployed_public_services);
        assert!(!tampered_rpc_content.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0].public_url = String::from("https://localhost/chain/head");
        let local_rpc_content = run.evaluate(&criteria, 6, true);
        assert!(!local_rpc_content.has_deployed_rpc_service);
        assert!(!local_rpc_content.has_deployed_public_service_content);
        assert!(!local_rpc_content.has_deployed_public_services);
        assert!(!local_rpc_content.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            "https://rpc.tensorvm.net@localhost/chain/head",
            public_service_content_path(PublicServiceKind::Rpc),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_000,
            64,
        );
        let obfuscated_local_rpc_content = run.evaluate(&criteria, 6, true);
        assert!(!obfuscated_local_rpc_content.has_deployed_rpc_service);
        assert!(!obfuscated_local_rpc_content.has_deployed_public_service_content);
        assert!(!obfuscated_local_rpc_content.has_deployed_public_services);
        assert!(!obfuscated_local_rpc_content.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] =
            public_service_content(PublicServiceKind::Rpc, b"independent-rpc-content");
        let mismatched_rpc_content_endpoint = run.evaluate(&criteria, 6, true);
        assert!(!mismatched_rpc_content_endpoint.has_deployed_rpc_service);
        assert!(!mismatched_rpc_content_endpoint.has_deployed_public_service_content);
        assert!(!mismatched_rpc_content_endpoint.has_deployed_public_services);
        assert!(!mismatched_rpc_content_endpoint.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            "https://rpc-content.tensorvm.net/chain/head",
            public_service_content_path(PublicServiceKind::Rpc),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_000,
            64,
        );
        let mismatched_rpc_content_authority = run.evaluate(&criteria, 6, true);
        assert!(!mismatched_rpc_content_authority.has_deployed_rpc_service);
        assert!(!mismatched_rpc_content_authority.has_deployed_public_service_content);
        assert!(!mismatched_rpc_content_authority.has_deployed_public_services);
        assert!(!mismatched_rpc_content_authority.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            "https://rpc.tensorvm.net/wrong",
            "/wrong",
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_000,
            64,
        );
        let wrong_rpc_content_path = run.evaluate(&criteria, 6, true);
        assert!(!wrong_rpc_content_path.has_deployed_rpc_service);
        assert!(!wrong_rpc_content_path.has_deployed_public_service_content);
        assert!(!wrong_rpc_content_path.has_deployed_public_services);
        assert!(!wrong_rpc_content_path.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            "https://rpc.tensorvm.net/chain/head?variant=raw",
            public_service_content_path(PublicServiceKind::Rpc),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_000,
            64,
        );
        let rpc_content_query = run.evaluate(&criteria, 6, true);
        assert!(!rpc_content_query.has_deployed_rpc_service);
        assert!(!rpc_content_query.has_deployed_public_service_content);
        assert!(!rpc_content_query.has_deployed_public_services);
        assert!(!rpc_content_query.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_061,
            64,
        );
        let content_after_run = run.evaluate(&criteria, 6, true);
        assert!(!content_after_run.has_deployed_rpc_service);
        assert!(!content_after_run.has_deployed_public_service_content);
        assert!(!content_after_run.has_deployed_public_services);
        assert!(!content_after_run.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content[0] = PublicServiceContentEvidence::new(
            PublicServiceKind::Rpc,
            hash_bytes(b"test", &[b"rpc-service"]),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            1_700_000_000,
            PUBLIC_SERVICE_MIN_CONTENT_BYTES - 1,
        );
        let undersized_rpc_content = run.evaluate(&criteria, 6, true);
        assert!(!undersized_rpc_content.has_deployed_rpc_service);
        assert!(!undersized_rpc_content.has_deployed_public_service_content);
        assert!(!undersized_rpc_content.has_deployed_public_services);
        assert!(!undersized_rpc_content.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.service_content
            .retain(|content| content.kind != PublicServiceKind::Faucet);
        let missing_faucet_content = run.evaluate(&criteria, 6, true);
        assert!(!missing_faucet_content.has_deployed_faucet_service);
        assert!(!missing_faucet_content.has_deployed_public_service_content);
        assert!(!missing_faucet_content.has_deployed_public_services);
        assert!(!missing_faucet_content.public_criterion_met);
        run.service_content = deployed_public_service_content();

        run.services[0].health_check_signature = [8; 32];
        let tampered_rpc_health = run.evaluate(&criteria, 6, true);
        assert!(!tampered_rpc_health.has_deployed_rpc_service);
        assert!(!tampered_rpc_health.has_deployed_public_services);
        assert!(!tampered_rpc_health.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"local-rpc-service"]),
                "https://localhost/health",
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        let local_rpc_url = run.evaluate(&criteria, 6, true);
        assert!(!local_rpc_url.has_deployed_rpc_service);
        assert!(!local_rpc_url.has_deployed_public_services);
        assert!(!local_rpc_url.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"obfuscated-local-rpc-service"]),
                "https://rpc.tensorvm.net@localhost/health",
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        let obfuscated_local_rpc_url = run.evaluate(&criteria, 6, true);
        assert!(!obfuscated_local_rpc_url.has_deployed_rpc_service);
        assert!(!obfuscated_local_rpc_url.has_deployed_public_services);
        assert!(!obfuscated_local_rpc_url.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"bad-health-path-rpc-service"]),
                public_service_url(PublicServiceKind::Rpc),
                "health",
            ),
            0,
            9,
            10,
            10,
        );
        let bad_health_path = run.evaluate(&criteria, 6, true);
        assert!(!bad_health_path.has_deployed_rpc_service);
        assert!(!bad_health_path.has_deployed_public_services);
        assert!(!bad_health_path.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"rpc-service"]),
                "https://rpc.tensorvm.net/health?probe=1",
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        let rpc_health_query = run.evaluate(&criteria, 6, true);
        assert!(!rpc_health_query.has_deployed_rpc_service);
        assert!(!rpc_health_query.has_deployed_public_services);
        assert!(!rpc_health_query.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"rpc-service"]),
                public_service_url(PublicServiceKind::Rpc),
                "/health",
            ),
            0,
            9,
            9,
            10,
        );
        let sparse_rpc_reachability = run.evaluate(&criteria, 6, true);
        assert!(!sparse_rpc_reachability.has_deployed_rpc_service);
        assert!(!sparse_rpc_reachability.has_deployed_public_services);
        assert!(!sparse_rpc_reachability.public_criterion_met);
        run.services = deployed_public_services(9);

        run.services[0] = PublicServiceEvidence::new(
            PublicServiceKind::Rpc,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[b"rpc-service"]),
                public_service_url(PublicServiceKind::Rpc),
                "/health",
            ),
            0,
            9,
            10,
            9,
        );
        let sparse_rpc_health_signatures = run.evaluate(&criteria, 6, true);
        assert!(!sparse_rpc_health_signatures.has_deployed_rpc_service);
        assert!(!sparse_rpc_health_signatures.has_deployed_public_services);
        assert!(!sparse_rpc_health_signatures.public_criterion_met);
        run.services = deployed_public_services(9);

        run.network_runtime.request_response_observed = false;
        let no_request_response = run.evaluate(&criteria, 6, true);
        assert!(!no_request_response.has_production_libp2p_runtime);
        assert!(no_request_response.has_deployed_public_services);
        assert!(!no_request_response.public_criterion_met);
        run.network_runtime = production_runtime_evidence();

        run.services
            .retain(|service| service.kind != PublicServiceKind::Telemetry);
        let missing_telemetry = run.evaluate(&criteria, 6, true);
        assert!(missing_telemetry.has_production_libp2p_runtime);
        assert!(!missing_telemetry.has_deployed_telemetry_service);
        assert!(!missing_telemetry.has_deployed_public_services);
        assert!(!missing_telemetry.public_criterion_met);

        run.services.push(public_service(
            PublicServiceKind::Telemetry,
            b"late-telemetry-service",
            1,
            9,
        ));
        let late_telemetry = run.evaluate(&criteria, 6, true);
        assert!(!late_telemetry.has_deployed_telemetry_service);
        assert!(!late_telemetry.public_criterion_met);

        run.services.pop();
        let mut unsigned_telemetry = public_service(
            PublicServiceKind::Telemetry,
            b"unsigned-telemetry-service",
            0,
            9,
        );
        unsigned_telemetry.signed_health_check_count = 0;
        assert!(!unsigned_telemetry.has_reachable_endpoint_proof());
        run.services.push(unsigned_telemetry);
        let unsigned_telemetry = run.evaluate(&criteria, 6, true);
        assert!(!unsigned_telemetry.has_deployed_telemetry_service);
        assert!(!unsigned_telemetry.public_criterion_met);
    }

    #[test]
    fn public_testnet_evidence_bundle_requires_publication_and_audit_records() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let mut bundle = complete_public_evidence_bundle();

        let complete = bundle.evaluate(&criteria, 6);
        assert!(complete.run_evidence.public_criterion_met);
        assert!(complete.has_published_evidence_bundle);
        assert!(complete.has_independent_auditor_records);
        assert!(complete.has_signed_run_window);
        assert!(complete.has_block_history);
        assert!(complete.has_finality_history);
        assert!(complete.has_operator_identity_attestations);
        assert!(complete.has_network_runtime_observations);
        assert!(complete.has_data_availability_measurements);
        assert!(complete.has_invalid_work_rejection_records);
        assert!(complete.has_reward_settlement_record_summary);
        assert!(complete.has_public_supporting_record_artifacts);
        assert!(complete.independently_checkable);
        assert!(!complete.full_spec_evidence_met);

        let full_spec_criteria = PublicTestnetCriteria::default();
        let full_spec_block_time = ChainParams::default().block_time_seconds;
        let full_spec_bundle = full_spec_public_evidence_bundle(full_spec_block_time);
        let full_spec_report = full_spec_bundle.evaluate(&full_spec_criteria, full_spec_block_time);
        assert!(full_spec_report.run_evidence.public_criterion_met);
        assert!(full_spec_report.independently_checkable);
        assert!(full_spec_report.full_spec_evidence_met);

        bundle.publication.manifest_signature = [9; 32];
        let tampered_manifest_signature = bundle.evaluate(&criteria, 6);
        assert!(!tampered_manifest_signature.has_published_evidence_bundle);
        assert!(!tampered_manifest_signature.independently_checkable);
        assert!(!tampered_manifest_signature.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.publication = PublicEvidencePublication::new(
            bundle.publication.bundle_id,
            bundle.publication.public_uri.clone(),
            bundle.publication.manifest_signer,
            2,
            bundle.publication.independent_auditor_count,
        );
        let overreported_manifest_signature_count = bundle.evaluate(&criteria, 6);
        assert!(!overreported_manifest_signature_count.has_published_evidence_bundle);
        assert!(!overreported_manifest_signature_count.independently_checkable);
        assert!(!overreported_manifest_signature_count.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.run_window_signature = [7; 32];
        let tampered_run_window = bundle.evaluate(&criteria, 6);
        assert!(!tampered_run_window.has_signed_run_window);
        assert!(!tampered_run_window.independently_checkable);
        assert!(!tampered_run_window.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.run.run_ended_at_unix_seconds = bundle.run.run_started_at_unix_seconds - 1;
        let invalid_run_window = bundle.evaluate(&criteria, 6);
        assert!(!invalid_run_window.has_signed_run_window);
        assert!(!invalid_run_window.run_evidence.has_required_run_duration);
        assert!(!invalid_run_window.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.manifest_signer = [0; 32];
        let missing_manifest_signer = bundle.evaluate(&criteria, 6);
        assert!(!missing_manifest_signer.has_published_evidence_bundle);
        assert!(!missing_manifest_signer.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("http://localhost:8545/evidence.json");
        let local_uri = bundle.evaluate(&criteria, 6);
        assert!(!local_uri.has_published_evidence_bundle);
        assert!(!local_uri.independently_checkable);
        assert!(!local_uri.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("https://localhost/evidence.json");
        let localhost_https_uri = bundle.evaluate(&criteria, 6);
        assert!(!localhost_https_uri.has_published_evidence_bundle);
        assert!(!localhost_https_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("https://192.168.1.2/evidence.json");
        let private_https_uri = bundle.evaluate(&criteria, 6);
        assert!(!private_https_uri.has_published_evidence_bundle);
        assert!(!private_https_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication = PublicEvidencePublication::new(
            bundle.publication.bundle_id,
            " https://evidence.tensorvm.net/public-evidence.json".to_owned(),
            bundle.publication.manifest_signer,
            bundle.publication.manifest_signature_count,
            bundle.publication.independent_auditor_count,
        );
        let leading_space_publication_uri = bundle.evaluate(&criteria, 6);
        assert!(!leading_space_publication_uri.has_published_evidence_bundle);
        assert!(!leading_space_publication_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication = PublicEvidencePublication::new(
            bundle.publication.bundle_id,
            "https://evidence.tensorvm.net/public-evidence.json ".to_owned(),
            bundle.publication.manifest_signer,
            bundle.publication.manifest_signature_count,
            bundle.publication.independent_auditor_count,
        );
        let trailing_space_publication_uri = bundle.evaluate(&criteria, 6);
        assert!(!trailing_space_publication_uri.has_published_evidence_bundle);
        assert!(!trailing_space_publication_uri.independently_checkable);

        assert!(public_evidence_uri_is_external(
            "https://evidence.tensorvm.net:443/public-evidence.json"
        ));
        assert!(public_evidence_uri_is_external(
            "https://[2001:4860:4860::8888]/public-evidence.json"
        ));
        assert!(public_evidence_uri_is_external(
            "https://[2001:4860:4860::8888]:443/public-evidence.json"
        ));
        for uri in [
            "https://evidence.tensorvm.net@localhost/public-evidence.json",
            "https://localhost@evidence.tensorvm.net/public-evidence.json",
            "https://evidence.tensorvm.net /public-evidence.json",
            " https://evidence.tensorvm.net/public-evidence.json",
            "https://evidence.tensorvm.net/public-evidence.json ",
            "https://evidence.tensorvm.net/public evidence.json",
            "https://evidence.tensorvm.net:bad/public-evidence.json",
            "https://evidence.tensorvm.net:0/public-evidence.json",
            "https://evidence.example.test/public-evidence.json",
            "https://evidence.tensorvm.example/public-evidence.json",
            "https://example.com/public-evidence.json",
            "https://sub.example.org/public-evidence.json",
            "https://evidence.invalid/public-evidence.json",
            "https://[2001:db8::1]x/public-evidence.json",
            "https://[2001:4860:4860::8888]:/public-evidence.json",
            "https://evidence.tensorvm.net",
            "https://evidence.tensorvm.net/",
            "https://evidence.tensorvm.net?manifest=1",
            "https://evidence.tensorvm.net#manifest",
            "https://evidence.tensorvm.net/public-evidence.json?download=1",
            "https://evidence.tensorvm.net/public-evidence.json#sha256",
            "https:///public-evidence.json",
        ] {
            assert!(!public_evidence_uri_is_external(uri));
        }

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri =
            String::from("https://evidence.tensorvm.net@localhost/public-evidence.json");
        let userinfo_obfuscated_uri = bundle.evaluate(&criteria, 6);
        assert!(!userinfo_obfuscated_uri.has_published_evidence_bundle);
        assert!(!userinfo_obfuscated_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri =
            String::from("https://evidence.tensorvm.net/public-evidence.json?download=1");
        let query_publication_uri = bundle.evaluate(&criteria, 6);
        assert!(!query_publication_uri.has_published_evidence_bundle);
        assert!(!query_publication_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("https://evidence.tensorvm.net/");
        let root_only_publication_uri = bundle.evaluate(&criteria, 6);
        assert!(!root_only_publication_uri.has_published_evidence_bundle);
        assert!(!root_only_publication_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("ipfs://");
        let empty_ipfs_uri = bundle.evaluate(&criteria, 6);
        assert!(!empty_ipfs_uri.has_published_evidence_bundle);
        assert!(!empty_ipfs_uri.has_independent_auditor_records);
        assert!(!empty_ipfs_uri.independently_checkable);

        assert!(public_evidence_uri_is_external(
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3g3/raw.json"
        ));
        assert!(public_evidence_uri_is_external(
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3g3/raw-records_2026-05.json"
        ));
        assert!(public_evidence_uri_is_external(
            "ar://abc_DEF-123/raw_records.json"
        ));
        assert!(public_evidence_uri_is_external("ar://abc_DEF-123"));
        for uri in [
            "ipfs://?cid",
            "ipfs://#cid",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3?download=1",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3#manifest",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/raw.json?download=1",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/raw.json#manifest",
            "ipfs://../manifest.json",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/../manifest.json",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/./manifest.json",
            "ipfs:///manifest.json",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3//manifest.json",
            " ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3 ",
            "ipfs://white space",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/bad space.json",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3/bad%20path.json",
            "ipfs://bafybeigdyrztxylvd7m5qkz6g2q6k7lb4w3g3g3g3g3g3g3g3g3g3g3g3\\raw.json",
            "ar:///",
        ] {
            assert!(!public_evidence_uri_is_external(uri));
        }

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("ipfs://?cid");
        let malformed_content_uri = bundle.evaluate(&criteria, 6);
        assert!(!malformed_content_uri.has_published_evidence_bundle);
        assert!(!malformed_content_uri.has_independent_auditor_records);
        assert!(!malformed_content_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.auditor_records.clear();
        let missing_auditor_records = bundle.evaluate(&criteria, 6);
        assert!(missing_auditor_records.has_published_evidence_bundle);
        assert!(!missing_auditor_records.has_independent_auditor_records);
        assert!(!missing_auditor_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.auditor_records[0].auditor_signature = [2; 32];
        let tampered_auditor_record = bundle.evaluate(&criteria, 6);
        assert!(!tampered_auditor_record.has_independent_auditor_records);
        assert!(!tampered_auditor_record.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.auditor_records[0].audit_uri = String::from("https://localhost/audit.json");
        let local_auditor_record = bundle.evaluate(&criteria, 6);
        assert!(!local_auditor_record.has_independent_auditor_records);
        assert!(!local_auditor_record.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.auditor_records[0] = PublicEvidenceAuditorRecord::new(
            &bundle.publication.bundle_id,
            &bundle.publication.public_uri,
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            bundle.run.run_started_at_unix_seconds,
        );
        let pre_run_end_auditor_record = bundle.evaluate(&criteria, 6);
        assert!(!pre_run_end_auditor_record.has_independent_auditor_records);
        assert!(!pre_run_end_auditor_record.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.auditor_records[0] = PublicEvidenceAuditorRecord::new(
            &bundle.publication.bundle_id,
            &bundle.publication.public_uri,
            bundle.publication.manifest_signer,
            "https://auditors.tensorvm.net/signer-audit.json",
            1_700_000_000,
        );
        let signer_as_auditor = bundle.evaluate(&criteria, 6);
        assert!(!signer_as_auditor.has_independent_auditor_records);
        assert!(!signer_as_auditor.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle
            .auditor_records
            .push(PublicEvidenceAuditorRecord::new(
                &bundle.publication.bundle_id,
                &bundle.publication.public_uri,
                address(b"public-evidence-auditor-extra"),
                "https://auditors.tensorvm.net/extra-audit.json",
                bundle.run.run_ended_at_unix_seconds,
            ));
        let extra_auditor_record = bundle.evaluate(&criteria, 6);
        assert!(!extra_auditor_record.has_independent_auditor_records);
        assert!(!extra_auditor_record.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.block_history_records = 9;
        let missing_block_history = bundle.evaluate(&criteria, 6);
        assert!(!missing_block_history.has_block_history);
        assert!(!missing_block_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let block_history_root = bundle.block_history_root;
        let overreported_block_history_count = bundle.run.observed_blocks + 1;
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::BlockHistory,
            block_history_root,
            overreported_block_history_count,
        );
        let overreported_block_history = bundle.evaluate(&criteria, 6);
        assert!(!overreported_block_history.has_block_history);
        assert!(overreported_block_history.has_public_supporting_record_artifacts);
        assert!(!overreported_block_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.block_history_signature = [6; 32];
        let tampered_block_history = bundle.evaluate(&criteria, 6);
        assert!(!tampered_block_history.has_block_history);
        assert!(!tampered_block_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.block_history_root = [0; 32];
        let missing_block_history_root = bundle.evaluate(&criteria, 6);
        assert!(!missing_block_history_root.has_block_history);
        assert!(!missing_block_history_root.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.finality_history_records = 9;
        let missing_finality_history = bundle.evaluate(&criteria, 6);
        assert!(!missing_finality_history.has_finality_history);
        assert!(!missing_finality_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let finality_history_root = bundle.finality_history_root;
        let overreported_finality_history_count = bundle.run.observed_blocks + 1;
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::FinalityHistory,
            finality_history_root,
            overreported_finality_history_count,
        );
        let overreported_finality_history = bundle.evaluate(&criteria, 6);
        assert!(!overreported_finality_history.has_finality_history);
        assert!(overreported_finality_history.has_public_supporting_record_artifacts);
        assert!(!overreported_finality_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.finality_history_signature = [5; 32];
        let tampered_finality_history = bundle.evaluate(&criteria, 6);
        assert!(!tampered_finality_history.has_finality_history);
        assert!(!tampered_finality_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestation_records = 2;
        let missing_operator_attestations = bundle.evaluate(&criteria, 6);
        assert!(!missing_operator_attestations.has_operator_identity_attestations);
        assert!(
            !missing_operator_attestations
                .run_evidence
                .external_operator_evidence
        );
        assert!(
            !missing_operator_attestations
                .run_evidence
                .public_criterion_met
        );
        assert!(!missing_operator_attestations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestation_records = 4;
        let overreported_operator_attestations = bundle.evaluate(&criteria, 6);
        assert!(!overreported_operator_attestations.has_operator_identity_attestations);
        assert!(
            !overreported_operator_attestations
                .run_evidence
                .external_operator_evidence
        );
        assert!(!overreported_operator_attestations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestations[0].operator_signature = [8; 32];
        let tampered_operator_attestation = bundle.evaluate(&criteria, 6);
        assert!(!tampered_operator_attestation.has_operator_identity_attestations);
        assert!(
            !tampered_operator_attestation
                .run_evidence
                .external_operator_evidence
        );
        assert!(!tampered_operator_attestation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestations[0].identity_uri =
            String::from("https://localhost/operator.json");
        let local_operator_attestation = bundle.evaluate(&criteria, 6);
        assert!(!local_operator_attestation.has_operator_identity_attestations);
        assert!(!local_operator_attestation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let stale_operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
        bundle.operator_identity_attestations[0] = PublicOperatorIdentityAttestation::new(
            PublicNodeRole::Miner,
            address(b"miner-a"),
            stale_operator_id,
            manifest_operator_identity_uri(&stale_operator_id),
            bundle.run.run_started_at_unix_seconds - 1,
        );
        let stale_operator_attestation = bundle.evaluate(&criteria, 6);
        assert!(!stale_operator_attestation.has_operator_identity_attestations);
        assert!(
            !stale_operator_attestation
                .run_evidence
                .external_operator_evidence
        );
        assert!(!stale_operator_attestation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestations.clear();
        let missing_signed_operator_records = bundle.evaluate(&criteria, 6);
        assert!(!missing_signed_operator_records.has_operator_identity_attestations);
        assert!(!missing_signed_operator_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observation_records = 2;
        let missing_network_runtime_observations = bundle.evaluate(&criteria, 6);
        assert!(!missing_network_runtime_observations.has_network_runtime_observations);
        assert!(!missing_network_runtime_observations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observations.pop();
        let missing_signed_network_runtime_observation = bundle.evaluate(&criteria, 6);
        assert!(!missing_signed_network_runtime_observation.has_network_runtime_observations);
        assert!(!missing_signed_network_runtime_observation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observations[0].operator_id =
            hash_bytes(b"test", &[b"unmatched-network-operator"]);
        let unmatched_network_operator = bundle.evaluate(&criteria, 6);
        assert!(!unmatched_network_operator.has_network_runtime_observations);
        assert!(!unmatched_network_operator.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observations[0].listen_address =
            String::from("/ip4/127.0.0.1/tcp/4001");
        let local_network_observation = bundle.evaluate(&criteria, 6);
        assert!(!local_network_observation.has_network_runtime_observations);
        assert!(!local_network_observation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observations[0].observed_at_unix_seconds =
            bundle.run.run_started_at_unix_seconds - 1;
        let stale_network_observation = bundle.evaluate(&criteria, 6);
        assert!(!stale_network_observation.has_network_runtime_observations);
        assert!(!stale_network_observation.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let network_runtime_root = bundle.network_runtime_observation_root;
        let underreported_network_runtime_count = bundle
            .operator_identity_attestation_records
            .saturating_sub(1);
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            network_runtime_root,
            underreported_network_runtime_count,
        );
        let underreported_network_runtime_observations = bundle.evaluate(&criteria, 6);
        assert!(!underreported_network_runtime_observations.has_network_runtime_observations);
        assert!(underreported_network_runtime_observations.has_operator_identity_attestations);
        assert!(underreported_network_runtime_observations.has_public_supporting_record_artifacts);
        assert!(!underreported_network_runtime_observations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let network_runtime_root = bundle.network_runtime_observation_root;
        let overreported_network_runtime_count = bundle
            .operator_identity_attestation_records
            .saturating_add(1);
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            network_runtime_root,
            overreported_network_runtime_count,
        );
        let overreported_network_runtime_observations = bundle.evaluate(&criteria, 6);
        assert!(!overreported_network_runtime_observations.has_network_runtime_observations);
        assert!(overreported_network_runtime_observations.has_operator_identity_attestations);
        assert!(overreported_network_runtime_observations.has_public_supporting_record_artifacts);
        assert!(!overreported_network_runtime_observations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.network_runtime_observation_signature = [3; 32];
        let tampered_network_runtime_observations = bundle.evaluate(&criteria, 6);
        assert!(!tampered_network_runtime_observations.has_network_runtime_observations);
        assert!(!tampered_network_runtime_observations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.run.network_runtime.gossip_propagation_observed = false;
        let no_network_runtime_observations = bundle.evaluate(&criteria, 6);
        assert!(!no_network_runtime_observations.has_network_runtime_observations);
        assert!(!no_network_runtime_observations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.data_availability_measurement_records = 19;
        let missing_data_availability_measurements = bundle.evaluate(&criteria, 6);
        assert!(!missing_data_availability_measurements.has_data_availability_measurements);
        assert!(!missing_data_availability_measurements.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let data_availability_root = bundle.data_availability_measurement_root;
        let overreported_data_availability_count = bundle.run.checked_receipts + 1;
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            data_availability_root,
            overreported_data_availability_count,
        );
        let overreported_data_availability_measurements = bundle.evaluate(&criteria, 6);
        assert!(!overreported_data_availability_measurements.has_data_availability_measurements);
        assert!(overreported_data_availability_measurements.has_public_supporting_record_artifacts);
        assert!(!overreported_data_availability_measurements.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.data_availability_measurement_signature = [4; 32];
        let tampered_data_availability_measurements = bundle.evaluate(&criteria, 6);
        assert!(!tampered_data_availability_measurements.has_data_availability_measurements);
        assert!(!tampered_data_availability_measurements.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.invalid_work_rejection_signature = [2; 32];
        let tampered_invalid_work_records = bundle.evaluate(&criteria, 6);
        assert!(!tampered_invalid_work_records.has_invalid_work_rejection_records);
        assert!(!tampered_invalid_work_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.invalid_work_rejection_records = 0;
        let missing_invalid_work_records = bundle.evaluate(&criteria, 6);
        assert!(!missing_invalid_work_records.has_invalid_work_rejection_records);
        assert!(!missing_invalid_work_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        let invalid_work_root = bundle.invalid_work_rejection_root;
        let overreported_invalid_work_count = bundle.run.invalid_receipts_submitted + 1;
        resign_record_summary_and_artifact(
            &mut bundle,
            PublicEvidenceRecordKind::InvalidWorkRejections,
            invalid_work_root,
            overreported_invalid_work_count,
        );
        let overreported_invalid_work_records = bundle.evaluate(&criteria, 6);
        assert!(!overreported_invalid_work_records.has_invalid_work_rejection_records);
        assert!(overreported_invalid_work_records.has_public_supporting_record_artifacts);
        assert!(!overreported_invalid_work_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.reward_settlement_signature = [1; 32];
        let tampered_reward_records = bundle.evaluate(&criteria, 6);
        assert!(!tampered_reward_records.has_reward_settlement_record_summary);
        assert!(!tampered_reward_records.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.supporting_artifacts.clear();
        let missing_supporting_artifacts = bundle.evaluate(&criteria, 6);
        assert!(!missing_supporting_artifacts.has_public_supporting_record_artifacts);
        assert!(!missing_supporting_artifacts.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.supporting_artifacts[0].artifact_signature = [1; 32];
        let tampered_supporting_artifact = bundle.evaluate(&criteria, 6);
        assert!(!tampered_supporting_artifact.has_public_supporting_record_artifacts);
        assert!(!tampered_supporting_artifact.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.supporting_artifacts[0].artifact_uri = String::from("https://localhost/raw.json");
        let local_supporting_artifact = bundle.evaluate(&criteria, 6);
        assert!(!local_supporting_artifact.has_public_supporting_record_artifacts);
        assert!(!local_supporting_artifact.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.supporting_artifacts[0].artifact_uri =
            String::from("https://evidence.tensorvm.net/");
        let root_only_supporting_artifact = bundle.evaluate(&criteria, 6);
        assert!(!root_only_supporting_artifact.has_public_supporting_record_artifacts);
        assert!(!root_only_supporting_artifact.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle
            .supporting_artifacts
            .push(bundle.supporting_artifacts[0].clone());
        let duplicate_supporting_artifact = bundle.evaluate(&criteria, 6);
        assert!(!duplicate_supporting_artifact.has_public_supporting_record_artifacts);
        assert!(!duplicate_supporting_artifact.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.run.services.clear();
        let missing_services = bundle.evaluate(&criteria, 6);
        assert!(missing_services.independently_checkable);
        assert!(!missing_services.run_evidence.public_criterion_met);
        assert!(!missing_services.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.run.service_content.clear();
        let missing_service_content = bundle.evaluate(&criteria, 6);
        assert!(missing_service_content.independently_checkable);
        assert!(
            !missing_service_content
                .run_evidence
                .has_deployed_public_service_content
        );
        assert!(!missing_service_content.run_evidence.public_criterion_met);
        assert!(!missing_service_content.full_spec_evidence_met);
    }

    #[test]
    fn public_testnet_evidence_manifest_parses_into_bundle() {
        let criteria = PublicTestnetCriteria {
            min_miners: 2,
            min_validators: 1,
            duration_days: 0,
            min_finality_rate_bps: 9_000,
            min_data_availability_bps: 9_500,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let manifest = complete_public_evidence_manifest_text();
        let parsed = parse_public_testnet_evidence_manifest(&manifest).unwrap();

        assert_eq!(parsed, complete_public_evidence_bundle());
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .has_independent_auditor_records
        );
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .has_invalid_work_rejection_records
        );
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .has_reward_settlement_record_summary
        );
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .has_public_supporting_record_artifacts
        );
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .run_evidence
                .has_deployed_public_service_content
        );
        assert!(
            parsed
                .evaluate(&criteria, 6)
                .run_evidence
                .public_criterion_met
        );
        assert!(parsed.evaluate(&criteria, 6).independently_checkable);
        assert!(!parsed.evaluate(&criteria, 6).full_spec_evidence_met);

        let false_runtime =
            manifest.replace("libp2p_runtime_used=true", "libp2p_runtime_used=false");
        let parsed_false_runtime = parse_public_testnet_evidence_manifest(&false_runtime).unwrap();
        assert!(!parsed_false_runtime.run.network_runtime.libp2p_runtime_used);
        assert!(
            !parsed_false_runtime
                .evaluate(&criteria, 6)
                .full_spec_evidence_met
        );

        let local_rpc_service = manifest.replace(
            "https://rpc.tensorvm.net/health",
            "https://localhost/health",
        );
        let parsed_local_rpc_service =
            parse_public_testnet_evidence_manifest(&local_rpc_service).unwrap();
        let local_rpc_report = parsed_local_rpc_service.evaluate(&criteria, 6);
        assert!(!local_rpc_report.run_evidence.has_deployed_rpc_service);
        assert!(!local_rpc_report.run_evidence.has_deployed_public_services);
        assert!(!local_rpc_report.full_spec_evidence_met);

        let trailing_public_uri = manifest.replace(
            "public_uri=https://tensorvm.net/tensorvm/public-evidence.json",
            "public_uri=https://tensorvm.net/tensorvm/public-evidence.json ",
        );
        let trailing_public_uri_report =
            parse_public_testnet_evidence_manifest(&trailing_public_uri)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(!trailing_public_uri_report.has_published_evidence_bundle);
        assert!(!trailing_public_uri_report.independently_checkable);
        assert!(!trailing_public_uri_report.full_spec_evidence_met);

        let auditor_uri = manifest_auditor_uri();
        let auditor_uri_with_space = manifest.replace(
            &format!("{auditor_uri},1700000060"),
            &format!("{auditor_uri} ,1700000060"),
        );
        let auditor_uri_with_space_report =
            parse_public_testnet_evidence_manifest(&auditor_uri_with_space)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(!auditor_uri_with_space_report.has_independent_auditor_records);
        assert!(!auditor_uri_with_space_report.independently_checkable);

        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let block_artifact_uri = public_evidence_supporting_artifact_uri(
            &bundle_id,
            PublicEvidenceRecordKind::BlockHistory,
        );
        let artifact_uri_with_space = manifest.replace(
            &format!("record_artifact=block-history,{block_artifact_uri},"),
            &format!("record_artifact=block-history,{block_artifact_uri} ,"),
        );
        let artifact_uri_with_space_report =
            parse_public_testnet_evidence_manifest(&artifact_uri_with_space)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(!artifact_uri_with_space_report.has_public_supporting_record_artifacts);
        assert!(!artifact_uri_with_space_report.independently_checkable);

        let miner_operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
        let operator_uri = manifest_operator_identity_uri(&miner_operator_id);
        let operator_uri_with_space = manifest.replace(
            &format!("{operator_uri},1700000000"),
            &format!(" {operator_uri},1700000000"),
        );
        let operator_uri_with_space_report =
            parse_public_testnet_evidence_manifest(&operator_uri_with_space)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(!operator_uri_with_space_report.has_operator_identity_attestations);
        assert!(!operator_uri_with_space_report.independently_checkable);

        let service_url_with_space = manifest.replace(
            "https://rpc.tensorvm.net/health,/health",
            "https://rpc.tensorvm.net/health ,/health",
        );
        let service_url_with_space_report =
            parse_public_testnet_evidence_manifest(&service_url_with_space)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(
            !service_url_with_space_report
                .run_evidence
                .has_deployed_rpc_service
        );
        assert!(
            !service_url_with_space_report
                .run_evidence
                .has_deployed_public_services
        );
        assert!(!service_url_with_space_report.full_spec_evidence_met);

        let service_content_url_with_space = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            "https://rpc.tensorvm.net/chain/head ,/chain/head",
        );
        let service_content_url_with_space_report =
            parse_public_testnet_evidence_manifest(&service_content_url_with_space)
                .unwrap()
                .evaluate(&criteria, 6);
        assert!(
            !service_content_url_with_space_report
                .run_evidence
                .has_deployed_public_service_content
        );
        assert!(!service_content_url_with_space_report.full_spec_evidence_met);

        let missing_operator_lines = manifest_without_line(&manifest, "operator=");
        let parsed_missing_operator_lines =
            parse_public_testnet_evidence_manifest(&missing_operator_lines).unwrap();
        let missing_operator_report = parsed_missing_operator_lines.evaluate(&criteria, 6);
        assert!(!missing_operator_report.has_operator_identity_attestations);
        assert!(!missing_operator_report.independently_checkable);
        assert!(!missing_operator_report.full_spec_evidence_met);

        let missing_auditor_lines = manifest_without_line(&manifest, "auditor=");
        let parsed_missing_auditor_lines =
            parse_public_testnet_evidence_manifest(&missing_auditor_lines).unwrap();
        let missing_auditor_report = parsed_missing_auditor_lines.evaluate(&criteria, 6);
        assert!(!missing_auditor_report.has_independent_auditor_records);
        assert!(!missing_auditor_report.independently_checkable);
        assert!(!missing_auditor_report.full_spec_evidence_met);

        let missing_artifact_lines = manifest_without_line(&manifest, "record_artifact=");
        let parsed_missing_artifact_lines =
            parse_public_testnet_evidence_manifest(&missing_artifact_lines).unwrap();
        let missing_artifact_report = parsed_missing_artifact_lines.evaluate(&criteria, 6);
        assert!(!missing_artifact_report.has_public_supporting_record_artifacts);
        assert!(!missing_artifact_report.independently_checkable);
        assert!(!missing_artifact_report.full_spec_evidence_met);

        let missing_service_content_lines = manifest_without_line(&manifest, "service_content=");
        let parsed_missing_service_content =
            parse_public_testnet_evidence_manifest(&missing_service_content_lines).unwrap();
        let missing_service_content_report = parsed_missing_service_content.evaluate(&criteria, 6);
        assert!(
            !missing_service_content_report
                .run_evidence
                .has_deployed_public_service_content
        );
        assert!(!missing_service_content_report.full_spec_evidence_met);

        let uppercase_hash = manifest_hash(b"test", b"public-evidence-bundle").to_uppercase();
        assert_eq!(
            parse_hash_hex(&uppercase_hash).unwrap(),
            hash_bytes(b"test", &[b"public-evidence-bundle"])
        );
        assert!(parse_hash_hex(&format!("z{}", "0".repeat(63))).is_err());
    }

    #[test]
    fn deployed_public_testnet_evidence_example_is_parseable_but_not_full_spec() {
        let manifest =
            include_str!("../../../deploy/tensorvm/manifests/public-testnet.evidence.example");
        assert_public_testnet_evidence_manifest_is_pending(manifest);
    }

    #[test]
    fn docs_public_testnet_evidence_manifest_is_parseable_but_not_full_spec() {
        let manifest = include_str!("../../../docs/tensorvm/public-testnet.evidence");
        assert_public_testnet_evidence_manifest_is_pending(manifest);
    }

    fn assert_public_testnet_evidence_manifest_is_pending(manifest: &str) {
        let parsed = parse_public_testnet_evidence_manifest(manifest).unwrap();
        let report = parsed.evaluate(
            &PublicTestnetCriteria::default(),
            ChainParams::default().block_time_seconds,
        );

        assert!(!report.has_published_evidence_bundle);
        assert!(!report.has_independent_auditor_records);
        assert!(report.has_signed_run_window);
        assert!(report.has_block_history);
        assert!(report.has_finality_history);
        assert!(!report.has_operator_identity_attestations);
        assert!(!report.has_network_runtime_observations);
        assert!(report.has_data_availability_measurements);
        assert!(report.has_invalid_work_rejection_records);
        assert!(report.has_reward_settlement_record_summary);
        assert!(!report.has_public_supporting_record_artifacts);
        assert!(!report.run_evidence.has_deployed_public_service_content);
        assert!(!report.independently_checkable);
        assert!(!report.run_evidence.public_criterion_met);
        assert!(!report.run_evidence.has_required_miners);
        assert!(!report.run_evidence.has_required_validators);
        assert!(!report.run_evidence.has_required_run_duration);
        assert!(!report.run_evidence.has_required_block_count);
        assert!(!report.full_spec_evidence_met);
    }

    #[test]
    fn public_testnet_evidence_manifest_rejects_malformed_input() {
        let manifest = complete_public_evidence_manifest_text();
        let cases = [
            manifest_without_line(&manifest, "version="),
            manifest.replace(
                PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION,
                "tensor-vm-public-testnet-evidence-v0",
            ),
            manifest_without_line(&manifest, "bundle_id="),
            manifest_without_line(&manifest, "public_uri="),
            manifest_without_line(&manifest, "manifest_signer="),
            manifest_without_line(&manifest, "manifest_signature="),
            manifest_without_line(&manifest, "block_history_root="),
            manifest_without_line(&manifest, "block_history_signature="),
            manifest_without_line(&manifest, "finality_history_root="),
            manifest_without_line(&manifest, "finality_history_signature="),
            manifest_without_line(&manifest, "network_runtime_observation_records="),
            manifest_without_line(&manifest, "network_runtime_observation_root="),
            manifest_without_line(&manifest, "network_runtime_observation_signature="),
            manifest_without_line(&manifest, "data_availability_measurement_root="),
            manifest_without_line(&manifest, "data_availability_measurement_signature="),
            manifest_without_line(&manifest, "invalid_work_rejection_records="),
            manifest_without_line(&manifest, "invalid_work_rejection_root="),
            manifest_without_line(&manifest, "invalid_work_rejection_signature="),
            manifest_without_line(&manifest, "reward_settlement_root="),
            manifest_without_line(&manifest, "reward_settlement_signature="),
            manifest_without_line(&manifest, "run_started_at_unix_seconds="),
            manifest_without_line(&manifest, "run_ended_at_unix_seconds="),
            manifest_without_line(&manifest, "run_window_signature="),
            manifest_without_line(&manifest, "observed_blocks="),
            manifest_without_line(&manifest, "dos_controls_enabled="),
            manifest.replace("bundle_id=0x", "bundle_id=0x12"),
            manifest.replace("bundle_id=0x", "bundle_id=0xz"),
            format!("{manifest}\nobserved_blocks=10"),
            manifest.replace("bundle_id=", " bundle_id="),
            manifest.replace("bundle_id=", "bundle_id ="),
            manifest.replace("manifest_signature_count=1", "manifest_signature_count=abc"),
            manifest.replace("dos_controls_enabled=true", "dos_controls_enabled=maybe"),
            manifest.replace("node=miner", "node=archive"),
            manifest.replace(
                "node=miner,",
                "node=miner,too,few,fields\n# removed original node=",
            ),
            manifest.replace("operator=miner", "operator=archive"),
            manifest.replace(
                "operator=miner,",
                "operator=miner,too,few,fields\n# removed original operator=",
            ),
            manifest.replace(
                "network_runtime_observation=",
                "network_runtime_observation=too,few,fields\n# removed original network_runtime_observation=",
            ),
            manifest.replace(
                "auditor=",
                "auditor=too,few,fields\n# removed original auditor=",
            ),
            manifest.replace("record_artifact=block-history", "record_artifact=archive"),
            manifest.replace(
                "record_artifact=block-history,",
                "record_artifact=block-history,too,few,fields\n# removed original record_artifact=",
            ),
            manifest.replace("service=rpc", "service=archive"),
            manifest.replace(
                "service=rpc,",
                "service=rpc,too,few,fields\n# removed original service=",
            ),
            manifest.replace("service_content=rpc", "service_content=archive"),
            manifest.replace(
                "service_content=rpc,",
                "service_content=rpc,too,few,fields\n# removed original service_content=",
            ),
            manifest.replace("reward_settlement_records=1", "unknown_field=1"),
            manifest.replace("reward_settlement_records=1", "malformed-line"),
        ];

        for case in cases {
            assert!(parse_public_testnet_evidence_manifest(&case).is_err());
        }
    }

    #[test]
    fn public_testnet_preflight_manifest_reports_launch_readiness() {
        let manifest = complete_public_preflight_manifest_text();
        let plan = parse_public_testnet_preflight_manifest(&manifest).unwrap();
        let report = plan.evaluate(ChainParams::default().block_time_seconds);

        assert_eq!(report.miner_count, 10);
        assert_eq!(report.validator_count, 5);
        assert_eq!(report.required_blocks, 100_800);
        assert!(report.has_required_miners);
        assert!(report.has_required_validators);
        assert!(report.has_positive_stakes);
        assert!(report.has_funded_faucet);
        assert!(report.has_cuda_kernels_available);
        assert_eq!(report.cuda_ready_miner_count, 10);
        assert!(report.has_cuda_ready_miners);
        assert_eq!(report.libp2p_ready_node_count, 15);
        assert!(report.has_libp2p_ready_nodes);
        assert!(report.has_production_libp2p_runtime);
        assert!(report.has_rpc_service_plan);
        assert!(report.has_explorer_service_plan);
        assert!(report.has_faucet_service_plan);
        assert!(report.has_telemetry_service_plan);
        assert!(report.has_public_service_content_plan);
        assert!(report.has_public_service_plan);
        assert!(report.local_shape_ready);
        assert!(report.deployment_plan_ready);
        assert!(report.can_start_public_run);

        let duplicate_service_endpoint = manifest.replace(
            &manifest_hash(b"test", b"explorer-service"),
            &manifest_hash(b"test", b"rpc-service"),
        );
        let duplicate_service_endpoint_report =
            parse_public_testnet_preflight_manifest(&duplicate_service_endpoint)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(duplicate_service_endpoint_report.has_rpc_service_plan);
        assert!(duplicate_service_endpoint_report.has_explorer_service_plan);
        assert!(duplicate_service_endpoint_report.has_public_service_content_plan);
        assert!(!duplicate_service_endpoint_report.has_public_service_plan);
        assert!(!duplicate_service_endpoint_report.deployment_plan_ready);
        assert!(!duplicate_service_endpoint_report.can_start_public_run);

        let mut missing_service_plan = plan.clone();
        missing_service_plan
            .services
            .retain(|service| service.kind != PublicServiceKind::Explorer);
        assert!(!missing_service_plan.has_distinct_ready_service_endpoint_ids());

        let rpc = plan
            .services
            .iter()
            .find(|service| service.kind == PublicServiceKind::Rpc)
            .unwrap();
        assert_eq!(public_https_host("https:///missing-host"), None);
        assert_eq!(
            public_https_host("https://rpc.tensorvm.net@localhost/health"),
            None
        );
        assert_eq!(
            public_https_host("https://rpc.tensorvm.net:bad/health"),
            None
        );
        assert_eq!(public_https_host("https://node/health"), None);
        assert_eq!(
            public_https_host("https://bad_host.tensorvm.net/health"),
            None
        );
        assert_eq!(public_https_host("https://-bad.tensorvm.net/health"), None);
        assert_eq!(
            public_https_host("https://rpc.tensorvm.net\\evil/health"),
            None
        );
        assert_eq!(public_https_host(" https://rpc.tensorvm.net/health"), None);
        assert_eq!(public_https_host("https://rpc.tensorvm.net/health "), None);
        assert_eq!(public_https_host("https://rpc.tensorvm.net/health\n"), None);
        assert_eq!(public_https_host("https://rpc.tensorvm.net/bad path"), None);
        assert_eq!(public_https_host("https://rpc.tensorvm.net /health"), None);
        assert_eq!(public_https_host("https://rpc[bad]/health"), None);
        assert_eq!(public_https_host("https://[not-ip]/health"), None);
        assert_eq!(
            public_https_host("https://2001:4860:4860::8888/health"),
            None
        );
        assert_eq!(
            public_https_host("https://rpc.tensorvm.net:443/health"),
            Some("rpc.tensorvm.net")
        );
        assert_eq!(
            public_https_path("https://rpc.tensorvm.net/health?probe=1"),
            None
        );
        assert_eq!(
            public_https_path("https://rpc.tensorvm.net/health#probe"),
            None
        );
        assert!(public_https_authorities_match(
            "https://rpc.tensorvm.net:443/health",
            "https://rpc.tensorvm.net/chain/head"
        ));
        assert!(!public_https_authorities_match(
            "https://rpc.tensorvm.net:444/health",
            "https://rpc.tensorvm.net/chain/head"
        ));
        assert!(!public_https_authorities_match(
            "https://rpc.tensorvm.net/health",
            "http://rpc.tensorvm.net/chain/head"
        ));
        assert!(!public_https_authorities_match(
            "https://rpc.tensorvm.net/health",
            "https://rpc-content.tensorvm.net/chain/head"
        ));
        assert!(public_https_authorities_match(
            "https://[2001:4860:4860::8888]/health",
            "https://[2001:4860:4860:0:0:0:0:8888]/chain/head"
        ));
        assert_eq!(public_https_host("https://[::1]:443/health"), Some("::1"));
        assert_eq!(
            public_https_host("https://[2001:4860:4860::8888]:443/health"),
            Some("2001:4860:4860::8888")
        );
        assert!(rpc.is_public_https_endpoint());
        assert!(rpc.has_public_content_surface());
        assert!(rpc.is_ready_for_public_run());
        let mut http_rpc = rpc.clone();
        http_rpc.public_url = String::from("http://rpc.tensorvm.net/health");
        assert!(!http_rpc.is_public_https_endpoint());

        let mut mismatched_health_path_rpc = rpc.clone();
        mismatched_health_path_rpc.public_url = String::from("https://rpc.tensorvm.net/wrong");
        assert!(!mismatched_health_path_rpc.is_ready_for_public_run());

        let mut root_health_path_rpc = rpc.clone();
        root_health_path_rpc.public_url = String::from("https://rpc.tensorvm.net/");
        assert!(!root_health_path_rpc.is_ready_for_public_run());

        let mut wrong_content_path_rpc = rpc.clone();
        wrong_content_path_rpc.content_url = String::from("https://rpc.tensorvm.net/wrong");
        assert!(!wrong_content_path_rpc.has_public_content_surface());
        assert!(!wrong_content_path_rpc.is_ready_for_public_run());

        let mut root_content_path_rpc = rpc.clone();
        root_content_path_rpc.content_url = String::from("https://rpc.tensorvm.net/");
        assert!(!root_content_path_rpc.has_public_content_surface());
        assert!(!root_content_path_rpc.is_ready_for_public_run());

        let mut http_content_rpc = rpc.clone();
        http_content_rpc.content_url = String::from("http://rpc.tensorvm.net/chain/head");
        assert!(!http_content_rpc.has_public_content_surface());
        assert!(!http_content_rpc.is_ready_for_public_run());

        let mut ipv6_loopback_rpc = rpc.clone();
        ipv6_loopback_rpc.public_url = String::from("https://[::1]:443/health");
        assert!(!ipv6_loopback_rpc.is_public_https_endpoint());

        let mut private_ip_rpc = rpc.clone();
        private_ip_rpc.public_url = String::from("https://10.0.0.5/health");
        assert!(!private_ip_rpc.is_public_https_endpoint());
        for host in [
            "100.64.0.1",
            "192.0.0.1",
            "192.0.2.10",
            "198.18.0.1",
            "198.51.100.10",
            "203.0.113.10",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "2001:db8::1",
            "ff02::1",
        ] {
            assert!(!public_host_is_external(host));
        }
        assert!(public_host_is_external("8.8.8.8"));
        assert!(public_host_is_external("2001:4860:4860::8888"));
        assert!(!public_host_is_external(""));
        assert!(!public_host_is_external("node"));
        assert!(!public_host_is_external("bad..tensorvm.net"));
        assert!(!public_host_is_external("123.456"));
        for host in [
            "example.com",
            "www.example.net",
            "rpc.example.test",
            "rpc.tensorvm.example",
            "operator.invalid",
        ] {
            assert!(!public_host_is_external(host));
        }

        let local_rpc = manifest.replace(
            "https://rpc.tensorvm.net/health",
            "https://localhost:8545/health",
        );
        let local_rpc_report = parse_public_testnet_preflight_manifest(&local_rpc)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!local_rpc_report.has_rpc_service_plan);
        assert!(!local_rpc_report.has_public_service_plan);
        assert!(!local_rpc_report.can_start_public_run);

        let obfuscated_local_rpc = manifest.replace(
            "https://rpc.tensorvm.net/health",
            "https://rpc.tensorvm.net@localhost/health",
        );
        let obfuscated_local_rpc_report =
            parse_public_testnet_preflight_manifest(&obfuscated_local_rpc)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(!obfuscated_local_rpc_report.has_rpc_service_plan);
        assert!(!obfuscated_local_rpc_report.has_public_service_plan);
        assert!(!obfuscated_local_rpc_report.can_start_public_run);

        let root_health_path = manifest.replace(
            "https://rpc.tensorvm.net/health,/health",
            "https://rpc.tensorvm.net/,/health",
        );
        let root_health_path_report = parse_public_testnet_preflight_manifest(&root_health_path)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!root_health_path_report.has_rpc_service_plan);
        assert!(!root_health_path_report.has_public_service_plan);
        assert!(!root_health_path_report.can_start_public_run);

        let bad_content_path = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            "https://rpc.tensorvm.net/wrong,/chain/head",
        );
        let bad_content_path_report = parse_public_testnet_preflight_manifest(&bad_content_path)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!bad_content_path_report.has_rpc_service_plan);
        assert!(!bad_content_path_report.has_public_service_content_plan);
        assert!(!bad_content_path_report.has_public_service_plan);
        assert!(!bad_content_path_report.can_start_public_run);

        let root_content_path = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            "https://rpc.tensorvm.net/,/chain/head",
        );
        let root_content_path_report = parse_public_testnet_preflight_manifest(&root_content_path)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!root_content_path_report.has_rpc_service_plan);
        assert!(!root_content_path_report.has_public_service_content_plan);
        assert!(!root_content_path_report.has_public_service_plan);
        assert!(!root_content_path_report.can_start_public_run);

        let health_url_with_space = manifest.replace(
            "https://rpc.tensorvm.net/health,/health",
            "https://rpc.tensorvm.net/health ,/health",
        );
        let health_url_with_space_report =
            parse_public_testnet_preflight_manifest(&health_url_with_space)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(!health_url_with_space_report.has_rpc_service_plan);
        assert!(!health_url_with_space_report.has_public_service_plan);
        assert!(!health_url_with_space_report.can_start_public_run);

        let content_url_with_space = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            " https://rpc.tensorvm.net/chain/head,/chain/head",
        );
        let content_url_with_space_report =
            parse_public_testnet_preflight_manifest(&content_url_with_space)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(!content_url_with_space_report.has_rpc_service_plan);
        assert!(!content_url_with_space_report.has_public_service_content_plan);
        assert!(!content_url_with_space_report.has_public_service_plan);
        assert!(!content_url_with_space_report.can_start_public_run);

        let health_query = manifest.replace(
            "https://rpc.tensorvm.net/health,/health",
            "https://rpc.tensorvm.net/health?probe=1,/health",
        );
        let health_query_report = parse_public_testnet_preflight_manifest(&health_query)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!health_query_report.has_rpc_service_plan);
        assert!(!health_query_report.has_public_service_plan);
        assert!(!health_query_report.can_start_public_run);

        let content_fragment = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            "https://rpc.tensorvm.net/chain/head#head,/chain/head",
        );
        let content_fragment_report = parse_public_testnet_preflight_manifest(&content_fragment)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!content_fragment_report.has_rpc_service_plan);
        assert!(!content_fragment_report.has_public_service_content_plan);
        assert!(!content_fragment_report.has_public_service_plan);
        assert!(!content_fragment_report.can_start_public_run);

        let mismatched_content_authority = manifest.replace(
            "https://rpc.tensorvm.net/chain/head,/chain/head",
            "https://rpc-content.tensorvm.net/chain/head,/chain/head",
        );
        let mismatched_content_authority_report =
            parse_public_testnet_preflight_manifest(&mismatched_content_authority)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(!mismatched_content_authority_report.has_rpc_service_plan);
        assert!(!mismatched_content_authority_report.has_public_service_content_plan);
        assert!(!mismatched_content_authority_report.has_public_service_plan);
        assert!(!mismatched_content_authority_report.can_start_public_run);

        let no_cuda = manifest.replace(
            "cuda_kernels_available=true",
            "cuda_kernels_available=false",
        );
        let no_cuda_report = parse_public_testnet_preflight_manifest(&no_cuda)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(no_cuda_report.local_shape_ready);
        assert!(!no_cuda_report.has_cuda_kernels_available);
        assert!(!no_cuda_report.has_cuda_ready_miners);
        assert!(!no_cuda_report.deployment_plan_ready);
        assert!(!no_cuda_report.can_start_public_run);

        let undercounted_cuda_miners =
            manifest.replace("cuda_ready_miner_count=10", "cuda_ready_miner_count=9");
        let undercounted_cuda_miner_report =
            parse_public_testnet_preflight_manifest(&undercounted_cuda_miners)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(undercounted_cuda_miner_report.has_cuda_kernels_available);
        assert_eq!(undercounted_cuda_miner_report.cuda_ready_miner_count, 9);
        assert!(!undercounted_cuda_miner_report.has_cuda_ready_miners);
        assert!(!undercounted_cuda_miner_report.deployment_plan_ready);
        assert!(!undercounted_cuda_miner_report.can_start_public_run);

        let undercounted_libp2p_nodes =
            manifest.replace("libp2p_ready_node_count=15", "libp2p_ready_node_count=14");
        let undercounted_libp2p_node_report =
            parse_public_testnet_preflight_manifest(&undercounted_libp2p_nodes)
                .unwrap()
                .evaluate(ChainParams::default().block_time_seconds);
        assert!(undercounted_libp2p_node_report.has_production_libp2p_runtime);
        assert_eq!(undercounted_libp2p_node_report.libp2p_ready_node_count, 14);
        assert!(!undercounted_libp2p_node_report.has_libp2p_ready_nodes);
        assert!(!undercounted_libp2p_node_report.deployment_plan_ready);
        assert!(!undercounted_libp2p_node_report.can_start_public_run);

        let no_auth = manifest.replace(
            "https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true",
            "https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,false,true",
        );
        let no_auth_report = parse_public_testnet_preflight_manifest(&no_auth)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!no_auth_report.has_telemetry_service_plan);
        assert!(!no_auth_report.can_start_public_run);
    }

    #[test]
    fn deployed_public_testnet_preflight_example_rejects_placeholder_domains() {
        let manifest =
            include_str!("../../../deploy/tensorvm/manifests/public-testnet.preflight.example");
        assert_public_testnet_preflight_manifest_is_pending(manifest);
    }

    #[test]
    fn docs_public_testnet_preflight_manifest_rejects_placeholder_domains() {
        let manifest = include_str!("../../../docs/tensorvm/public-testnet.preflight");
        assert_public_testnet_preflight_manifest_is_pending(manifest);
    }

    fn assert_public_testnet_preflight_manifest_is_pending(manifest: &str) {
        let plan = parse_public_testnet_preflight_manifest(manifest).unwrap();
        let report = plan.evaluate(ChainParams::default().block_time_seconds);

        assert!(report.local_shape_ready);
        assert!(!report.deployment_plan_ready);
        assert!(!report.can_start_public_run);
    }

    #[test]
    fn public_testnet_preflight_manifest_rejects_malformed_input() {
        let manifest = complete_public_preflight_manifest_text();
        let cases = [
            manifest_without_line(&manifest, "version="),
            manifest.replace(
                PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
                "tensor-vm-public-testnet-preflight-v0",
            ),
            manifest_without_line(&manifest, "miner_count="),
            manifest.replace("miner_count=10", "miner_count=abc"),
            manifest.replace(
                "cuda_kernels_available=true",
                "cuda_kernels_available=maybe",
            ),
            manifest_without_line(&manifest, "cuda_ready_miner_count="),
            manifest.replace("cuda_ready_miner_count=10", "cuda_ready_miner_count=abc"),
            manifest_without_line(&manifest, "libp2p_ready_node_count="),
            manifest.replace("libp2p_ready_node_count=15", "libp2p_ready_node_count=abc"),
            format!("{manifest}\nminer_count=10"),
            manifest.replace("miner_count=", " miner_count="),
            manifest.replace("miner_count=", "miner_count ="),
            manifest.replace("service=rpc", "service=archive"),
            manifest.replace(
                "service=rpc,",
                "service=rpc,too,few,fields\n# removed original service=",
            ),
            manifest.replace("service=rpc,", "service=rpc,zz"),
            format!("{manifest}\nunknown_field=1\n"),
            manifest.replace("service=rpc", "malformed-line"),
        ];

        for case in cases {
            assert!(parse_public_testnet_preflight_manifest(&case).is_err());
        }
    }

    #[test]
    fn public_testnet_run_evidence_filters_unsigned_and_short_lived_nodes() {
        let criteria = PublicTestnetCriteria {
            min_miners: 1,
            min_validators: 1,
            duration_days: 1,
            min_finality_rate_bps: 1,
            min_data_availability_bps: 1,
            min_invalid_work_rejections: 1,
            min_reward_settlement_records: 1,
        };
        let run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"unsigned-miner"),
                    hash_bytes(b"test", &[b"unsigned-miner-operator"]),
                    0,
                    9,
                    0,
                ),
                PublicNodeEvidence::miner(
                    address(b"late-miner"),
                    hash_bytes(b"test", &[b"late-miner-operator"]),
                    1,
                    9,
                    8,
                ),
                PublicNodeEvidence::validator(
                    address(b"zero-operator-validator"),
                    [0; 32],
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: PublicNetworkRuntimeEvidence::default(),
            services: Vec::new(),
            service_content: Vec::new(),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
            finalized_blocks: 11,
            checked_receipts: 0,
            available_receipts: 0,
            invalid_receipts_submitted: 0,
            invalid_receipts_rejected: 0,
            reward_settlement_records: 0,
        };

        assert!(run.nodes[0].covers_run(0));
        assert!(!run.nodes[0].has_external_operator_proof());
        assert!(!run.nodes[1].covers_run(run.observed_blocks));
        assert!(!run.nodes[2].has_external_operator_proof());

        let report = run.evaluate(&criteria, 6, true);
        assert_eq!(report.miner_count, 0);
        assert_eq!(report.validator_count, 0);
        assert_eq!(report.observed_duration_seconds, 60);
        assert_eq!(report.required_duration_seconds, 86_400);
        assert_eq!(report.required_blocks, 14_400);
        assert_eq!(report.finality_rate_bps, 10_000);
        assert_eq!(report.data_availability_bps, 0);
        assert_eq!(report.invalid_work_rejection_rate_bps, 0);
        assert!(!report.external_operator_evidence);
        assert!(!report.has_required_finality);
        assert!(!report.has_required_run_duration);
        assert!(!report.has_production_libp2p_runtime);
        assert!(!report.has_deployed_rpc_service);
        assert!(!report.has_deployed_explorer_service);
        assert!(!report.has_deployed_faucet_service);
        assert!(!report.has_deployed_telemetry_service);
        assert!(!report.has_deployed_public_services);
        assert!(!report.has_invalid_work_rejection_evidence);
        assert!(!report.has_reward_settlement_records);
        assert!(!report.public_criterion_met);
    }
}
