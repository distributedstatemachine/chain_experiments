use crate::chain::ChainParams;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::p2p::Libp2pControlPlaneConfig;
#[cfg(feature = "cuda-kernels")]
use crate::runtime::cuda_device_count;
use crate::runtime::cuda_kernels_compiled;
use crate::testnet::{
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceRecordKind,
    PublicEvidenceSupportingArtifact, PublicNodeEvidence, PublicNodeRole,
    PublicOperatorIdentityAttestation, PublicServiceContentEvidence, PublicServiceEndpoint,
    PublicServiceEvidence, PublicServiceKind, PublicTestnetCriteria,
    parse_public_testnet_evidence_manifest, parse_public_testnet_preflight_manifest,
    sign_public_evidence_record, sign_public_run_window,
};
use crate::types::{Address, Hash, address, hash_bytes};
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliCommand {
    MinerRegister {
        stake: u64,
    },
    MinerStart {
        wallet: String,
        device: String,
        node: String,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorStatus,
    ServiceInit {
        data_dir: String,
    },
    ServicePeerAdd {
        data_dir: String,
        peer_id: String,
        address: String,
    },
    ServiceServe {
        listen: String,
        p2p_listen: String,
        data_dir: String,
        auth_token: String,
        max_requests: usize,
    },
    PublicEvidenceValidate {
        manifest: String,
    },
    PublicEvidenceServiceHealth {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        first_seen_block: u64,
        last_seen_block: u64,
        reachable_observation_count: u64,
        signed_health_check_count: u64,
    },
    PublicEvidenceServiceHealthFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        health_path: String,
        observation_file: String,
    },
    PublicEvidenceServiceContent {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        content_root: Hash,
        observed_at_unix_seconds: u64,
        min_content_bytes: u64,
    },
    PublicEvidenceServiceContentFromBytes {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_hex: String,
    },
    PublicEvidenceServiceContentFromFile {
        kind: PublicServiceKind,
        endpoint_id: Hash,
        public_url: String,
        content_path: String,
        observed_at_unix_seconds: u64,
        content_file: String,
    },
    PublicEvidenceRecordSummary {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifact {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_root: Hash,
        record_count: u64,
    },
    PublicEvidenceRecordArtifactFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordArtifactFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        artifact_uri: String,
        record_file: String,
    },
    PublicEvidenceRecordSummaryFromRoots {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_roots: Vec<Hash>,
    },
    PublicEvidenceRecordSummaryFromFile {
        kind: PublicEvidenceRecordKind,
        bundle_id: Hash,
        manifest_signer: Address,
        record_file: String,
    },
    PublicEvidenceNetworkObservation {
        operator_id: Hash,
        peer_id: String,
        listen_address: String,
        observed_at_unix_seconds: u64,
        gossip_topic_count: u64,
        request_response_protocol_count: u64,
        bootstrap_peer_count: u64,
        max_transmit_bytes: u64,
        request_timeout_seconds: u64,
        max_concurrent_streams: u64,
        idle_connection_timeout_seconds: u64,
    },
    PublicEvidenceNetworkObservationFromServiceLog {
        operator_id: Hash,
        listen_address: String,
        observed_at_unix_seconds: u64,
        service_log: String,
    },
    PublicEvidencePublication {
        bundle_id: Hash,
        public_uri: String,
        manifest_signer: Address,
        manifest_signature_count: u64,
        independent_auditor_count: u64,
    },
    PublicEvidenceAuditorRecord {
        bundle_id: Hash,
        public_uri: String,
        auditor_id: Address,
        audit_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicEvidenceRunWindow {
        bundle_id: Hash,
        manifest_signer: Address,
        run_started_at_unix_seconds: u64,
        run_ended_at_unix_seconds: u64,
        observed_blocks: u64,
    },
    PublicEvidenceRunWindowFromFile {
        bundle_id: Hash,
        manifest_signer: Address,
        block_observation_file: String,
    },
    PublicEvidenceNodeHeartbeat {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        first_seen_block: u64,
        last_seen_block: u64,
        signed_heartbeat_count: u64,
    },
    PublicEvidenceNodeHeartbeatFromFile {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        heartbeat_file: String,
    },
    PublicEvidenceOperatorAttestation {
        role: PublicNodeRole,
        address: Address,
        operator_id: Hash,
        identity_uri: String,
        observed_at_unix_seconds: u64,
    },
    PublicTestnetPreflight {
        manifest: String,
    },
}

pub fn parse_cli_args(args: &[String]) -> Result<CliCommand> {
    let parts: Vec<&str> = args.iter().map(String::as_str).collect();
    parse_cli_parts(&parts)
}

pub fn parse_cli_parts(args: &[&str]) -> Result<CliCommand> {
    match args {
        ["miner", "register", "--stake", stake] => Ok(CliCommand::MinerRegister {
            stake: parse_u64(stake)?,
        }),
        [
            "miner",
            "start",
            "--wallet",
            wallet,
            "--device",
            device,
            "--node",
            node,
        ] => Ok(CliCommand::MinerStart {
            wallet: (*wallet).to_owned(),
            device: (*device).to_owned(),
            node: (*node).to_owned(),
        }),
        ["miner", "status"] => Ok(CliCommand::MinerStatus),
        ["validator", "register", "--stake", stake] => Ok(CliCommand::ValidatorRegister {
            stake: parse_u64(stake)?,
        }),
        ["validator", "start", "--wallet", wallet, "--node", node] => {
            Ok(CliCommand::ValidatorStart {
                wallet: (*wallet).to_owned(),
                node: (*node).to_owned(),
            })
        }
        ["validator", "status"] => Ok(CliCommand::ValidatorStatus),
        ["service", "init", "--data-dir", data_dir] => Ok(CliCommand::ServiceInit {
            data_dir: (*data_dir).to_owned(),
        }),
        [
            "service",
            "peer",
            "add",
            "--data-dir",
            data_dir,
            "--peer-id",
            peer_id,
            "--address",
            address,
        ] => Ok(CliCommand::ServicePeerAdd {
            data_dir: (*data_dir).to_owned(),
            peer_id: (*peer_id).to_owned(),
            address: (*address).to_owned(),
        }),
        [
            "service",
            "serve",
            "--listen",
            listen,
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::ServiceServe {
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        ["public-evidence", "validate", "--manifest", manifest] => {
            Ok(CliCommand::PublicEvidenceValidate {
                manifest: (*manifest).to_owned(),
            })
        }
        [
            "public-evidence",
            "service-health",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--health-path",
            health_path,
            "--first-block",
            first_seen_block,
            "--last-block",
            last_seen_block,
            "--reachable-count",
            reachable_observation_count,
            "--signed-health-check-count",
            signed_health_check_count,
        ] => Ok(CliCommand::PublicEvidenceServiceHealth {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            health_path: (*health_path).to_owned(),
            first_seen_block: parse_u64(first_seen_block)?,
            last_seen_block: parse_u64(last_seen_block)?,
            reachable_observation_count: parse_u64(reachable_observation_count)?,
            signed_health_check_count: parse_u64(signed_health_check_count)?,
        }),
        [
            "public-evidence",
            "service-health-from-file",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--health-path",
            health_path,
            "--observation-file",
            observation_file,
        ] => Ok(CliCommand::PublicEvidenceServiceHealthFromFile {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            health_path: (*health_path).to_owned(),
            observation_file: (*observation_file).to_owned(),
        }),
        [
            "public-evidence",
            "service-content",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--content-path",
            content_path,
            "--content-root",
            content_root,
            "--observed-at",
            observed_at,
            "--min-content-bytes",
            min_content_bytes,
        ] => Ok(CliCommand::PublicEvidenceServiceContent {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            content_path: (*content_path).to_owned(),
            content_root: parse_hash_argument(content_root)?,
            observed_at_unix_seconds: parse_u64(observed_at)?,
            min_content_bytes: parse_u64(min_content_bytes)?,
        }),
        [
            "public-evidence",
            "service-content-from-bytes",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--content-path",
            content_path,
            "--observed-at",
            observed_at,
            "--content-hex",
            content_hex,
        ] => Ok(CliCommand::PublicEvidenceServiceContentFromBytes {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            content_path: (*content_path).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at)?,
            content_hex: (*content_hex).to_owned(),
        }),
        [
            "public-evidence",
            "service-content-from-file",
            "--kind",
            kind,
            "--endpoint-id",
            endpoint_id,
            "--public-url",
            public_url,
            "--content-path",
            content_path,
            "--observed-at",
            observed_at,
            "--content-file",
            content_file,
        ] => Ok(CliCommand::PublicEvidenceServiceContentFromFile {
            kind: parse_public_service_kind(kind)?,
            endpoint_id: parse_hash_argument(endpoint_id)?,
            public_url: (*public_url).to_owned(),
            content_path: (*content_path).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at)?,
            content_file: (*content_file).to_owned(),
        }),
        [
            "public-evidence",
            "record-summary",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--record-root",
            record_root,
            "--record-count",
            record_count,
        ] => Ok(CliCommand::PublicEvidenceRecordSummary {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            record_root: parse_hash_argument(record_root)?,
            record_count: parse_u64(record_count)?,
        }),
        [
            "public-evidence",
            "record-artifact",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--artifact-uri",
            artifact_uri,
            "--record-root",
            record_root,
            "--record-count",
            record_count,
        ] => Ok(CliCommand::PublicEvidenceRecordArtifact {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            artifact_uri: (*artifact_uri).to_owned(),
            record_root: parse_hash_argument(record_root)?,
            record_count: parse_u64(record_count)?,
        }),
        [
            "public-evidence",
            "record-artifact-from-roots",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--artifact-uri",
            artifact_uri,
            "--record-roots",
            record_roots,
        ] => Ok(CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            artifact_uri: (*artifact_uri).to_owned(),
            record_roots: parse_hash_list_argument(record_roots)?,
        }),
        [
            "public-evidence",
            "record-artifact-from-file",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--artifact-uri",
            artifact_uri,
            "--record-file",
            record_file,
        ] => Ok(CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            artifact_uri: (*artifact_uri).to_owned(),
            record_file: (*record_file).to_owned(),
        }),
        [
            "public-evidence",
            "record-summary-from-roots",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--record-roots",
            record_roots,
        ] => Ok(CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            record_roots: parse_hash_list_argument(record_roots)?,
        }),
        [
            "public-evidence",
            "record-summary-from-file",
            "--kind",
            kind,
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--record-file",
            record_file,
        ] => Ok(CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind: parse_public_evidence_record_kind(kind)?,
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            record_file: (*record_file).to_owned(),
        }),
        [
            "public-evidence",
            "network-observation",
            "--operator-id",
            operator_id,
            "--peer-id",
            peer_id,
            "--listen-address",
            listen_address,
            "--observed-at",
            observed_at_unix_seconds,
            "--gossip-topics",
            gossip_topic_count,
            "--request-response-protocols",
            request_response_protocol_count,
            "--bootstrap-peers",
            bootstrap_peer_count,
            "--max-transmit-bytes",
            max_transmit_bytes,
            "--request-timeout-seconds",
            request_timeout_seconds,
            "--max-concurrent-streams",
            max_concurrent_streams,
            "--idle-timeout-seconds",
            idle_connection_timeout_seconds,
        ] => Ok(CliCommand::PublicEvidenceNetworkObservation {
            operator_id: parse_hash_argument(operator_id)?,
            peer_id: (*peer_id).to_owned(),
            listen_address: (*listen_address).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at_unix_seconds)?,
            gossip_topic_count: parse_u64(gossip_topic_count)?,
            request_response_protocol_count: parse_u64(request_response_protocol_count)?,
            bootstrap_peer_count: parse_u64(bootstrap_peer_count)?,
            max_transmit_bytes: parse_u64(max_transmit_bytes)?,
            request_timeout_seconds: parse_u64(request_timeout_seconds)?,
            max_concurrent_streams: parse_u64(max_concurrent_streams)?,
            idle_connection_timeout_seconds: parse_u64(idle_connection_timeout_seconds)?,
        }),
        [
            "public-evidence",
            "network-observation-from-service-log",
            "--operator-id",
            operator_id,
            "--listen-address",
            listen_address,
            "--observed-at",
            observed_at_unix_seconds,
            "--service-log",
            service_log,
        ] => Ok(CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: parse_hash_argument(operator_id)?,
            listen_address: (*listen_address).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at_unix_seconds)?,
            service_log: (*service_log).to_owned(),
        }),
        [
            "public-evidence",
            "publication",
            "--bundle-id",
            bundle_id,
            "--public-uri",
            public_uri,
            "--manifest-signer",
            manifest_signer,
            "--manifest-signature-count",
            manifest_signature_count,
            "--independent-auditor-count",
            independent_auditor_count,
        ] => Ok(CliCommand::PublicEvidencePublication {
            bundle_id: parse_hash_argument(bundle_id)?,
            public_uri: (*public_uri).to_owned(),
            manifest_signer: parse_hash_argument(manifest_signer)?,
            manifest_signature_count: parse_u64(manifest_signature_count)?,
            independent_auditor_count: parse_u64(independent_auditor_count)?,
        }),
        [
            "public-evidence",
            "auditor-record",
            "--bundle-id",
            bundle_id,
            "--public-uri",
            public_uri,
            "--auditor-id",
            auditor_id,
            "--audit-uri",
            audit_uri,
            "--observed-at",
            observed_at_unix_seconds,
        ] => Ok(CliCommand::PublicEvidenceAuditorRecord {
            bundle_id: parse_hash_argument(bundle_id)?,
            public_uri: (*public_uri).to_owned(),
            auditor_id: parse_hash_argument(auditor_id)?,
            audit_uri: (*audit_uri).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at_unix_seconds)?,
        }),
        [
            "public-evidence",
            "run-window",
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--started-at",
            run_started_at_unix_seconds,
            "--ended-at",
            run_ended_at_unix_seconds,
            "--observed-blocks",
            observed_blocks,
        ] => Ok(CliCommand::PublicEvidenceRunWindow {
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            run_started_at_unix_seconds: parse_u64(run_started_at_unix_seconds)?,
            run_ended_at_unix_seconds: parse_u64(run_ended_at_unix_seconds)?,
            observed_blocks: parse_u64(observed_blocks)?,
        }),
        [
            "public-evidence",
            "run-window-from-file",
            "--bundle-id",
            bundle_id,
            "--manifest-signer",
            manifest_signer,
            "--block-observation-file",
            block_observation_file,
        ] => Ok(CliCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: parse_hash_argument(bundle_id)?,
            manifest_signer: parse_hash_argument(manifest_signer)?,
            block_observation_file: (*block_observation_file).to_owned(),
        }),
        [
            "public-evidence",
            "node-heartbeat",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--first-block",
            first_seen_block,
            "--last-block",
            last_seen_block,
            "--heartbeat-count",
            signed_heartbeat_count,
        ] => Ok(CliCommand::PublicEvidenceNodeHeartbeat {
            role: parse_public_node_role(role)?,
            address: parse_hash_argument(address)?,
            operator_id: parse_hash_argument(operator_id)?,
            first_seen_block: parse_u64(first_seen_block)?,
            last_seen_block: parse_u64(last_seen_block)?,
            signed_heartbeat_count: parse_u64(signed_heartbeat_count)?,
        }),
        [
            "public-evidence",
            "node-heartbeat-from-file",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--heartbeat-file",
            heartbeat_file,
        ] => Ok(CliCommand::PublicEvidenceNodeHeartbeatFromFile {
            role: parse_public_node_role(role)?,
            address: parse_hash_argument(address)?,
            operator_id: parse_hash_argument(operator_id)?,
            heartbeat_file: (*heartbeat_file).to_owned(),
        }),
        [
            "public-evidence",
            "operator-attestation",
            "--role",
            role,
            "--address",
            address,
            "--operator-id",
            operator_id,
            "--identity-uri",
            identity_uri,
            "--observed-at",
            observed_at_unix_seconds,
        ] => Ok(CliCommand::PublicEvidenceOperatorAttestation {
            role: parse_public_node_role(role)?,
            address: parse_hash_argument(address)?,
            operator_id: parse_hash_argument(operator_id)?,
            identity_uri: (*identity_uri).to_owned(),
            observed_at_unix_seconds: parse_u64(observed_at_unix_seconds)?,
        }),
        ["public-testnet", "preflight", "--manifest", manifest] => {
            Ok(CliCommand::PublicTestnetPreflight {
                manifest: (*manifest).to_owned(),
            })
        }
        _ => Err(TvmError::InvalidReceipt("invalid cli command")),
    }
}

