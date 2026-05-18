use crate::chain::ChainParams;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::testnet::{
    PublicTestnetCriteria, parse_public_testnet_evidence_manifest,
    parse_public_testnet_preflight_manifest,
};
use crate::types::address;

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
    PublicEvidenceValidate {
        manifest: String,
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
        ["public-evidence", "validate", "--manifest", manifest] => {
            Ok(CliCommand::PublicEvidenceValidate {
                manifest: (*manifest).to_owned(),
            })
        }
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
        CliCommand::PublicEvidenceValidate { manifest } => {
            format!("validate public evidence manifest {manifest}")
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
        CliCommand::PublicEvidenceValidate { .. } | CliCommand::PublicTestnetPreflight { .. } => {
            Ok(describe_command(command))
        }
    }
}

pub fn validate_public_evidence_manifest(input: &str) -> Result<String> {
    let bundle = parse_public_testnet_evidence_manifest(input)?;
    let report = bundle.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );
    Ok(format!(
        "public_evidence_full_spec={}\npublic_criterion={}\nindependently_checkable={}\npublished_evidence_bundle={}\nblock_history={}\nfinality_history={}\noperator_identity_attestations={}\ndata_availability_measurements={}\nminers={}\nvalidators={}\nobserved_blocks={}\nrequired_blocks={}\nfinality_rate_bps={}\ndata_availability_bps={}\ninvalid_receipts_submitted={}\ninvalid_receipts_rejected={}\ninvalid_work_rejection_rate_bps={}\nreward_settlement_records={}\nexternal_operator_evidence={}\nrequired_miners={}\nrequired_validators={}\nrequired_block_count={}\nrequired_finality={}\nrequired_data_availability={}\ninvalid_work_rejection_evidence={}\nreward_settlement_evidence={}\nproduction_libp2p_runtime={}\ndeployed_rpc_service={}\ndeployed_explorer_service={}\ndeployed_faucet_service={}\ndeployed_telemetry_service={}\ndeployed_public_services={}",
        report.full_spec_evidence_met,
        report.run_evidence.public_criterion_met,
        report.independently_checkable,
        report.has_published_evidence_bundle,
        report.has_block_history,
        report.has_finality_history,
        report.has_operator_identity_attestations,
        report.has_data_availability_measurements,
        report.run_evidence.miner_count,
        report.run_evidence.validator_count,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::testnet::{
        PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
        PublicNodeEvidence, PublicNodeRole, PublicServiceEvidence, PublicServiceKind,
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

    fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
        let service = PublicServiceEvidence::new(kind, hash_bytes(b"test", &[label]), 0, 9, 10, 10);
        hex(&service.health_check_signature)
    }

    fn evidence_manifest() -> String {
        format!(
            "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
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
            manifest_hash(b"public-evidence-bundle"),
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

        let public_command = CliCommand::PublicEvidenceValidate {
            manifest: "evidence.txt".to_owned(),
        };
        assert_eq!(
            execute_reference_cli_command(&public_command).unwrap(),
            describe_command(&public_command)
        );
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
    }

    #[test]
    fn validate_public_evidence_manifest_reports_default_criteria_status() {
        let report = validate_public_evidence_manifest(&evidence_manifest()).unwrap();
        assert!(report.contains("public_evidence_full_spec=false"));
        assert!(report.contains("public_criterion=false"));
        assert!(report.contains("independently_checkable=true"));
        assert!(report.contains("published_evidence_bundle=true"));
        assert!(report.contains("block_history=true"));
        assert!(report.contains("finality_history=true"));
        assert!(report.contains("operator_identity_attestations=true"));
        assert!(report.contains("data_availability_measurements=true"));
        assert!(report.contains("miners=2"));
        assert!(report.contains("validators=1"));
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
