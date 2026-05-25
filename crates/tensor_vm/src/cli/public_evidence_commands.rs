#[cfg(test)]
pub(crate) use super::public_evidence_block_window_commands::BlockHeightWindowArgs;
#[cfg(test)]
pub(crate) use super::public_evidence_bundle_commands::EvidenceBundleIdArgs;
pub(crate) use super::public_evidence_network_commands::EvidenceNetworkCommand;
#[cfg(test)]
pub(crate) use super::public_evidence_network_commands::{
    NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
    NetworkObservationProtocolCountsArgs, NetworkObservationTargetArgs,
    NetworkObservationTransportLimitsArgs,
};
pub(crate) use super::public_evidence_node_commands::EvidenceNodeCommand;
#[cfg(test)]
pub(crate) use super::public_evidence_node_commands::{
    NodeHeartbeatArgs, NodeHeartbeatFromFileArgs, OperatorAttestationArgs, PublicNodeIdentityArgs,
    PublicNodeRoleArg,
};
#[cfg(test)]
pub(crate) use super::public_evidence_observation_commands::ObservationTimestampArgs;
#[cfg(test)]
pub(crate) use super::public_evidence_operator_commands::OperatorIdArgs;
#[cfg(test)]
pub(crate) use super::public_evidence_publication_commands::PublicationBundleArgs;
pub(crate) use super::public_evidence_publication_commands::{AuditorRecordArgs, PublicationArgs};
#[cfg(test)]
pub(crate) use super::public_evidence_record_artifact_commands::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs,
    RecordArtifactLocatorArgs,
};
pub(crate) use super::public_evidence_record_commands::EvidenceRecordCommand;
#[cfg(test)]
pub(crate) use super::public_evidence_record_commands::{
    PublicEvidenceRecordContextArgs, PublicEvidenceRecordKindArg, RecordFileArgs, RecordRootArgs,
    RecordRootsArgs, RecordSummaryArgs, RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
};
pub(crate) use super::public_evidence_run_window_commands::EvidenceRunCommand;
#[cfg(test)]
pub(crate) use super::public_evidence_run_window_commands::{
    RunWindowArgs, RunWindowContextArgs, RunWindowFromFileArgs,
};
pub(crate) use super::public_evidence_service_commands::EvidenceServiceCommand;
#[cfg(test)]
pub(crate) use super::public_evidence_service_commands::{
    PublicServiceEndpointArgs, PublicServiceKindArg, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceContentTargetArgs,
    ServiceHealthArgs, ServiceHealthFromFileArgs, ServiceHealthPathArgs,
};
#[cfg(test)]
pub(crate) use super::public_evidence_signing_commands::ManifestSignerArgs;
use clap::{Args, Subcommand, ValueHint};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum PublicCommand {
    #[command(about = "Validate a public-testnet preflight manifest.")]
    Preflight(PublicTestnetManifestArgs),
    #[command(about = "Generate or validate public-testnet evidence.")]
    #[command(subcommand)]
    Evidence(EvidenceCommand),
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case", arg_required_else_help = true)]
pub(crate) enum EvidenceCommand {
    #[command(about = "Validate a public-testnet evidence manifest.")]
    Validate(PublicEvidenceManifestArgs),
    #[command(about = "Generate publication evidence for an evidence bundle.")]
    Publish(PublicationArgs),
    #[command(about = "Generate independent auditor evidence.")]
    Audit(AuditorRecordArgs),
    #[command(about = "Generate run-window evidence.")]
    #[command(subcommand)]
    Run(EvidenceRunCommand),
    #[command(about = "Generate node and operator evidence.")]
    #[command(subcommand)]
    Node(EvidenceNodeCommand),
    #[command(about = "Generate public service evidence.")]
    #[command(subcommand)]
    Service(EvidenceServiceCommand),
    #[command(about = "Generate public libp2p network evidence.")]
    #[command(subcommand)]
    Network(EvidenceNetworkCommand),
    #[command(about = "Generate supporting-record evidence.")]
    #[command(subcommand)]
    Record(EvidenceRecordCommand),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicTestnetManifestArgs {
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath, help = "Public-testnet preflight manifest to validate.")]
    manifest: PathBuf,
}

impl PublicTestnetManifestArgs {
    #[cfg(test)]
    pub(crate) fn new(manifest: PathBuf) -> Self {
        Self { manifest }
    }

    pub fn path(&self) -> &Path {
        &self.manifest
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub(crate) struct PublicEvidenceManifestArgs {
    #[arg(value_name = "PATH", value_hint = ValueHint::FilePath, help = "Public-testnet evidence manifest to validate.")]
    manifest: PathBuf,
}

impl PublicEvidenceManifestArgs {
    #[cfg(test)]
    pub(crate) fn new(manifest: PathBuf) -> Self {
        Self { manifest }
    }

    pub fn path(&self) -> &Path {
        &self.manifest
    }
}