pub fn describe_command(command: &CliCommand) -> String {
    match command {
        CliCommand::MinerRegister { stake } => format!("register miner with stake {stake}"),
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => format!("start miner wallet={wallet} device={device} node={node}"),
        CliCommand::MinerStatus => "show miner status".to_owned(),
        CliCommand::ValidatorRegister { stake } => format!("register validator with stake {stake}"),
        CliCommand::ValidatorStart { wallet, node } => {
            format!("start validator wallet={wallet} node={node}")
        }
        CliCommand::ValidatorStatus => "show validator status".to_owned(),
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
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            auth_token: _,
            max_requests,
        } => {
            let p2p_config = Libp2pControlPlaneConfig::default();
            format!(
                "serve RPC explorer faucet telemetry over mandatory libp2p listen={listen} p2p_listen={p2p_listen} data_dir={data_dir} max_requests={max_requests} max_transmit_bytes={} request_timeout_seconds={} max_concurrent_streams={} idle_timeout_seconds={}",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            )
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

pub fn execute_reference_cli_command(command: &CliCommand) -> Result<String> {
    let params = ChainParams::default();
    match command {
        CliCommand::MinerRegister { stake } => {
            ensure_minimum_stake(*stake, params.miner_min_stake)?;
            Ok(format!(
                "command=miner_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.miner_min_stake
            ))
        }
        CliCommand::MinerStart {
            wallet,
            device,
            node,
        } => {
            let address = wallet_address_hex(wallet)?;
            let device_readiness = miner_device_readiness(device)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=miner_start\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\n{}\nreference_backend_ready=true",
                device_readiness.report()
            ))
        }
        CliCommand::MinerStatus => Ok(format!(
            "command=miner_status\nmin_stake={}\nreference_backend_ready=true\nstatus_source=rpc_or_node_store_required",
            params.miner_min_stake
        )),
        CliCommand::ValidatorRegister { stake } => {
            ensure_minimum_stake(*stake, params.validator_min_stake)?;
            Ok(format!(
                "command=validator_register\nstake={stake}\nmin_stake={}\nstake_sufficient=true",
                params.validator_min_stake
            ))
        }
        CliCommand::ValidatorStart { wallet, node } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            Ok(format!(
                "command=validator_start\nwallet={wallet}\naddress={address}\nnode={node}\nreference_verifier_ready=true"
            ))
        }
        CliCommand::ValidatorStatus => Ok(format!(
            "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
            params.validator_min_stake
        )),
        CliCommand::ServiceInit { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_init\ndata_dir={data_dir}\nnode_store_ready=true"
            ))
        }
        CliCommand::ServicePeerAdd {
            data_dir,
            peer_id,
            address,
        } => {
            ensure_data_dir(data_dir)?;
            let record = crate::p2p::PeerRecord::from_strings(peer_id, address)?;
            let peer_id = record.peer_id()?;
            Ok(format!(
                "command=service_peer_add\ndata_dir={data_dir}\npeer_id={peer_id}\naddress={address}\npeer_book_ready=true"
            ))
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            auth_token,
            max_requests,
        } => {
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            Ok(format!(
                "command=service_serve\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::PublicEvidenceServiceHealth {
            kind,
            endpoint_id,
            public_url,
            health_path,
            first_seen_block,
            last_seen_block,
            reachable_observation_count,
            signed_health_check_count,
        } => service_health_evidence_line(ServiceHealthEvidenceLine {
            kind: *kind,
            endpoint_id: *endpoint_id,
            public_url,
            health_path,
            first_seen_block: *first_seen_block,
            last_seen_block: *last_seen_block,
            reachable_observation_count: *reachable_observation_count,
            signed_health_check_count: *signed_health_check_count,
        }),
        CliCommand::PublicEvidenceServiceHealthFromFile {
            kind,
            endpoint_id,
            public_url,
            health_path,
            observation_file,
        } => service_health_evidence_line_from_file(
            *kind,
            *endpoint_id,
            public_url,
            health_path,
            observation_file,
        ),
        CliCommand::PublicEvidenceServiceContent {
            kind,
            endpoint_id,
            public_url,
            content_path,
            content_root,
            observed_at_unix_seconds,
            min_content_bytes,
        } => service_content_evidence_line(
            *kind,
            *endpoint_id,
            public_url,
            content_path,
            *content_root,
            *observed_at_unix_seconds,
            *min_content_bytes,
        ),
        CliCommand::PublicEvidenceServiceContentFromBytes {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_hex,
        } => {
            let content_bytes = parse_hex_bytes_argument(content_hex)?;
            service_content_evidence_line_from_bytes(
                *kind,
                *endpoint_id,
                public_url,
                content_path,
                *observed_at_unix_seconds,
                &content_bytes,
            )
        }
        CliCommand::PublicEvidenceServiceContentFromFile {
            kind,
            endpoint_id,
            public_url,
            content_path,
            observed_at_unix_seconds,
            content_file,
        } => {
            let content_bytes = std::fs::read(content_file)
                .map_err(|_| TvmError::Storage("failed to read service content file"))?;
            service_content_evidence_line_from_bytes(
                *kind,
                *endpoint_id,
                public_url,
                content_path,
                *observed_at_unix_seconds,
                &content_bytes,
            )
        }
        CliCommand::PublicEvidenceRecordSummary {
            kind,
            bundle_id,
            manifest_signer,
            record_root,
            record_count,
        } => record_summary_evidence_lines(
            *kind,
            *bundle_id,
            *manifest_signer,
            *record_root,
            *record_count,
        ),
        CliCommand::PublicEvidenceRecordArtifact {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_root,
            record_count,
        } => record_artifact_evidence_line(
            *kind,
            *bundle_id,
            *manifest_signer,
            artifact_uri,
            *record_root,
            *record_count,
        ),
        CliCommand::PublicEvidenceRecordArtifactFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_roots,
        } => {
            let record_root = aggregate_public_evidence_record_roots(*kind, record_roots)?;
            record_artifact_evidence_line(
                *kind,
                *bundle_id,
                *manifest_signer,
                artifact_uri,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordArtifactFromFile {
            kind,
            bundle_id,
            manifest_signer,
            artifact_uri,
            record_file,
        } => {
            let record_roots = public_evidence_record_roots_from_file(*kind, record_file)?;
            let record_root = aggregate_public_evidence_record_roots(*kind, &record_roots)?;
            record_artifact_evidence_line(
                *kind,
                *bundle_id,
                *manifest_signer,
                artifact_uri,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromRoots {
            kind,
            bundle_id,
            manifest_signer,
            record_roots,
        } => {
            let record_root = aggregate_public_evidence_record_roots(*kind, record_roots)?;
            record_summary_evidence_lines(
                *kind,
                *bundle_id,
                *manifest_signer,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceRecordSummaryFromFile {
            kind,
            bundle_id,
            manifest_signer,
            record_file,
        } => {
            let record_roots = public_evidence_record_roots_from_file(*kind, record_file)?;
            let record_root = aggregate_public_evidence_record_roots(*kind, &record_roots)?;
            record_summary_evidence_lines(
                *kind,
                *bundle_id,
                *manifest_signer,
                record_root,
                record_roots.len() as u64,
            )
        }
        CliCommand::PublicEvidenceNetworkObservation {
            operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds,
            gossip_topic_count,
            request_response_protocol_count,
            bootstrap_peer_count,
            max_transmit_bytes,
            request_timeout_seconds,
            max_concurrent_streams,
            idle_connection_timeout_seconds,
        } => network_observation_evidence_line(NetworkObservationEvidenceLine {
            operator_id: *operator_id,
            peer_id,
            listen_address,
            observed_at_unix_seconds: *observed_at_unix_seconds,
            gossip_topic_count: *gossip_topic_count,
            request_response_protocol_count: *request_response_protocol_count,
            bootstrap_peer_count: *bootstrap_peer_count,
            max_transmit_bytes: *max_transmit_bytes,
            request_timeout_seconds: *request_timeout_seconds,
            max_concurrent_streams: *max_concurrent_streams,
            idle_connection_timeout_seconds: *idle_connection_timeout_seconds,
        }),
        CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id,
            listen_address,
            observed_at_unix_seconds,
            service_log,
        } => {
            let log_contents = std::fs::read_to_string(service_log)
                .map_err(|_| TvmError::Storage("failed to read service log file"))?;
            network_observation_evidence_line_from_service_log(
                *operator_id,
                listen_address,
                *observed_at_unix_seconds,
                &log_contents,
            )
        }
        CliCommand::PublicEvidencePublication {
            bundle_id,
            public_uri,
            manifest_signer,
            manifest_signature_count,
            independent_auditor_count,
        } => publication_evidence_lines(
            *bundle_id,
            public_uri,
            *manifest_signer,
            *manifest_signature_count,
            *independent_auditor_count,
        ),
        CliCommand::PublicEvidenceAuditorRecord {
            bundle_id,
            public_uri,
            auditor_id,
            audit_uri,
            observed_at_unix_seconds,
        } => auditor_record_evidence_line(
            *bundle_id,
            public_uri,
            *auditor_id,
            audit_uri,
            *observed_at_unix_seconds,
        ),
        CliCommand::PublicEvidenceRunWindow {
            bundle_id,
            manifest_signer,
            run_started_at_unix_seconds,
            run_ended_at_unix_seconds,
            observed_blocks,
        } => run_window_evidence_line(
            *bundle_id,
            *manifest_signer,
            *run_started_at_unix_seconds,
            *run_ended_at_unix_seconds,
            *observed_blocks,
        ),
        CliCommand::PublicEvidenceRunWindowFromFile {
            bundle_id,
            manifest_signer,
            block_observation_file,
        } => {
            run_window_evidence_line_from_file(*bundle_id, *manifest_signer, block_observation_file)
        }
        CliCommand::PublicEvidenceNodeHeartbeat {
            role,
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        } => node_heartbeat_evidence_line(
            *role,
            *address,
            *operator_id,
            *first_seen_block,
            *last_seen_block,
            *signed_heartbeat_count,
        ),
        CliCommand::PublicEvidenceNodeHeartbeatFromFile {
            role,
            address,
            operator_id,
            heartbeat_file,
        } => node_heartbeat_evidence_line_from_file(*role, *address, *operator_id, heartbeat_file),
        CliCommand::PublicEvidenceOperatorAttestation {
            role,
            address,
            operator_id,
            identity_uri,
            observed_at_unix_seconds,
        } => operator_identity_attestation_evidence_line(
            *role,
            *address,
            *operator_id,
            identity_uri,
            *observed_at_unix_seconds,
        ),
        CliCommand::PublicEvidenceValidate { .. } | CliCommand::PublicTestnetPreflight { .. } => {
            Ok(describe_command(command))
        }
    }
}

struct ServiceHealthEvidenceLine<'a> {
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &'a str,
    health_path: &'a str,
    first_seen_block: u64,
    last_seen_block: u64,
    reachable_observation_count: u64,
    signed_health_check_count: u64,
}

fn service_health_evidence_line(input: ServiceHealthEvidenceLine<'_>) -> Result<String> {
    if input.last_seen_block < input.first_seen_block {
        return Err(TvmError::InvalidReceipt(
            "service health block range is invalid",
        ));
    }
    let evidence = PublicServiceEvidence::new(
        input.kind,
        PublicServiceEndpoint::new(input.endpoint_id, input.public_url, input.health_path),
        input.first_seen_block,
        input.last_seen_block,
        input.reachable_observation_count,
        input.signed_health_check_count,
    );
    if !evidence.has_reachable_endpoint_proof() {
        return Err(TvmError::InvalidReceipt("invalid service health evidence"));
    }
    Ok(format!(
        "service={},{},{},{},{},{},{},{},{}",
        public_service_kind_tag(input.kind),
        hex(&evidence.endpoint_id),
        evidence.public_url,
        evidence.health_path,
        evidence.first_seen_block,
        evidence.last_seen_block,
        evidence.reachable_observation_count,
        evidence.signed_health_check_count,
        hex(&evidence.health_check_signature)
    ))
}

struct ServiceHealthObservationSummary {
    first_seen_block: u64,
    last_seen_block: u64,
    reachable_observation_count: u64,
    signed_health_check_count: u64,
}

fn service_health_evidence_line_from_file(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    health_path: &str,
    observation_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(observation_file)
        .map_err(|_| TvmError::Storage("failed to read service health observation file"))?;
    let summary = service_health_observation_summary_from_file(&contents)?;
    service_health_evidence_line(ServiceHealthEvidenceLine {
        kind,
        endpoint_id,
        public_url,
        health_path,
        first_seen_block: summary.first_seen_block,
        last_seen_block: summary.last_seen_block,
        reachable_observation_count: summary.reachable_observation_count,
        signed_health_check_count: summary.signed_health_check_count,
    })
}

fn service_health_observation_summary_from_file(
    contents: &str,
) -> Result<ServiceHealthObservationSummary> {
    let mut observed_blocks = BTreeSet::new();
    let mut reachable_observation_count = 0_u64;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "service health observation line has leading or trailing whitespace",
            ));
        }
        let (block, reachable) = parse_service_health_observation_line(line)?;
        if !observed_blocks.insert(block) {
            return Err(TvmError::InvalidReceipt(
                "duplicate service health observation block",
            ));
        }
        if reachable {
            reachable_observation_count = reachable_observation_count.saturating_add(1);
        }
    }
    let Some(first_seen_block) = observed_blocks.iter().next().copied() else {
        return Err(TvmError::InvalidReceipt(
            "service health observation file has no observations",
        ));
    };
    let last_seen_block = observed_blocks.iter().next_back().copied().unwrap();
    let signed_health_check_count = observed_blocks.len() as u64;
    let expected_observation_count = last_seen_block
        .checked_sub(first_seen_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "service health observation block range is invalid",
        ))?;
    if signed_health_check_count != expected_observation_count {
        return Err(TvmError::InvalidReceipt(
            "service health observation blocks must be contiguous",
        ));
    }
    Ok(ServiceHealthObservationSummary {
        first_seen_block,
        last_seen_block,
        reachable_observation_count,
        signed_health_check_count,
    })
}

fn parse_service_health_observation_line(line: &str) -> Result<(u64, bool)> {
    let record =
        line.strip_prefix("service_health_observation=")
            .ok_or(TvmError::InvalidReceipt(
                "unsupported service health observation line",
            ))?;
    let fields: Vec<&str> = record.split(',').collect();
    if fields.len() != 2 {
        return Err(TvmError::InvalidReceipt(
            "malformed service health observation",
        ));
    }
    let block = parse_u64(fields[0])?;
    let reachable = match fields[1] {
        "reachable" => true,
        "unreachable" => false,
        _ => {
            return Err(TvmError::InvalidReceipt(
                "invalid service health observation status",
            ));
        }
    };
    Ok((block, reachable))
}

fn service_content_evidence_line(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    content_path: &str,
    content_root: Hash,
    observed_at_unix_seconds: u64,
    min_content_bytes: u64,
) -> Result<String> {
    let evidence = PublicServiceContentEvidence::new(
        kind,
        endpoint_id,
        public_url,
        content_path,
        content_root,
        observed_at_unix_seconds,
        min_content_bytes,
    );
    if !evidence.has_external_content_proof() {
        return Err(TvmError::InvalidReceipt("invalid service content evidence"));
    }
    Ok(format!(
        "service_content={},{},{},{},{},{},{},{}",
        public_service_kind_tag(kind),
        hex(&evidence.endpoint_id),
        evidence.public_url,
        evidence.content_path,
        hex(&evidence.content_root),
        evidence.observed_at_unix_seconds,
        evidence.min_content_bytes,
        hex(&evidence.content_signature)
    ))
}

fn public_service_content_root(content_bytes: &[u8]) -> Hash {
    hash_bytes(
        b"tensor-vm-public-service-content-root-v1",
        &[content_bytes],
    )
}

fn service_content_evidence_line_from_bytes(
    kind: PublicServiceKind,
    endpoint_id: Hash,
    public_url: &str,
    content_path: &str,
    observed_at_unix_seconds: u64,
    content_bytes: &[u8],
) -> Result<String> {
    service_content_evidence_line(
        kind,
        endpoint_id,
        public_url,
        content_path,
        public_service_content_root(content_bytes),
        observed_at_unix_seconds,
        content_bytes.len() as u64,
    )
}

fn publication_evidence_lines(
    bundle_id: Hash,
    public_uri: &str,
    manifest_signer: Address,
    manifest_signature_count: u64,
    independent_auditor_count: u64,
) -> Result<String> {
    let publication = PublicEvidencePublication::new(
        bundle_id,
        public_uri.to_owned(),
        manifest_signer,
        manifest_signature_count,
        independent_auditor_count,
    );
    if !publication.is_published_and_independently_checkable() {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence publication",
        ));
    }
    Ok(format!(
        "bundle_id={}\npublic_uri={}\nmanifest_signer={}\nmanifest_signature={}\nmanifest_signature_count={}\nindependent_auditor_count={}",
        hex(&publication.bundle_id),
        publication.public_uri,
        hex(&publication.manifest_signer),
        hex(&publication.manifest_signature),
        publication.manifest_signature_count,
        publication.independent_auditor_count
    ))
}

fn auditor_record_evidence_line(
    bundle_id: Hash,
    public_uri: &str,
    auditor_id: Address,
    audit_uri: &str,
    observed_at_unix_seconds: u64,
) -> Result<String> {
    let auditor = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        public_uri,
        auditor_id,
        audit_uri.to_owned(),
        observed_at_unix_seconds,
    );
    if !auditor.has_external_auditor_proof(&bundle_id, public_uri) {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence auditor record",
        ));
    }
    Ok(format!(
        "auditor={},{},{},{}",
        hex(&auditor.auditor_id),
        auditor.audit_uri,
        auditor.observed_at_unix_seconds,
        hex(&auditor.auditor_signature)
    ))
}

fn run_window_evidence_line(
    bundle_id: Hash,
    manifest_signer: Address,
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if run_ended_at_unix_seconds < run_started_at_unix_seconds {
        return Err(TvmError::InvalidReceipt(
            "public run window block range is invalid",
        ));
    }
    if observed_blocks == 0 {
        return Err(TvmError::InvalidReceipt(
            "observed blocks argument is empty",
        ));
    }
    let signature = sign_public_run_window(
        &manifest_signer,
        &bundle_id,
        run_started_at_unix_seconds,
        run_ended_at_unix_seconds,
        observed_blocks,
    );
    Ok(format!(
        "run_started_at_unix_seconds={run_started_at_unix_seconds}\nrun_ended_at_unix_seconds={run_ended_at_unix_seconds}\nrun_window_signature={}\nobserved_blocks={observed_blocks}",
        hex(&signature)
    ))
}

struct RunWindowObservationSummary {
    run_started_at_unix_seconds: u64,
    run_ended_at_unix_seconds: u64,
    observed_blocks: u64,
}

fn run_window_evidence_line_from_file(
    bundle_id: Hash,
    manifest_signer: Address,
    block_observation_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(block_observation_file)
        .map_err(|_| TvmError::Storage("failed to read run-window block observation file"))?;
    let summary = run_window_observation_summary_from_file(&contents)?;
    run_window_evidence_line(
        bundle_id,
        manifest_signer,
        summary.run_started_at_unix_seconds,
        summary.run_ended_at_unix_seconds,
        summary.observed_blocks,
    )
}

fn run_window_observation_summary_from_file(contents: &str) -> Result<RunWindowObservationSummary> {
    let mut observations = BTreeMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "run-window observation line has leading or trailing whitespace",
            ));
        }
        let (block, timestamp) = parse_run_window_observation_line(line)?;
        if timestamp == 0 {
            return Err(TvmError::InvalidReceipt(
                "run-window observation timestamp is empty",
            ));
        }
        if observations.insert(block, timestamp).is_some() {
            return Err(TvmError::InvalidReceipt(
                "duplicate run-window observation block",
            ));
        }
    }
    let Some((&first_block, &run_started_at_unix_seconds)) = observations.iter().next() else {
        return Err(TvmError::InvalidReceipt(
            "run-window observation file has no observations",
        ));
    };
    let (&last_block, &run_ended_at_unix_seconds) = observations.iter().next_back().unwrap();
    let observed_blocks = observations.len() as u64;
    let expected_block_count = last_block
        .checked_sub(first_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "run-window observation block range is invalid",
        ))?;
    if observed_blocks != expected_block_count {
        return Err(TvmError::InvalidReceipt(
            "run-window observation blocks must be contiguous",
        ));
    }
    let mut previous_timestamp = None;
    for timestamp in observations.values().copied() {
        if let Some(previous) = previous_timestamp
            && timestamp < previous
        {
            return Err(TvmError::InvalidReceipt(
                "run-window observation timestamps must be monotonic",
            ));
        }
        previous_timestamp = Some(timestamp);
    }
    Ok(RunWindowObservationSummary {
        run_started_at_unix_seconds,
        run_ended_at_unix_seconds,
        observed_blocks,
    })
}

