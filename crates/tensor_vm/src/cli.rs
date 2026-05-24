#[cfg(test)]
use crate::error::TvmError;
#[cfg(all(test, feature = "cuda-kernels"))]
use crate::runtime::cuda_device_count;
#[cfg(test)]
use crate::runtime::cuda_kernels_compiled;
#[cfg(test)]
use crate::testnet::sign_public_evidence_record;
#[cfg(test)]
use libp2p::PeerId;

mod arguments;
mod commands;
mod descriptions;
mod execution;
mod local_description_values;
mod local_descriptions;
mod local_execution;
mod local_execution_values;
mod local_role_descriptions;
mod local_role_execution;
mod local_service_descriptions;
mod local_service_execution;
mod network_evidence;
mod network_observation;
mod node_evidence;
mod public_evidence_descriptions;
mod public_evidence_execution;
mod public_evidence_network_descriptions;
mod public_evidence_network_execution;
mod public_evidence_node_descriptions;
mod public_evidence_node_execution;
mod public_evidence_publication_descriptions;
mod public_evidence_publication_execution;
mod public_evidence_record_descriptions;
mod public_evidence_record_execution;
mod public_evidence_run_window_descriptions;
mod public_evidence_run_window_execution;
mod public_evidence_service_descriptions;
mod public_evidence_service_execution;
mod publication_evidence;
mod record_evidence;
mod reports;
mod run_window_evidence;
mod service_evidence;
mod validation;

#[cfg(test)]
use arguments::{
    parse_hash_argument, parse_public_evidence_record_kind, parse_public_node_role,
    parse_public_service_kind, public_evidence_record_kind_tag, public_service_kind_tag,
};
pub use commands::{
    AuditorRecordArgs, DataDirArgs, HashList, LocalCpuCommand, LocalCpuVerifyArgs,
    LocalTestnetCommand, MinerCommand, MinerRunArgs, MinerStartArgs, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    OperatorAttestationArgs, ProposerCommand, PublicEvidenceCommand, PublicEvidenceManifestArgs,
    PublicEvidenceRecordKindArg, PublicNodeRoleArg, PublicServiceKindArg, PublicTestnetCommand,
    PublicTestnetManifestArgs, PublicationArgs, RecordArtifactArgs, RecordArtifactFromFileArgs,
    RecordArtifactFromRootsArgs, RecordSummaryArgs, RecordSummaryFromFileArgs,
    RecordSummaryFromRootsArgs, RoleRuntimeArgs, RunWindowArgs, RunWindowFromFileArgs,
    ServiceBlockArgs, ServiceCommand, ServiceContentArgs, ServiceContentFromBytesArgs,
    ServiceContentFromFileArgs, ServiceHealthArgs, ServiceHealthFromFileArgs, ServicePeerAddArgs,
    ServicePeerCommand, ServiceReadinessArgs, ServiceRuntimeArgs, ServiceServeArgs, StakeArgs,
    TvmdCli, TvmdCommand, ValidatorCommand, ValidatorRunArgs, ValidatorStartArgs,
};
pub use descriptions::describe_cli_command;
pub use execution::execute_cli_command;
#[cfg(test)]
use network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line_from_service_log,
    network_observation_root, service_log_field,
};
#[cfg(test)]
use network_observation::network_observation_multiaddr_is_public;
#[cfg(test)]
use network_observation::{public_dns_host, public_dns_host_is_well_formed};
#[cfg(test)]
use node_evidence::node_heartbeat_observation_summary_from_file;
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
