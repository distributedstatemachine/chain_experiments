use super::{
    KeyValueReportWriter, miner_device_readiness::miner_device_readiness,
    operator_validation::wallet_address,
};
use crate::{chain::ChainParams, hash::hex};

pub fn check_miner_registration(stake: u64) -> std::result::Result<String, String> {
    let params = ChainParams::default();
    ensure_minimum_stake(stake, params.miner_min_stake)?;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "miner_register");
    report.field("stake", stake);
    report.field("min_stake", params.miner_min_stake);
    report.field("stake_sufficient", true);
    Ok(report.finish())
}

pub fn check_validator_registration(stake: u64) -> std::result::Result<String, String> {
    let params = ChainParams::default();
    ensure_minimum_stake(stake, params.validator_min_stake)?;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "validator_register");
    report.field("stake", stake);
    report.field("min_stake", params.validator_min_stake);
    report.field("stake_sufficient", true);
    Ok(report.finish())
}

pub fn check_miner_start(
    wallet: &str,
    device: &str,
    node: &str,
) -> std::result::Result<String, String> {
    let address = wallet_address_hex(wallet)?;
    let device_readiness = miner_device_readiness(device)?;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "miner_start");
    report.field("wallet", wallet);
    report.field("address", address);
    report.field("device", device);
    report.field("node", node);
    device_readiness.write_report_fields(&mut report);
    report.field("reference_backend_ready", true);
    Ok(report.finish())
}

pub fn check_validator_start(wallet: &str, node: &str) -> std::result::Result<String, String> {
    let address = wallet_address_hex(wallet)?;
    let mut report = KeyValueReportWriter::new();
    report.field("command", "validator_start");
    report.field("wallet", wallet);
    report.field("address", address);
    report.field("node", node);
    report.field("reference_verifier_ready", true);
    Ok(report.finish())
}

pub fn miner_status() -> String {
    let params = ChainParams::default();
    let mut report = KeyValueReportWriter::new();
    report.field("command", "miner_status");
    report.field("min_stake", params.miner_min_stake);
    report.field("reference_backend_ready", true);
    report.field("status_source", "rpc_or_node_store_required");
    report.finish()
}

pub fn validator_status() -> String {
    let params = ChainParams::default();
    let mut report = KeyValueReportWriter::new();
    report.field("command", "validator_status");
    report.field("min_stake", params.validator_min_stake);
    report.field("reference_verifier_ready", true);
    report.field("status_source", "rpc_or_node_store_required");
    report.finish()
}

fn ensure_minimum_stake(stake: u64, minimum: u64) -> std::result::Result<(), String> {
    if stake < minimum {
        return Err("insufficient stake".to_owned());
    }
    Ok(())
}

fn wallet_address_hex(wallet: &str) -> std::result::Result<String, String> {
    Ok(hex(&wallet_address(wallet)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::KeyValueReport;

    fn report_fields(report: &str) -> KeyValueReport<'_> {
        KeyValueReport::parse_strict(report).expect("operator report must parse")
    }

    #[test]
    fn operator_check_reports_are_parseable() {
        let miner_register = check_miner_registration(ChainParams::default().miner_min_stake)
            .expect("miner registration report");
        let miner_register = report_fields(&miner_register);
        assert_eq!(miner_register.value("command"), Some("miner_register"));
        assert_eq!(miner_register.value("stake_sufficient"), Some("true"));

        let validator_register =
            check_validator_registration(ChainParams::default().validator_min_stake)
                .expect("validator registration report");
        let validator_register = report_fields(&validator_register);
        assert_eq!(
            validator_register.value("command"),
            Some("validator_register")
        );
        assert_eq!(validator_register.value("stake_sufficient"), Some("true"));

        let miner_start = check_miner_start("miner.key", "cpu", "/ip4/127.0.0.1/tcp/4001")
            .expect("miner check report");
        let miner_start = report_fields(&miner_start);
        assert_eq!(miner_start.value("command"), Some("miner_start"));
        assert_eq!(miner_start.value("wallet"), Some("miner.key"));
        assert_eq!(miner_start.value("device_backend"), Some("cpu-reference"));
        assert_eq!(miner_start.value("reference_backend_ready"), Some("true"));

        let validator_start = check_validator_start("validator.key", "/ip4/127.0.0.1/tcp/4001")
            .expect("validator check report");
        let validator_start = report_fields(&validator_start);
        assert_eq!(validator_start.value("command"), Some("validator_start"));
        assert_eq!(
            validator_start.value("reference_verifier_ready"),
            Some("true")
        );
    }

    #[test]
    fn operator_status_reports_are_parseable() {
        let miner_status = miner_status();
        let miner_status = report_fields(&miner_status);
        assert_eq!(miner_status.value("command"), Some("miner_status"));
        assert_eq!(
            miner_status.value("status_source"),
            Some("rpc_or_node_store_required")
        );

        let validator_status = validator_status();
        let validator_status = report_fields(&validator_status);
        assert_eq!(validator_status.value("command"), Some("validator_status"));
        assert_eq!(
            validator_status.value("reference_verifier_ready"),
            Some("true")
        );
    }

    #[test]
    fn operator_checks_reject_retired_miner_device_spelling() {
        assert_eq!(
            check_miner_start("miner.key", "cpu-reference", "/ip4/127.0.0.1/tcp/4001").unwrap_err(),
            "unsupported miner device"
        );
    }
}