fn parse_run_window_observation_line(line: &str) -> Result<(u64, u64)> {
    let record = line
        .strip_prefix("run_window_observation=")
        .ok_or(TvmError::InvalidReceipt(
            "unsupported run-window observation line",
        ))?;
    let fields: Vec<&str> = record.split(',').collect();
    if fields.len() != 2 {
        return Err(TvmError::InvalidReceipt("malformed run-window observation"));
    }
    Ok((parse_u64(fields[0])?, parse_u64(fields[1])?))
}

fn node_heartbeat_evidence_line(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    first_seen_block: u64,
    last_seen_block: u64,
    signed_heartbeat_count: u64,
) -> Result<String> {
    if address == [0; 32] {
        return Err(TvmError::InvalidReceipt("node address argument is empty"));
    }
    if last_seen_block < first_seen_block {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat block range is invalid",
        ));
    }
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
        PublicNodeRole::Validator => PublicNodeEvidence::validator(
            address,
            operator_id,
            first_seen_block,
            last_seen_block,
            signed_heartbeat_count,
        ),
    };
    if !node.has_external_operator_proof() {
        return Err(TvmError::InvalidReceipt("invalid node heartbeat evidence"));
    }
    Ok(format!(
        "node={},{},{},{},{},{},{}",
        public_node_role_tag(node.role),
        hex(&node.address),
        hex(&node.operator_id),
        node.first_seen_block,
        node.last_seen_block,
        node.signed_heartbeat_count,
        hex(&node.heartbeat_signature)
    ))
}

struct NodeHeartbeatObservationSummary {
    first_seen_block: u64,
    last_seen_block: u64,
    signed_heartbeat_count: u64,
}

fn node_heartbeat_evidence_line_from_file(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    heartbeat_file: &str,
) -> Result<String> {
    let contents = std::fs::read_to_string(heartbeat_file)
        .map_err(|_| TvmError::Storage("failed to read node heartbeat observation file"))?;
    let summary =
        node_heartbeat_observation_summary_from_file(role, address, operator_id, &contents)?;
    node_heartbeat_evidence_line(
        role,
        address,
        operator_id,
        summary.first_seen_block,
        summary.last_seen_block,
        summary.signed_heartbeat_count,
    )
}

fn node_heartbeat_observation_summary_from_file(
    expected_role: PublicNodeRole,
    expected_address: Address,
    expected_operator_id: Hash,
    contents: &str,
) -> Result<NodeHeartbeatObservationSummary> {
    let mut observed_blocks = BTreeSet::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "node heartbeat observation line has leading or trailing whitespace",
            ));
        }
        let (role, address, operator_id, block) = parse_node_heartbeat_observation_line(line)?;
        if role != expected_role
            || address != expected_address
            || operator_id != expected_operator_id
        {
            return Err(TvmError::InvalidReceipt(
                "node heartbeat observation identity mismatch",
            ));
        }
        if !observed_blocks.insert(block) {
            return Err(TvmError::InvalidReceipt(
                "duplicate node heartbeat observation block",
            ));
        }
    }
    let Some(first_seen_block) = observed_blocks.iter().next().copied() else {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat observation file has no observations",
        ));
    };
    let last_seen_block = observed_blocks.iter().next_back().copied().unwrap();
    let signed_heartbeat_count = observed_blocks.len() as u64;
    let expected_heartbeat_count = last_seen_block
        .checked_sub(first_seen_block)
        .and_then(|span| span.checked_add(1))
        .ok_or(TvmError::InvalidReceipt(
            "node heartbeat observation block range is invalid",
        ))?;
    if signed_heartbeat_count != expected_heartbeat_count {
        return Err(TvmError::InvalidReceipt(
            "node heartbeat observation blocks must be contiguous",
        ));
    }
    Ok(NodeHeartbeatObservationSummary {
        first_seen_block,
        last_seen_block,
        signed_heartbeat_count,
    })
}

fn parse_node_heartbeat_observation_line(
    line: &str,
) -> Result<(PublicNodeRole, Address, Hash, u64)> {
    let record =
        line.strip_prefix("node_heartbeat_observation=")
            .ok_or(TvmError::InvalidReceipt(
                "unsupported node heartbeat observation line",
            ))?;
    let fields: Vec<&str> = record.split(',').collect();
    if fields.len() != 4 {
        return Err(TvmError::InvalidReceipt(
            "malformed node heartbeat observation",
        ));
    }
    Ok((
        parse_public_node_role(fields[0])?,
        parse_hash_argument(fields[1])?,
        parse_hash_argument(fields[2])?,
        parse_u64(fields[3])?,
    ))
}

fn operator_identity_attestation_evidence_line(
    role: PublicNodeRole,
    address: Address,
    operator_id: Hash,
    identity_uri: &str,
    observed_at_unix_seconds: u64,
) -> Result<String> {
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        address,
        operator_id,
        identity_uri.to_owned(),
        observed_at_unix_seconds,
    );
    if !attestation.has_external_identity_proof() {
        return Err(TvmError::InvalidReceipt(
            "invalid operator identity attestation",
        ));
    }
    Ok(format!(
        "operator={},{},{},{},{},{}",
        public_node_role_tag(attestation.role),
        hex(&attestation.address),
        hex(&attestation.operator_id),
        attestation.identity_uri,
        attestation.observed_at_unix_seconds,
        hex(&attestation.operator_signature)
    ))
}

fn record_summary_evidence_lines(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if record_root == [0; 32] {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    if record_count == 0 {
        return Err(TvmError::InvalidReceipt("record count argument is empty"));
    }
    let field_prefix = public_evidence_record_field_prefix(kind);
    let signature = sign_public_evidence_record(
        &manifest_signer,
        &bundle_id,
        kind,
        &record_root,
        record_count,
    );
    Ok(format!(
        "{field_prefix}_records={record_count}\n{field_prefix}_root={}\n{field_prefix}_signature={}",
        hex(&record_root),
        hex(&signature)
    ))
}

fn record_artifact_evidence_line(
    kind: PublicEvidenceRecordKind,
    bundle_id: Hash,
    manifest_signer: Address,
    artifact_uri: &str,
    record_root: Hash,
    record_count: u64,
) -> Result<String> {
    if bundle_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("bundle id argument is empty"));
    }
    if manifest_signer == [0; 32] {
        return Err(TvmError::InvalidReceipt(
            "manifest signer argument is empty",
        ));
    }
    if record_root == [0; 32] {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    if record_count == 0 {
        return Err(TvmError::InvalidReceipt("record count argument is empty"));
    }
    let artifact = PublicEvidenceSupportingArtifact::new(
        &bundle_id,
        &manifest_signer,
        kind,
        artifact_uri.to_owned(),
        record_root,
        record_count,
    );
    if !artifact.is_public_and_signed(&bundle_id, &manifest_signer) {
        return Err(TvmError::InvalidReceipt("invalid public evidence artifact"));
    }
    Ok(format!(
        "record_artifact={},{},{},{},{}",
        public_evidence_record_kind_tag(kind),
        artifact.artifact_uri,
        hex(&artifact.record_root),
        artifact.record_count,
        hex(&artifact.artifact_signature)
    ))
}

fn aggregate_public_evidence_record_roots(
    kind: PublicEvidenceRecordKind,
    record_roots: &[Hash],
) -> Result<Hash> {
    if record_roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record roots argument is empty"));
    }
    if record_roots.contains(&[0; 32]) {
        return Err(TvmError::InvalidReceipt("record root argument is empty"));
    }
    let mut unique_roots = BTreeSet::new();
    if record_roots.iter().any(|root| !unique_roots.insert(*root)) {
        return Err(TvmError::InvalidReceipt("duplicate record root argument"));
    }
    let record_count = (record_roots.len() as u64).to_le_bytes();
    let mut encoded_roots = Vec::with_capacity(record_roots.len() * 32);
    for root in record_roots {
        encoded_roots.extend_from_slice(root);
    }
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-record-root-aggregation-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            &record_count,
            &encoded_roots,
        ],
    ))
}

fn public_evidence_record_roots_from_file(
    kind: PublicEvidenceRecordKind,
    record_file: &str,
) -> Result<Vec<Hash>> {
    let contents = std::fs::read_to_string(record_file)
        .map_err(|_| TvmError::Storage("failed to read public evidence record file"))?;
    let mut roots = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line != raw_line {
            return Err(TvmError::InvalidReceipt(
                "public evidence record line has leading or trailing whitespace",
            ));
        }
        roots.push(public_evidence_record_root_from_line(kind, line)?);
    }
    if roots.is_empty() {
        return Err(TvmError::InvalidReceipt("record file has no roots"));
    }
    Ok(roots)
}

fn public_evidence_record_root_from_line(
    kind: PublicEvidenceRecordKind,
    line: &str,
) -> Result<Hash> {
    if let Some(root) = line.strip_prefix("record_root=") {
        return parse_record_file_root(root);
    }
    if kind == PublicEvidenceRecordKind::NetworkRuntimeObservations
        && let Some(record) = line.strip_prefix("network_runtime_observation=")
    {
        return network_observation_root_from_record_line(record);
    }
    if let Some(prefix) = supporting_record_line_prefix(kind)
        && line.starts_with(prefix)
    {
        return supporting_record_root_from_line(kind, line, prefix);
    }
    Err(TvmError::InvalidReceipt(
        "unsupported public evidence record line",
    ))
}

fn supporting_record_line_prefix(kind: PublicEvidenceRecordKind) -> Option<&'static str> {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => Some("block_history_record="),
        PublicEvidenceRecordKind::FinalityHistory => Some("finality_history_record="),
        PublicEvidenceRecordKind::NetworkRuntimeObservations => None,
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            Some("data_availability_measurement=")
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => Some("invalid_work_rejection="),
        PublicEvidenceRecordKind::RewardSettlements => Some("reward_settlement="),
    }
}

fn supporting_record_root_from_line(
    kind: PublicEvidenceRecordKind,
    line: &str,
    prefix: &str,
) -> Result<Hash> {
    let payload = line.strip_prefix(prefix).ok_or(TvmError::InvalidReceipt(
        "unsupported public evidence record line",
    ))?;
    if payload.is_empty() || payload.trim() != payload {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-supporting-record-root-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            line.as_bytes(),
        ],
    ))
}

fn parse_record_file_root(root: &str) -> Result<Hash> {
    if root.trim() != root {
        return Err(TvmError::InvalidReceipt("invalid record root file line"));
    }
    parse_hash_argument(root)
}

fn network_observation_root_from_record_line(record: &str) -> Result<Hash> {
    let fields = record.split(',').collect::<Vec<_>>();
    if fields.len() != 13 {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    parse_record_file_root(fields[11])
}

struct NetworkObservationEvidenceLine<'a> {
    operator_id: Hash,
    peer_id: &'a str,
    listen_address: &'a str,
    observed_at_unix_seconds: u64,
    gossip_topic_count: u64,
    request_response_protocol_count: u64,
    bootstrap_peer_count: u64,
    max_transmit_bytes: u64,
    request_timeout_seconds: u64,
    max_concurrent_streams: u64,
    idle_connection_timeout_seconds: u64,
}

fn network_observation_evidence_line(input: NetworkObservationEvidenceLine<'_>) -> Result<String> {
    if input.operator_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("operator id argument is empty"));
    }
    if input.observed_at_unix_seconds == 0 {
        return Err(TvmError::InvalidReceipt("observed-at argument is empty"));
    }
    if input.gossip_topic_count == 0 {
        return Err(TvmError::InvalidReceipt("gossip topics argument is empty"));
    }
    if input.request_response_protocol_count == 0 {
        return Err(TvmError::InvalidReceipt(
            "request-response protocols argument is empty",
        ));
    }
    if input.bootstrap_peer_count == 0 {
        return Err(TvmError::InvalidReceipt(
            "bootstrap peers argument is empty",
        ));
    }
    if input.max_transmit_bytes == 0
        || input.request_timeout_seconds == 0
        || input.max_concurrent_streams == 0
        || input.idle_connection_timeout_seconds == 0
    {
        return Err(TvmError::InvalidReceipt(
            "network runtime control arguments must be positive",
        ));
    }

    let peer_id = input
        .peer_id
        .parse::<PeerId>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p peer id"))?;
    let listen_address = input
        .listen_address
        .parse::<Multiaddr>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))?;
    if !network_observation_multiaddr_is_public(&listen_address) {
        return Err(TvmError::InvalidReceipt(
            "network observation address is not public",
        ));
    }

    let peer_id = peer_id.to_string();
    let listen_address = listen_address.to_string();
    let root = network_observation_root(&input, &peer_id, &listen_address);
    let signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&input.operator_id, &root],
    );
    Ok(format!(
        "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
        hex(&input.operator_id),
        peer_id,
        listen_address,
        input.observed_at_unix_seconds,
        input.gossip_topic_count,
        input.request_response_protocol_count,
        input.bootstrap_peer_count,
        input.max_transmit_bytes,
        input.request_timeout_seconds,
        input.max_concurrent_streams,
        input.idle_connection_timeout_seconds,
        hex(&root),
        hex(&signature)
    ))
}

fn network_observation_evidence_line_from_service_log(
    operator_id: Hash,
    listen_address: &str,
    observed_at_unix_seconds: u64,
    service_log: &str,
) -> Result<String> {
    if service_log_field(service_log, "command")? != "service_serve" {
        return Err(TvmError::InvalidReceipt("service log is not service_serve"));
    }
    if service_log_field(service_log, "p2p_runtime")? != "libp2p" {
        return Err(TvmError::InvalidReceipt(
            "service log does not prove libp2p runtime",
        ));
    }
    network_observation_evidence_line(NetworkObservationEvidenceLine {
        operator_id,
        peer_id: service_log_field(service_log, "p2p_peer_id")?,
        listen_address,
        observed_at_unix_seconds,
        gossip_topic_count: parse_u64(service_log_field(service_log, "p2p_gossipsub_topics")?)?,
        request_response_protocol_count: parse_u64(service_log_field(
            service_log,
            "p2p_request_response_protocols",
        )?)?,
        bootstrap_peer_count: parse_u64(service_log_field(service_log, "p2p_bootstrap_peers")?)?,
        max_transmit_bytes: parse_u64(service_log_field(service_log, "p2p_max_transmit_bytes")?)?,
        request_timeout_seconds: parse_u64(service_log_field(
            service_log,
            "p2p_request_timeout_seconds",
        )?)?,
        max_concurrent_streams: parse_u64(service_log_field(
            service_log,
            "p2p_max_concurrent_streams",
        )?)?,
        idle_connection_timeout_seconds: parse_u64(service_log_field(
            service_log,
            "p2p_idle_timeout_seconds",
        )?)?,
    })
}

fn service_log_field<'a>(service_log: &'a str, key: &str) -> Result<&'a str> {
    let prefix = format!("{key}=");
    let mut found = None;
    for line in service_log.lines() {
        if let Some(value) = line.strip_prefix(&prefix) {
            if found.is_some() {
                return Err(TvmError::InvalidReceipt("duplicate service log field"));
            }
            if value.is_empty() || value.trim() != value {
                return Err(TvmError::InvalidReceipt("invalid service log field"));
            }
            found = Some(value);
        }
    }
    found.ok_or(TvmError::InvalidReceipt("missing service log field"))
}

fn network_observation_root(
    input: &NetworkObservationEvidenceLine<'_>,
    peer_id: &str,
    listen_address: &str,
) -> Hash {
    let observed_at = input.observed_at_unix_seconds.to_le_bytes();
    let gossip_topics = input.gossip_topic_count.to_le_bytes();
    let request_response_protocols = input.request_response_protocol_count.to_le_bytes();
    let bootstrap_peers = input.bootstrap_peer_count.to_le_bytes();
    let max_transmit_bytes = input.max_transmit_bytes.to_le_bytes();
    let request_timeout = input.request_timeout_seconds.to_le_bytes();
    let max_streams = input.max_concurrent_streams.to_le_bytes();
    let idle_timeout = input.idle_connection_timeout_seconds.to_le_bytes();
    hash_bytes(
        b"tensor-vm-network-runtime-observation-v1",
        &[
            &input.operator_id,
            peer_id.as_bytes(),
            listen_address.as_bytes(),
            &observed_at,
            &gossip_topics,
            &request_response_protocols,
            &bootstrap_peers,
            &max_transmit_bytes,
            &request_timeout,
            &max_streams,
            &idle_timeout,
        ],
    )
}

fn network_observation_multiaddr_is_public(address: &Multiaddr) -> bool {
    let mut saw_public_address = false;
    let mut saw_tcp_listen_port = false;
    for protocol in address.iter() {
        match protocol {
            Protocol::Ip4(ip) if public_ipv4(ip) => saw_public_address = true,
            Protocol::Ip6(ip) if public_ipv6(ip) => saw_public_address = true,
            Protocol::Dns(host) | Protocol::Dns4(host) | Protocol::Dns6(host)
                if public_dns_host(host.as_ref()) =>
            {
                saw_public_address = true;
            }
            Protocol::Tcp(port) if port != 0 => saw_tcp_listen_port = true,
            Protocol::Tcp(_) => {
                return false;
            }
            Protocol::Ip4(_)
            | Protocol::Ip6(_)
            | Protocol::Dns(_)
            | Protocol::Dns4(_)
            | Protocol::Dns6(_) => {
                return false;
            }
            _ => {}
        }
    }
    saw_public_address && saw_tcp_listen_port
}

fn public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();
    let is_shared_address_space = a == 100 && (64..=127).contains(&b);
    let is_protocol_assignment = a == 192 && b == 0 && c == 0;
    let is_documentation = (a == 192 && b == 0 && c == 2)
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113);
    let is_benchmarking = a == 198 && (b == 18 || b == 19);
    let is_multicast = (224..=239).contains(&a);
    let is_reserved_or_broadcast = (240..=255).contains(&a);
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || is_shared_address_space
        || is_protocol_assignment
        || is_documentation
        || is_benchmarking
        || is_multicast
        || is_reserved_or_broadcast)
}

fn public_ipv6(ip: Ipv6Addr) -> bool {
    let first_segment = ip.segments()[0];
    let unique_local = (first_segment & 0xfe00) == 0xfc00;
    let link_local = (first_segment & 0xffc0) == 0xfe80;
    let documentation = ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8;
    !(ip.is_unspecified()
        || ip.is_loopback()
        || unique_local
        || link_local
        || ip.is_multicast()
        || documentation)
}

fn public_dns_host(host: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || special_use_dns_name(&host)
        || host.contains('@')
        || host
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return false;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => public_ipv4(ip),
        Ok(IpAddr::V6(ip)) => public_ipv6(ip),
        Err(_) => public_dns_host_is_well_formed(&host),
    }
}

