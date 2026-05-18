use crate::chain::ChainParams;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{
    PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind, PublicTestnetCriteria,
    parse_public_testnet_evidence_manifest, parse_public_testnet_preflight_manifest,
};
use crate::types::{Hash, address};
use std::net::SocketAddr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    MinerRegister {
        stake: u64,
    },
    MinerStart {
        wallet: String,
        device: String,
        node: String,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorStatus,
    ServiceInit {
        data_dir: String,
    },
    ServiceServe {
        listen: String,
        data_dir: String,
        auth_token: String,
        max_requests: usize,
    },
    PublicEvidenceValidate {
        manifest: String,
    },
    PublicEvidenceServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    PublicTestnetPreflight {
        manifest: String,
    },
}

pub fn parse_cli_args(args: &[String]) -> Result<CliCommand> {
    let parts: Vec<&str> = args.iter().map(String::as_str).collect();
    parse_cli_parts(&parts)
}

pub fn parse_cli_parts(args: &[&str]) -> Result<CliCommand> {
    match args {
        ["miner", "register", "--stake", stake] => Ok(CliCommand::MinerRegister {
            stake: parse_u64(stake)?,
        }),
        [
            "miner",
            "start",
            "--wallet",
            wallet,
            "--device",
            device,
            "--node",
            node,
        ] => Ok(CliCommand::MinerStart {
            wallet: (*wallet).to_owned(),
            device: (*device).to_owned(),
            node: (*node).to_owned(),
        }),
        ["miner", "status"] => Ok(CliCommand::MinerStatus),
        ["validator", "register", "--stake", stake] => Ok(CliCommand::ValidatorRegister {
            stake: parse_u64(stake)?,
        }),
        ["validator", "start", "--wallet", wallet, "--node", node] => {
            Ok(CliCommand::ValidatorStart {
                wallet: (*wallet).to_owned(),
                node: (*node).to_owned(),
            })
        }
        ["validator", "status"] => Ok(CliCommand::ValidatorStatus),
        ["service", "init", "--data-dir", data_dir] => Ok(CliCommand::ServiceInit {
            data_dir: (*data_dir).to_owned(),
        }),
        [
            "service",
            "serve",
            "--listen",
            listen,
            "--data-dir",
            data_dir,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::ServiceServe {
            listen: (*listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        ["public-evidence", "validate", "--manifest", manifest] => {
            Ok(CliCommand::PublicEvidenceValidate {
                manifest: (*manifest).to_owned(),
            })
        }
        [
            "public-evidence",
            "service-health",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--health-path",
            health_path,
            "--first-block",
            first_seen_block,
            "--last-block",
            last_seen_block,
            "--reachable-count",
            reachable_observation_count,
            "--signed-health-check-count",
            signed_health_check_count,
        ] => Ok(CliCommand::PublicEvidenceServiceHealth {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            health_path: (*health_path).to_owned(),
            first_seen_block: parse_u64(first_seen_block)?,
            last_seen_block: parse_u64(last_seen_block)?,
            reachable_observation_count: parse_u64(reachable_observation_count)?,
            signed_health_check_count: parse_u64(signed_health_check_count)?,
        }),
        ["public-testnet", "preflight", "--manifest", manifest] => {
            Ok(CliCommand::PublicTestnetPreflight {
                manifest: (*manifest).to_owned(),
            })
        }
        _ => Err(TvmError::InvalidReceipt("invalid cli command")),
    }
}

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { stake } => format!("register miner with stake {stake}"),
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => format!("start miner wallet={wallet} device={device} node={node}"),
        CliCommand::MinerStatus => "show miner status".to_owned(),
        CliCommand::ValidatorRegister { stake } => format!("register validator with stake {stake}"),
        CliCommand::ValidatorStart { wallet, node } => {
            format!("start validator wallet={wallet} node={node}")
        }
        CliCommand::ValidatorStatus => "show validator status".to_owned(),
        CliCommand::ServiceInit { data_dir } => {
            format!("initialize service node store data_dir={data_dir}")
        }
        CliCommand::ServiceServe {
            listen,
            data_dir,
            auth_token: _,
            max_requests,
        } => {
            format!(
                "serve RPC explorer faucet telemetry listen={listen} data_dir={data_dir} max_requests={max_requests}"
            )
        }
        CliCommand::PublicEvidenceValidate { manifest } => {
            format!("validate public evidence manifest {manifest}")
        }
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            public_url,
            health_path,
            ..
        } => {
            format!(
                "generate {} service health evidence public_url={public_url} health_path={health_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicTestnetPreflight { manifest } => {
            format!("run public testnet preflight manifest {manifest}")
        }
    }
}

pub fn execute_reference_cli_command(command: &CliCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        CliCommand::MinerRegister { stake } => {
            ensure_minimum_stake(*stake, params.miner_min_stake)?;
            Ok(format!(
                "command=miner_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.miner_min_stake
            ))
        }
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => {
            let address = wallet_address_hex(wallet)?;
            ensure_device(device)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=miner_start\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\nreference_backend_ready=true"
            ))
        }
        CliCommand::MinerStatus => Ok(format!(
            "command=miner_status\nmin_stake={}\nreference_backend_ready=true\nstatus_source=rpc_or_node_store_required",
            params.miner_min_stake
        )),
        CliCommand::ValidatorRegister { stake } => {
            ensure_minimum_stake(*stake, params.validator_min_stake)?;
            Ok(format!(
                "command=validator_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.validator_min_stake
            ))
        }
        CliCommand::ValidatorStart { wallet, node } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=validator_start\nwallet={wallet}\naddress={address}\nnode={node}\nreference_verifier_ready=true"
            ))
        }
        CliCommand::ValidatorStatus => Ok(format!(
            "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
            params.validator_min_stake
        )),
        CliCommand::ServiceInit { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_init\ndata_dir={data_dir}\nnode_store_ready=true"
            ))
        }
        CliCommand::ServiceServe {
            listen,
            data_dir,
            auth_token,
            max_requests,
        } => {
            ensure_listen_addr(listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            Ok(format!(
                "command=service_serve\nlisten={listen}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true"
            ))
        }
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            endpoint_id,
            public_url,
            health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        } => service_health_evidence_line(ServiceHealthEvidenceLine {
            kind: *kind,
            endpoint_id: *endpoint_id,
            public_url,
            health_path,
            first_seen_block: *first_seen_block,
            last_seen_block: *last_seen_block,
            reachable_observation_count: *reachable_observation_count,
            signed_health_check_count: *signed_health_check_count,
        }),
        CliCommand::PublicEvidenceValidate { .. } | CliCommand::PublicTestnetPreflight { .. } => {
            Ok(describe_command(command))
        }
    }
}

