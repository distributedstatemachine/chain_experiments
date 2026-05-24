#[cfg(test)]
use crate::error::TvmError;
#[cfg(all(test, feature = "cuda-kernels"))]
use crate::runtime::cuda_device_count;
#[cfg(test)]
use crate::runtime::cuda_kernels_compiled;
#[cfg(test)]
use crate::testnet::sign_public_evidence_record;
mod commands;
mod evidence_fields;
mod local_commands;
#[cfg(test)]
mod local_execution;
#[cfg(test)]
mod local_p2p_report_fields;
#[cfg(test)]
mod local_role_execution;
#[cfg(test)]
mod local_service_execution;
mod network_evidence;
mod node_evidence;
mod public_evidence_commands;
mod public_evidence_execution;
mod public_evidence_network_execution;
mod public_evidence_node_execution;
mod public_evidence_publication_execution;
mod public_evidence_record_execution;
mod public_evidence_run_window_execution;
mod public_evidence_service_commands;
mod public_evidence_service_execution;
mod publication_evidence;
mod record_evidence;
mod reports;
mod run_window_evidence;
mod service_evidence;
mod validation;
mod value_types;

pub use commands::{
    AddressArg, AuditorRecordArgs, DataDirArgs, EvidenceCommand, EvidenceNetworkCommand,
    EvidenceNodeCommand, EvidenceRecordCommand, EvidenceRunCommand, EvidenceServiceCommand,
    HashArg, HexBytesArg, LocalCpuVerifyArgs, LocalnetCommand, MinerCheckArgs, MinerCommand,
    MinerDeviceArg, MinerRunArgs, NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
    NodeBlockArgs, NodeCheckArgs, NodeCommand, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    NodePeerAddArgs, NodePeerCommand, NodeRuntimeArgs, NodeServeArgs, OperatorAttestationArgs,
    ProposerCommand, PublicCommand, PublicEvidenceManifestArgs, PublicEvidenceRecordKindArg,
    PublicNodeRoleArg, PublicServiceKindArg, PublicTestnetManifestArgs, PublicationArgs,
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs, RoleRuntimeArgs, RunWindowArgs,
    RunWindowFromFileArgs, ServiceContentArgs, ServiceContentFromBytesArgs,
    ServiceContentFromFileArgs, ServiceHealthArgs, ServiceHealthFromFileArgs, StakeArgs, TvmdCli,
    TvmdCommand, ValidatorCheckArgs, ValidatorCommand, ValidatorRunArgs,
};
#[cfg(test)]
use evidence_fields::{public_evidence_record_kind_tag, public_service_kind_tag};
#[cfg(test)]
use network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line_from_service_log,
    network_observation_root, service_log_field,
};
#[cfg(test)]
use node_evidence::node_heartbeat_observation_summary_from_file;
pub(crate) use public_evidence_execution::execute_public_evidence_command;
#[cfg(test)]
use record_evidence::{
    public_evidence_record_root_from_line, public_evidence_record_roots_from_file,
    supporting_record_line_prefix, supporting_record_root_from_line,
    validate_supporting_record_payload,
};
pub use reports::{validate_public_evidence_manifest, validate_public_testnet_preflight_manifest};
#[cfg(test)]
use run_window_evidence::run_window_observation_summary_from_file;
#[cfg(test)]
use service_evidence::{public_service_content_root, service_health_observation_summary_from_file};

#[cfg(test)]
mod tests;
