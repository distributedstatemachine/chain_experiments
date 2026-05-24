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
    Ok(format!(
        "command=miner_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
        params.miner_min_stake
    ))
}

pub fn check_validator_registration(stake: u64) -> std::result::Result<String, String> {
    let params = ChainParams::default();
    ensure_minimum_stake(stake, params.validator_min_stake)?;
    Ok(format!(
        "command=validator_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
        params.validator_min_stake
    ))
}

pub fn check_miner_start(
    wallet: &str,
    device: &str,
    node: &str,
) -> std::result::Result<String, String> {
    let address = wallet_address_hex(wallet)?;
    let device_readiness = miner_device_readiness(device)?;
    Ok(format!(
        "command=miner_start\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\n{}\nreference_backend_ready=true",
        device_readiness.report()
    ))
}

pub fn check_validator_start(wallet: &str, node: &str) -> std::result::Result<String, String> {
    let address = wallet_address_hex(wallet)?;
    Ok(format!(
        "command=validator_start\nwallet={wallet}\naddress={address}\nnode={node}\nreference_verifier_ready=true"
    ))
}

pub fn miner_status() -> String {
    let params = ChainParams::default();
    format!(
        "command=miner_status\nmin_stake={}\nreference_backend_ready=true\nstatus_source=rpc_or_node_store_required",
        params.miner_min_stake
    )
}

pub fn validator_status() -> String {
    let params = ChainParams::default();
    format!(
        "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
        params.validator_min_stake
    )
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
    fn report(&self) -> String {
        match self {
            Self::CpuReference => format!(
                "device_backend=cpu-reference\ncuda_kernels_compiled={}",
                cuda_kernels_compiled()
            ),
            #[cfg(feature = "cuda-kernels")]
            Self::Cuda {
                device_index,
                device_count,
            } => format!(
                "device_backend=cuda\ngpu_backend_ready=true\ncuda_kernels_compiled=true\ncuda_device_index={device_index}\ncuda_device_count={device_count}"
            ),
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