struct ServiceHealthEvidenceLine<'a> {
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &'a str,
    health_path: &'a str,
    first_seen_block: u64,
    last_seen_block: u64,
    reachable_observation_count: u64,
    signed_health_check_count: u64,
}

fn service_health_evidence_line(input: ServiceHealthEvidenceLine<'_>) -> Result<String> {
    if input.last_seen_block < input.first_seen_block {
        return Err(TvmError::InvalidReceipt(
            "service health block range is invalid",
        ));
    }
    let evidence = PublicServiceEvidence::new(
        input.kind,
        PublicServiceEndpoint::new(input.endpoint_id, input.public_url, input.health_path),
        input.first_seen_block,
        input.last_seen_block,
        input.reachable_observation_count,
        input.signed_health_check_count,
    );
    if !evidence.has_reachable_endpoint_proof() {
        return Err(TvmError::InvalidReceipt("invalid service health evidence"));
    }
    Ok(format!(
        "service={},{},{},{},{},{},{},{},{}",
        public_service_kind_tag(input.kind),
        hex(&evidence.endpoint_id),
        evidence.public_url,
        evidence.health_path,
        evidence.first_seen_block,
        evidence.last_seen_block,
        evidence.reachable_observation_count,
        evidence.signed_health_check_count,
        hex(&evidence.health_check_signature)
    ))
}

