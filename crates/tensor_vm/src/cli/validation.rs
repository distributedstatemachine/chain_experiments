#[cfg(test)]
use crate::error::{Result, TvmError};
#[cfg(test)]
use crate::hash::hex;
#[cfg(all(test, feature = "cuda-kernels"))]
use crate::runtime::cuda_device_count;
#[cfg(test)]
use crate::runtime::cuda_kernels_compiled;
#[cfg(test)]
use crate::types::address;
use std::path::Path;

#[cfg(test)]
pub(super) fn ensure_minimum_stake(stake: u64, minimum: u64) -> Result<()> {
    if stake < minimum {
        return Err(TvmError::InsufficientStake);
    }
    Ok(())
}

pub(super) fn path_argument(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
pub(super) fn wallet_address_hex(wallet: &Path) -> Result<String> {
    let wallet = path_argument(wallet);
    if wallet.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("wallet argument is empty"));
    }
    Ok(hex(&address(wallet.as_bytes())))
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(test)]
pub(super) enum MinerDeviceReadiness {
    CpuReference,
    #[cfg(feature = "cuda-kernels")]
    Cuda {
        device_index: u32,
        device_count: u32,
    },
}

#[cfg(test)]
impl MinerDeviceReadiness {
    pub(super) fn report(&self) -> String {
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

#[cfg(test)]
pub(super) fn miner_device_readiness(device: &str) -> Result<MinerDeviceReadiness> {
    let device = device.trim();
    if device.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("device argument is empty"));
    }
    if matches!(device, "cpu" | "cpu-reference") {
        return Ok(MinerDeviceReadiness::CpuReference);
    }

    let Some(cuda_index) = device.strip_prefix("cuda:") else {
        return Err(TvmError::InvalidReceipt("unsupported miner device"));
    };
    if cuda_index.is_empty() {
        return Err(TvmError::InvalidReceipt("invalid cuda device"));
    }
    let device_index = cuda_index
        .parse::<u32>()
        .map_err(|_| TvmError::InvalidReceipt("invalid cuda device"))?;
    #[cfg(not(feature = "cuda-kernels"))]
    {
        let _ = device_index;
        Err(TvmError::InvalidReceipt("cuda kernels not compiled"))
    }
    #[cfg(feature = "cuda-kernels")]
    {
        let device_count = cuda_device_count()?;
        if device_index >= device_count {
            return Err(TvmError::InvalidReceipt("cuda device unavailable"));
        }
        Ok(MinerDeviceReadiness::Cuda {
            device_index,
            device_count,
        })
    }
}

#[cfg(test)]
pub(super) fn ensure_data_dir(data_dir: &Path) -> Result<()> {
    let data_dir = path_argument(data_dir);
    if data_dir.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("data dir argument is empty"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
pub(super) fn ensure_auth_token(auth_token: &str) -> Result<()> {
    if auth_token.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("auth token argument is empty"));
    }
    Ok(())
}
