use super::CliCommand;
use super::local_descriptions::describe_local_command;
use super::public_evidence_descriptions::describe_public_evidence_command;

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { .. }
        | CliCommand::MinerStart { .. }
        | CliCommand::MinerRun { .. }
        | CliCommand::MinerStatus
        | CliCommand::ValidatorRegister { .. }
        | CliCommand::ValidatorStart { .. }
        | CliCommand::ValidatorRun { .. }
        | CliCommand::ValidatorStatus
        | CliCommand::ProposerRun { .. }
        | CliCommand::ServiceInit { .. }
        | CliCommand::ServicePeerAdd { .. }
        | CliCommand::ServiceReadiness { .. }
        | CliCommand::ServiceServe { .. }
        | CliCommand::ServiceStatus { .. }
        | CliCommand::ServiceBlock { .. }
        | CliCommand::LocalTestnetSeed { .. }
        | CliCommand::LocalCpuVerify { .. } => describe_local_command(command),
        CliCommand::PublicEvidenceValidate { .. }
        | CliCommand::PublicEvidenceServiceHealth { .. }
        | CliCommand::PublicEvidenceServiceHealthFromFile { .. }
        | CliCommand::PublicEvidenceServiceContent { .. }
        | CliCommand::PublicEvidenceServiceContentFromBytes { .. }
        | CliCommand::PublicEvidenceServiceContentFromFile { .. }
        | CliCommand::PublicEvidenceRecordSummary { .. }
        | CliCommand::PublicEvidenceRecordArtifact { .. }
        | CliCommand::PublicEvidenceRecordArtifactFromRoots { .. }
        | CliCommand::PublicEvidenceRecordArtifactFromFile { .. }
        | CliCommand::PublicEvidenceRecordSummaryFromRoots { .. }
        | CliCommand::PublicEvidenceRecordSummaryFromFile { .. }
        | CliCommand::PublicEvidenceNetworkObservation { .. }
        | CliCommand::PublicEvidenceNetworkObservationFromServiceLog { .. }
        | CliCommand::PublicEvidencePublication { .. }
        | CliCommand::PublicEvidenceAuditorRecord { .. }
        | CliCommand::PublicEvidenceRunWindow { .. }
        | CliCommand::PublicEvidenceRunWindowFromFile { .. }
        | CliCommand::PublicEvidenceNodeHeartbeat { .. }
        | CliCommand::PublicEvidenceNodeHeartbeatFromFile { .. }
        | CliCommand::PublicEvidenceOperatorAttestation { .. }
        | CliCommand::PublicTestnetPreflight { .. } => describe_public_evidence_command(command),
    }
}
