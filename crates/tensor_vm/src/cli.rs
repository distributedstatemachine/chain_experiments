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
use libp2p::{Multiaddr, PeerId};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;

mod network_observation;

use network_observation::network_observation_multiaddr_is_public;
#[cfg(test)]
use network_observation::{public_dns_host, public_dns_host_is_well_formed};

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
    MinerRun {
        wallet: String,
        device: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    MinerStatus,
    ValidatorRegister {
        stake: u64,
    },
    ValidatorStart {
        wallet: String,
        node: String,
    },
    ValidatorRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ValidatorStatus,
    ProposerRun {
        wallet: String,
        node: String,
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceInit {
        data_dir: String,
    },
    ServicePeerAdd {
        data_dir: String,
        peer_id: String,
        address: String,
    },
    ServiceReadiness {
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
    },
    ServiceServe {
        listen: String,
        p2p_listen: String,
        data_dir: String,
        identity_seed: Option<Hash>,
        auth_token: String,
        max_requests: usize,
    },
    ServiceStatus {
        data_dir: String,
    },
    ServiceBlock {
        data_dir: String,
        height: u64,
    },
    LocalTestnetSeed {
        data_dir: String,
    },
    LocalCpuVerify {
        data_dir: String,
        json: bool,
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
        [
            "miner",
            "run",
            "--wallet",
            wallet,
            "--device",
            device,
            "--node",
            node,
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
        ] => Ok(CliCommand::MinerRun {
            wallet: (*wallet).to_owned(),
            device: (*device).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: None,
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        [
            "miner",
            "run",
            "--wallet",
            wallet,
            "--device",
            device,
            "--node",
            node,
            "--listen",
            listen,
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
            "--identity-seed",
            identity_seed,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::MinerRun {
            wallet: (*wallet).to_owned(),
            device: (*device).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: Some(parse_hash_argument(identity_seed)?),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
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
        [
            "validator",
            "run",
            "--wallet",
            wallet,
            "--node",
            node,
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
        ] => Ok(CliCommand::ValidatorRun {
            wallet: (*wallet).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: None,
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        [
            "validator",
            "run",
            "--wallet",
            wallet,
            "--node",
            node,
            "--listen",
            listen,
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
            "--identity-seed",
            identity_seed,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::ValidatorRun {
            wallet: (*wallet).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: Some(parse_hash_argument(identity_seed)?),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        ["validator", "status"] => Ok(CliCommand::ValidatorStatus),
        [
            "proposer",
            "run",
            "--wallet",
            wallet,
            "--node",
            node,
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
        ] => Ok(CliCommand::ProposerRun {
            wallet: (*wallet).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: None,
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        [
            "proposer",
            "run",
            "--wallet",
            wallet,
            "--node",
            node,
            "--listen",
            listen,
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
            "--identity-seed",
            identity_seed,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::ProposerRun {
            wallet: (*wallet).to_owned(),
            node: (*node).to_owned(),
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: Some(parse_hash_argument(identity_seed)?),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
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
            "readiness",
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
        ] => Ok(CliCommand::ServiceReadiness {
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: None,
        }),
        [
            "service",
            "readiness",
            "--p2p-listen",
            p2p_listen,
            "--data-dir",
            data_dir,
            "--identity-seed",
            identity_seed,
        ] => Ok(CliCommand::ServiceReadiness {
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: Some(parse_hash_argument(identity_seed)?),
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
            identity_seed: None,
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
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
            "--identity-seed",
            identity_seed,
            "--auth-token",
            auth_token,
            "--max-requests",
            max_requests,
        ] => Ok(CliCommand::ServiceServe {
            listen: (*listen).to_owned(),
            p2p_listen: (*p2p_listen).to_owned(),
            data_dir: (*data_dir).to_owned(),
            identity_seed: Some(parse_hash_argument(identity_seed)?),
            auth_token: (*auth_token).to_owned(),
            max_requests: parse_usize(max_requests)?,
        }),
        ["service", "status", "--data-dir", data_dir] => Ok(CliCommand::ServiceStatus {
            data_dir: (*data_dir).to_owned(),
        }),
        [
            "service",
            "block",
            "--data-dir",
            data_dir,
            "--height",
            height,
        ] => Ok(CliCommand::ServiceBlock {
            data_dir: (*data_dir).to_owned(),
            height: parse_u64(height)?,
        }),
        ["local-testnet", "seed", "--data-dir", data_dir] => Ok(CliCommand::LocalTestnetSeed {
            data_dir: (*data_dir).to_owned(),
        }),
        ["local-cpu", "verify", "--data-dir", data_dir, "--json"] => {
            Ok(CliCommand::LocalCpuVerify {
                data_dir: (*data_dir).to_owned(),
                json: true,
            })
        }
        ["local-cpu", "verify", "--data-dir", data_dir] => Ok(CliCommand::LocalCpuVerify {
            data_dir: (*data_dir).to_owned(),
            json: false,
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

fn identity_report(identity_seed: Option<Hash>) -> String {
    match identity_seed {
        Some(seed) => format!("p2p_identity_seeded=true\np2p_identity_seed={}", hex(&seed)),
        None => "p2p_identity_seeded=false".to_owned(),
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
        CliCommand::MinerRun {
            wallet,
            device,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            let device_readiness = miner_device_readiness(device)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=miner_run\nrole=miner\nwallet={wallet}\naddress={address}\ndevice={device}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{}\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrole_runtime_ready=true",
                device_readiness.report(),
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
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
        CliCommand::ValidatorRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=validator_run\nrole=validator\nwallet={wallet}\naddress={address}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nreference_verifier_ready=true\nrole_runtime_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ValidatorStatus => Ok(format!(
            "command=validator_status\nmin_stake={}\nreference_verifier_ready=true\nstatus_source=rpc_or_node_store_required",
            params.validator_min_stake
        )),
        CliCommand::ProposerRun {
            wallet,
            node,
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            let address = wallet_address_hex(wallet)?;
            ensure_node_endpoint(node)?;
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=proposer_run\nrole=proposer\nwallet={wallet}\naddress={address}\nnode={node}\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nproposer_ready=true\nrole_runtime_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
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
        CliCommand::ServiceReadiness {
            p2p_listen,
            data_dir,
            identity_seed,
        } => {
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=service_readiness\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nnode_store_required=true\nlibp2p_ready=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ServiceServe {
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        } => {
            ensure_listen_addr(listen)?;
            ensure_libp2p_multiaddr(p2p_listen)?;
            ensure_data_dir(data_dir)?;
            ensure_auth_token(auth_token)?;
            let p2p_config = Libp2pControlPlaneConfig::default();
            let identity = identity_report(*identity_seed);
            Ok(format!(
                "command=service_serve\nlisten={listen}\np2p_listen={p2p_listen}\np2p_runtime=libp2p\np2p_gossipsub=enabled\np2p_identify=enabled\np2p_kademlia=enabled\np2p_request_response=enabled\n{identity}\np2p_max_transmit_bytes={}\np2p_request_timeout_seconds={}\np2p_max_concurrent_streams={}\np2p_idle_timeout_seconds={}\ndata_dir={data_dir}\nauth_enabled=true\nmax_requests={max_requests}\nrpc_routes=enabled\nexplorer_routes=enabled\nfaucet_routes=enabled\ntelemetry_routes=enabled\nnode_store_required=true",
                p2p_config.max_gossipsub_transmit_bytes,
                p2p_config.request_timeout_seconds,
                p2p_config.max_concurrent_request_streams,
                p2p_config.idle_connection_timeout_seconds
            ))
        }
        CliCommand::ServiceStatus { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_status\ndata_dir={data_dir}\nstatus_source=node_store"
            ))
        }
        CliCommand::ServiceBlock { data_dir, height } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=service_block\ndata_dir={data_dir}\nheight={height}\nstatus_source=node_store"
            ))
        }
        CliCommand::LocalTestnetSeed { data_dir } => {
            ensure_data_dir(data_dir)?;
            Ok(format!(
                "command=local_testnet_seed\ndata_dir={data_dir}\nlocal_cpu_seed_ready=true"
            ))
        }
        CliCommand::LocalCpuVerify { data_dir, json } => {
            ensure_data_dir(data_dir)?;
            if *json {
                Ok(format!(
                    "{{\"command\":\"local_cpu_verify\",\"data_dir\":\"{}\",\"structured_verifier_ready\":true}}",
                    json_escape(data_dir)
                ))
            } else {
                Ok(format!(
                    "command=local_cpu_verify\ndata_dir={data_dir}\nstructured_verifier_ready=true"
                ))
            }
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
    validate_supporting_record_payload(kind, payload)?;
    Ok(hash_bytes(
        b"tensor-vm-public-evidence-supporting-record-root-v1",
        &[
            public_evidence_record_kind_tag(kind).as_bytes(),
            line.as_bytes(),
        ],
    ))
}

fn validate_supporting_record_payload(kind: PublicEvidenceRecordKind, payload: &str) -> Result<()> {
    let fields = payload.split(',').collect::<Vec<_>>();
    if fields
        .iter()
        .any(|field| field.is_empty() || field.trim() != *field)
    {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    match kind {
        PublicEvidenceRecordKind::BlockHistory => {
            require_supporting_record_field_count(&fields, 2)?;
            parse_u64(fields[0])?;
            parse_hash_argument(fields[1])?;
        }
        PublicEvidenceRecordKind::FinalityHistory => {
            require_supporting_record_field_count(&fields, 3)?;
            parse_u64(fields[0])?;
            parse_hash_argument(fields[1])?;
            require_supporting_record_status(fields[2], &["finalized", "unfinalized"])?;
        }
        PublicEvidenceRecordKind::NetworkRuntimeObservations => {
            return Err(TvmError::InvalidReceipt(
                "invalid public evidence supporting record line",
            ));
        }
        PublicEvidenceRecordKind::DataAvailabilityMeasurements => {
            require_supporting_record_field_count(&fields, 3)?;
            parse_hash_argument(fields[0])?;
            require_supporting_record_status(fields[1], &["available", "unavailable"])?;
            parse_u64(fields[2])?;
        }
        PublicEvidenceRecordKind::InvalidWorkRejections => {
            require_supporting_record_field_count(&fields, 3)?;
            parse_hash_argument(fields[0])?;
            require_supporting_record_status(fields[1], &["rejected"])?;
            parse_u64(fields[2])?;
        }
        PublicEvidenceRecordKind::RewardSettlements => {
            require_supporting_record_field_count(&fields, 4)?;
            parse_hash_argument(fields[0])?;
            parse_hash_argument(fields[1])?;
            parse_hash_argument(fields[2])?;
            parse_u64(fields[3])?;
        }
    }
    Ok(())
}

fn require_supporting_record_field_count(fields: &[&str], expected: usize) -> Result<()> {
    if fields.len() != expected {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(())
}

fn require_supporting_record_status(status: &str, allowed: &[&str]) -> Result<()> {
    if !allowed.contains(&status) {
        return Err(TvmError::InvalidReceipt(
            "invalid public evidence supporting record line",
        ));
    }
    Ok(())
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
    if fields.iter().any(|field| field.trim() != *field) {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    let operator_id = parse_hash_argument(fields[0])?;
    if operator_id == [0; 32] {
        return Err(TvmError::InvalidReceipt("operator id argument is empty"));
    }
    let peer_id = fields[1]
        .parse::<PeerId>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p peer id"))?
        .to_string();
    let listen_address = fields[2]
        .parse::<Multiaddr>()
        .map_err(|_| TvmError::InvalidReceipt("invalid libp2p multiaddr"))?;
    if !network_observation_multiaddr_is_public(&listen_address) {
        return Err(TvmError::InvalidReceipt(
            "network observation address is not public",
        ));
    }
    let listen_address = listen_address.to_string();
    let input = NetworkObservationEvidenceLine {
        operator_id,
        peer_id: &peer_id,
        listen_address: &listen_address,
        observed_at_unix_seconds: parse_u64(fields[3])?,
        gossip_topic_count: parse_u64(fields[4])?,
        request_response_protocol_count: parse_u64(fields[5])?,
        bootstrap_peer_count: parse_u64(fields[6])?,
        max_transmit_bytes: parse_u64(fields[7])?,
        request_timeout_seconds: parse_u64(fields[8])?,
        max_concurrent_streams: parse_u64(fields[9])?,
        idle_connection_timeout_seconds: parse_u64(fields[10])?,
    };
    if input.observed_at_unix_seconds == 0
        || input.gossip_topic_count == 0
        || input.request_response_protocol_count == 0
        || input.bootstrap_peer_count == 0
        || input.max_transmit_bytes == 0
        || input.request_timeout_seconds == 0
        || input.max_concurrent_streams == 0
        || input.idle_connection_timeout_seconds == 0
    {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    let record_root = parse_record_file_root(fields[11])?;
    let record_signature = parse_record_file_root(fields[12])?;
    let expected_root = network_observation_root(&input, &peer_id, &listen_address);
    let expected_signature = hash_bytes(
        b"tensor-vm-network-runtime-observation-signature-v1",
        &[&operator_id, &expected_root],
    );
    if record_root != expected_root || record_signature != expected_signature {
        return Err(TvmError::InvalidReceipt(
            "invalid network observation record line",
        ));
    }
    Ok(record_root)
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
    let mut hashes = Vec::new();
    for part in value.split(',') {
        if part.is_empty() || part.trim() != part {
            return Err(TvmError::InvalidReceipt("invalid hash list argument"));
        }
        hashes.push(parse_hash_argument(part)?);
    }
    Ok(hashes)
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

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn ensure_auth_token(auth_token: &str) -> Result<()> {
    if auth_token.trim().is_empty() {
        return Err(TvmError::InvalidReceipt("auth token argument is empty"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
