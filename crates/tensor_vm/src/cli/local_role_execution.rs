use super::commands::{MinerCommand, ProposerCommand, ValidatorCommand, ValidatorRunArgs};
use super::validation::{
    ensure_auth_token, ensure_data_dir, ensure_minimum_stake, miner_device_readiness,
    path_argument, wallet_address_hex,
};
use crate::app::{KeyValueReportWriter, p2p_identity_report};
use crate::chain::ChainParams;
use crate::error::Result;
use crate::p2p::Libp2pControlPlaneConfig;

pub(super) fn execute_miner_command(
    command: &MinerCommand,
    params: &ChainParams,
) -> Result<String> {
    match command {
        MinerCommand::Register(args) => {
            ensure_minimum_stake(args.stake, params.miner_min_stake)?;
            let mut report = KeyValueReportWriter::new();
            report.field("command", "miner_register");
            report.field("stake", args.stake);
            report.field("min_stake", params.miner_min_stake);
            report.field("stake_sufficient", true);
            Ok(report.finish())
        }
        MinerCommand::Check(args) => {
            let address = wallet_address_hex(&args.wallet)?;
            let device_readiness = miner_device_readiness(&args.device)?;
            let wallet = path_argument(&args.wallet);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "miner_start");
            report.field("wallet", wallet);
            report.field("address", address);
            report.field("device", &args.device);
            report.field("node", &args.node);
            report.append_report(&device_readiness.report());
            report.field("reference_backend_ready", true);
            Ok(report.finish())
        }
        MinerCommand::Run(args) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let address = wallet_address_hex(&args.wallet)?;
            let device_readiness = miner_device_readiness(&args.device)?;
            ensure_data_dir(&node_runtime.data_dir)?;
            ensure_auth_token(&node_runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity =
                p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
            let wallet = path_argument(&args.wallet);
            let data_dir = path_argument(&node_runtime.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "miner_run");
            report.field("role", "miner");
            report.field("wallet", wallet);
            report.field("address", address);
            report.field("device", &args.device);
            report.field("node", &runtime.node);
            report.field("listen", node_runtime.listen);
            report.field("p2p_listen", &node_runtime.p2p_listen);
            write_libp2p_fixture_fields(&mut report);
            report.append_report(&device_readiness.report());
            report.append_report(&identity);
            write_libp2p_limit_fields(&mut report, &p2p_config);
            report.field("data_dir", data_dir);
            report.field("auth_enabled", true);
            report.field("max_requests", node_runtime.max_requests);
            report.field("role_runtime_ready", true);
            Ok(report.finish())
        }
        MinerCommand::Status => {
            let mut report = KeyValueReportWriter::new();
            report.field("command", "miner_status");
            report.field("min_stake", params.miner_min_stake);
            report.field("reference_backend_ready", true);
            report.field("status_source", "rpc_or_node_store_required");
            Ok(report.finish())
        }
    }
}

pub(super) fn execute_validator_command(
    command: &ValidatorCommand,
    params: &ChainParams,
) -> Result<String> {
    match command {
        ValidatorCommand::Register(args) => {
            ensure_minimum_stake(args.stake, params.validator_min_stake)?;
            let mut report = KeyValueReportWriter::new();
            report.field("command", "validator_register");
            report.field("stake", args.stake);
            report.field("min_stake", params.validator_min_stake);
            report.field("stake_sufficient", true);
            Ok(report.finish())
        }
        ValidatorCommand::Check(args) => {
            let address = wallet_address_hex(&args.wallet)?;
            let wallet = path_argument(&args.wallet);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "validator_start");
            report.field("wallet", wallet);
            report.field("address", address);
            report.field("node", &args.node);
            report.field("reference_verifier_ready", true);
            Ok(report.finish())
        }
        ValidatorCommand::Run(args) => execute_validator_run("validator", args),
        ValidatorCommand::Status => {
            let mut report = KeyValueReportWriter::new();
            report.field("command", "validator_status");
            report.field("min_stake", params.validator_min_stake);
            report.field("reference_verifier_ready", true);
            report.field("status_source", "rpc_or_node_store_required");
            Ok(report.finish())
        }
    }
}

pub(super) fn execute_proposer_command(command: &ProposerCommand) -> Result<String> {
    match command {
        ProposerCommand::Run(args) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let address = wallet_address_hex(&args.wallet)?;
            ensure_data_dir(&node_runtime.data_dir)?;
            ensure_auth_token(&node_runtime.auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity =
                p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
            let wallet = path_argument(&args.wallet);
            let data_dir = path_argument(&node_runtime.data_dir);
            let mut report = KeyValueReportWriter::new();
            report.field("command", "proposer_run");
            report.field("role", "proposer");
            report.field("wallet", wallet);
            report.field("address", address);
            report.field("node", &runtime.node);
            report.field("listen", node_runtime.listen);
            report.field("p2p_listen", &node_runtime.p2p_listen);
            write_libp2p_fixture_fields(&mut report);
            report.append_report(&identity);
            write_libp2p_limit_fields(&mut report, &p2p_config);
            report.field("data_dir", data_dir);
            report.field("auth_enabled", true);
            report.field("max_requests", node_runtime.max_requests);
            report.field("proposer_ready", true);
            report.field("role_runtime_ready", true);
            Ok(report.finish())
        }
    }
}

fn execute_validator_run(role: &str, args: &ValidatorRunArgs) -> Result<String> {
    let runtime = &args.runtime;
    let node_runtime = &runtime.node_runtime;
    let address = wallet_address_hex(&args.wallet)?;
    ensure_data_dir(&node_runtime.data_dir)?;
    ensure_auth_token(&node_runtime.auth_token)?;
    let p2p_config = Libp2pControlPlaneConfig::default();
    let identity = p2p_identity_report(node_runtime.identity_seed.map(|seed| seed.into_hash()));
    let wallet = path_argument(&args.wallet);
    let data_dir = path_argument(&node_runtime.data_dir);
    let mut report = KeyValueReportWriter::new();
    report.field("command", format!("{role}_run"));
    report.field("role", role);
    report.field("wallet", wallet);
    report.field("address", address);
    report.field("node", &runtime.node);
    report.field("listen", node_runtime.listen);
    report.field("p2p_listen", &node_runtime.p2p_listen);
    write_libp2p_fixture_fields(&mut report);
    report.append_report(&identity);
    write_libp2p_limit_fields(&mut report, &p2p_config);
    report.field("data_dir", data_dir);
    report.field("auth_enabled", true);
    report.field("max_requests", node_runtime.max_requests);
    report.field("reference_verifier_ready", true);
    report.field("role_runtime_ready", true);
    Ok(report.finish())
}

fn write_libp2p_fixture_fields(report: &mut KeyValueReportWriter) {
    report.field("p2p_runtime", "libp2p");
    report.field("p2p_gossipsub", "enabled");
    report.field("p2p_identify", "enabled");
    report.field("p2p_kademlia", "enabled");
    report.field("p2p_request_response", "enabled");
}

fn write_libp2p_limit_fields(
    report: &mut KeyValueReportWriter,
    p2p_config: &Libp2pControlPlaneConfig,
) {
    report.field(
        "p2p_max_transmit_bytes",
        p2p_config.max_gossipsub_transmit_bytes,
    );
    report.field(
        "p2p_request_timeout_seconds",
        p2p_config.request_timeout_seconds,
    );
    report.field(
        "p2p_max_concurrent_streams",
        p2p_config.max_concurrent_request_streams,
    );
    report.field(
        "p2p_idle_timeout_seconds",
        p2p_config.idle_connection_timeout_seconds,
    );
}
