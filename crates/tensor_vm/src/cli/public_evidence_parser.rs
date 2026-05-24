use super::public_evidence_network_parser::{
    NetworkObservationArgs, NetworkObservationFromServiceLogArgs,
};
use super::public_evidence_node_parser::{
    NodeHeartbeatArgs, NodeHeartbeatFromFileArgs, OperatorAttestationArgs,
};
use super::public_evidence_publication_parser::{AuditorRecordArgs, PublicationArgs};
use super::public_evidence_record_parser::{
    RecordArtifactArgs, RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs,
};
use super::public_evidence_run_window_parser::{RunWindowArgs, RunWindowFromFileArgs};
use super::public_evidence_service_parser::{
    ServiceContentArgs, ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs,
};
use clap::{Args, Subcommand};

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PublicEvidenceCommand {
    Validate(ManifestArgs),
    ServiceHealth(ServiceHealthArgs),
    ServiceHealthFromFile(ServiceHealthFromFileArgs),
    ServiceContent(ServiceContentArgs),
    ServiceContentFromBytes(ServiceContentFromBytesArgs),
    ServiceContentFromFile(ServiceContentFromFileArgs),
    RecordSummary(RecordSummaryArgs),
    RecordArtifact(RecordArtifactArgs),
    RecordArtifactFromRoots(RecordArtifactFromRootsArgs),
    RecordArtifactFromFile(RecordArtifactFromFileArgs),
    RecordSummaryFromRoots(RecordSummaryFromRootsArgs),
    RecordSummaryFromFile(RecordSummaryFromFileArgs),
    NetworkObservation(NetworkObservationArgs),
    NetworkObservationFromServiceLog(NetworkObservationFromServiceLogArgs),
    Publication(PublicationArgs),
    AuditorRecord(AuditorRecordArgs),
    RunWindow(RunWindowArgs),
    RunWindowFromFile(RunWindowFromFileArgs),
    NodeHeartbeat(NodeHeartbeatArgs),
    NodeHeartbeatFromFile(NodeHeartbeatFromFileArgs),
    OperatorAttestation(OperatorAttestationArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Args)]
pub struct ManifestArgs {
    #[arg(long)]
    pub manifest: String,
}
