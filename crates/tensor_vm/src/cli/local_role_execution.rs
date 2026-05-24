use super::commands::{MinerCommand, ProposerCommand, ValidatorCommand, ValidatorRunArgs};
use super::local_fixture_reports::{
    write_default_libp2p_limit_fields, write_libp2p_fixture_fields,
};
use super::validation::{
    ensure_auth_token, ensure_data_dir, miner_device_readiness, path_argument, wallet_address_hex,
};
use crate::app::{
    KeyValueReportWriter, check_miner_registration, check_miner_start,
    check_validator_registration, check_validator_start, miner_status, p2p_identity_report,
    validator_status,
};
use crate::error::{Result, TvmError};

pub(super) fn execute_miner_command(command: &MinerCommand) -> Result<String> {
    match command {
        MinerCommand::Register(args) => operator_check_result(check_miner_registration(args.stake)),
        MinerCommand::Check(args) => operator_check_result(check_miner_start(
            &path_argument(&args.wallet),
            &args.device,
            &args.node.to_string(),
        )),
        MinerCommand::Run(args) => {
            let runtime = &args.runtime;
            let node_runtime = &runtime.node_runtime;
            let address = wallet_address_hex(&args.wallet)?;
            let device_readiness = miner_device_readiness(&args.device)?;
            ensure_data_dir(&node_runtime.data_dir)?;
            ensure_auth_token(&node_runtime.auth_token)?;
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
            write_default_libp2p_limit_fields(&mut report);
            report.field("data_dir", data_dir);
            report.field("auth_enabled", true);
            report.field("max_requests", node_runtime.max_requests);
            report.field("role_runtime_ready", true);
            Ok(report.finish())
        }
        MinerCommand::Status => Ok(miner_status()),
    }
}

pub(super) fn execute_validator_command(command: &ValidatorCommand) -> Result<String> {
    match command {
        ValidatorCommand::Register(args) => {
            operator_check_result(check_validator_registration(args.stake))
        }
        ValidatorCommand::Check(args) => operator_check_result(check_validator_start(
            &path_argument(&args.wallet),
            &args.node.to_string(),
        )),
        ValidatorCommand::Run(args) => execute_validator_run("validator", args),
        ValidatorCommand::Status => Ok(validator_status()),
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
            write_default_libp2p_limit_fields(&mut report);
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
    write_default_libp2p_limit_fields(&mut report);
    report.field("data_dir", data_dir);
    report.field("auth_enabled", true);
    report.field("max_requests", node_runtime.max_requests);
    report.field("reference_verifier_ready", true);
    report.field("role_runtime_ready", true);
    Ok(report.finish())
}

fn operator_check_result(result: std::result::Result<String, String>) -> Result<String> {
    result.map_err(operator_check_error)
}

fn operator_check_error(error: String) -> TvmError {
    match error.as_str() {
        "insufficient stake" => TvmError::InsufficientStake,
        "wallet argument is empty" => TvmError::InvalidReceipt("wallet argument is empty"),
        "device argument is empty" => TvmError::InvalidReceipt("device argument is empty"),
        "unsupported miner device" => TvmError::InvalidReceipt("unsupported miner device"),
        "invalid cuda device" => TvmError::InvalidReceipt("invalid cuda device"),
        "cuda kernels not compiled" => TvmError::InvalidReceipt("cuda kernels not compiled"),
        "cuda device unavailable" => TvmError::InvalidReceipt("cuda device unavailable"),
        _ => TvmError::InvalidReceipt("operator check failed"),
    }
}