fn special_use_dns_name(host: &str) -> bool {
    host == "local"
        || host == "test"
        || host == "example"
        || host == "invalid"
        || host == "example.com"
        || host == "example.net"
        || host == "example.org"
        || host.ends_with(".example.com")
        || host.ends_with(".example.net")
        || host.ends_with(".example.org")
        || host.ends_with(".test")
        || host.ends_with(".example")
        || host.ends_with(".invalid")
}

fn public_dns_host_is_well_formed(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    let mut label_count = 0;
    let mut labels = host.split('.').peekable();
    while let Some(label) = labels.next() {
        label_count += 1;
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let bytes = label.as_bytes();
        if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
            return false;
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return false;
        }
        if labels.peek().is_none() && !bytes.iter().any(|byte| byte.is_ascii_alphabetic()) {
            return false;
        }
    }
    label_count >= 2
}

pub fn validate_public_evidence_manifest(input: &str) -> Result<String> {
    let bundle = parse_public_testnet_evidence_manifest(input)?;
    let report = bundle.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );
    Ok(format!(
        "public_evidence_full_spec={}\npublic_criterion={}\nindependently_checkable={}\npublished_evidence_bundle={}\nindependent_auditor_records={}\nsigned_run_window={}\nblock_history={}\nfinality_history={}\noperator_identity_attestations={}\nnetwork_runtime_observations={}\ndata_availability_measurements={}\nsigned_invalid_work_rejection_records={}\nsigned_reward_settlement_records={}\nsupporting_record_artifacts={}\nminers={}\nvalidators={}\nrun_started_at_unix_seconds={}\nrun_ended_at_unix_seconds={}\nobserved_duration_seconds={}\nrequired_duration_seconds={}\nobserved_blocks={}\nrequired_blocks={}\nfinality_rate_bps={}\ndata_availability_bps={}\ninvalid_receipts_submitted={}\ninvalid_receipts_rejected={}\ninvalid_work_rejection_rate_bps={}\nreward_settlement_records={}\nexternal_operator_evidence={}\nrequired_miners={}\nrequired_validators={}\nrequired_run_duration={}\nrequired_block_count={}\nrequired_finality={}\nrequired_data_availability={}\ninvalid_work_rejection_evidence={}\nreward_settlement_evidence={}\nproduction_libp2p_runtime={}\ndeployed_rpc_service={}\ndeployed_explorer_service={}\ndeployed_faucet_service={}\ndeployed_telemetry_service={}\ndeployed_public_service_content={}\ndeployed_public_services={}",
        report.full_spec_evidence_met,
        report.run_evidence.public_criterion_met,
        report.independently_checkable,
        report.has_published_evidence_bundle,
        report.has_independent_auditor_records,
        report.has_signed_run_window,
        report.has_block_history,
        report.has_finality_history,
        report.has_operator_identity_attestations,
        report.has_network_runtime_observations,
        report.has_data_availability_measurements,
        report.has_invalid_work_rejection_records,
        report.has_reward_settlement_record_summary,
        report.has_public_supporting_record_artifacts,
        report.run_evidence.miner_count,
        report.run_evidence.validator_count,
        report.run_evidence.run_started_at_unix_seconds,
        report.run_evidence.run_ended_at_unix_seconds,
        report.run_evidence.observed_duration_seconds,
        report.run_evidence.required_duration_seconds,
        report.run_evidence.observed_blocks,
        report.run_evidence.required_blocks,
        report.run_evidence.finality_rate_bps,
        report.run_evidence.data_availability_bps,
        report.run_evidence.invalid_receipts_submitted,
        report.run_evidence.invalid_receipts_rejected,
        report.run_evidence.invalid_work_rejection_rate_bps,
        report.run_evidence.reward_settlement_records,
        report.run_evidence.external_operator_evidence,
        report.run_evidence.has_required_miners,
        report.run_evidence.has_required_validators,
        report.run_evidence.has_required_run_duration,
        report.run_evidence.has_required_block_count,
        report.run_evidence.has_required_finality,
        report.run_evidence.has_required_data_availability,
        report.run_evidence.has_invalid_work_rejection_evidence,
        report.run_evidence.has_reward_settlement_records,
        report.run_evidence.has_production_libp2p_runtime,
        report.run_evidence.has_deployed_rpc_service,
        report.run_evidence.has_deployed_explorer_service,
        report.run_evidence.has_deployed_faucet_service,
        report.run_evidence.has_deployed_telemetry_service,
        report.run_evidence.has_deployed_public_service_content,
        report.run_evidence.has_deployed_public_services,
    ))
}

pub fn validate_public_testnet_preflight_manifest(input: &str) -> Result<String> {
    let plan = parse_public_testnet_preflight_manifest(input)?;
    let report = plan.evaluate(ChainParams::default().block_time_seconds);
    Ok(format!(
        "public_testnet_preflight_ready={}\nlocal_shape_ready={}\ndeployment_plan_ready={}\nminers={}\nvalidators={}\nrequired_blocks={}\nrequired_miners={}\nrequired_validators={}\npositive_stakes={}\nfunded_faucet={}\ncuda_kernels_available={}\ncuda_ready_miner_count={}\ncuda_ready_miners={}\nlibp2p_ready_node_count={}\nlibp2p_ready_nodes={}\nproduction_libp2p_runtime={}\nrpc_service_plan={}\nexplorer_service_plan={}\nfaucet_service_plan={}\ntelemetry_service_plan={}\npublic_service_content_planned={}\npublic_services_planned={}",
        report.can_start_public_run,
        report.local_shape_ready,
        report.deployment_plan_ready,
        report.miner_count,
        report.validator_count,
        report.required_blocks,
        report.has_required_miners,
        report.has_required_validators,
        report.has_positive_stakes,
        report.has_funded_faucet,
        report.has_cuda_kernels_available,
        report.cuda_ready_miner_count,
        report.has_cuda_ready_miners,
        report.libp2p_ready_node_count,
        report.has_libp2p_ready_nodes,
        report.has_production_libp2p_runtime,
        report.has_rpc_service_plan,
        report.has_explorer_service_plan,
        report.has_faucet_service_plan,
        report.has_telemetry_service_plan,
        report.has_public_service_content_plan,
        report.has_public_service_plan,
    ))
}

fn parse_u64(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

fn parse_usize(value: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|_| TvmError::InvalidReceipt("invalid numeric argument"))
}

fn parse_public_service_kind(value: &str) -> Result<PublicServiceKind> {
    match value {
        "rpc" => Ok(PublicServiceKind::Rpc),
        "explorer" => Ok(PublicServiceKind::Explorer),
        "faucet" => Ok(PublicServiceKind::Faucet),
        "telemetry" => Ok(PublicServiceKind::Telemetry),
        _ => Err(TvmError::InvalidReceipt("invalid public service kind")),
    }
}

fn public_service_kind_tag(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "rpc",
        PublicServiceKind::Explorer => "explorer",
        PublicServiceKind::Faucet => "faucet",
        PublicServiceKind::Telemetry => "telemetry",
    }
}

fn parse_public_node_role(value: &str) -> Result<PublicNodeRole> {
    match value {
        "miner" => Ok(PublicNodeRole::Miner),
        "validator" => Ok(PublicNodeRole::Validator),
        _ => Err(TvmError::InvalidReceipt("invalid public node role")),
    }
}

fn public_node_role_tag(role: PublicNodeRole) -> &'static str {
    match role {
        PublicNodeRole::Miner => "miner",
        PublicNodeRole::Validator => "validator",
    }
}

fn parse_public_evidence_record_kind(value: &str) -> Result<PublicEvidenceRecordKind> {
    match value {
        "block-history" => Ok(PublicEvidenceRecordKind::BlockHistory),
        "finality-history" => Ok(PublicEvidenceRecordKind::FinalityHistory),
        "network-runtime" => Ok(PublicEvidenceRecordKind::NetworkRuntimeObservations),
        "data-availability" => Ok(PublicEvidenceRecordKind::DataAvailabilityMeasurements),
        "invalid-work" => Ok(PublicEvidenceRecordKind::InvalidWorkRejections),
        "reward-settlement" => Ok(PublicEvidenceRecordKind::RewardSettlements),
        _ => Err(TvmError::InvalidReceipt(
            "invalid public evidence record kind",
        )),
    }
}

fn public_evidence_record_kind_tag(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block-history",
        PublicEvidenceRecordKind::FinalityHistory => "finality-history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network-runtime",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data-availability",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid-work",
        PublicEvidenceRecordKind::RewardSettlements => "reward-settlement",
    }
}

fn public_evidence_record_field_prefix(kind: PublicEvidenceRecordKind) -> &'static str {
    match kind {
        PublicEvidenceRecordKind::BlockHistory => "block_history",
        PublicEvidenceRecordKind::FinalityHistory => "finality_history",
        PublicEvidenceRecordKind::NetworkRuntimeObservations => "network_runtime_observation",
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => "data_availability_measurement",
        PublicEvidenceRecordKind::InvalidWorkRejections => "invalid_work_rejection",
        PublicEvidenceRecordKind::RewardSettlements => "reward_settlement",
    }
}

fn parse_hash_argument(value: &str) -> Result<Hash> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(TvmError::InvalidReceipt("invalid hash argument"));
    }
    let mut out = [0u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let high = parse_hash_nibble(value.as_bytes()[index * 2])?;
        let low = parse_hash_nibble(value.as_bytes()[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(out)
}

fn parse_hash_list_argument(value: &str) -> Result<Vec<Hash>> {
    if value.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("empty hash list argument"));
    }
    value
        .split(',')
        .map(|part| parse_hash_argument(part.trim()))
        .collect()
}

