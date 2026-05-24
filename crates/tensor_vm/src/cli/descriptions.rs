use super::CliCommand;
use super::arguments::{
    public_evidence_record_kind_tag, public_node_role_tag, public_service_kind_tag,
};
use crate::hash::hex;
use crate::p2p::Libp2pControlPlaneConfig;
use crate::types::Hash;

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { stake } => format!("register miner with stake {stake}"),
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => format!("start miner wallet={wallet} device={device} node={node}"),
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run miner role wallet={wallet} device={device} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::MinerStatus => "show miner status".to_owned(),
        CliCommand::ValidatorRegister { stake } => format!("register validator with stake {stake}"),
        CliCommand::ValidatorStart { wallet, node } => {
            format!("start validator wallet={wallet} node={node}")
        }
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run validator role wallet={wallet} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ValidatorStatus => "show validator status".to_owned(),
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "run proposer role wallet={wallet} node={node} listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceInit { data_dir } => {
            format!("initialize service node store data_dir={data_dir}")
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            format!(
                "add libp2p bootstrap peer data_dir={data_dir} peer_id={peer_id} address={address}"
            )
        }
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "check mandatory libp2p service readiness p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_description(*identity_seed);
            format!(
                "serve RPC explorer faucet telemetry over mandatory libp2p listen={listen} p2p_listen={p2p_listen} data_dir={data_dir}{identity} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
        }
        CliCommand::ServiceStatus { data_dir } => {
            format!("show service node store status data_dir={data_dir}")
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            format!("show service node store block data_dir={data_dir} height={height}")
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            format!("seed local CPU testnet data_dir={data_dir}")
        }
        CliCommand::LocalCpuVerify { data_dir, json } => {
            format!("verify local CPU node evidence data_dir={data_dir} json={json}")
        }
        CliCommand::PublicEvidenceValidate { manifest } => {
            format!("validate public evidence manifest {manifest}")
        }
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            public_url,
            health_path,
            ..
        } => {
            format!(
                "generate {} service health evidence public_url={public_url} health_path={health_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind,
            public_url,
            health_path,
            observation_file,
            ..
        } => {
            format!(
                "generate {} service health evidence from captured observations observation_file={observation_file} public_url={public_url} health_path={health_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceServiceContent {
            kind,
            public_url,
            content_path,
            ..
        } => {
            format!(
                "generate {} service content evidence public_url={public_url} content_path={content_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind,
            public_url,
            content_path,
            ..
        } => {
            format!(
                "generate {} service content evidence from observed bytes public_url={public_url} content_path={content_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind,
            public_url,
            content_path,
            content_file,
            ..
        } => {
            format!(
                "generate {} service content evidence from captured file content_file={content_file} public_url={public_url} content_path={content_path}",
                public_service_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceRecordSummary {
            kind, record_count, ..
        } => {
            format!(
                "generate {} public evidence record summary records={record_count}",
                public_evidence_record_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceRecordArtifact {
            kind, artifact_uri, ..
        } => {
            format!(
                "generate {} public evidence artifact locator artifact_uri={artifact_uri}",
                public_evidence_record_kind_tag(*kind)
            )
        }
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind,
            artifact_uri,
            record_roots,
            ..
        } => {
            format!(
                "generate {} public evidence artifact locator from {} roots artifact_uri={artifact_uri}",
                public_evidence_record_kind_tag(*kind),
                record_roots.len()
            )
        }
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind,
            artifact_uri,
            record_file,
            ..
        } => {
            format!(
                "generate {} public evidence artifact locator from record file record_file={record_file} artifact_uri={artifact_uri}",
                public_evidence_record_kind_tag(*kind),
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind, record_roots, ..
        } => {
            format!(
                "generate {} public evidence record summary from {} roots",
                public_evidence_record_kind_tag(*kind),
                record_roots.len()
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind, record_file, ..
        } => {
            format!(
                "generate {} public evidence record summary from record file record_file={record_file}",
                public_evidence_record_kind_tag(*kind),
            )
        }
        CliCommand::PublicEvidenceNetworkObservation {
            peer_id,
            listen_address,
            ..
        } => {
            format!(
                "generate signed libp2p network observation peer_id={peer_id} listen_address={listen_address}"
            )
        }
        CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            listen_address,
            service_log,
            ..
        } => {
            format!(
                "generate signed libp2p network observation from service log service_log={service_log} listen_address={listen_address}"
            )
        }
        CliCommand::PublicEvidencePublication { public_uri, .. } => {
            format!("generate public evidence publication signature public_uri={public_uri}")
        }
        CliCommand::PublicEvidenceAuditorRecord {
            auditor_id,
            audit_uri,
            ..
        } => {
            format!(
                "generate public evidence auditor record auditor_id={} audit_uri={audit_uri}",
                hex(auditor_id)
            )
        }
        CliCommand::PublicEvidenceRunWindow {
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
            ..
        } => {
            format!(
                "generate public evidence run window started={run_started_at_unix_seconds} ended={run_ended_at_unix_seconds} observed_blocks={observed_blocks}"
            )
        }
        CliCommand::PublicEvidenceRunWindowFromFile {
            block_observation_file,
            ..
        } => {
            format!(
                "generate public evidence run window from captured block observations block_observation_file={block_observation_file}"
            )
        }
        CliCommand::PublicEvidenceNodeHeartbeat { role, address, .. } => {
            format!(
                "generate {} node heartbeat evidence address={}",
                public_node_role_tag(*role),
                hex(address)
            )
        }
        CliCommand::PublicEvidenceNodeHeartbeatFromFile {
            role,
            address,
            heartbeat_file,
            ..
        } => {
            format!(
                "generate {} node heartbeat evidence from captured observations heartbeat_file={heartbeat_file} address={}",
                public_node_role_tag(*role),
                hex(address)
            )
        }
        CliCommand::PublicEvidenceOperatorAttestation {
            role,
            address,
            identity_uri,
            ..
        } => {
            format!(
                "generate {} operator identity attestation address={} identity_uri={identity_uri}",
                public_node_role_tag(*role),
                hex(address)
            )
        }
        CliCommand::PublicTestnetPreflight { manifest } => {
            format!("run public testnet preflight manifest {manifest}")
        }
    }
}

fn identity_description(identity_seed: Option<Hash>) -> String {
    identity_seed
        .map(|seed| format!(" identity_seed={}", hex(&seed)))
        .unwrap_or_default()
}
