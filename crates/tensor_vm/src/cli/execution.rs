use super::CliCommand;
use super::local_execution::execute_local_cli_command;
use super::public_evidence_execution::execute_public_evidence_cli_command;
use crate::error::Result;

pub fn execute_reference_cli_command(command: &CliCommand) -> Result<String> {
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
        | CliCommand::LocalCpuVerify { .. } => execute_local_cli_command(command),
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
        | CliCommand::PublicTestnetPreflight { .. } => execute_public_evidence_cli_command(command),
    }
}
