use super::KeyValueReportWriter;
#[cfg(feature = "cuda-kernels")]
use crate::runtime::cuda_device_count;
use crate::runtime::cuda_kernels_compiled;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum MinerDeviceReadiness {
    CpuReference,
    #[cfg(feature = "cuda-kernels")]
    Cuda {
        device_index: u32,
        device_count: u32,
    },
}

impl MinerDeviceReadiness {
    pub(super) fn write_report_fields(&self, report: &mut KeyValueReportWriter) {
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

pub(super) fn miner_device_readiness(
    device: &str,
) -> std::result::Result<MinerDeviceReadiness, String> {
    let device = device.trim();
    if device.is_empty() {
        return Err("device argument is empty".to_owned());
    }
    if device == "cpu" {
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