pub fn validate_public_evidence_manifest(input: &str) -> Result<String> {
    let bundle = parse_public_testnet_evidence_manifest(input)?;
    let report = bundle.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );
    Ok(format!(
        "public_evidence_full_spec={}\npublic_criterion={}\nindependently_checkable={}\npublished_evidence_bundle={}\nsigned_run_window={}\nblock_history={}\nfinality_history={}\noperator_identity_attestations={}\nnetwork_runtime_observations={}\ndata_availability_measurements={}\nminers={}\nvalidators={}\nrun_started_at_unix_seconds={}\nrun_ended_at_unix_seconds={}\nobserved_duration_seconds={}\nrequired_duration_seconds={}\nobserved_blocks={}\nrequired_blocks={}\nfinality_rate_bps={}\ndata_availability_bps={}\ninvalid_receipts_submitted={}\ninvalid_receipts_rejected={}\ninvalid_work_rejection_rate_bps={}\nreward_settlement_records={}\nexternal_operator_evidence={}\nrequired_miners={}\nrequired_validators={}\nrequired_run_duration={}\nrequired_block_count={}\nrequired_finality={}\nrequired_data_availability={}\ninvalid_work_rejection_evidence={}\nreward_settlement_evidence={}\nproduction_libp2p_runtime={}\ndeployed_rpc_service={}\ndeployed_explorer_service={}\ndeployed_faucet_service={}\ndeployed_telemetry_service={}\ndeployed_public_services={}",
        report.full_spec_evidence_met,
        report.run_evidence.public_criterion_met,
        report.independently_checkable,
        report.has_published_evidence_bundle,
        report.has_signed_run_window,
        report.has_block_history,
        report.has_finality_history,
        report.has_operator_identity_attestations,
        report.has_network_runtime_observations,
        report.has_data_availability_measurements,
        report.run_evidence.miner_count,
        report.run_evidence.validator_count,
        report.run_evidence.run_started_at_unix_seconds,
        report.run_evidence.run_ended_at_unix_seconds,
        report.run_evidence.observed_duration_seconds,
        report.run_evidence.required_duration_seconds,
        report.run_evidence.observed_blocks,
        report.run_evidence.required_blocks,
        report.run_evidence.finality_rate_bps,
        report.run_evidence.data_availability_bps,
        report.run_evidence.invalid_receipts_submitted,
        report.run_evidence.invalid_receipts_rejected,
        report.run_evidence.invalid_work_rejection_rate_bps,
        report.run_evidence.reward_settlement_records,
        report.run_evidence.external_operator_evidence,
        report.run_evidence.has_required_miners,
        report.run_evidence.has_required_validators,
        report.run_evidence.has_required_run_duration,
        report.run_evidence.has_required_block_count,
        report.run_evidence.has_required_finality,
        report.run_evidence.has_required_data_availability,
        report.run_evidence.has_invalid_work_rejection_evidence,
        report.run_evidence.has_reward_settlement_records,
        report.run_evidence.has_production_libp2p_runtime,
        report.run_evidence.has_deployed_rpc_service,
        report.run_evidence.has_deployed_explorer_service,
        report.run_evidence.has_deployed_faucet_service,
        report.run_evidence.has_deployed_telemetry_service,
        report.run_evidence.has_deployed_public_services,
    ))
}

pub fn validate_public_testnet_preflight_manifest(input: &str) -> Result<String> {
    let plan = parse_public_testnet_preflight_manifest(input)?;
    let report = plan.evaluate(ChainParams::default().block_time_seconds);
    Ok(format!(
        "public_testnet_preflight_ready={}\nlocal_shape_ready={}\ndeployment_plan_ready={}\nminers={}\nvalidators={}\nrequired_blocks={}\nrequired_miners={}\nrequired_validators={}\npositive_stakes={}\nfunded_faucet={}\ncuda_kernels_available={}\nproduction_libp2p_runtime={}\nrpc_service_plan={}\nexplorer_service_plan={}\nfaucet_service_plan={}\ntelemetry_service_plan={}\npublic_services_planned={}",
        report.can_start_public_run,
        report.local_shape_ready,
        report.deployment_plan_ready,
        report.miner_count,
        report.validator_count,
        report.required_blocks,
        report.has_required_miners,
        report.has_required_validators,
        report.has_positive_stakes,
        report.has_funded_faucet,
        report.has_cuda_kernels_available,
        report.has_production_libp2p_runtime,
        report.has_rpc_service_plan,
        report.has_explorer_service_plan,
        report.has_faucet_service_plan,
        report.has_telemetry_service_plan,
        report.has_public_service_plan,
    ))
}

fn parse_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

fn parse_usize(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

fn parse_public_service_kind(value: &str) -> Result<PublicServiceKind> {
    match value {
        "rpc" => Ok(PublicServiceKind::Rpc),
        "explorer" => Ok(PublicServiceKind::Explorer),
        "faucet" => Ok(PublicServiceKind::Faucet),
        "telemetry" => Ok(PublicServiceKind::Telemetry),
        _ => Err(TvmError::InvalidReceipt("invalid public service kind")),
    }
}

fn public_service_kind_tag(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "rpc",
        PublicServiceKind::Explorer => "explorer",
        PublicServiceKind::Faucet => "faucet",
        PublicServiceKind::Telemetry => "telemetry",
    }
}

