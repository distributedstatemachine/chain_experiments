use super::KeyValueReportWriter;
#[cfg(feature = "cuda-kernels")]
use crate::runtime::cuda_device_count;
use crate::{
    chain::ChainParams,
    hash::hex,
    runtime::cuda_kernels_compiled,
    types::{Address, address},
};

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

pub fn validate_miner_runtime(
    wallet: &str,
    device: &str,
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    wallet_address(wallet)?;
    miner_device_readiness(device)?;
    validate_service_runtime(data_dir, auth_token)
}

pub fn validate_role_runtime(
    wallet: &str,
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    wallet_address(wallet)?;
    validate_service_runtime(data_dir, auth_token)
}

pub fn validate_service_runtime(
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    ensure_non_empty(data_dir, "data dir")?;
    ensure_non_empty(auth_token, "auth token")
}

pub fn validate_data_dir(data_dir: &str) -> std::result::Result<(), String> {
    ensure_non_empty(data_dir, "data dir")
}

fn ensure_minimum_stake(stake: u64, minimum: u64) -> std::result::Result<(), String> {
    if stake < minimum {
        return Err("insufficient stake".to_owned());
    }
    Ok(())
}

fn ensure_non_empty(value: &str, name: &'static str) -> std::result::Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} argument is empty"));
    }
    Ok(())
}

fn wallet_address_hex(wallet: &str) -> std::result::Result<String, String> {
    Ok(hex(&wallet_address(wallet)?))
}

fn wallet_address(wallet: &str) -> std::result::Result<Address, String> {
    ensure_non_empty(wallet, "wallet")?;
    Ok(address(wallet.as_bytes()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MinerDeviceReadiness {
    CpuReference,
    #[cfg(feature = "cuda-kernels")]
    Cuda {
        device_index: u32,
        device_count: u32,
    },
}

impl MinerDeviceReadiness {
    fn write_report_fields(&self, report: &mut KeyValueReportWriter) {
        match self {
            Self::CpuReference => {
                report.field("device_backend", "cpu-reference");
                report.field("cuda_kernels_compiled", cuda_kernels_compiled());
            }
            #[cfg(feature = "cuda-kernels")]
            Self::Cuda {
                device_index,
                device_count,
            } => {
                report.field("device_backend", "cuda");
                report.field("gpu_backend_ready", true);
                report.field("cuda_kernels_compiled", true);
                report.field("cuda_device_index", device_index);
                report.field("cuda_device_count", device_count);
            }
        }
    }
}

fn miner_device_readiness(device: &str) -> std::result::Result<MinerDeviceReadiness, String> {
    let device = device.trim();
    if device.is_empty() {
        return Err("device argument is empty".to_owned());
    }
    if matches!(device, "cpu" | "cpu-reference") {
        return Ok(MinerDeviceReadiness::CpuReference);
    }

    let Some(cuda_index) = device.strip_prefix("cuda:") else {
        return Err("unsupported miner device".to_owned());
    };
    if cuda_index.is_empty() {
        return Err("invalid cuda device".to_owned());
    }
    let device_index = cuda_index
        .parse::<u32>()
        .map_err(|_| "invalid cuda device".to_owned())?;
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let _ = device_index;
        Err("cuda kernels not compiled".to_owned())
    }
    #[cfg(feature = "cuda-kernels")]
    {
        let device_count = cuda_device_count().map_err(|error| error.to_string())?;
        if device_index >= device_count {
            return Err("cuda device unavailable".to_owned());
        }
        Ok(MinerDeviceReadiness::Cuda {
            device_index,
            device_count,
        })
    }
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
}
