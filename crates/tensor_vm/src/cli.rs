#[cfg(test)]
use crate::error::TvmError;
#[cfg(test)]
use crate::testnet::sign_public_evidence_record;
mod commands;
mod evidence_fields;
mod local_commands;
mod local_node_commands;
mod local_role_commands;
mod local_runtime_args;
mod localnet_commands;
mod network_evidence;
mod node_evidence;
mod public_evidence_block_window_commands;
mod public_evidence_bundle_commands;
mod public_evidence_commands;
mod public_evidence_execution;
mod public_evidence_network_commands;
mod public_evidence_network_execution;
mod public_evidence_node_commands;
mod public_evidence_node_execution;
mod public_evidence_observation_commands;
mod public_evidence_operator_commands;
mod public_evidence_publication_commands;
mod public_evidence_publication_execution;
mod public_evidence_record_artifact_commands;
mod public_evidence_record_commands;
mod public_evidence_record_execution;
mod public_evidence_run_window_commands;
mod public_evidence_run_window_execution;
mod public_evidence_service_commands;
mod public_evidence_service_execution;
mod public_evidence_signing_commands;
mod publication_evidence;
mod record_evidence;
mod record_evidence_roots;
mod record_supporting_evidence;
mod reports;
mod run_window_evidence;
mod service_evidence;
mod validation;
mod value_types;

pub use commands::TvmdCli;
pub(crate) use commands::TvmdCommand;
#[cfg(test)]
use evidence_fields::{public_evidence_record_kind_tag, public_service_kind_tag};
#[cfg(test)]
pub(crate) use local_commands::{
    BootstrapPeerArgs, DataDirArgs, IdentitySeedArgs, LocalCpuVerifyArgs, MinerCheckArgs,
    MinerRunArgs, NodeBlockArgs, NodeCheckArgs, NodePeerAddArgs, NodeRuntimeArgs, NodeServeArgs,
    P2pListenArgs, RoleNodeArgs, RoleWalletArgs, StakeArgs, ValidatorCheckArgs, ValidatorRunArgs,
};
pub(crate) use local_commands::{
    LocalnetCommand, MinerCommand, NodeCommand, NodePeerCommand, ProposerCommand, RoleRuntimeArgs,
    ValidatorCommand,
};
#[cfg(test)]
use network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line_from_service_log,
    network_observation_root, service_log_field,
};
#[cfg(test)]
use node_evidence::node_heartbeat_observation_summary_from_file;
#[cfg(test)]
pub(crate) use public_evidence_commands::{
    AuditorRecordArgs, BlockHeightWindowArgs, EvidenceBundleIdArgs, EvidenceNetworkCommand,
    EvidenceNodeCommand, EvidenceRecordCommand, EvidenceRunCommand, EvidenceServiceCommand,
    ManifestSignerArgs, NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
    NetworkObservationProtocolCountsArgs, NetworkObservationTargetArgs,
    NetworkObservationTransportLimitsArgs, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs,
    ObservationTimestampArgs, OperatorAttestationArgs, OperatorIdArgs, PublicEvidenceManifestArgs,
    PublicEvidenceRecordContextArgs, PublicEvidenceRecordKindArg, PublicNodeIdentityArgs,
    PublicNodeRoleArg, PublicServiceEndpointArgs, PublicServiceKindArg, PublicTestnetManifestArgs,
    PublicationArgs, PublicationBundleArgs, RecordArtifactArgs, RecordArtifactFromFileArgs,
    RecordArtifactFromRootsArgs, RecordArtifactLocatorArgs, RecordFileArgs, RecordRootArgs,
    RecordRootsArgs, RecordSummaryArgs, RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
    RunWindowArgs, RunWindowContextArgs, RunWindowFromFileArgs, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceContentTargetArgs,
    ServiceHealthArgs, ServiceHealthFromFileArgs, ServiceHealthPathArgs,
};
pub(crate) use public_evidence_commands::{EvidenceCommand, PublicCommand};
pub(crate) use public_evidence_execution::execute_public_evidence_command;
#[cfg(test)]
use record_evidence_roots::{
    public_evidence_record_root_from_line, public_evidence_record_roots_from_file,
};
#[cfg(test)]
use record_supporting_evidence::{
    supporting_record_line_prefix, supporting_record_root_from_line,
    validate_supporting_record_payload,
};
pub use reports::{validate_public_evidence_manifest, validate_public_testnet_preflight_manifest};
#[cfg(test)]
use run_window_evidence::run_window_observation_summary_from_file;
#[cfg(test)]
use service_evidence::{public_service_content_root, service_health_observation_summary_from_file};
#[cfg(test)]
pub(crate) use value_types::{AddressArg, HashArg, HexBytesArg, MinerDeviceArg};

#[cfg(test)]
mod tests;