fn parse_hex_bytes_argument(value: &str) -> Result<Vec<u8>> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return Err(TvmError::InvalidReceipt("invalid hex bytes argument"));
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    for chunk in value.as_bytes().chunks_exact(2) {
        let high = parse_hex_nibble(chunk[0])?;
        let low = parse_hex_nibble(chunk[1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn parse_hash_nibble(value: u8) -> Result<u8> {
    parse_hex_nibble(value).map_err(|_| TvmError::InvalidReceipt("invalid hash argument"))
}

fn parse_hex_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(TvmError::InvalidReceipt("invalid hex bytes argument")),
    }
}

fn ensure_minimum_stake(stake: u64, minimum: u64) -> Result<()> {
    if stake < minimum {
        return Err(TvmError::InsufficientStake);
    }
    Ok(())
}

fn wallet_address_hex(wallet: &str) -> Result<String> {
    let wallet = wallet.trim();
    if wallet.is_empty() {
        return Err(TvmError::InvalidReceipt("wallet argument is empty"));
    }
    Ok(hex(&address(wallet.as_bytes())))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MinerDeviceReadiness {
    CpuReference,
    #[cfg(feature = "cuda-kernels")]
    Cuda {
        device_index: u32,
        device_count: u32,
    },
}

impl MinerDeviceReadiness {
    fn report(&self) -> String {
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

fn miner_device_readiness(device: &str) -> Result<MinerDeviceReadiness> {
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

fn ensure_node_endpoint(node: &str) -> Result<()> {
    ensure_libp2p_multiaddr(node)
        .map_err(|_| TvmError::InvalidReceipt("unsupported libp2p node endpoint"))
}

fn ensure_listen_addr(listen: &str) -> Result<()> {
    listen
        .parse::<SocketAddr>()
        .map(|_| ())
        .map_err(|_| TvmError::InvalidReceipt("invalid service listen address"))
}

fn ensure_libp2p_multiaddr(address: &str) -> Result<()> {
    address
        .trim()
        .parse::<Multiaddr>()
        .map(|_| ())
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))
}

fn ensure_data_dir(data_dir: &str) -> Result<()> {
    if data_dir.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("data dir argument is empty"));
    }
    Ok(())
}

fn ensure_auth_token(auth_token: &str) -> Result<()> {
    if auth_token.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("auth token argument is empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hex;
    use crate::testnet::{
        PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
        PublicEvidencePublication, PublicEvidenceRecordSummaries, PublicNetworkRuntimeEvidence,
        PublicNodeEvidence, PublicNodeRole, PublicOperatorIdentityAttestation,
        PublicServiceContentEvidence, PublicServiceEndpoint, PublicServiceEvidence,
        PublicServiceKind, PublicTestnetEvidenceBundle, PublicTestnetRunEvidence,
        aggregate_public_evidence_record_roots, public_network_runtime_observations_for_run,
    };
    use crate::types::{Hash, address, hash_bytes};

    fn manifest_hash(label: &[u8]) -> String {
        hex(&hash_bytes(b"test", &[label]))
    }

    fn manifest_address(label: &[u8]) -> String {
        hex(&address(label))
    }

    fn manifest_node_signature(
        role: PublicNodeRole,
        address_label: &[u8],
        operator_label: &[u8],
    ) -> String {
        let node_address = address(address_label);
        let operator_id = hash_bytes(b"test", &[operator_label]);
        let node = match role {
            PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
            PublicNodeRole::Validator => {
                PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
            }
        };
        hex(&node.heartbeat_signature)
    }

    fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
        format!("https://operators.tensorvm.net/{}", hex(operator_id))
    }

    fn manifest_operator_signature(
        role: PublicNodeRole,
        address_label: &[u8],
        operator_label: &[u8],
    ) -> String {
        let node_address = address(address_label);
        let operator_id = hash_bytes(b"test", &[operator_label]);
        let attestation = PublicOperatorIdentityAttestation::new(
            role,
            node_address,
            operator_id,
            manifest_operator_identity_uri(&operator_id),
            1_700_000_000,
        );
        hex(&attestation.operator_signature)
    }

    fn public_service_url(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
            PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
            PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
            PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
        }
    }

    fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
        let service = PublicServiceEvidence::new(
            kind,
            PublicServiceEndpoint::new(
                hash_bytes(b"test", &[label]),
                public_service_url(kind),
                "/health",
            ),
            0,
            9,
            10,
            10,
        );
        hex(&service.health_check_signature)
    }

    fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
            PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
            PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
            PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
        }
    }

    fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
        match kind {
            PublicServiceKind::Rpc => "/chain/head",
            PublicServiceKind::Explorer => "/explorer",
            PublicServiceKind::Faucet => "/faucet/page",
            PublicServiceKind::Telemetry => "/telemetry/dashboard",
        }
    }

    fn public_service_content(
        kind: PublicServiceKind,
        label: &[u8],
    ) -> PublicServiceContentEvidence {
        PublicServiceContentEvidence::new(
            kind,
            hash_bytes(b"test", &[label]),
            public_service_content_url(kind),
            public_service_content_path(kind),
            hash_bytes(b"test", &[label, b"content-root"]),
            1_700_000_000,
            64,
        )
    }

    fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
        let content = public_service_content(kind, label);
        format!(
            "service_content={},{},{},{},{},{},{},{}",
            public_service_kind_tag(kind),
            hex(&content.endpoint_id),
            content.public_url,
            content.content_path,
            hex(&content.content_root),
            content.observed_at_unix_seconds,
            content.min_content_bytes,
            hex(&content.content_signature)
        )
    }

    fn manifest_publication_signature() -> String {
        let publication = PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        );
        hex(&publication.manifest_signature)
    }

    fn manifest_publication() -> PublicEvidencePublication {
        PublicEvidencePublication::new(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
            address(b"public-evidence-publisher"),
            1,
            1,
        )
    }

    fn manifest_auditor_uri() -> String {
        format!(
            "https://auditors.tensorvm.net/{}/0",
            manifest_hash(b"public-evidence-bundle")
        )
    }

    fn manifest_auditor_signature() -> String {
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let record = PublicEvidenceAuditorRecord::new(
            &bundle_id,
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            1_700_000_060,
        );
        hex(&record.auditor_signature)
    }

    fn manifest_artifact_line(
        kind: PublicEvidenceRecordKind,
        root_label: &[u8],
        record_count: u64,
    ) -> String {
        manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
    }

    fn manifest_artifact_line_for_root(
        kind: PublicEvidenceRecordKind,
        record_root: Hash,
        record_count: u64,
    ) -> String {
        let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
        let artifact_uri = format!(
            "https://evidence.tensorvm.net/{}/{}.json",
            manifest_hash(b"public-evidence-bundle"),
            public_evidence_record_kind_tag(kind)
        );
        let signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &bundle_id,
            kind,
            &artifact_uri,
            &record_root,
            record_count,
        );
        format!(
            "record_artifact={},{},{},{},{}",
            public_evidence_record_kind_tag(kind),
            artifact_uri,
            hex(&record_root),
            record_count,
            hex(&signature)
        )
    }

    fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
        let record_roots = public_network_runtime_observations_for_run(run)
            .iter()
            .map(|observation| observation.record_root)
            .collect::<Vec<_>>();
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_roots,
        )
        .expect("generated network observation roots should aggregate")
    }

    fn manifest_network_observation_lines() -> String {
        public_network_runtime_observations_for_run(&manifest_bundle().run)
            .iter()
            .map(|observation| {
                format!(
                    "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                    hex(&observation.operator_id),
                    observation.peer_id,
                    observation.listen_address,
                    observation.observed_at_unix_seconds,
                    observation.gossip_topic_count,
                    observation.request_response_protocol_count,
                    observation.bootstrap_peer_count,
                    observation.max_transmit_bytes,
                    observation.request_timeout_seconds,
                    observation.max_concurrent_streams,
                    observation.idle_connection_timeout_seconds,
                    hex(&observation.record_root),
                    hex(&observation.observation_signature)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn manifest_bundle() -> PublicTestnetEvidenceBundle {
        let run = PublicTestnetRunEvidence {
            nodes: vec![
                PublicNodeEvidence::miner(
                    address(b"miner-a"),
                    hash_bytes(b"test", &[b"miner-a-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::miner(
                    address(b"miner-b"),
                    hash_bytes(b"test", &[b"miner-b-operator"]),
                    0,
                    9,
                    10,
                ),
                PublicNodeEvidence::validator(
                    address(b"validator-a"),
                    hash_bytes(b"test", &[b"validator-a-operator"]),
                    0,
                    9,
                    10,
                ),
            ],
            network_runtime: PublicNetworkRuntimeEvidence {
                libp2p_runtime_used: true,
                peer_discovery_observed: true,
                gossip_propagation_observed: true,
                request_response_observed: true,
                dos_controls_enabled: true,
            },
            services: vec![
                PublicServiceEvidence::new(
                    PublicServiceKind::Rpc,
                    PublicServiceEndpoint::new(
                        hash_bytes(b"test", &[b"rpc-service"]),
                        public_service_url(PublicServiceKind::Rpc),
                        "/health",
                    ),
                    0,
                    9,
                    10,
                    10,
                ),
                PublicServiceEvidence::new(
                    PublicServiceKind::Explorer,
                    PublicServiceEndpoint::new(
                        hash_bytes(b"test", &[b"explorer-service"]),
                        public_service_url(PublicServiceKind::Explorer),
                        "/health",
                    ),
                    0,
                    9,
                    10,
                    10,
                ),
                PublicServiceEvidence::new(
                    PublicServiceKind::Faucet,
                    PublicServiceEndpoint::new(
                        hash_bytes(b"test", &[b"faucet-service"]),
                        public_service_url(PublicServiceKind::Faucet),
                        "/health",
                    ),
                    0,
                    9,
                    10,
                    10,
                ),
                PublicServiceEvidence::new(
                    PublicServiceKind::Telemetry,
                    PublicServiceEndpoint::new(
                        hash_bytes(b"test", &[b"telemetry-service"]),
                        public_service_url(PublicServiceKind::Telemetry),
                        "/health",
                    ),
                    0,
                    9,
                    10,
                    10,
                ),
            ],
            service_content: vec![
                public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
                public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
                public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
                public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
            ],
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
            finalized_blocks: 10,
            checked_receipts: 20,
            available_receipts: 19,
            invalid_receipts_submitted: 1,
            invalid_receipts_rejected: 1,
            reward_settlement_records: 1,
        };
        let network_runtime_observation_root = network_runtime_root_for_run(&run);
        PublicTestnetEvidenceBundle::new(
            run,
            manifest_publication(),
            PublicEvidenceRecordSummaries {
                block_history_records: 10,
                block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
                finality_history_records: 10,
                finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
                operator_identity_attestation_records: 3,
                network_runtime_observation_records: 3,
                network_runtime_observation_root,
                data_availability_measurement_records: 20,
                data_availability_measurement_root: hash_bytes(
                    b"test",
                    &[b"data-availability-root"],
                ),
                invalid_work_rejection_records: 1,
                invalid_work_rejection_root: hash_bytes(b"test", &[b"invalid-work-root"]),
                reward_settlement_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
            },
        )
    }

    fn evidence_manifest() -> String {
        format!(
            "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
auditor={},{},1700000060,{}
{}
{}
{}
{}
{}
{}
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
operator=miner,{},{},{},1700000000,{}
operator=miner,{},{},{},1700000000,{}
operator=validator,{},{},{},1700000000,{}
{}
network_runtime_observation_records=3
network_runtime_observation_root={}
network_runtime_observation_signature={}
data_availability_measurement_records=20
data_availability_measurement_root={}
data_availability_measurement_signature={}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
run_window_signature={}
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root={}
invalid_work_rejection_signature={}
reward_settlement_records=1
reward_settlement_root={}
reward_settlement_signature={}
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.net/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.net/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.net/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,0,9,10,10,{}
{}
{}
{}
{}
",
            manifest_hash(b"public-evidence-bundle"),
            manifest_address(b"public-evidence-publisher"),
            manifest_publication_signature(),
            manifest_address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            manifest_auditor_signature(),
            manifest_artifact_line(
                PublicEvidenceRecordKind::BlockHistory,
                b"block-history-root",
                10
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::FinalityHistory,
                b"finality-history-root",
                10
            ),
            manifest_artifact_line_for_root(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                manifest_bundle().network_runtime_observation_root,
                3
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                b"data-availability-root",
                20
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::InvalidWorkRejections,
                b"invalid-work-root",
                1
            ),
            manifest_artifact_line(
                PublicEvidenceRecordKind::RewardSettlements,
                b"reward-settlement-root",
                1
            ),
            manifest_hash(b"block-history-root"),
            hex(&manifest_bundle().block_history_signature),
            manifest_hash(b"finality-history-root"),
            hex(&manifest_bundle().finality_history_signature),
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            manifest_address(b"miner-b"),
            manifest_hash(b"miner-b-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
            manifest_address(b"validator-a"),
            manifest_hash(b"validator-a-operator"),
            manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
            manifest_operator_signature(
                PublicNodeRole::Validator,
                b"validator-a",
                b"validator-a-operator"
            ),
            manifest_network_observation_lines(),
            hex(&manifest_bundle().network_runtime_observation_root),
            hex(&manifest_bundle().network_runtime_observation_signature),
            manifest_hash(b"data-availability-root"),
            hex(&manifest_bundle().data_availability_measurement_signature),
            hex(&manifest_bundle().run_window_signature),
            manifest_hash(b"invalid-work-root"),
            hex(&manifest_bundle().invalid_work_rejection_signature),
            manifest_hash(b"reward-settlement-root"),
            hex(&manifest_bundle().reward_settlement_signature),
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            manifest_address(b"miner-b"),
            manifest_hash(b"miner-b-operator"),
            manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
            manifest_address(b"validator-a"),
            manifest_hash(b"validator-a-operator"),
            manifest_node_signature(
                PublicNodeRole::Validator,
                b"validator-a",
                b"validator-a-operator"
            ),
            manifest_hash(b"rpc-service"),
            manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
            manifest_hash(b"telemetry-service"),
            manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
            manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
            manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
            manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
            manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
        )
    }

    fn preflight_manifest() -> String {
        format!(
            "\
version={PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION}
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,{},https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,{},https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
",
            manifest_hash(b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_hash(b"telemetry-service"),
        )
    }

    #[test]
    fn parses_documented_miner_commands() {
        assert_eq!(
            parse_cli_parts(&["miner", "register", "--stake", "100"]).unwrap(),
            CliCommand::MinerRegister { stake: 100 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "miner",
                "start",
                "--wallet",
                "miner.key",
                "--device",
                "cpu",
                "--node",
                "/ip4/127.0.0.1/tcp/4001"
            ])
            .unwrap(),
            CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cpu".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["miner", "status"]).unwrap(),
            CliCommand::MinerStatus
        );
    }

    #[test]
    fn parses_documented_validator_commands() {
        assert_eq!(
            parse_cli_parts(&["validator", "register", "--stake", "10000"]).unwrap(),
            CliCommand::ValidatorRegister { stake: 10_000 }
        );
        assert_eq!(
            parse_cli_parts(&[
                "validator",
                "start",
                "--wallet",
                "validator.key",
                "--node",
                "/ip4/127.0.0.1/tcp/4001"
            ])
            .unwrap(),
            CliCommand::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["validator", "status"]).unwrap(),
            CliCommand::ValidatorStatus
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "validate",
                "--manifest",
                "docs/tensorvm/public-testnet.evidence"
            ])
            .unwrap(),
            CliCommand::PublicEvidenceValidate {
                manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-testnet",
                "preflight",
                "--manifest",
                "docs/tensorvm/public-testnet.preflight"
            ])
            .unwrap(),
            CliCommand::PublicTestnetPreflight {
                manifest: "docs/tensorvm/public-testnet.preflight".to_owned(),
            }
        );
        let bundle_id = manifest_hash(b"public-evidence-bundle");
        let manifest_signer = manifest_address(b"public-evidence-publisher");
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "publication",
                "--bundle-id",
                &bundle_id,
                "--public-uri",
                "https://tensorvm.net/tensorvm/public-evidence.json",
                "--manifest-signer",
                &manifest_signer,
                "--manifest-signature-count",
                "1",
                "--independent-auditor-count",
                "1",
            ])
            .unwrap(),
            CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "auditor-record",
                "--bundle-id",
                &bundle_id,
                "--public-uri",
                "https://tensorvm.net/tensorvm/public-evidence.json",
                "--auditor-id",
                &manifest_address(b"public-evidence-auditor-0"),
                "--audit-uri",
                &manifest_auditor_uri(),
                "--observed-at",
                "1700000060",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 1_700_000_060,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "run-window",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--started-at",
                "1700000000",
                "--ended-at",
                "1700000060",
                "--observed-blocks",
                "10",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRunWindow {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 10,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "run-window-from-file",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--block-observation-file",
                "artifacts/block-observations.records",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRunWindowFromFile {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                block_observation_file: "artifacts/block-observations.records".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "node-heartbeat",
                "--role",
                "miner",
                "--address",
                &manifest_address(b"miner-a"),
                "--operator-id",
                &manifest_hash(b"miner-a-operator"),
                "--first-block",
                "0",
                "--last-block",
                "9",
                "--heartbeat-count",
                "10",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceNodeHeartbeat {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                first_seen_block: 0,
                last_seen_block: 9,
                signed_heartbeat_count: 10,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "node-heartbeat-from-file",
                "--role",
                "miner",
                "--address",
                &manifest_address(b"miner-a"),
                "--operator-id",
                &manifest_hash(b"miner-a-operator"),
                "--heartbeat-file",
                "artifacts/miner-a-heartbeats.records",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceNodeHeartbeatFromFile {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                heartbeat_file: "artifacts/miner-a-heartbeats.records".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "operator-attestation",
                "--role",
                "miner",
                "--address",
                &manifest_address(b"miner-a"),
                "--operator-id",
                &manifest_hash(b"miner-a-operator"),
                "--identity-uri",
                "https://operators.tensorvm.net/miner-a",
                "--observed-at",
                "1700000000",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            }
        );
        let endpoint_id = manifest_hash(b"rpc-service");
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-health",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.net/health",
                "--health-path",
                "/health",
                "--first-block",
                "0",
                "--last-block",
                "9",
                "--reachable-count",
                "10",
                "--signed-health-check-count",
                "10",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-health-from-file",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.net/health",
                "--health-path",
                "/health",
                "--observation-file",
                "artifacts/rpc-health.records",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceHealthFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                observation_file: "artifacts/rpc-health.records".to_owned(),
            }
        );
        let content_root = manifest_hash(b"rpc-service-content");
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-content",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.net/chain/head",
                "--content-path",
                "/chain/head",
                "--content-root",
                &content_root,
                "--observed-at",
                "1700000000",
                "--min-content-bytes",
                "64",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service-content"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            }
        );
        let content_hex = hex(&[42_u8; 64]);
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-content-from-bytes",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.net/chain/head",
                "--content-path",
                "/chain/head",
                "--observed-at",
                "1700000000",
                "--content-hex",
                &content_hex,
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "service-content-from-file",
                "--kind",
                "rpc",
                "--endpoint-id",
                &endpoint_id,
                "--public-url",
                "https://rpc.tensorvm.net/chain/head",
                "--content-path",
                "/chain/head",
                "--observed-at",
                "1700000000",
                "--content-file",
                "artifacts/rpc-chain-head.body",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceServiceContentFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_file: "artifacts/rpc-chain-head.body".to_owned(),
            }
        );
        let peer_id = PeerId::random().to_string();
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "network-observation",
                "--operator-id",
                &manifest_hash(b"network-operator"),
                "--peer-id",
                &peer_id,
                "--listen-address",
                "/dns/node-a.tensorvm.net/tcp/4001",
                "--observed-at",
                "1700000000",
                "--gossip-topics",
                "5",
                "--request-response-protocols",
                "3",
                "--bootstrap-peers",
                "2",
                "--max-transmit-bytes",
                "1048576",
                "--request-timeout-seconds",
                "10",
                "--max-concurrent-streams",
                "128",
                "--idle-timeout-seconds",
                "60",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceNetworkObservation {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                peer_id: peer_id.clone(),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                gossip_topic_count: 5,
                request_response_protocol_count: 3,
                bootstrap_peer_count: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "network-observation-from-service-log",
                "--operator-id",
                &manifest_hash(b"network-operator"),
                "--listen-address",
                "/dns/node-a.tensorvm.net/tcp/4001",
                "--observed-at",
                "1700000000",
                "--service-log",
                "artifacts/node-a-service.log",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                service_log: "artifacts/node-a-service.log".to_owned(),
            }
        );
        let record_root = manifest_hash(b"network-runtime-root");
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-summary",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--record-root",
                &record_root,
                "--record-count",
                "4",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-artifact",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--artifact-uri",
                "https://evidence.tensorvm.net/network-runtime.json",
                "--record-root",
                &record_root,
                "--record-count",
                "4",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            }
        );
        let record_roots = format!(
            "{},{}",
            manifest_hash(b"network-observation-a"),
            manifest_hash(b"network-observation-b")
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-summary-from-roots",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--record-roots",
                &record_roots,
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: vec![
                    hash_bytes(b"test", &[b"network-observation-a"]),
                    hash_bytes(b"test", &[b"network-observation-b"]),
                ],
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-artifact-from-roots",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--artifact-uri",
                "https://evidence.tensorvm.net/network-runtime.json",
                "--record-roots",
                &record_roots,
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordArtifactFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_roots: vec![
                    hash_bytes(b"test", &[b"network-observation-a"]),
                    hash_bytes(b"test", &[b"network-observation-b"]),
                ],
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-summary-from-file",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--record-file",
                "artifacts/network-runtime.records",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordSummaryFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_file: "artifacts/network-runtime.records".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "public-evidence",
                "record-artifact-from-file",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &bundle_id,
                "--manifest-signer",
                &manifest_signer,
                "--artifact-uri",
                "https://evidence.tensorvm.net/network-runtime.json",
                "--record-file",
                "artifacts/network-runtime.records",
            ])
            .unwrap(),
            CliCommand::PublicEvidenceRecordArtifactFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_file: "artifacts/network-runtime.records".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&["service", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
            CliCommand::ServiceInit {
                data_dir: "/var/lib/tensorvm".to_owned(),
            }
        );
        let bootstrap_peer = PeerId::random().to_string();
        assert_eq!(
            parse_cli_parts(&[
                "service",
                "peer",
                "add",
                "--data-dir",
                "/var/lib/tensorvm",
                "--peer-id",
                &bootstrap_peer,
                "--address",
                "/dns/bootstrap.tensorvm.net/tcp/4001",
            ])
            .unwrap(),
            CliCommand::ServicePeerAdd {
                data_dir: "/var/lib/tensorvm".to_owned(),
                peer_id: bootstrap_peer.clone(),
                address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
            }
        );
        assert_eq!(
            parse_cli_parts(&[
                "service",
                "serve",
                "--listen",
                "0.0.0.0:8545",
                "--p2p-listen",
                "/ip4/0.0.0.0/tcp/4001",
                "--data-dir",
                "/var/lib/tensorvm",
                "--auth-token",
                "secret",
                "--max-requests",
                "0",
            ])
            .unwrap(),
            CliCommand::ServiceServe {
                listen: "0.0.0.0:8545".to_owned(),
                p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            }
        );
    }

    #[test]
    fn rejects_invalid_cli() {
        assert!(parse_cli_parts(&["miner", "register"]).is_err());
        assert!(parse_cli_parts(&["validator", "register", "--stake", "abc"]).is_err());
    }

    #[test]
    fn parse_cli_args_and_describe_commands() {
        let args = vec![
            "miner".to_owned(),
            "register".to_owned(),
            "--stake".to_owned(),
            "250".to_owned(),
        ];
        let command = parse_cli_args(&args).unwrap();
        assert_eq!(command, CliCommand::MinerRegister { stake: 250 });
        let bootstrap_peer = PeerId::random().to_string();

        let commands = [
            (
                CliCommand::MinerRegister { stake: 1 },
                "register miner with stake 1",
            ),
            (
                CliCommand::MinerStart {
                    wallet: "miner.key".to_owned(),
                    device: "cpu".to_owned(),
                    node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                },
                "start miner wallet=miner.key device=cpu node=/ip4/127.0.0.1/tcp/4001",
            ),
            (CliCommand::MinerStatus, "show miner status"),
            (
                CliCommand::ValidatorRegister { stake: 10 },
                "register validator with stake 10",
            ),
            (
                CliCommand::ValidatorStart {
                    wallet: "validator.key".to_owned(),
                    node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                },
                "start validator wallet=validator.key node=/ip4/127.0.0.1/tcp/4001",
            ),
            (CliCommand::ValidatorStatus, "show validator status"),
            (
                CliCommand::ServiceInit {
                    data_dir: "/var/lib/tensorvm".to_owned(),
                },
                "initialize service node store data_dir=/var/lib/tensorvm",
            ),
            (
                CliCommand::ServicePeerAdd {
                    data_dir: "/var/lib/tensorvm".to_owned(),
                    peer_id: bootstrap_peer.clone(),
                    address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
                },
                "add libp2p bootstrap peer data_dir=/var/lib/tensorvm peer_id=",
            ),
            (
                CliCommand::ServiceServe {
                    listen: "0.0.0.0:8545".to_owned(),
                    p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
                    data_dir: "/var/lib/tensorvm".to_owned(),
                    auth_token: "secret".to_owned(),
                    max_requests: 0,
                },
                "serve RPC explorer faucet telemetry over mandatory libp2p listen=0.0.0.0:8545 p2p_listen=/ip4/0.0.0.0/tcp/4001 data_dir=/var/lib/tensorvm max_requests=0 max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
            ),
            (
                CliCommand::PublicEvidenceValidate {
                    manifest: "evidence.txt".to_owned(),
                },
                "validate public evidence manifest evidence.txt",
            ),
            (
                CliCommand::PublicTestnetPreflight {
                    manifest: "preflight.txt".to_owned(),
                },
                "run public testnet preflight manifest preflight.txt",
            ),
        ];
        for (command, description) in commands {
            let actual = describe_command(&command);
            if matches!(command, CliCommand::ServicePeerAdd { .. }) {
                assert!(actual.starts_with(description));
                assert!(actual.contains("address=/dns/bootstrap.tensorvm.net/tcp/4001"));
            } else {
                assert_eq!(actual, description);
            }
        }

        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            }),
            "generate rpc service health evidence public_url=https://rpc.tensorvm.net/health health_path=/health"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceHealthFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                observation_file: "artifacts/rpc-health.records".to_owned(),
            }),
            "generate rpc service health evidence from captured observations observation_file=artifacts/rpc-health.records public_url=https://rpc.tensorvm.net/health health_path=/health"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Explorer,
                endpoint_id: hash_bytes(b"test", &[b"explorer-service"]),
                public_url: "https://explorer.tensorvm.net/explorer".to_owned(),
                content_path: "/explorer".to_owned(),
                content_root: hash_bytes(b"test", &[b"explorer-service-content"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            }),
            "generate explorer service content evidence public_url=https://explorer.tensorvm.net/explorer content_path=/explorer"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Faucet,
                endpoint_id: hash_bytes(b"test", &[b"faucet-service"]),
                public_url: "https://faucet.tensorvm.net/faucet/page".to_owned(),
                content_path: "/faucet/page".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex: hex(&[1_u8; 64]),
            }),
            "generate faucet service content evidence from observed bytes public_url=https://faucet.tensorvm.net/faucet/page content_path=/faucet/page"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceServiceContentFromFile {
                kind: PublicServiceKind::Telemetry,
                endpoint_id: hash_bytes(b"test", &[b"telemetry-service"]),
                public_url: "https://telemetry.tensorvm.net/telemetry/dashboard".to_owned(),
                content_path: "/telemetry/dashboard".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_file: "artifacts/telemetry-dashboard.body".to_owned(),
            }),
            "generate telemetry service content evidence from captured file content_file=artifacts/telemetry-dashboard.body public_url=https://telemetry.tensorvm.net/telemetry/dashboard content_path=/telemetry/dashboard"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            }),
            "generate public evidence publication signature public_uri=https://tensorvm.net/tensorvm/public-evidence.json"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRunWindow {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 10,
            }),
            "generate public evidence run window started=1700000000 ended=1700000060 observed_blocks=10"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRunWindowFromFile {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                block_observation_file: "artifacts/block-observations.records".to_owned(),
            }),
            "generate public evidence run window from captured block observations block_observation_file=artifacts/block-observations.records"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 1_700_000_000,
            }),
            format!(
                "generate public evidence auditor record auditor_id={} audit_uri={}",
                manifest_address(b"public-evidence-auditor-0"),
                manifest_auditor_uri()
            )
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: vec![
                    hash_bytes(b"test", &[b"network-observation-a"]),
                    hash_bytes(b"test", &[b"network-observation-b"]),
                ],
            }),
            "generate network-runtime public evidence record summary from 2 roots"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::InvalidWorkRejections,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: hash_bytes(b"test", &[b"invalid-work-root"]),
                record_count: 1,
            }),
            "generate invalid-work public evidence record summary records=1"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::RewardSettlements,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
                record_count: 1,
            }),
            "generate reward-settlement public evidence record summary records=1"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            }),
            "generate network-runtime public evidence artifact locator artifact_uri=https://evidence.tensorvm.net/network-runtime.json"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordArtifactFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_roots: vec![
                    hash_bytes(b"test", &[b"network-observation-a"]),
                    hash_bytes(b"test", &[b"network-observation-b"]),
                ],
            }),
            "generate network-runtime public evidence artifact locator from 2 roots artifact_uri=https://evidence.tensorvm.net/network-runtime.json"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordSummaryFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_file: "artifacts/network-runtime.records".to_owned(),
            }),
            "generate network-runtime public evidence record summary from record file record_file=artifacts/network-runtime.records"
        );
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceRecordArtifactFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_file: "artifacts/network-runtime.records".to_owned(),
            }),
            "generate network-runtime public evidence artifact locator from record file record_file=artifacts/network-runtime.records artifact_uri=https://evidence.tensorvm.net/network-runtime.json"
        );
        let peer_id = PeerId::random().to_string();
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceNetworkObservation {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                peer_id: peer_id.clone(),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                gossip_topic_count: 5,
                request_response_protocol_count: 3,
                bootstrap_peer_count: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            }),
            format!(
                "generate signed libp2p network observation peer_id={peer_id} listen_address=/dns/node-a.tensorvm.net/tcp/4001"
            )
        );
        assert_eq!(
            describe_command(
                &CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
                    operator_id: hash_bytes(b"test", &[b"network-operator"]),
                    listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                    observed_at_unix_seconds: 1_700_000_000,
                    service_log: "artifacts/node-a-service.log".to_owned(),
                }
            ),
            "generate signed libp2p network observation from service log service_log=artifacts/node-a-service.log listen_address=/dns/node-a.tensorvm.net/tcp/4001"
        );

        let node_roles = [
            (
                PublicNodeRole::Miner,
                address(b"miner-a"),
                "generate miner node heartbeat evidence address=",
            ),
            (
                PublicNodeRole::Validator,
                address(b"validator-a"),
                "generate validator node heartbeat evidence address=",
            ),
        ];
        for (role, node_address, prefix) in node_roles {
            assert_eq!(
                describe_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                    role,
                    address: node_address,
                    operator_id: hash_bytes(b"test", &[b"operator"]),
                    first_seen_block: 0,
                    last_seen_block: 9,
                    signed_heartbeat_count: 10,
                }),
                format!("{prefix}{}", hex(&node_address))
            );
        }
        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceNodeHeartbeatFromFile {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"operator"]),
                heartbeat_file: "artifacts/miner-a-heartbeats.records".to_owned(),
            }),
            format!(
                "generate miner node heartbeat evidence from captured observations heartbeat_file=artifacts/miner-a-heartbeats.records address={}",
                hex(&address(b"miner-a"))
            )
        );

        assert_eq!(
            describe_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            }),
            format!(
                "generate miner operator identity attestation address={} identity_uri=https://operators.tensorvm.net/miner-a",
                manifest_address(b"miner-a")
            )
        );

        let record_kinds = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                "generate block-history public evidence record summary records=10",
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                "generate finality-history public evidence record summary records=10",
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                "generate network-runtime public evidence record summary records=10",
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                "generate data-availability public evidence record summary records=10",
            ),
        ];
        for (kind, expected) in record_kinds {
            assert_eq!(
                describe_command(&CliCommand::PublicEvidenceRecordSummary {
                    kind,
                    bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    manifest_signer: address(b"public-evidence-publisher"),
                    record_root: hash_bytes(b"test", &[b"record-root"]),
                    record_count: 10,
                }),
                expected
            );
        }
    }

    #[test]
    fn execute_reference_cli_command_reports_miner_and_validator_readiness() {
        let miner_register =
            execute_reference_cli_command(&CliCommand::MinerRegister { stake: 100 }).unwrap();
        assert!(miner_register.contains("command=miner_register"));
        assert!(miner_register.contains("min_stake=100"));
        assert!(miner_register.contains("stake_sufficient=true"));

        let miner_start = execute_reference_cli_command(&CliCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .unwrap();
        assert!(miner_start.contains("command=miner_start"));
        assert!(miner_start.contains("wallet=miner.key"));
        assert!(miner_start.contains("device=cpu"));
        assert!(miner_start.contains("device_backend=cpu-reference"));
        assert!(miner_start.contains(&format!(
            "cuda_kernels_compiled={}",
            cuda_kernels_compiled()
        )));
        assert!(miner_start.contains("node=/ip4/127.0.0.1/tcp/4001"));
        assert!(miner_start.contains(&format!("address={}", hex(&address(b"miner.key")))));
        assert!(miner_start.contains("reference_backend_ready=true"));

        let validator_register =
            execute_reference_cli_command(&CliCommand::ValidatorRegister { stake: 10_000 })
                .unwrap();
        assert!(validator_register.contains("command=validator_register"));
        assert!(validator_register.contains("min_stake=10000"));

        let validator_start = execute_reference_cli_command(&CliCommand::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        })
        .unwrap();
        assert!(validator_start.contains("command=validator_start"));
        assert!(validator_start.contains("reference_verifier_ready=true"));

        let miner_status = execute_reference_cli_command(&CliCommand::MinerStatus).unwrap();
        assert!(miner_status.contains("command=miner_status"));
        assert!(miner_status.contains("status_source=rpc_or_node_store_required"));

        let validator_status = execute_reference_cli_command(&CliCommand::ValidatorStatus).unwrap();
        assert!(validator_status.contains("command=validator_status"));
        assert!(validator_status.contains("status_source=rpc_or_node_store_required"));

        let service_init = execute_reference_cli_command(&CliCommand::ServiceInit {
            data_dir: "/var/lib/tensorvm".to_owned(),
        })
        .unwrap();
        assert!(service_init.contains("command=service_init"));
        assert!(service_init.contains("node_store_ready=true"));

        let bootstrap_peer = PeerId::random().to_string();
        let service_peer_add = execute_reference_cli_command(&CliCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: bootstrap_peer.clone(),
            address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
        })
        .unwrap();
        assert!(service_peer_add.contains("command=service_peer_add"));
        assert!(service_peer_add.contains(&format!("peer_id={bootstrap_peer}")));
        assert!(service_peer_add.contains("peer_book_ready=true"));

        let service_serve = execute_reference_cli_command(&CliCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            auth_token: "secret".to_owned(),
            max_requests: 0,
        })
        .unwrap();
        assert!(service_serve.contains("command=service_serve"));
        assert!(service_serve.contains("p2p_runtime=libp2p"));
        assert!(service_serve.contains("p2p_gossipsub=enabled"));
        assert!(service_serve.contains("p2p_identify=enabled"));
        assert!(service_serve.contains("p2p_kademlia=enabled"));
        assert!(service_serve.contains("p2p_request_response=enabled"));
        assert!(service_serve.contains("p2p_max_transmit_bytes=1048576"));
        assert!(service_serve.contains("p2p_request_timeout_seconds=10"));
        assert!(service_serve.contains("p2p_max_concurrent_streams=128"));
        assert!(service_serve.contains("p2p_idle_timeout_seconds=60"));
        assert!(service_serve.contains("auth_enabled=true"));
        assert!(service_serve.contains("rpc_routes=enabled"));
        assert!(service_serve.contains("explorer_routes=enabled"));
        assert!(service_serve.contains("faucet_routes=enabled"));
        assert!(service_serve.contains("telemetry_routes=enabled"));
        assert!(service_serve.contains("node_store_required=true"));

        let public_command = CliCommand::PublicEvidenceValidate {
            manifest: "evidence.txt".to_owned(),
        };
        assert_eq!(
            execute_reference_cli_command(&public_command).unwrap(),
            describe_command(&public_command)
        );

        let publication = execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .unwrap();
        assert!(publication.contains(&format!(
            "bundle_id={}",
            manifest_hash(b"public-evidence-bundle")
        )));
        assert!(
            publication.contains("public_uri=https://tensorvm.net/tensorvm/public-evidence.json")
        );
        assert!(publication.contains(&format!(
            "manifest_signer={}",
            manifest_address(b"public-evidence-publisher")
        )));
        assert!(publication.contains(&format!(
            "manifest_signature={}",
            manifest_publication_signature()
        )));
        assert!(publication.contains("manifest_signature_count=1"));
        assert!(publication.contains("independent_auditor_count=1"));

        let auditor_record =
            execute_reference_cli_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 1_700_000_060,
            })
            .unwrap();
        assert_eq!(
            auditor_record,
            format!(
                "auditor={},{},1700000060,{}",
                manifest_address(b"public-evidence-auditor-0"),
                manifest_auditor_uri(),
                manifest_auditor_signature()
            )
        );

        let run_window = execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .unwrap();
        assert_eq!(
            run_window,
            format!(
                "run_started_at_unix_seconds=1700000000\nrun_ended_at_unix_seconds=1700000060\nrun_window_signature={}\nobserved_blocks=10",
                hex(&manifest_bundle().run_window_signature)
            )
        );
        let run_window_observation_file = std::env::temp_dir().join(format!(
            "tensor-vm-run-window-{}.records",
            std::process::id()
        ));
        let run_window_observations = (0..10)
            .map(|block| {
                let timestamp = if block == 9 {
                    1_700_000_060
                } else {
                    1_700_000_000 + block * 6
                };
                format!("run_window_observation={block},{timestamp}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&run_window_observation_file, run_window_observations).unwrap();
        let run_window_from_file =
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindowFromFile {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                block_observation_file: run_window_observation_file.to_string_lossy().into_owned(),
            })
            .unwrap();
        std::fs::remove_file(&run_window_observation_file).unwrap();
        assert_eq!(run_window_from_file, run_window);

        let node_cases = [
            (
                PublicNodeRole::Miner,
                b"miner-a".as_slice(),
                b"miner-a-operator".as_slice(),
                "miner",
            ),
            (
                PublicNodeRole::Validator,
                b"validator-a".as_slice(),
                b"validator-a-operator".as_slice(),
                "validator",
            ),
        ];
        for (role, address_label, operator_label, tag) in node_cases {
            let node = execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                role,
                address: address(address_label),
                operator_id: hash_bytes(b"test", &[operator_label]),
                first_seen_block: 0,
                last_seen_block: 9,
                signed_heartbeat_count: 10,
            })
            .unwrap();
            assert!(node.starts_with(&format!(
                "node={tag},{},{}",
                hex(&address(address_label)),
                hex(&hash_bytes(b"test", &[operator_label]))
            )));
            assert!(node.ends_with(&manifest_node_signature(
                role,
                address_label,
                operator_label
            )));
            let heartbeat_file = std::env::temp_dir().join(format!(
                "tensor-vm-node-heartbeat-{}-{}.records",
                std::process::id(),
                tag
            ));
            let heartbeat_records = (0..10)
                .map(|block| {
                    format!(
                        "node_heartbeat_observation={tag},{},{},{}",
                        hex(&address(address_label)),
                        hex(&hash_bytes(b"test", &[operator_label])),
                        block
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&heartbeat_file, heartbeat_records).unwrap();
            let node_from_file =
                execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeatFromFile {
                    role,
                    address: address(address_label),
                    operator_id: hash_bytes(b"test", &[operator_label]),
                    heartbeat_file: heartbeat_file.to_string_lossy().into_owned(),
                })
                .unwrap();
            std::fs::remove_file(&heartbeat_file).unwrap();
            assert_eq!(node_from_file, node);
        }

        let operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
        let operator_identity_uri = manifest_operator_identity_uri(&operator_id);
        let operator_attestation =
            execute_reference_cli_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id,
                identity_uri: operator_identity_uri.clone(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .unwrap();
        assert_eq!(
            operator_attestation,
            format!(
                "operator=miner,{},{},{operator_identity_uri},1700000000,{}",
                manifest_address(b"miner-a"),
                manifest_hash(b"miner-a-operator"),
                manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator")
            )
        );

        let service_health =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .unwrap();
        assert!(service_health.starts_with("service=rpc,"));
        assert!(service_health.contains("https://rpc.tensorvm.net/health,/health,0,9,10,10"));
        assert!(service_health.ends_with(&manifest_service_signature(
            PublicServiceKind::Rpc,
            b"rpc-service"
        )));
        let health_observation_file = std::env::temp_dir().join(format!(
            "tensor-vm-service-health-{}-{}.records",
            std::process::id(),
            manifest_hash(b"rpc-service").as_bytes()[0]
        ));
        let health_observations = (0..10)
            .map(|block| format!("service_health_observation={block},reachable"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&health_observation_file, health_observations).unwrap();
        let service_health_from_file =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealthFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                observation_file: health_observation_file.to_string_lossy().into_owned(),
            })
            .unwrap();
        std::fs::remove_file(&health_observation_file).unwrap();
        assert_eq!(service_health_from_file, service_health);
        let additional_service_cases: [(PublicServiceKind, &[u8], &str); 3] = [
            (PublicServiceKind::Explorer, b"explorer-service", "explorer"),
            (PublicServiceKind::Faucet, b"faucet-service", "faucet"),
            (
                PublicServiceKind::Telemetry,
                b"telemetry-service",
                "telemetry",
            ),
        ];
        for (kind, label, tag) in additional_service_cases {
            let line = execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind,
                endpoint_id: hash_bytes(b"test", &[label]),
                public_url: public_service_url(kind).to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .unwrap();
            assert!(line.starts_with(&format!("service={tag},")));
            assert!(line.contains(public_service_url(kind)));
            assert!(line.ends_with(&manifest_service_signature(kind, label)));
        }

        let service_content =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
                content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .unwrap();
        assert!(service_content.starts_with("service_content=rpc,"));
        assert!(service_content.contains("https://rpc.tensorvm.net/chain/head,/chain/head"));
        assert_eq!(
            service_content,
            manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service")
        );
        let observed_content = vec![7_u8; 80];
        let observed_content_root = public_service_content_root(&observed_content);
        let service_content_from_bytes =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
                content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex: hex(&observed_content),
            })
            .unwrap();
        assert!(service_content_from_bytes.starts_with("service_content=rpc,"));
        assert!(
            service_content_from_bytes
                .contains(&format!("{},1700000000,80,", hex(&observed_content_root)))
        );
        let content_file = std::env::temp_dir().join(format!(
            "tensor-vm-service-content-{}-{}.body",
            std::process::id(),
            observed_content_root[0]
        ));
        std::fs::write(&content_file, &observed_content).unwrap();
        let service_content_from_file =
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: public_service_content_url(PublicServiceKind::Rpc).to_owned(),
                content_path: public_service_content_path(PublicServiceKind::Rpc).to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_file: content_file.to_string_lossy().into_owned(),
            })
            .unwrap();
        std::fs::remove_file(&content_file).unwrap();
        assert_eq!(service_content_from_file, service_content_from_bytes);

        let peer_id = PeerId::random().to_string();
        let network_observation =
            execute_reference_cli_command(&CliCommand::PublicEvidenceNetworkObservation {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                peer_id: peer_id.clone(),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                gossip_topic_count: 5,
                request_response_protocol_count: 3,
                bootstrap_peer_count: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            })
            .unwrap();
        let observation_input = NetworkObservationEvidenceLine {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            peer_id: &peer_id,
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001",
            observed_at_unix_seconds: 1_700_000_000,
            gossip_topic_count: 5,
            request_response_protocol_count: 3,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        };
        let observation_root = network_observation_root(
            &observation_input,
            &peer_id,
            "/dns/node-a.tensorvm.net/tcp/4001",
        );
        let observation_signature = hash_bytes(
            b"tensor-vm-network-runtime-observation-signature-v1",
            &[&observation_input.operator_id, &observation_root],
        );
        assert_eq!(
            network_observation,
            format!(
                "network_runtime_observation={},{peer_id},/dns/node-a.tensorvm.net/tcp/4001,1700000000,5,3,2,1048576,10,128,60,{},{}",
                hex(&observation_input.operator_id),
                hex(&observation_root),
                hex(&observation_signature)
            )
        );
        let service_log = format!(
            "\
command=service_serve
p2p_runtime=libp2p
p2p_peer_id={peer_id}
p2p_gossipsub_topics=5
p2p_request_response_protocols=3
p2p_bootstrap_peers=2
p2p_max_transmit_bytes=1048576
p2p_request_timeout_seconds=10
p2p_max_concurrent_streams=128
p2p_idle_timeout_seconds=60
"
        );
        assert_eq!(
            service_log_field(&service_log, "p2p_peer_id").unwrap(),
            peer_id
        );
        let network_observation_from_service_log =
            network_observation_evidence_line_from_service_log(
                hash_bytes(b"test", &[b"network-operator"]),
                "/dns/node-a.tensorvm.net/tcp/4001",
                1_700_000_000,
                &service_log,
            )
            .unwrap();
        assert_eq!(network_observation_from_service_log, network_observation);

        let service_log_file = std::env::temp_dir().join(format!(
            "tensor-vm-service-log-{}-{}.log",
            std::process::id(),
            observation_root[0]
        ));
        std::fs::write(&service_log_file, &service_log).unwrap();
        let network_observation_from_file = execute_reference_cli_command(
            &CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
                operator_id: hash_bytes(b"test", &[b"network-operator"]),
                listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                service_log: service_log_file.to_string_lossy().into_owned(),
            },
        )
        .unwrap();
        std::fs::remove_file(&service_log_file).unwrap();
        assert_eq!(network_observation_from_file, network_observation);

        assert_eq!(
            execute_reference_cli_command(
                &CliCommand::PublicEvidenceNetworkObservationFromServiceLog {
                    operator_id: hash_bytes(b"test", &[b"network-operator"]),
                    listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
                    observed_at_unix_seconds: 1_700_000_000,
                    service_log: service_log_file.to_string_lossy().into_owned(),
                }
            )
            .unwrap_err()
            .to_string(),
            "storage error: failed to read service log file"
        );
        assert_eq!(
            network_observation_evidence_line_from_service_log(
                hash_bytes(b"test", &[b"network-operator"]),
                "/dns/node-a.tensorvm.net/tcp/4001",
                1_700_000_000,
                "command=service_init\np2p_runtime=libp2p\n",
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: service log is not service_serve"
        );
        assert_eq!(
            network_observation_evidence_line_from_service_log(
                hash_bytes(b"test", &[b"network-operator"]),
                "/dns/node-a.tensorvm.net/tcp/4001",
                1_700_000_000,
                "command=service_serve\np2p_runtime=shim\n",
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: service log does not prove libp2p runtime"
        );
        assert_eq!(
            service_log_field("command=service_serve\n", "p2p_peer_id")
                .unwrap_err()
                .to_string(),
            "invalid receipt: missing service log field"
        );
        assert_eq!(
            service_log_field("p2p_runtime=libp2p\np2p_runtime=libp2p\n", "p2p_runtime")
                .unwrap_err()
                .to_string(),
            "invalid receipt: duplicate service log field"
        );
        assert_eq!(
            service_log_field("p2p_runtime= libp2p\n", "p2p_runtime")
                .unwrap_err()
                .to_string(),
            "invalid receipt: invalid service log field"
        );

        let record_cases: [(PublicEvidenceRecordKind, &[u8], u64, &str, String); 6] = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                b"block-history-root",
                10,
                "block_history",
                hex(&manifest_bundle().block_history_signature),
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                b"finality-history-root",
                10,
                "finality_history",
                hex(&manifest_bundle().finality_history_signature),
            ),
            (
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                b"network-runtime-root",
                3,
                "network_runtime_observation",
                hex(&manifest_bundle().network_runtime_observation_signature),
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                b"data-availability-root",
                20,
                "data_availability_measurement",
                hex(&manifest_bundle().data_availability_measurement_signature),
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                b"invalid-work-root",
                1,
                "invalid_work_rejection",
                hex(&manifest_bundle().invalid_work_rejection_signature),
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                b"reward-settlement-root",
                1,
                "reward_settlement",
                hex(&manifest_bundle().reward_settlement_signature),
            ),
        ];
        for (kind, root_label, count, field_prefix, expected_signature) in record_cases {
            let record_root =
                if matches!(kind, PublicEvidenceRecordKind::NetworkRuntimeObservations) {
                    manifest_bundle().network_runtime_observation_root
                } else {
                    hash_bytes(b"test", &[root_label])
                };
            let root = hex(&record_root);
            let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
            let manifest_signer = address(b"public-evidence-publisher");
            let line = execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummary {
                kind,
                bundle_id,
                manifest_signer,
                record_root,
                record_count: count,
            })
            .unwrap();
            assert_eq!(
                line,
                format!(
                    "{field_prefix}_records={count}\n{field_prefix}_root={root}\n{field_prefix}_signature={expected_signature}"
                )
            );

            let artifact_uri = format!(
                "https://evidence.tensorvm.net/{}/{}.json",
                manifest_hash(b"public-evidence-bundle"),
                public_evidence_record_kind_tag(kind)
            );
            let artifact_signature = crate::testnet::sign_public_evidence_artifact(
                &manifest_signer,
                &bundle_id,
                kind,
                &artifact_uri,
                &record_root,
                count,
            );
            let artifact_line =
                execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                    kind,
                    bundle_id,
                    manifest_signer,
                    artifact_uri: artifact_uri.clone(),
                    record_root,
                    record_count: count,
                })
                .unwrap();
            assert_eq!(
                artifact_line,
                format!(
                    "record_artifact={},{artifact_uri},{root},{count},{}",
                    public_evidence_record_kind_tag(kind),
                    hex(&artifact_signature)
                )
            );
        }

        let roots = vec![
            hash_bytes(b"test", &[b"network-observation-a"]),
            hash_bytes(b"test", &[b"network-observation-b"]),
        ];
        let aggregate_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &roots,
        )
        .unwrap();
        let aggregate_signature = sign_public_evidence_record(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &aggregate_root,
            roots.len() as u64,
        );
        let aggregate_line =
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: roots.clone(),
            })
            .unwrap();
        assert_eq!(
            aggregate_line,
            format!(
                "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
                hex(&aggregate_root),
                hex(&aggregate_signature)
            )
        );
        let aggregate_artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
        let aggregate_artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            aggregate_artifact_uri,
            &aggregate_root,
            roots.len() as u64,
        );
        let aggregate_artifact_line =
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifactFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: aggregate_artifact_uri.to_owned(),
                record_roots: roots,
            })
            .unwrap();
        assert_eq!(
            aggregate_artifact_line,
            format!(
                "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
                hex(&aggregate_root),
                hex(&aggregate_artifact_signature)
            )
        );

        let record_file_roots = vec![
            observation_root,
            hash_bytes(b"test", &[b"network-observation-b"]),
        ];
        let record_file_aggregate_root = aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_file_roots,
        )
        .unwrap();
        let record_file = std::env::temp_dir().join(format!(
            "tensor-vm-network-records-{}-{}.records",
            std::process::id(),
            record_file_aggregate_root[0]
        ));
        std::fs::write(
            &record_file,
            format!(
                "# captured network-runtime records\n\n{network_observation}\nrecord_root={}\n",
                hex(&record_file_roots[1])
            ),
        )
        .unwrap();
        let record_file_path = record_file.to_string_lossy().into_owned();
        let record_file_roots_from_disk = public_evidence_record_roots_from_file(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_file_path,
        )
        .unwrap();
        assert_eq!(record_file_roots_from_disk, record_file_roots);
        let record_file_summary =
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_file: record_file_path.clone(),
            })
            .unwrap();
        let record_file_signature = sign_public_evidence_record(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &record_file_aggregate_root,
            record_file_roots.len() as u64,
        );
        assert_eq!(
            record_file_summary,
            format!(
                "network_runtime_observation_records=2\nnetwork_runtime_observation_root={}\nnetwork_runtime_observation_signature={}",
                hex(&record_file_aggregate_root),
                hex(&record_file_signature)
            )
        );
        let record_file_artifact =
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifactFromFile {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: aggregate_artifact_uri.to_owned(),
                record_file: record_file_path.clone(),
            })
            .unwrap();
        let record_file_artifact_signature = crate::testnet::sign_public_evidence_artifact(
            &address(b"public-evidence-publisher"),
            &hash_bytes(b"test", &[b"public-evidence-bundle"]),
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            aggregate_artifact_uri,
            &record_file_aggregate_root,
            record_file_roots.len() as u64,
        );
        assert_eq!(
            record_file_artifact,
            format!(
                "record_artifact=network-runtime,{aggregate_artifact_uri},{},2,{}",
                hex(&record_file_aggregate_root),
                hex(&record_file_artifact_signature)
            )
        );
        std::fs::remove_file(&record_file).unwrap();
        assert_eq!(
            supporting_record_line_prefix(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            None
        );

        let supporting_record_cases = [
            (
                PublicEvidenceRecordKind::BlockHistory,
                "block_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "block_history",
            ),
            (
                PublicEvidenceRecordKind::FinalityHistory,
                "finality_history_record=0,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,finalized",
                "finality_history",
            ),
            (
                PublicEvidenceRecordKind::DataAvailabilityMeasurements,
                "data_availability_measurement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,available,0",
                "data_availability_measurement",
            ),
            (
                PublicEvidenceRecordKind::InvalidWorkRejections,
                "invalid_work_rejection=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,rejected,0",
                "invalid_work_rejection",
            ),
            (
                PublicEvidenceRecordKind::RewardSettlements,
                "reward_settlement=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,miner,validator,0",
                "reward_settlement",
            ),
        ];
        for (kind, raw_line, field_prefix) in supporting_record_cases {
            let raw_root = supporting_record_root_from_line(
                kind,
                raw_line,
                supporting_record_line_prefix(kind).unwrap(),
            )
            .unwrap();
            assert_eq!(
                public_evidence_record_root_from_line(kind, raw_line).unwrap(),
                raw_root
            );
            let extra_root =
                hash_bytes(b"test", &[public_evidence_record_kind_tag(kind).as_bytes()]);
            let roots = vec![raw_root, extra_root];
            let aggregate_root = aggregate_public_evidence_record_roots(kind, &roots).unwrap();
            let raw_record_file = std::env::temp_dir().join(format!(
                "tensor-vm-{}-records-{}-{}.records",
                public_evidence_record_kind_tag(kind),
                std::process::id(),
                aggregate_root[0]
            ));
            std::fs::write(
                &raw_record_file,
                format!(
                    "# raw supporting records\n{raw_line}\nrecord_root={}\n",
                    hex(&extra_root)
                ),
            )
            .unwrap();
            let raw_record_file_path = raw_record_file.to_string_lossy().into_owned();
            assert_eq!(
                public_evidence_record_roots_from_file(kind, &raw_record_file_path).unwrap(),
                roots
            );
            let summary =
                execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromFile {
                    kind,
                    bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    manifest_signer: address(b"public-evidence-publisher"),
                    record_file: raw_record_file_path.clone(),
                })
                .unwrap();
            let signature = sign_public_evidence_record(
                &address(b"public-evidence-publisher"),
                &hash_bytes(b"test", &[b"public-evidence-bundle"]),
                kind,
                &aggregate_root,
                roots.len() as u64,
            );
            assert_eq!(
                summary,
                format!(
                    "{field_prefix}_records=2\n{field_prefix}_root={}\n{field_prefix}_signature={}",
                    hex(&aggregate_root),
                    hex(&signature)
                )
            );
            let artifact_uri = format!(
                "https://evidence.tensorvm.net/{}.json",
                public_evidence_record_kind_tag(kind)
            );
            let artifact =
                execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifactFromFile {
                    kind,
                    bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    manifest_signer: address(b"public-evidence-publisher"),
                    artifact_uri: artifact_uri.clone(),
                    record_file: raw_record_file_path,
                })
                .unwrap();
            let artifact_signature = crate::testnet::sign_public_evidence_artifact(
                &address(b"public-evidence-publisher"),
                &hash_bytes(b"test", &[b"public-evidence-bundle"]),
                kind,
                &artifact_uri,
                &aggregate_root,
                roots.len() as u64,
            );
            assert_eq!(
                artifact,
                format!(
                    "record_artifact={},{},{},2,{}",
                    public_evidence_record_kind_tag(kind),
                    artifact_uri,
                    hex(&aggregate_root),
                    hex(&artifact_signature)
                )
            );
            std::fs::remove_file(&raw_record_file).unwrap();
        }
        assert_eq!(
            public_evidence_record_roots_from_file(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &record_file_path,
            )
            .unwrap_err()
            .to_string(),
            "storage error: failed to read public evidence record file"
        );
        let empty_record_file = std::env::temp_dir().join(format!(
            "tensor-vm-empty-records-{}-{}.records",
            std::process::id(),
            record_file_aggregate_root[1]
        ));
        std::fs::write(&empty_record_file, "# no roots yet\n\n").unwrap();
        assert_eq!(
            public_evidence_record_roots_from_file(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                &empty_record_file.to_string_lossy(),
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: record file has no roots"
        );
        std::fs::remove_file(&empty_record_file).unwrap();
        assert_eq!(
            public_evidence_record_root_from_line(
                PublicEvidenceRecordKind::FinalityHistory,
                "network_runtime_observation=bad",
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: unsupported public evidence record line"
        );
        assert_eq!(
            public_evidence_record_root_from_line(
                PublicEvidenceRecordKind::BlockHistory,
                &format!(
                    "record_root= {}",
                    hex(&hash_bytes(b"test", &[b"bad-whitespace"]))
                ),
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: invalid record root file line"
        );
        assert_eq!(
            public_evidence_record_root_from_line(
                PublicEvidenceRecordKind::NetworkRuntimeObservations,
                "network_runtime_observation=bad",
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: invalid network observation record line"
        );
        assert_eq!(
            public_evidence_record_root_from_line(
                PublicEvidenceRecordKind::BlockHistory,
                "block_history_record= ",
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: invalid public evidence supporting record line"
        );
        let whitespace_record_file = std::env::temp_dir().join(format!(
            "tensor-vm-whitespace-record-{}.records",
            std::process::id()
        ));
        std::fs::write(&whitespace_record_file, " block_history_record=0\n").unwrap();
        let whitespace_record_path = whitespace_record_file.to_string_lossy().into_owned();
        assert_eq!(
            public_evidence_record_roots_from_file(
                PublicEvidenceRecordKind::BlockHistory,
                &whitespace_record_path,
            )
            .unwrap_err()
            .to_string(),
            "invalid receipt: public evidence record line has leading or trailing whitespace"
        );
        std::fs::remove_file(&whitespace_record_file).unwrap();
    }

    #[test]
    fn miner_start_requires_real_cuda_readiness_for_cuda_devices() {
        let cuda_start = CliCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cuda:0".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        };

        #[cfg(not(feature = "cuda-kernels"))]
        assert_eq!(
            execute_reference_cli_command(&cuda_start)
                .unwrap_err()
                .to_string(),
            "invalid receipt: cuda kernels not compiled"
        );

        #[cfg(feature = "cuda-kernels")]
        {
            let device_count = cuda_device_count().unwrap_or(0);
            if device_count > 0 {
                let report = execute_reference_cli_command(&cuda_start).unwrap();
                assert!(report.contains("device_backend=cuda"));
                assert!(report.contains("gpu_backend_ready=true"));
                assert!(report.contains("cuda_kernels_compiled=true"));
                assert!(report.contains("cuda_device_index=0"));
                assert!(report.contains(&format!("cuda_device_count={device_count}")));
            }
            assert!(
                execute_reference_cli_command(&CliCommand::MinerStart {
                    wallet: "miner.key".to_owned(),
                    device: format!("cuda:{device_count}"),
                    node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                })
                .is_err()
            );
        }
    }

    #[test]
    fn execute_reference_cli_command_rejects_invalid_local_args() {
        assert!(execute_reference_cli_command(&CliCommand::MinerRegister { stake: 99 }).is_err());
        assert!(
            execute_reference_cli_command(&CliCommand::ValidatorRegister { stake: 9_999 }).is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: " ".to_owned(),
                device: "cpu".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "gpu0".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cuda:abc".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cuda:".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: " ".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cpu".to_owned(),
                node: "http://localhost:8545".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "localhost:8545".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceInit {
                data_dir: " ".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServicePeerAdd {
                data_dir: "/var/lib/tensorvm".to_owned(),
                peer_id: "not-a-peer-id".to_owned(),
                address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServicePeerAdd {
                data_dir: "/var/lib/tensorvm".to_owned(),
                peer_id: PeerId::random().to_string(),
                address: "not-a-multiaddr".to_owned(),
            })
            .is_err()
        );
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        assert!(
            execute_reference_cli_command(&CliCommand::ServicePeerAdd {
                data_dir: "/var/lib/tensorvm".to_owned(),
                peer_id: peer_a.to_string(),
                address: format!("/dns/bootstrap.tensorvm.net/tcp/4001/p2p/{peer_b}"),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "localhost:8545".to_owned(),
                p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "127.0.0.1:8545".to_owned(),
                p2p_listen: "not-a-multiaddr".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "127.0.0.1:8545".to_owned(),
                p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                data_dir: " ".to_owned(),
                auth_token: "secret".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::ServiceServe {
                listen: "127.0.0.1:8545".to_owned(),
                p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                auth_token: " ".to_owned(),
                max_requests: 0,
            })
            .is_err()
        );
        assert!(
            parse_cli_parts(&[
                "service",
                "serve",
                "--listen",
                "127.0.0.1:8545",
                "--p2p-listen",
                "/ip4/127.0.0.1/tcp/4001",
                "--data-dir",
                "/var/lib/tensorvm",
                "--auth-token",
                "secret",
                "--max-requests",
                "abc",
            ])
            .is_err()
        );
        let peer_id = PeerId::random().to_string();
        let make_network_observation = |operator_id,
                                        peer_id: String,
                                        listen_address: String,
                                        observed_at_unix_seconds,
                                        gossip_topic_count,
                                        request_response_protocol_count,
                                        bootstrap_peer_count,
                                        max_transmit_bytes| {
            CliCommand::PublicEvidenceNetworkObservation {
                operator_id,
                peer_id,
                listen_address,
                observed_at_unix_seconds,
                gossip_topic_count,
                request_response_protocol_count,
                bootstrap_peer_count,
                max_transmit_bytes,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_connection_timeout_seconds: 60,
            }
        };
        let operator_id = hash_bytes(b"test", &[b"network-operator"]);
        let public_listen_address = "/dns/node-a.tensorvm.net/tcp/4001".to_owned();
        for invalid in [
            make_network_observation(
                [0; 32],
                peer_id.clone(),
                public_listen_address.clone(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                public_listen_address.clone(),
                0,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                public_listen_address.clone(),
                1_700_000_000,
                0,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                public_listen_address.clone(),
                1_700_000_000,
                5,
                0,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                public_listen_address.clone(),
                1_700_000_000,
                5,
                3,
                0,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                public_listen_address.clone(),
                1_700_000_000,
                5,
                3,
                2,
                0,
            ),
            make_network_observation(
                operator_id,
                "not-a-peer-id".to_owned(),
                public_listen_address.clone(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "not-a-multiaddr".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/ip4/127.0.0.1/tcp/4001".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/ip4/8.8.8.8".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/ip4/8.8.8.8/tcp/0".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/ip4/8.8.8.8/udp/4001".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/ip4/203.0.113.10/tcp/4001".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/dns/bad_host.tensorvm.net/tcp/4001".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
            make_network_observation(
                operator_id,
                peer_id.clone(),
                "/dns/node.tensorvm.example/tcp/4001".to_owned(),
                1_700_000_000,
                5,
                3,
                2,
                1_048_576,
            ),
        ] {
            assert!(execute_reference_cli_command(&invalid).is_err());
        }
        assert!(parse_public_service_kind("archive").is_err());
        assert_eq!(
            parse_public_node_role("miner").unwrap(),
            PublicNodeRole::Miner
        );
        assert_eq!(
            parse_public_node_role("validator").unwrap(),
            PublicNodeRole::Validator
        );
        assert!(parse_public_node_role("observer").is_err());
        assert_eq!(
            parse_public_evidence_record_kind("block-history").unwrap(),
            PublicEvidenceRecordKind::BlockHistory
        );
        assert_eq!(
            parse_public_evidence_record_kind("finality-history").unwrap(),
            PublicEvidenceRecordKind::FinalityHistory
        );
        assert_eq!(
            parse_public_evidence_record_kind("network-runtime").unwrap(),
            PublicEvidenceRecordKind::NetworkRuntimeObservations
        );
        assert_eq!(
            parse_public_evidence_record_kind("data-availability").unwrap(),
            PublicEvidenceRecordKind::DataAvailabilityMeasurements
        );
        assert_eq!(
            parse_public_evidence_record_kind("invalid-work").unwrap(),
            PublicEvidenceRecordKind::InvalidWorkRejections
        );
        assert_eq!(
            parse_public_evidence_record_kind("reward-settlement").unwrap(),
            PublicEvidenceRecordKind::RewardSettlements
        );
        assert!(parse_public_evidence_record_kind("operator-identity").is_err());
        assert!(parse_hash_argument("12").is_err());
        assert!(parse_hash_argument(&"g".repeat(64)).is_err());
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "http://127.0.0.1/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.example.test/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 10,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: [0; 32],
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 10,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealth {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_seen_block: 0,
                last_seen_block: 9,
                reachable_observation_count: 0,
                signed_health_check_count: 10,
            })
            .is_err()
        );
        let partial_health = service_health_observation_summary_from_file(
            "service_health_observation=0,reachable\nservice_health_observation=1,unreachable\n",
        )
        .unwrap();
        assert_eq!(partial_health.first_seen_block, 0);
        assert_eq!(partial_health.last_seen_block, 1);
        assert_eq!(partial_health.reachable_observation_count, 1);
        assert_eq!(partial_health.signed_health_check_count, 2);
        for invalid_health_observations in [
            "# no observations\n\n",
            " service_health_observation=0,reachable\n",
            "service_health_observation=0,reachable\nservice_health_observation=0,reachable\n",
            "service_health_observation=0,reachable\nservice_health_observation=2,reachable\n",
            "service_health_observation=0,ok\n",
            "service_health_observation=0\n",
            "record_root=00\n",
        ] {
            assert!(
                service_health_observation_summary_from_file(invalid_health_observations).is_err()
            );
        }
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceHealthFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                observation_file: std::env::temp_dir()
                    .join(format!(
                        "missing-tensor-vm-service-health-{}.records",
                        std::process::id()
                    ))
                    .to_string_lossy()
                    .into_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://localhost/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
                content_path: "/wrong".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: [0; 32],
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 0,
                min_content_bytes: 64,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContent {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                observed_at_unix_seconds: 1_700_000_000,
                min_content_bytes: 63,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex: "zz".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex: "abc".to_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromBytes {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_hex: hex(&[1_u8; 63]),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceServiceContentFromFile {
                kind: PublicServiceKind::Rpc,
                endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
                content_file: std::env::temp_dir()
                    .join("tensor-vm-missing-service-content-file.body")
                    .to_string_lossy()
                    .into_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://evidence.tensorvm.example/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "http://127.0.0.1/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: " https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json ".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json?download=1"
                    .to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: [0; 32],
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: [0; 32],
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 0,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 2,
                independent_auditor_count: 1,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidencePublication {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address(b"public-evidence-publisher"),
                manifest_signature_count: 1,
                independent_auditor_count: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: [0; 32],
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://localhost/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: "https://localhost/audit.json".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceAuditorRecord {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address(b"public-evidence-auditor-0"),
                audit_uri: manifest_auditor_uri(),
                observed_at_unix_seconds: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindow {
                bundle_id: [0; 32],
                manifest_signer: address(b"public-evidence-publisher"),
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindow {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: [0; 32],
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindow {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                run_started_at_unix_seconds: 1_700_000_060,
                run_ended_at_unix_seconds: 1_700_000_000,
                observed_blocks: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindow {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                run_started_at_unix_seconds: 1_700_000_000,
                run_ended_at_unix_seconds: 1_700_000_060,
                observed_blocks: 0,
            })
            .is_err()
        );
        let run_window_summary = run_window_observation_summary_from_file(
            "run_window_observation=7,1700000000\nrun_window_observation=8,1700000006\n",
        )
        .unwrap();
        assert_eq!(
            run_window_summary.run_started_at_unix_seconds,
            1_700_000_000
        );
        assert_eq!(run_window_summary.run_ended_at_unix_seconds, 1_700_000_006);
        assert_eq!(run_window_summary.observed_blocks, 2);
        for invalid_run_window_observations in [
            "# no observations\n\n",
            " run_window_observation=0,1700000000\n",
            "run_window_observation=0,1700000000\nrun_window_observation=0,1700000001\n",
            "run_window_observation=0,1700000000\nrun_window_observation=2,1700000012\n",
            "run_window_observation=0,1700000006\nrun_window_observation=1,1700000000\n",
            "run_window_observation=0,0\n",
            "run_window_observation=0\n",
            "service_health_observation=0,reachable\n",
        ] {
            assert!(
                run_window_observation_summary_from_file(invalid_run_window_observations).is_err()
            );
        }
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRunWindowFromFile {
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                block_observation_file: std::env::temp_dir()
                    .join(format!(
                        "missing-tensor-vm-run-window-{}.records",
                        std::process::id()
                    ))
                    .to_string_lossy()
                    .into_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                role: PublicNodeRole::Miner,
                address: [0; 32],
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                first_seen_block: 0,
                last_seen_block: 9,
                signed_heartbeat_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: [0; 32],
                first_seen_block: 0,
                last_seen_block: 9,
                signed_heartbeat_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                first_seen_block: 10,
                last_seen_block: 9,
                signed_heartbeat_count: 10,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeat {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                first_seen_block: 0,
                last_seen_block: 9,
                signed_heartbeat_count: 0,
            })
            .is_err()
        );
        let miner_address_hex = manifest_address(b"miner-a");
        let miner_operator_hex = manifest_hash(b"miner-a-operator");
        let heartbeat_summary = node_heartbeat_observation_summary_from_file(
            PublicNodeRole::Miner,
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            &format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},1\n"
            ),
        )
        .unwrap();
        assert_eq!(heartbeat_summary.first_seen_block, 0);
        assert_eq!(heartbeat_summary.last_seen_block, 1);
        assert_eq!(heartbeat_summary.signed_heartbeat_count, 2);
        for invalid_heartbeat_observations in [
            "# no observations\n\n".to_owned(),
            format!(
                " node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"
            ),
            format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"
            ),
            format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},2\n"
            ),
            format!(
                "node_heartbeat_observation=validator,{miner_address_hex},{miner_operator_hex},0\n"
            ),
            format!(
                "node_heartbeat_observation=miner,{},{} ,0\n",
                miner_address_hex, miner_operator_hex
            ),
            format!("node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex}\n"),
            "service_health_observation=0,reachable\n".to_owned(),
        ] {
            assert!(
                node_heartbeat_observation_summary_from_file(
                    PublicNodeRole::Miner,
                    address(b"miner-a"),
                    hash_bytes(b"test", &[b"miner-a-operator"]),
                    &invalid_heartbeat_observations,
                )
                .is_err()
            );
        }
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceNodeHeartbeatFromFile {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                heartbeat_file: std::env::temp_dir()
                    .join(format!(
                        "missing-tensor-vm-node-heartbeat-{}.records",
                        std::process::id()
                    ))
                    .to_string_lossy()
                    .into_owned(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: [0; 32],
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: [0; 32],
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                identity_uri: "https://localhost/miner-a".to_owned(),
                observed_at_unix_seconds: 1_700_000_000,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceOperatorAttestation {
                role: PublicNodeRole::Miner,
                address: address(b"miner-a"),
                operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at_unix_seconds: 0,
            })
            .is_err()
        );
        assert!(
            parse_cli_parts(&[
                "public-evidence",
                "record-summary",
                "--kind",
                "operator-identity",
                "--bundle-id",
                &manifest_hash(b"public-evidence-bundle"),
                "--manifest-signer",
                &manifest_address(b"public-evidence-publisher"),
                "--record-root",
                &manifest_hash(b"network-runtime-root"),
                "--record-count",
                "4",
            ])
            .is_err()
        );
        assert!(
            parse_cli_parts(&[
                "public-evidence",
                "record-artifact",
                "--kind",
                "operator-identity",
                "--bundle-id",
                &manifest_hash(b"public-evidence-bundle"),
                "--manifest-signer",
                &manifest_address(b"public-evidence-publisher"),
                "--artifact-uri",
                "https://evidence.tensorvm.net/network-runtime.json",
                "--record-root",
                &manifest_hash(b"network-runtime-root"),
                "--record-count",
                "4",
            ])
            .is_err()
        );
        assert!(
            parse_cli_parts(&[
                "public-evidence",
                "record-summary-from-roots",
                "--kind",
                "network-runtime",
                "--bundle-id",
                &manifest_hash(b"public-evidence-bundle"),
                "--manifest-signer",
                &manifest_address(b"public-evidence-publisher"),
                "--record-roots",
                "",
            ])
            .is_err()
        );
        let valid_record_summary = CliCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        };
        assert!(execute_reference_cli_command(&valid_record_summary).is_ok());
        let valid_record_artifact = CliCommand::PublicEvidenceRecordArtifact {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        };
        assert!(execute_reference_cli_command(&valid_record_artifact).is_ok());
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: [0; 32],
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: [0; 32],
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: [0; 32],
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummary {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: [0; 32],
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: [0; 32],
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://localhost/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: [0; 32],
                record_count: 4,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifact {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
                record_count: 0,
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: Vec::new(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: vec![[0; 32]],
            })
            .is_err()
        );
        let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordSummaryFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                record_roots: vec![duplicate_record_root, duplicate_record_root],
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifactFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_roots: Vec::new(),
            })
            .is_err()
        );
        assert!(
            execute_reference_cli_command(&CliCommand::PublicEvidenceRecordArtifactFromRoots {
                kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
                bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
                manifest_signer: address(b"public-evidence-publisher"),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_roots: vec![duplicate_record_root, duplicate_record_root],
            })
            .is_err()
        );
    }

    #[test]
    fn network_observation_public_address_filter_rejects_local_targets() {
        assert!(network_observation_multiaddr_is_public(
            &"/ip4/8.8.8.8/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip4/8.8.8.8".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip4/8.8.8.8/tcp/0".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip4/8.8.8.8/udp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip4/0.0.0.0/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip4/10.0.0.1/tcp/4001".parse().unwrap()
        ));
        for address in [
            "/ip4/100.64.0.1/tcp/4001",
            "/ip4/192.0.0.1/tcp/4001",
            "/ip4/192.0.2.10/tcp/4001",
            "/ip4/198.18.0.1/tcp/4001",
            "/ip4/198.51.100.10/tcp/4001",
            "/ip4/203.0.113.10/tcp/4001",
            "/ip4/224.0.0.1/tcp/4001",
            "/ip4/240.0.0.1/tcp/4001",
            "/ip4/255.255.255.255/tcp/4001",
        ] {
            assert!(!network_observation_multiaddr_is_public(
                &address.parse().unwrap()
            ));
        }
        assert!(network_observation_multiaddr_is_public(
            &"/ip6/2001:4860:4860::8888/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip6/::1/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip6/fc00::1/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip6/fe80::1/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip6/2001:db8::1/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/ip6/ff02::1/tcp/4001".parse().unwrap()
        ));
        assert!(network_observation_multiaddr_is_public(
            &"/dns/node.tensorvm.net/tcp/4001".parse().unwrap()
        ));
        assert!(!public_dns_host_is_well_formed(""));
        assert!(!public_dns_host_is_well_formed("node"));
        assert!(!public_dns_host_is_well_formed(&"a".repeat(254)));
        for address in [
            "/dns/node/tcp/4001",
            "/dns/bad_host.tensorvm.net/tcp/4001",
            "/dns/-bad.tensorvm.net/tcp/4001",
            "/dns/bad-.tensorvm.net/tcp/4001",
            "/dns/bad..tensorvm.net/tcp/4001",
            "/dns/node.tensorvm.example/tcp/4001",
            "/dns/example.com/tcp/4001",
            "/dns/123.456/tcp/4001",
        ] {
            assert!(!network_observation_multiaddr_is_public(
                &address.parse().unwrap()
            ));
        }
        assert!(!network_observation_multiaddr_is_public(
            &"/dns/localhost/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/dns/node.local/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/dns/203.0.113.10/tcp/4001".parse().unwrap()
        ));
        assert!(!network_observation_multiaddr_is_public(
            &"/dns4/10.0.0.1/tcp/4001".parse().unwrap()
        ));
        assert!(!public_dns_host("2001:db8::1"));
        assert!(public_dns_host("2001:4860:4860::8888"));
    }

    #[test]
    fn validate_public_evidence_manifest_reports_default_criteria_status() {
        let report = validate_public_evidence_manifest(&evidence_manifest()).unwrap();
        assert!(report.contains("public_evidence_full_spec=false"));
        assert!(report.contains("public_criterion=false"));
        assert!(report.contains("independently_checkable=true"));
        assert!(report.contains("published_evidence_bundle=true"));
        assert!(report.contains("independent_auditor_records=true"));
        assert!(report.contains("signed_run_window=true"));
        assert!(report.contains("block_history=true"));
        assert!(report.contains("finality_history=true"));
        assert!(report.contains("operator_identity_attestations=true"));
        assert!(report.contains("network_runtime_observations=true"));
        assert!(report.contains("data_availability_measurements=true"));
        assert!(report.contains("signed_invalid_work_rejection_records=true"));
        assert!(report.contains("signed_reward_settlement_records=true"));
        assert!(report.contains("supporting_record_artifacts=true"));
        assert!(report.contains("miners=2"));
        assert!(report.contains("validators=1"));
        assert!(report.contains("run_started_at_unix_seconds=1700000000"));
        assert!(report.contains("run_ended_at_unix_seconds=1700000060"));
        assert!(report.contains("observed_duration_seconds=60"));
        assert!(report.contains("required_duration_seconds=604800"));
        assert!(report.contains("observed_blocks=10"));
        assert!(report.contains("required_blocks=100800"));
        assert!(report.contains("finality_rate_bps=10000"));
        assert!(report.contains("data_availability_bps=9500"));
        assert!(report.contains("invalid_receipts_submitted=1"));
        assert!(report.contains("invalid_receipts_rejected=1"));
        assert!(report.contains("invalid_work_rejection_rate_bps=10000"));
        assert!(report.contains("reward_settlement_records=1"));
        assert!(report.contains("external_operator_evidence=true"));
        assert!(report.contains("required_miners=false"));
        assert!(report.contains("required_validators=false"));
        assert!(report.contains("required_run_duration=false"));
        assert!(report.contains("required_block_count=false"));
        assert!(report.contains("required_finality=true"));
        assert!(report.contains("required_data_availability=true"));
        assert!(report.contains("invalid_work_rejection_evidence=true"));
        assert!(report.contains("reward_settlement_evidence=true"));
        assert!(report.contains("production_libp2p_runtime=true"));
        assert!(report.contains("deployed_rpc_service=true"));
        assert!(report.contains("deployed_explorer_service=true"));
        assert!(report.contains("deployed_faucet_service=true"));
        assert!(report.contains("deployed_telemetry_service=true"));
        assert!(report.contains("deployed_public_service_content=true"));
        assert!(report.contains("deployed_public_services=true"));

        let insufficient_operator_records = evidence_manifest().replace(
            "operator_identity_attestation_records=3",
            "operator_identity_attestation_records=2",
        );
        let insufficient_operator_report =
            validate_public_evidence_manifest(&insufficient_operator_records).unwrap();
        assert!(insufficient_operator_report.contains("operator_identity_attestations=false"));
        assert!(insufficient_operator_report.contains("external_operator_evidence=false"));
        assert!(insufficient_operator_report.contains("public_criterion=false"));

        let missing_auditor_records = evidence_manifest()
            .lines()
            .filter(|line| !line.starts_with("auditor="))
            .collect::<Vec<_>>()
            .join("\n");
        let missing_auditor_report =
            validate_public_evidence_manifest(&missing_auditor_records).unwrap();
        assert!(missing_auditor_report.contains("published_evidence_bundle=true"));
        assert!(missing_auditor_report.contains("independent_auditor_records=false"));
        assert!(missing_auditor_report.contains("independently_checkable=false"));

        let missing_artifacts = evidence_manifest()
            .lines()
            .filter(|line| !line.starts_with("record_artifact="))
            .collect::<Vec<_>>()
            .join("\n");
        let missing_artifacts_report =
            validate_public_evidence_manifest(&missing_artifacts).unwrap();
        assert!(missing_artifacts_report.contains("supporting_record_artifacts=false"));
        assert!(missing_artifacts_report.contains("independently_checkable=false"));

        let missing_service_content = evidence_manifest()
            .lines()
            .filter(|line| !line.starts_with("service_content="))
            .collect::<Vec<_>>()
            .join("\n");
        let missing_service_content_report =
            validate_public_evidence_manifest(&missing_service_content).unwrap();
        assert!(missing_service_content_report.contains("deployed_public_service_content=false"));
        assert!(missing_service_content_report.contains("deployed_public_services=false"));

        assert!(validate_public_evidence_manifest("bad-manifest").is_err());
    }

    #[test]
    fn validate_public_testnet_preflight_manifest_reports_launch_readiness() {
        let report = validate_public_testnet_preflight_manifest(&preflight_manifest()).unwrap();
        assert!(report.contains("public_testnet_preflight_ready=true"));
        assert!(report.contains("local_shape_ready=true"));
        assert!(report.contains("deployment_plan_ready=true"));
        assert!(report.contains("miners=10"));
        assert!(report.contains("validators=5"));
        assert!(report.contains("required_blocks=100800"));
        assert!(report.contains("required_miners=true"));
        assert!(report.contains("required_validators=true"));
        assert!(report.contains("positive_stakes=true"));
        assert!(report.contains("funded_faucet=true"));
        assert!(report.contains("cuda_kernels_available=true"));
        assert!(report.contains("cuda_ready_miner_count=10"));
        assert!(report.contains("cuda_ready_miners=true"));
        assert!(report.contains("libp2p_ready_node_count=15"));
        assert!(report.contains("libp2p_ready_nodes=true"));
        assert!(report.contains("production_libp2p_runtime=true"));
        assert!(report.contains("rpc_service_plan=true"));
        assert!(report.contains("explorer_service_plan=true"));
        assert!(report.contains("faucet_service_plan=true"));
        assert!(report.contains("telemetry_service_plan=true"));
        assert!(report.contains("public_service_content_planned=true"));
        assert!(report.contains("public_services_planned=true"));

        assert!(validate_public_testnet_preflight_manifest("bad-manifest").is_err());
    }
}
