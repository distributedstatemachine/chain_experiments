use super::public_manifest_fields::{
    exact_manifest_record_fields, exact_manifest_scalar, parse_hash_hex, parse_manifest_bool,
    parse_manifest_u64, parse_manifest_usize, parse_service_kind, reject_manifest_key_whitespace,
    required_bool, required_u64, required_usize,
};
use super::{
    PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION, PublicDeploymentServicePlan,
    PublicNetworkRuntimeEvidence, PublicTestnetCriteria, PublicTestnetPreflightPlan, TestnetConfig,
};
use crate::error::{Result, TvmError};
use std::collections::BTreeSet;

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
        let scalar = exact_manifest_scalar(value)?;
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
    let fields = exact_manifest_record_fields(value, 8, "malformed preflight service plan")?;
    Ok(PublicDeploymentServicePlan {
        kind: parse_service_kind(fields[0])?,
        endpoint_id: parse_hash_hex(fields[1])?,
        public_url: fields[2].to_owned(),
        health_path: fields[3].to_owned(),
        content_url: fields[4].to_owned(),
        content_path: fields[5].to_owned(),
        auth_enabled: parse_manifest_bool(fields[6])?,
        rate_limit_enabled: parse_manifest_bool(fields[7])?,
    })
}
