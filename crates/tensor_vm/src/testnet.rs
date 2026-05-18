use crate::chain::{BlockVote, ChainParams, JobState, LocalChain, TensorBlock, Transaction};
use crate::error::{Result, TvmError};
use crate::explorer::ExplorerSummary;
use crate::faucet::Faucet;
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
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub const PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-evidence-v1";
pub const PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION: &str = "tensor-vm-public-testnet-preflight-v1";

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

    pub fn is_ready_for_public_run(&self) -> bool {
        self.endpoint_id != [0; 32]
            && self.is_public_https_endpoint()
            && self.health_path.starts_with('/')
            && self.health_path.len() > 1
            && self.auth_enabled
            && self.rate_limit_enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetPreflightPlan {
    pub config: TestnetConfig,
    pub criteria: PublicTestnetCriteria,
    pub cuda_kernels_available: bool,
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
    pub has_production_libp2p_runtime: bool,
    pub has_rpc_service_plan: bool,
    pub has_explorer_service_plan: bool,
    pub has_faucet_service_plan: bool,
    pub has_telemetry_service_plan: bool,
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
        let has_production_libp2p_runtime = self.network_runtime.has_production_libp2p_runtime();
        let has_rpc_service_plan = self.has_ready_service_plan(PublicServiceKind::Rpc);
        let has_explorer_service_plan = self.has_ready_service_plan(PublicServiceKind::Explorer);
        let has_faucet_service_plan = self.has_ready_service_plan(PublicServiceKind::Faucet);
        let has_telemetry_service_plan = self.has_ready_service_plan(PublicServiceKind::Telemetry);
        let has_public_service_plan = has_rpc_service_plan
            && has_explorer_service_plan
            && has_faucet_service_plan
            && has_telemetry_service_plan;
        let local_shape_ready = has_required_miners
            && has_required_validators
            && has_positive_stakes
            && has_funded_faucet
            && required_blocks > 0;
        let deployment_plan_ready =
            self.cuda_kernels_available && has_production_libp2p_runtime && has_public_service_plan;
        PublicTestnetPreflightReport {
            miner_count: self.config.miner_count,
            validator_count: self.config.validator_count,
            required_blocks,
            has_required_miners,
            has_required_validators,
            has_positive_stakes,
            has_funded_faucet,
            has_cuda_kernels_available: self.cuda_kernels_available,
            has_production_libp2p_runtime,
            has_rpc_service_plan,
            has_explorer_service_plan,
            has_faucet_service_plan,
            has_telemetry_service_plan,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidence {
    pub miner_count: usize,
    pub validator_count: usize,
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
    pub has_deployed_public_services: bool,
    pub has_required_miners: bool,
    pub has_required_validators: bool,
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
pub struct PublicServiceEvidence {
    pub kind: PublicServiceKind,
    pub endpoint_id: Hash,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
    pub reachable_observation_count: u64,
    pub signed_health_check_count: u64,
    pub health_check_signature: Signature,
}

impl PublicServiceEvidence {
    pub fn new(
        kind: PublicServiceKind,
        endpoint_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    ) -> Self {
        let message = public_service_health_message(
            kind,
            &endpoint_id,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        );
        Self {
            kind,
            endpoint_id,
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
                &self.endpoint_id,
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
            && self.reachable_observation_count > 0
            && self.signed_health_check_count > 0
            && self.signed_health_check_valid()
    }

    pub fn is_reachable_for_run(&self, observed_blocks: u64) -> bool {
        self.covers_run(observed_blocks) && self.has_reachable_endpoint_proof()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEvidencePublication {
    pub bundle_id: Hash,
    pub public_uri: String,
    pub manifest_signature_count: u64,
    pub independent_auditor_count: u64,
}

impl PublicEvidencePublication {
    pub fn is_published_and_independently_checkable(&self) -> bool {
        let uri = self.public_uri.trim();
        self.bundle_id != [0; 32]
            && public_evidence_uri_is_external(uri)
            && self.manifest_signature_count > 0
            && self.independent_auditor_count > 0
    }
}

pub fn parse_public_testnet_evidence_manifest(input: &str) -> Result<PublicTestnetEvidenceBundle> {
    let mut builder = PublicEvidenceManifestBuilder::default();
    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or(TvmError::InvalidReceipt("malformed evidence manifest line"))?;
        builder.set(key.trim(), value.trim())?;
    }
    builder.finish()
}

pub fn parse_public_testnet_preflight_manifest(input: &str) -> Result<PublicTestnetPreflightPlan> {
    let mut builder = PublicTestnetPreflightManifestBuilder::default();
    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(TvmError::InvalidReceipt(
            "malformed preflight manifest line",
        ))?;
        builder.set(key.trim(), value.trim())?;
    }
    builder.finish()
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
        self.operator_id != [0; 32]
            && self.signed_heartbeat_count > 0
            && self.heartbeat_signature_valid()
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetRunEvidence {
    pub nodes: Vec<PublicNodeEvidence>,
    pub network_runtime: PublicNetworkRuntimeEvidence,
    pub services: Vec<PublicServiceEvidence>,
    pub observed_blocks: u64,
    pub finalized_blocks: u64,
    pub checked_receipts: u64,
    pub available_receipts: u64,
    pub invalid_receipts_submitted: u64,
    pub invalid_receipts_rejected: u64,
    pub reward_settlement_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundle {
    pub run: PublicTestnetRunEvidence,
    pub publication: PublicEvidencePublication,
    pub block_history_records: u64,
    pub finality_history_records: u64,
    pub operator_identity_attestation_records: u64,
    pub data_availability_measurement_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicTestnetEvidenceBundleReport {
    pub run_evidence: PublicTestnetEvidence,
    pub has_published_evidence_bundle: bool,
    pub has_block_history: bool,
    pub has_finality_history: bool,
    pub has_operator_identity_attestations: bool,
    pub has_data_availability_measurements: bool,
    pub independently_checkable: bool,
    pub full_spec_evidence_met: bool,
}

#[derive(Default)]
struct PublicEvidenceManifestBuilder {
    version_seen: bool,
    bundle_id: Option<Hash>,
    public_uri: Option<String>,
    manifest_signature_count: Option<u64>,
    independent_auditor_count: Option<u64>,
    block_history_records: Option<u64>,
    finality_history_records: Option<u64>,
    operator_identity_attestation_records: Option<u64>,
    data_availability_measurement_records: Option<u64>,
    libp2p_runtime_used: Option<bool>,
    peer_discovery_observed: Option<bool>,
    gossip_propagation_observed: Option<bool>,
    request_response_observed: Option<bool>,
    dos_controls_enabled: Option<bool>,
    nodes: Vec<PublicNodeEvidence>,
    services: Vec<PublicServiceEvidence>,
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
    libp2p_runtime_used: Option<bool>,
    peer_discovery_observed: Option<bool>,
    gossip_propagation_observed: Option<bool>,
    request_response_observed: Option<bool>,
    dos_controls_enabled: Option<bool>,
    services: Vec<PublicDeploymentServicePlan>,
}

impl PublicTestnetPreflightManifestBuilder {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "version" => {
                if value != PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION {
                    return Err(TvmError::InvalidReceipt(
                        "unsupported preflight manifest version",
                    ));
                }
                self.version_seen = true;
            }
            "miner_count" => self.miner_count = Some(parse_manifest_usize(value)?),
            "validator_count" => self.validator_count = Some(parse_manifest_usize(value)?),
            "miner_stake" => self.miner_stake = Some(parse_manifest_u64(value)?),
            "validator_stake" => self.validator_stake = Some(parse_manifest_u64(value)?),
            "faucet_balance" => self.faucet_balance = Some(parse_manifest_u64(value)?),
            "faucet_drip" => self.faucet_drip = Some(parse_manifest_u64(value)?),
            "cuda_kernels_available" => {
                self.cuda_kernels_available = Some(parse_manifest_bool(value)?);
            }
            "libp2p_runtime_used" => self.libp2p_runtime_used = Some(parse_manifest_bool(value)?),
            "peer_discovery_observed" => {
                self.peer_discovery_observed = Some(parse_manifest_bool(value)?);
            }
            "gossip_propagation_observed" => {
                self.gossip_propagation_observed = Some(parse_manifest_bool(value)?);
            }
            "request_response_observed" => {
                self.request_response_observed = Some(parse_manifest_bool(value)?);
            }
            "dos_controls_enabled" => self.dos_controls_enabled = Some(parse_manifest_bool(value)?),
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
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() != 6 {
        return Err(TvmError::InvalidReceipt("malformed preflight service plan"));
    }
    Ok(PublicDeploymentServicePlan {
        kind: parse_service_kind(fields[0])?,
        endpoint_id: parse_hash_hex(fields[1])?,
        public_url: fields[2].to_owned(),
        health_path: fields[3].to_owned(),
        auth_enabled: parse_manifest_bool(fields[4])?,
        rate_limit_enabled: parse_manifest_bool(fields[5])?,
    })
}

impl PublicEvidenceManifestBuilder {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "version" => {
                if value != PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION {
                    return Err(TvmError::InvalidReceipt(
                        "unsupported evidence manifest version",
                    ));
                }
                self.version_seen = true;
            }
            "bundle_id" => self.bundle_id = Some(parse_hash_hex(value)?),
            "public_uri" => self.public_uri = Some(value.to_owned()),
            "manifest_signature_count" => {
                self.manifest_signature_count = Some(parse_manifest_u64(value)?);
            }
            "independent_auditor_count" => {
                self.independent_auditor_count = Some(parse_manifest_u64(value)?);
            }
            "block_history_records" => {
                self.block_history_records = Some(parse_manifest_u64(value)?);
            }
            "finality_history_records" => {
                self.finality_history_records = Some(parse_manifest_u64(value)?);
            }
            "operator_identity_attestation_records" => {
                self.operator_identity_attestation_records = Some(parse_manifest_u64(value)?);
            }
            "data_availability_measurement_records" => {
                self.data_availability_measurement_records = Some(parse_manifest_u64(value)?);
            }
            "libp2p_runtime_used" => self.libp2p_runtime_used = Some(parse_manifest_bool(value)?),
            "peer_discovery_observed" => {
                self.peer_discovery_observed = Some(parse_manifest_bool(value)?);
            }
            "gossip_propagation_observed" => {
                self.gossip_propagation_observed = Some(parse_manifest_bool(value)?);
            }
            "request_response_observed" => {
                self.request_response_observed = Some(parse_manifest_bool(value)?);
            }
            "dos_controls_enabled" => self.dos_controls_enabled = Some(parse_manifest_bool(value)?),
            "node" => self.nodes.push(parse_manifest_node(value)?),
            "service" => self.services.push(parse_manifest_service(value)?),
            "observed_blocks" => self.observed_blocks = Some(parse_manifest_u64(value)?),
            "finalized_blocks" => self.finalized_blocks = Some(parse_manifest_u64(value)?),
            "checked_receipts" => self.checked_receipts = Some(parse_manifest_u64(value)?),
            "available_receipts" => self.available_receipts = Some(parse_manifest_u64(value)?),
            "invalid_receipts_submitted" => {
                self.invalid_receipts_submitted = Some(parse_manifest_u64(value)?);
            }
            "invalid_receipts_rejected" => {
                self.invalid_receipts_rejected = Some(parse_manifest_u64(value)?);
            }
            "reward_settlement_records" => {
                self.reward_settlement_records = Some(parse_manifest_u64(value)?);
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
                observed_blocks: required_u64(self.observed_blocks)?,
                finalized_blocks: required_u64(self.finalized_blocks)?,
                checked_receipts: required_u64(self.checked_receipts)?,
                available_receipts: required_u64(self.available_receipts)?,
                invalid_receipts_submitted: required_u64(self.invalid_receipts_submitted)?,
                invalid_receipts_rejected: required_u64(self.invalid_receipts_rejected)?,
                reward_settlement_records: required_u64(self.reward_settlement_records)?,
            },
            publication: PublicEvidencePublication {
                bundle_id: required_hash(self.bundle_id)?,
                public_uri: required_string(self.public_uri)?,
                manifest_signature_count: required_u64(self.manifest_signature_count)?,
                independent_auditor_count: required_u64(self.independent_auditor_count)?,
            },
            block_history_records: required_u64(self.block_history_records)?,
            finality_history_records: required_u64(self.finality_history_records)?,
            operator_identity_attestation_records: required_u64(
                self.operator_identity_attestation_records,
            )?,
            data_availability_measurement_records: required_u64(
                self.data_availability_measurement_records,
            )?,
        })
    }
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

fn parse_manifest_service(value: &str) -> Result<PublicServiceEvidence> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() != 7 {
        return Err(TvmError::InvalidReceipt("malformed service evidence"));
    }
    let kind = parse_service_kind(fields[0])?;
    let endpoint_id = parse_hash_hex(fields[1])?;
    let first_seen_block = parse_manifest_u64(fields[2])?;
    let last_seen_block = parse_manifest_u64(fields[3])?;
    let reachable_observation_count = parse_manifest_u64(fields[4])?;
    let signed_health_check_count = parse_manifest_u64(fields[5])?;
    let mut evidence = PublicServiceEvidence::new(
        kind,
        endpoint_id,
        first_seen_block,
        last_seen_block,
        reachable_observation_count,
        signed_health_check_count,
    );
    evidence.health_check_signature = parse_hash_hex(fields[6])?;
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
    let rest = url.trim().strip_prefix("https://")?;
    let authority = rest.split('/').next().unwrap_or_default();
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        &bracketed[..end]
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    (!host.is_empty()).then_some(host)
}

fn public_host_is_external(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let lowercase_host = host.to_ascii_lowercase();
    if lowercase_host == "localhost" || lowercase_host.ends_with(".local") {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => public_ipv4_is_external(ip),
        Ok(IpAddr::V6(ip)) => public_ipv6_is_external(ip),
        Err(_) => true,
    }
}

fn public_ipv4_is_external(ip: Ipv4Addr) -> bool {
    !(ip.is_loopback() || ip.is_unspecified() || ip.is_private() || ip.is_link_local())
}

fn public_ipv6_is_external(ip: Ipv6Addr) -> bool {
    !(ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local() || ip.is_unicast_link_local())
}

fn public_evidence_uri_is_external(uri: &str) -> bool {
    if let Some(host) = public_https_host(uri) {
        return public_host_is_external(host);
    }
    content_addressed_uri_has_identifier(uri, "ipfs://")
        || content_addressed_uri_has_identifier(uri, "ar://")
}

fn content_addressed_uri_has_identifier(uri: &str, scheme: &str) -> bool {
    let Some(rest) = uri.strip_prefix(scheme) else {
        return false;
    };
    rest.trim_matches('/')
        .split('/')
        .next()
        .is_some_and(|identifier| !identifier.is_empty())
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

fn public_service_health_message(
    kind: PublicServiceKind,
    endpoint_id: &Hash,
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
            endpoint_id,
            &first_seen,
            &last_seen,
            &reachable_count,
            &signed_count,
        ],
    )
}

impl PublicTestnetEvidenceBundle {
    pub fn evaluate(
        &self,
        criteria: &PublicTestnetCriteria,
        block_time_seconds: u64,
        external_operator_evidence: bool,
    ) -> PublicTestnetEvidenceBundleReport {
        let run_evidence =
            self.run
                .evaluate(criteria, block_time_seconds, external_operator_evidence);
        let has_published_evidence_bundle =
            self.publication.is_published_and_independently_checkable();
        let has_block_history =
            self.run.observed_blocks > 0 && self.block_history_records >= self.run.observed_blocks;
        let has_finality_history = self.run.observed_blocks > 0
            && self.finality_history_records >= self.run.observed_blocks;
        let required_operator_attestations =
            (run_evidence.miner_count + run_evidence.validator_count) as u64;
        let has_operator_identity_attestations = required_operator_attestations > 0
            && self.operator_identity_attestation_records >= required_operator_attestations;
        let has_data_availability_measurements = self.run.checked_receipts > 0
            && self.data_availability_measurement_records >= self.run.checked_receipts;
        let independently_checkable = has_published_evidence_bundle
            && has_block_history
            && has_finality_history
            && has_operator_identity_attestations
            && has_data_availability_measurements
            && run_evidence.has_invalid_work_rejection_evidence
            && run_evidence.has_reward_settlement_records;
        let full_spec_evidence_met = run_evidence.public_criterion_met && independently_checkable;
        PublicTestnetEvidenceBundleReport {
            run_evidence,
            has_published_evidence_bundle,
            has_block_history,
            has_finality_history,
            has_operator_identity_attestations,
            has_data_availability_measurements,
            independently_checkable,
            full_spec_evidence_met,
        }
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
        let finality_rate_bps = ratio_parts_to_bps(self.finalized_blocks, self.observed_blocks);
        let data_availability_bps =
            ratio_parts_to_bps(self.available_receipts, self.checked_receipts);
        let invalid_work_rejection_rate_bps = ratio_parts_to_bps(
            self.invalid_receipts_rejected,
            self.invalid_receipts_submitted,
        );
        let has_required_miners = miner_count >= criteria.min_miners;
        let has_required_validators = validator_count >= criteria.min_validators;
        let has_required_block_count = self.observed_blocks >= required_blocks;
        let has_required_finality = finality_rate_bps >= criteria.min_finality_rate_bps;
        let has_required_data_availability =
            data_availability_bps >= criteria.min_data_availability_bps;
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
        let has_deployed_rpc_service = self.has_reachable_service(PublicServiceKind::Rpc);
        let has_deployed_explorer_service = self.has_reachable_service(PublicServiceKind::Explorer);
        let has_deployed_faucet_service = self.has_reachable_service(PublicServiceKind::Faucet);
        let has_deployed_telemetry_service =
            self.has_reachable_service(PublicServiceKind::Telemetry);
        let has_deployed_public_services = has_deployed_rpc_service
            && has_deployed_explorer_service
            && has_deployed_faucet_service
            && has_deployed_telemetry_service;
        let public_criterion_met = has_required_miners
            && has_required_validators
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
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
            has_required_block_count,
            has_required_finality,
            has_required_data_availability,
            has_invalid_work_rejection_evidence,
            has_reward_settlement_records,
            public_criterion_met,
        }
    }

    fn has_reachable_service(&self, kind: PublicServiceKind) -> bool {
        self.services.iter().any(|service| {
            service.kind == kind && service.is_reachable_for_run(self.observed_blocks)
        })
    }

    fn independent_operator_counts(&self) -> (usize, usize) {
        let mut miners = BTreeSet::new();
        let mut validators = BTreeSet::new();
        for node in &self.nodes {
            if !node.covers_run(self.observed_blocks) || !node.has_external_operator_proof() {
                continue;
            }
            match node.role {
                PublicNodeRole::Miner => {
                    miners.insert(node.operator_id);
                }
                PublicNodeRole::Validator => {
                    validators.insert(node.operator_id);
                }
            }
        }
        (miners.len(), validators.len())
    }
}

#[derive(Clone, Debug)]
pub struct LocalTestnet {
    pub chain: LocalChain,
    pub faucet: Faucet,
    pub miners: Vec<Address>,
    pub validators: Vec<Address>,
}

impl LocalTestnet {
    pub fn new(config: TestnetConfig, finalized_randomness: Hash) -> Self {
        let params = ChainParams::default();
        let mut chain = LocalChain::with_params(params, finalized_randomness);
        let mut miners = Vec::with_capacity(config.miner_count);
        let mut validators = Vec::with_capacity(config.validator_count);
        for i in 0..config.miner_count {
            let miner = address(format!("testnet-miner-{i}").as_bytes());
            chain.register_miner(miner, config.miner_stake).unwrap();
            miners.push(miner);
        }
        for i in 0..config.validator_count {
            let validator = address(format!("testnet-validator-{i}").as_bytes());
            chain
                .register_validator(validator, config.validator_stake)
                .unwrap();
            validators.push(validator);
        }
        Self {
            chain,
            faucet: Faucet::new(config.faucet_balance, config.faucet_drip),
            miners,
            validators,
        }
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
        let observed_blocks = self.chain.blocks.len() as u64;
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
        let has_deployed_public_services = false;
        let public_criterion_met = false;
        PublicTestnetEvidence {
            miner_count: self.miners.len(),
            validator_count: self.validators.len(),
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
            has_deployed_public_services,
            has_required_miners,
            has_required_validators,
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
    days.saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        / block_time_seconds.max(1)
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
        PublicServiceEvidence::new(
            kind,
            hash_bytes(b"test", &[label]),
            first_seen_block,
            last_seen_block,
            10,
            10,
        )
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
        PublicTestnetEvidenceBundle {
            run: complete_public_run_evidence(),
            publication: PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: String::from("https://example.test/tensorvm/public-evidence.json"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            },
            block_history_records: 10,
            finality_history_records: 10,
            operator_identity_attestation_records: 3,
            data_availability_measurement_records: 20,
        }
    }

    fn manifest_hash(domain: &[u8], label: &[u8]) -> String {
        hex(&hash_bytes(domain, &[label]))
    }

    fn manifest_address(label: &[u8]) -> String {
        hex(&address(label))
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

    fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
        hex(&public_service(kind, label, 0, 9).health_check_signature)
    }

    fn complete_public_evidence_manifest_text() -> String {
        format!(
            "\
# TensorVM external public evidence manifest
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}

bundle_id=0x{}
public_uri=https://example.test/tensorvm/public-evidence.json
manifest_signature_count=1
independent_auditor_count=1
block_history_records=10
finality_history_records=10
operator_identity_attestation_records=3
data_availability_measurement_records=20
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
reward_settlement_records=1
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},0,9,10,10,{}
service=explorer,{},0,9,10,10,{}
service=faucet,{},0,9,10,10,{}
service=telemetry,{},0,9,10,10,{}
",
            manifest_hash(b"test", b"public-evidence-bundle"),
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
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.example/health,/health,true,true
service=explorer,{},https://explorer.tensorvm.example/health,/health,true,true
service=faucet,{},https://faucet.tensorvm.example/health,/health,true,true
service=telemetry,{},https://telemetry.tensorvm.example/health,/health,true,true
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
        assert!(sufficient.has_required_block_count);
        assert!(sufficient.has_required_finality);
        assert!(sufficient.has_required_data_availability);
        assert!(sufficient.has_invalid_work_rejection_evidence);
        assert!(sufficient.has_reward_settlement_records);
        assert!(sufficient.has_production_libp2p_runtime);
        assert!(sufficient.has_deployed_public_services);
        assert!(sufficient.public_criterion_met);

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
        assert!(complete.has_deployed_public_services);
        assert!(complete.public_criterion_met);

        run.services[0].health_check_signature = [8; 32];
        let tampered_rpc_health = run.evaluate(&criteria, 6, true);
        assert!(!tampered_rpc_health.has_deployed_rpc_service);
        assert!(!tampered_rpc_health.has_deployed_public_services);
        assert!(!tampered_rpc_health.public_criterion_met);
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

        let complete = bundle.evaluate(&criteria, 6, true);
        assert!(complete.run_evidence.public_criterion_met);
        assert!(complete.has_published_evidence_bundle);
        assert!(complete.has_block_history);
        assert!(complete.has_finality_history);
        assert!(complete.has_operator_identity_attestations);
        assert!(complete.has_data_availability_measurements);
        assert!(complete.independently_checkable);
        assert!(complete.full_spec_evidence_met);

        bundle.publication.public_uri = String::from("http://localhost:8545/evidence.json");
        let local_uri = bundle.evaluate(&criteria, 6, true);
        assert!(!local_uri.has_published_evidence_bundle);
        assert!(!local_uri.independently_checkable);
        assert!(!local_uri.full_spec_evidence_met);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("https://localhost/evidence.json");
        let localhost_https_uri = bundle.evaluate(&criteria, 6, true);
        assert!(!localhost_https_uri.has_published_evidence_bundle);
        assert!(!localhost_https_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("https://192.168.1.2/evidence.json");
        let private_https_uri = bundle.evaluate(&criteria, 6, true);
        assert!(!private_https_uri.has_published_evidence_bundle);
        assert!(!private_https_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.publication.public_uri = String::from("ipfs://");
        let empty_ipfs_uri = bundle.evaluate(&criteria, 6, true);
        assert!(!empty_ipfs_uri.has_published_evidence_bundle);
        assert!(!empty_ipfs_uri.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.block_history_records = 9;
        let missing_block_history = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_block_history.has_block_history);
        assert!(!missing_block_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.finality_history_records = 9;
        let missing_finality_history = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_finality_history.has_finality_history);
        assert!(!missing_finality_history.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.operator_identity_attestation_records = 2;
        let missing_operator_attestations = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_operator_attestations.has_operator_identity_attestations);
        assert!(!missing_operator_attestations.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.data_availability_measurement_records = 19;
        let missing_data_availability_measurements = bundle.evaluate(&criteria, 6, true);
        assert!(!missing_data_availability_measurements.has_data_availability_measurements);
        assert!(!missing_data_availability_measurements.independently_checkable);

        bundle = complete_public_evidence_bundle();
        bundle.run.services.clear();
        let missing_services = bundle.evaluate(&criteria, 6, true);
        assert!(missing_services.independently_checkable);
        assert!(!missing_services.run_evidence.public_criterion_met);
        assert!(!missing_services.full_spec_evidence_met);
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
        assert!(parsed.evaluate(&criteria, 6, true).full_spec_evidence_met);

        let false_runtime =
            manifest.replace("libp2p_runtime_used=true", "libp2p_runtime_used=false");
        let parsed_false_runtime = parse_public_testnet_evidence_manifest(&false_runtime).unwrap();
        assert!(!parsed_false_runtime.run.network_runtime.libp2p_runtime_used);
        assert!(
            !parsed_false_runtime
                .evaluate(&criteria, 6, true)
                .full_spec_evidence_met
        );

        let uppercase_hash = manifest_hash(b"test", b"public-evidence-bundle").to_uppercase();
        assert_eq!(
            parse_hash_hex(&uppercase_hash).unwrap(),
            hash_bytes(b"test", &[b"public-evidence-bundle"])
        );
        assert!(parse_hash_hex(&format!("z{}", "0".repeat(63))).is_err());
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
            manifest_without_line(&manifest, "observed_blocks="),
            manifest_without_line(&manifest, "dos_controls_enabled="),
            manifest.replace("bundle_id=0x", "bundle_id=0x12"),
            manifest.replace("bundle_id=0x", "bundle_id=0xz"),
            manifest.replace("manifest_signature_count=1", "manifest_signature_count=abc"),
            manifest.replace("dos_controls_enabled=true", "dos_controls_enabled=maybe"),
            manifest.replace("node=miner", "node=archive"),
            manifest.replace(
                "node=miner,",
                "node=miner,too,few,fields\n# removed original node=",
            ),
            manifest.replace("service=rpc", "service=archive"),
            manifest.replace(
                "service=rpc,",
                "service=rpc,too,few,fields\n# removed original service=",
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
        assert!(report.has_production_libp2p_runtime);
        assert!(report.has_rpc_service_plan);
        assert!(report.has_explorer_service_plan);
        assert!(report.has_faucet_service_plan);
        assert!(report.has_telemetry_service_plan);
        assert!(report.has_public_service_plan);
        assert!(report.local_shape_ready);
        assert!(report.deployment_plan_ready);
        assert!(report.can_start_public_run);

        let rpc = plan
            .services
            .iter()
            .find(|service| service.kind == PublicServiceKind::Rpc)
            .unwrap();
        assert_eq!(public_https_host("https:///missing-host"), None);
        assert_eq!(
            public_https_host("https://rpc.tensorvm.example:443/health"),
            Some("rpc.tensorvm.example")
        );
        assert_eq!(public_https_host("https://[::1]:443/health"), Some("::1"));
        assert!(rpc.is_public_https_endpoint());
        assert!(rpc.is_ready_for_public_run());
        let mut http_rpc = rpc.clone();
        http_rpc.public_url = String::from("http://rpc.tensorvm.example/health");
        assert!(!http_rpc.is_public_https_endpoint());

        let mut ipv6_loopback_rpc = rpc.clone();
        ipv6_loopback_rpc.public_url = String::from("https://[::1]:443/health");
        assert!(!ipv6_loopback_rpc.is_public_https_endpoint());

        let mut private_ip_rpc = rpc.clone();
        private_ip_rpc.public_url = String::from("https://10.0.0.5/health");
        assert!(!private_ip_rpc.is_public_https_endpoint());

        let local_rpc = manifest.replace(
            "https://rpc.tensorvm.example/health",
            "https://localhost:8545/health",
        );
        let local_rpc_report = parse_public_testnet_preflight_manifest(&local_rpc)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!local_rpc_report.has_rpc_service_plan);
        assert!(!local_rpc_report.has_public_service_plan);
        assert!(!local_rpc_report.can_start_public_run);

        let no_cuda = manifest.replace(
            "cuda_kernels_available=true",
            "cuda_kernels_available=false",
        );
        let no_cuda_report = parse_public_testnet_preflight_manifest(&no_cuda)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(no_cuda_report.local_shape_ready);
        assert!(!no_cuda_report.has_cuda_kernels_available);
        assert!(!no_cuda_report.deployment_plan_ready);
        assert!(!no_cuda_report.can_start_public_run);

        let no_auth = manifest.replace(
            "https://telemetry.tensorvm.example/health,/health,true,true",
            "https://telemetry.tensorvm.example/health,/health,false,true",
        );
        let no_auth_report = parse_public_testnet_preflight_manifest(&no_auth)
            .unwrap()
            .evaluate(ChainParams::default().block_time_seconds);
        assert!(!no_auth_report.has_telemetry_service_plan);
        assert!(!no_auth_report.can_start_public_run);
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
        assert_eq!(report.required_blocks, 14_400);
        assert_eq!(report.finality_rate_bps, 10_000);
        assert_eq!(report.data_availability_bps, 0);
        assert_eq!(report.invalid_work_rejection_rate_bps, 0);
        assert!(!report.external_operator_evidence);
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