fn parse_hash_argument(value: &str) -> Result<Hash> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash argument"));
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let high = parse_hash_nibble(value.as_bytes()[index * 2])?;
        let low = parse_hash_nibble(value.as_bytes()[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(out)
}

fn parse_hash_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid hash argument")),
    }
}

fn ensure_minimum_stake(stake: u64, minimum: u64) -> Result<()> {
    if stake < minimum {
        return Err(TvmError::InsufficientStake);
    }
    Ok(())
}

fn wallet_address_hex(wallet: &str) -> Result<String> {
    let wallet = wallet.trim();
    if wallet.is_empty() {
        return Err(TvmError::InvalidReceipt("wallet argument is empty"));
    }
    Ok(hex(&address(wallet.as_bytes())))
}

fn ensure_device(device: &str) -> Result<()> {
    if device.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("device argument is empty"));
    }
    Ok(())
}

fn ensure_node_endpoint(node: &str) -> Result<()> {
    let node = node.trim();
    if node.starts_with("http://")
        || node.starts_with("https://")
        || node.starts_with("tcp://")
        || node.starts_with("/ip4/")
        || node.starts_with("/ip6/")
        || node.starts_with("/dns/")
        || node.starts_with("/dns4/")
        || node.starts_with("/dns6/")
    {
        return Ok(());
    }
    Err(TvmError::InvalidReceipt("unsupported node endpoint"))
}

fn ensure_listen_addr(listen: &str) -> Result<()> {
    listen
        .parse::<SocketAddr>()
        .map(|_| ())
        .map_err(|_| TvmError::InvalidReceipt("invalid service listen address"))
}

fn ensure_data_dir(data_dir: &str) -> Result<()> {
    if data_dir.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("data dir argument is empty"));
    }
    Ok(())
}

