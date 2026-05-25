use super::miner_device_readiness::miner_device_readiness;
use crate::types::{Address, address};

pub(super) fn validate_miner_runtime(
    wallet: &str,
    device: &str,
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    wallet_address(wallet)?;
    miner_device_readiness(device)?;
    validate_service_runtime(data_dir, auth_token)
}

pub(super) fn validate_role_runtime(
    wallet: &str,
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    wallet_address(wallet)?;
    validate_service_runtime(data_dir, auth_token)
}

pub(super) fn validate_service_runtime(
    data_dir: &str,
    auth_token: &str,
) -> std::result::Result<(), String> {
    ensure_non_empty(data_dir, "data dir")?;
    ensure_non_empty(auth_token, "auth token")
}

pub(super) fn validate_data_dir(data_dir: &str) -> std::result::Result<(), String> {
    ensure_non_empty(data_dir, "data dir")
}

pub(super) fn wallet_address(wallet: &str) -> std::result::Result<Address, String> {
    ensure_non_empty(wallet, "wallet")?;
    Ok(address(wallet.as_bytes()))
}

fn ensure_non_empty(value: &str, name: &'static str) -> std::result::Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} argument is empty"));
    }
    Ok(())
}