fn ensure_auth_token(auth_token: &str) -> Result<()> {
    if auth_token.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("auth token argument is empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::testnet::{
        PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
        PublicEvidencePublication, PublicEvidenceRecordSummaries, PublicNetworkRuntimeEvidence,
        PublicNodeEvidence, PublicNodeRole, PublicServiceEndpoint, PublicServiceEvidence,
        PublicServiceKind, PublicTestnetEvidenceBundle, PublicTestnetRunEvidence,
    };
    use crate::types::{address, hash_bytes};

    fn manifest_hash(label: &[u8]) -> String {
        hex(&hash_bytes(b"test", &[label]))
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

    fn public_service_url(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "https://rpc.tensorvm.example/health",
            PublicServiceKind::Explorer => "https://explorer.tensorvm.example/health",
            PublicServiceKind::Faucet => "https://faucet.tensorvm.example/health",
            PublicServiceKind::Telemetry => "https://telemetry.tensorvm.example/health",
        }
    }

    fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
        let service = PublicServiceEvidence::new(
            kind,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[label]),
                public_service_url(kind),
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        hex(&service.health_check_signature)
    }

    fn manifest_publication_signature() -> String {
        let publication = PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://example.test/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        );
        hex(&publication.manifest_signature)
    }

    fn manifest_publication() -> PublicEvidencePublication {
        PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://example.test/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        )
    }

    fn manifest_bundle() -> PublicTestnetEvidenceBundle {
        PublicTestnetEvidenceBundle::new(
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
                network_runtime: PublicNetworkRuntimeEvidence {
                    libp2p_runtime_used: true,
                    peer_discovery_observed: true,
                    gossip_propagation_observed: true,
                    request_response_observed: true,
                    dos_controls_enabled: true,
                },
                services: vec![
                    PublicServiceEvidence::new(
                        PublicServiceKind::Rpc,
                        PublicServiceEndpoint::new(
                            hash_bytes(b"test", &[b"rpc-service"]),
                            public_service_url(PublicServiceKind::Rpc),
                            "/health",
                        ),
                        0,
                        9,
                        10,
                        10,
                    ),
                    PublicServiceEvidence::new(
                        PublicServiceKind::Explorer,
                        PublicServiceEndpoint::new(
                            hash_bytes(b"test", &[b"explorer-service"]),
                            public_service_url(PublicServiceKind::Explorer),
                            "/health",
                        ),
                        0,
                        9,
                        10,
                        10,
                    ),
                    PublicServiceEvidence::new(
                        PublicServiceKind::Faucet,
                        PublicServiceEndpoint::new(
                            hash_bytes(b"test", &[b"faucet-service"]),
                            public_service_url(PublicServiceKind::Faucet),
                            "/health",
                        ),
                        0,
                        9,
                        10,
                        10,
                    ),
                    PublicServiceEvidence::new(
                        PublicServiceKind::Telemetry,
                        PublicServiceEndpoint::new(
                            hash_bytes(b"test", &[b"telemetry-service"]),
                            public_service_url(PublicServiceKind::Telemetry),
                            "/health",
                        ),
                        0,
                        9,
                        10,
                        10,
                    ),
                ],
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 10,
                finalized_blocks: 10,
                checked_receipts: 20,
                available_receipts: 19,
                invalid_receipts_submitted: 1,
                invalid_receipts_rejected: 1,
                reward_settlement_records: 1,
            },
            manifest_publication(),
            PublicEvidenceRecordSummaries {
                block_history_records: 10,
                block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
                finality_history_records: 10,
                finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
                operator_identity_attestation_records: 3,
                network_runtime_observation_records: 4,
                network_runtime_observation_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                data_availability_measurement_records: 20,
                data_availability_measurement_root: hash_bytes(
                    b"test",
                    &[b"data-availability-root"],
                ),
            },
        )
    }

    fn evidence_manifest() -> String {
        format!(
            "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
public_uri=https://example.test/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
network_runtime_observation_records=4
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
reward_settlement_records=1
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.example/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.example/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.example/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.example/health,/health,0,9,10,10,{}
",
            manifest_hash(b"public-evidence-bundle"),
            manifest_address(b"public-evidence-publisher"),
            manifest_publication_signature(),
            manifest_hash(b"block-history-root"),
            hex(&manifest_bundle().block_history_signature),
            manifest_hash(b"finality-history-root"),
            hex(&manifest_bundle().finality_history_signature),
            manifest_hash(b"network-runtime-root"),
            hex(&manifest_bundle().network_runtime_observation_signature),
            manifest_hash(b"data-availability-root"),
            hex(&manifest_bundle().data_availability_measurement_signature),
            hex(&manifest_bundle().run_window_signature),
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            manifest_address(b"miner-b"),
            manifest_hash(b"miner-b-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
            manifest_address(b"validator-a"),
            manifest_hash(b"validator-a-operator"),
            manifest_node_signature(
                PublicNodeRole::Validator,
                b"validator-a",
                b"validator-a-operator"
            ),
            manifest_hash(b"rpc-service"),
            manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
            manifest_hash(b"telemetry-service"),
            manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
        )
    }

    fn preflight_manifest() -> String {
        format!(
            "\
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
            manifest_hash(b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_hash(b"telemetry-service"),
        )
    }

    #[test]
    fn parses_documented_miner_commands() {
        assert_eq!(
            parse_cli_parts(&["miner", "register", "--stake", "100"]).unwrap(),
            CliCommand::MinerRegister { stake: 100 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "miner",
                "start",
                "--wallet",
                "miner.key",
                "--device",
                "cuda:0",
                "--node",
                "http://localhost:8545"
            ])
            .unwrap(),
            CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cuda:0".to_owned(),
                node: "http://localhost:8545".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["miner", "status"]).unwrap(),
            CliCommand::MinerStatus
        );
    }

    #[test]
    fn parses_documented_validator_commands() {
        assert_eq!(
            parse_cli_parts(&["validator", "register", "--stake", "10000"]).unwrap(),
            CliCommand::ValidatorRegister { stake: 10_000 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "validator",
                "start",
                "--wallet",
                "validator.key",
                "--node",
                "http://localhost:8545"
            ])
            .unwrap(),
            CliCommand::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "http://localhost:8545".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["validator", "status"]).unwrap(),
            CliCommand::ValidatorStatus
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "validate",
                "--manifest",
                "docs/tensorvm/public-testnet.evidence"
            ])
            .unwrap(),
            CliCommand::PublicEvidenceValidate {
                manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-testnet",
                "preflight",
                "--manifest",
                "docs/tensorvm/public-testnet.preflight"
            ])
            .unwrap(),
            CliCommand::PublicTestnetPreflight {
                manifest: "docs/tensorvm/public-testnet.preflight".to_owned(),
            }
        );
        let endpoint_id = manifest_hash(b"rpc-service");
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-health",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.example/health",
                "--health-path",
                "/health",
                "--first-block",
                "0",
                "--last-block",
                "9",
                "--reachable-count",
                "10",
                "--signed-health-check-count",
                "10",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            }
        );
        assert_eq!(
            parse_cli_parts(&["service", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
            CliCommand::ServiceInit {
                data_dir: "/var/lib/tensorvm".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "service",
                "serve",
                "--listen",
                "0.0.0.0:8545",
                "--data-dir",
                "/var/lib/tensorvm",
                "--auth-token",
                "secret",
                "--max-requests",
                "0",
            ])
            .unwrap(),
            CliCommand::ServiceServe {
                listen: "0.0.0.0:8545".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            }
        );
    }

    #[test]
    fn rejects_invalid_cli() {
        assert!(parse_cli_parts(&["miner", "register"]).is_err());
        assert!(parse_cli_parts(&["validator", "register", "--stake", "abc"]).is_err());
    }

    #[test]
    fn parse_cli_args_and_describe_commands() {
        let args = vec![
            "miner".to_owned(),
            "register".to_owned(),
            "--stake".to_owned(),
            "250".to_owned(),
        ];
        let command = parse_cli_args(&args).unwrap();
        assert_eq!(command, CliCommand::MinerRegister { stake: 250 });

        let commands = [
            (
                CliCommand::MinerRegister { stake: 1 },
                "register miner with stake 1",
            ),
            (
                CliCommand::MinerStart {
                    wallet: "miner.key".to_owned(),
                    device: "cuda:0".to_owned(),
                    node: "http://localhost:8545".to_owned(),
                },
                "start miner wallet=miner.key device=cuda:0 node=http://localhost:8545",
            ),
            (CliCommand::MinerStatus, "show miner status"),
            (
                CliCommand::ValidatorRegister { stake: 10 },
                "register validator with stake 10",
            ),
            (
                CliCommand::ValidatorStart {
                    wallet: "validator.key".to_owned(),
                    node: "http://localhost:8545".to_owned(),
                },
                "start validator wallet=validator.key node=http://localhost:8545",
            ),
            (CliCommand::ValidatorStatus, "show validator status"),
            (
                CliCommand::ServiceInit {
                    data_dir: "/var/lib/tensorvm".to_owned(),
                },
                "initialize service node store data_dir=/var/lib/tensorvm",
            ),
            (
                CliCommand::ServiceServe {
                    listen: "0.0.0.0:8545".to_owned(),
                    data_dir: "/var/lib/tensorvm".to_owned(),
                    auth_token: "secret".to_owned(),
                    max_requests: 0,
                },
                "serve RPC explorer faucet telemetry listen=0.0.0.0:8545 data_dir=/var/lib/tensorvm max_requests=0",
            ),
            (
                CliCommand::PublicEvidenceValidate {
                    manifest: "evidence.txt".to_owned(),
                },
                "validate public evidence manifest evidence.txt",
            ),
            (
                CliCommand::PublicTestnetPreflight {
                    manifest: "preflight.txt".to_owned(),
                },
                "run public testnet preflight manifest preflight.txt",
            ),
        ];
        for (command, description) in commands {
            assert_eq!(describe_command(&command), description);
        }

        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            }),
            "generate rpc service health evidence public_url=https://rpc.tensorvm.example/health health_path=/health"
        );
    }

    #[test]
    fn execute_reference_cli_command_reports_miner_and_validator_readiness() {
        let miner_register =
            execute_reference_cli_command(&CliCommand::MinerRegister { stake: 100 }).unwrap();
        assert!(miner_register.contains("command=miner_register"));
        assert!(miner_register.contains("min_stake=100"));
        assert!(miner_register.contains("stake_sufficient=true"));

        let miner_start = execute_reference_cli_command(&CliCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:0".to_owned(),
            node: "http://localhost:8545".to_owned(),
        })
        .unwrap();
        assert!(miner_start.contains("command=miner_start"));
        assert!(miner_start.contains("wallet=miner.key"));
        assert!(miner_start.contains("device=cuda:0"));
        assert!(miner_start.contains("node=http://localhost:8545"));
        assert!(miner_start.contains(&format!("address={}", hex(&address(b"miner.key")))));
        assert!(miner_start.contains("reference_backend_ready=true"));

        let validator_register =
            execute_reference_cli_command(&CliCommand::ValidatorRegister { stake: 10_000 })
                .unwrap();
        assert!(validator_register.contains("command=validator_register"));
        assert!(validator_register.contains("min_stake=10000"));

        let validator_start = execute_reference_cli_command(&CliCommand::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .unwrap();
        assert!(validator_start.contains("command=validator_start"));
        assert!(validator_start.contains("reference_verifier_ready=true"));

        let miner_status = execute_reference_cli_command(&CliCommand::MinerStatus).unwrap();
        assert!(miner_status.contains("command=miner_status"));
        assert!(miner_status.contains("status_source=rpc_or_node_store_required"));

        let validator_status = execute_reference_cli_command(&CliCommand::ValidatorStatus).unwrap();
        assert!(validator_status.contains("command=validator_status"));
        assert!(validator_status.contains("status_source=rpc_or_node_store_required"));

        let service_init = execute_reference_cli_command(&CliCommand::ServiceInit {
            data_dir: "/var/lib/tensorvm".to_owned(),
        })
        .unwrap();
        assert!(service_init.contains("command=service_init"));
        assert!(service_init.contains("node_store_ready=true"));

        let service_serve = execute_reference_cli_command(&CliCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            auth_token: "secret".to_owned(),
            max_requests: 0,
        })
        .unwrap();
        assert!(service_serve.contains("command=service_serve"));
        assert!(service_serve.contains("auth_enabled=true"));
        assert!(service_serve.contains("rpc_routes=enabled"));
        assert!(service_serve.contains("explorer_routes=enabled"));
        assert!(service_serve.contains("faucet_routes=enabled"));
        assert!(service_serve.contains("telemetry_routes=enabled"));
        assert!(service_serve.contains("node_store_required=true"));

        let public_command = CliCommand::PublicEvidenceValidate {
            manifest: "evidence.txt".to_owned(),
        };
        assert_eq!(
            execute_reference_cli_command(&public_command).unwrap(),
            describe_command(&public_command)
        );

        let service_health =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .unwrap();
        assert!(service_health.starts_with("service=rpc,"));
        assert!(service_health.contains("https://rpc.tensorvm.example/health,/health,0,9,10,10"));
        assert!(service_health.ends_with(&manifest_service_signature(
            PublicServiceKind::Rpc,
            b"rpc-service"
        )));
        let additional_service_cases: [(PublicServiceKind, &[u8], &str); 3] = [
            (PublicServiceKind::Explorer, b"explorer-service", "explorer"),
            (PublicServiceKind::Faucet, b"faucet-service", "faucet"),
            (
                PublicServiceKind::Telemetry,
                b"telemetry-service",
                "telemetry",
            ),
        ];
        for (kind, label, tag) in additional_service_cases {
            let line = execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind,
                endpoint_id: hash_bytes(b"test", &[label]),
                public_url: public_service_url(kind).to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .unwrap();
            assert!(line.starts_with(&format!("service={tag},")));
            assert!(line.contains(public_service_url(kind)));
            assert!(line.ends_with(&manifest_service_signature(kind, label)));
        }
    }

    #[test]
    fn execute_reference_cli_command_rejects_invalid_local_args() {
        assert!(execute_reference_cli_command(&CliCommand::MinerRegister { stake: 99 }).is_err());
        assert!(
            execute_reference_cli_command(&CliCommand::ValidatorRegister { stake: 9_999 }).is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: " ".to_owned(),
                device: "cuda:0".to_owned(),
                node: "http://localhost:8545".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: " ".to_owned(),
                node: "http://localhost:8545".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "localhost:8545".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceInit {
                data_dir: " ".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "localhost:8545".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "127.0.0.1:8545".to_owned(),
                data_dir: " ".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "127.0.0.1:8545".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: " ".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            parse_cli_parts(&[
                "service",
                "serve",
                "--listen",
                "127.0.0.1:8545",
                "--data-dir",
                "/var/lib/tensorvm",
                "--auth-token",
                "secret",
                "--max-requests",
                "abc",
            ])
            .is_err()
        );
        assert!(parse_public_service_kind("archive").is_err());
        assert!(parse_hash_argument("12").is_err());
        assert!(parse_hash_argument(&"g".repeat(64)).is_err());
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "http://127.0.0.1/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 10,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: [0; 32],
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.example/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 0,
                signed_health_check_count: 10,
            })
            .is_err()
        );
    }

    #[test]
    fn validate_public_evidence_manifest_reports_default_criteria_status() {
        let report = validate_public_evidence_manifest(&evidence_manifest()).unwrap();
        assert!(report.contains("public_evidence_full_spec=false"));
        assert!(report.contains("public_criterion=false"));
        assert!(report.contains("independently_checkable=true"));
        assert!(report.contains("published_evidence_bundle=true"));
        assert!(report.contains("signed_run_window=true"));
        assert!(report.contains("block_history=true"));
        assert!(report.contains("finality_history=true"));
        assert!(report.contains("operator_identity_attestations=true"));
        assert!(report.contains("network_runtime_observations=true"));
        assert!(report.contains("data_availability_measurements=true"));
        assert!(report.contains("miners=2"));
        assert!(report.contains("validators=1"));
        assert!(report.contains("run_started_at_unix_seconds=1700000000"));
        assert!(report.contains("run_ended_at_unix_seconds=1700000060"));
        assert!(report.contains("observed_duration_seconds=60"));
        assert!(report.contains("required_duration_seconds=604800"));
        assert!(report.contains("observed_blocks=10"));
        assert!(report.contains("required_blocks=100800"));
        assert!(report.contains("finality_rate_bps=10000"));
        assert!(report.contains("data_availability_bps=9500"));
        assert!(report.contains("invalid_receipts_submitted=1"));
        assert!(report.contains("invalid_receipts_rejected=1"));
        assert!(report.contains("invalid_work_rejection_rate_bps=10000"));
        assert!(report.contains("reward_settlement_records=1"));
        assert!(report.contains("external_operator_evidence=true"));
        assert!(report.contains("required_miners=false"));
        assert!(report.contains("required_validators=false"));
        assert!(report.contains("required_run_duration=false"));
        assert!(report.contains("required_block_count=false"));
        assert!(report.contains("required_finality=true"));
        assert!(report.contains("required_data_availability=true"));
        assert!(report.contains("invalid_work_rejection_evidence=true"));
        assert!(report.contains("reward_settlement_evidence=true"));
        assert!(report.contains("production_libp2p_runtime=true"));
        assert!(report.contains("deployed_rpc_service=true"));
        assert!(report.contains("deployed_explorer_service=true"));
        assert!(report.contains("deployed_faucet_service=true"));
        assert!(report.contains("deployed_telemetry_service=true"));
        assert!(report.contains("deployed_public_services=true"));

        let insufficient_operator_records = evidence_manifest().replace(
            "operator_identity_attestation_records=3",
            "operator_identity_attestation_records=2",
        );
        let insufficient_operator_report =
            validate_public_evidence_manifest(&insufficient_operator_records).unwrap();
        assert!(insufficient_operator_report.contains("operator_identity_attestations=false"));
        assert!(insufficient_operator_report.contains("external_operator_evidence=false"));
        assert!(insufficient_operator_report.contains("public_criterion=false"));

        assert!(validate_public_evidence_manifest("bad-manifest").is_err());
    }

    #[test]
    fn validate_public_testnet_preflight_manifest_reports_launch_readiness() {
        let report = validate_public_testnet_preflight_manifest(&preflight_manifest()).unwrap();
        assert!(report.contains("public_testnet_preflight_ready=true"));
        assert!(report.contains("local_shape_ready=true"));
        assert!(report.contains("deployment_plan_ready=true"));
        assert!(report.contains("miners=10"));
        assert!(report.contains("validators=5"));
        assert!(report.contains("required_blocks=100800"));
        assert!(report.contains("required_miners=true"));
        assert!(report.contains("required_validators=true"));
        assert!(report.contains("positive_stakes=true"));
        assert!(report.contains("funded_faucet=true"));
        assert!(report.contains("cuda_kernels_available=true"));
        assert!(report.contains("production_libp2p_runtime=true"));
        assert!(report.contains("rpc_service_plan=true"));
        assert!(report.contains("explorer_service_plan=true"));
        assert!(report.contains("faucet_service_plan=true"));
        assert!(report.contains("telemetry_service_plan=true"));
        assert!(report.contains("public_services_planned=true"));

        assert!(validate_public_testnet_preflight_manifest("bad-manifest").is_err());
    }
}
