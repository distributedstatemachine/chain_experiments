use crate::chain::ChainParams;
use crate::error::{Result, TvmError};
use crate::hash::hex;
use crate::p2p::Libp2pControlPlaneConfig;
#[cfg(all(test, feature = "cuda-kernels"))]
use crate::runtime::cuda_device_count;
#[cfg(test)]
use crate::runtime::cuda_kernels_compiled;
use crate::testnet::{
    PublicEvidenceRecordKind, PublicEvidenceSupportingArtifact, PublicNodeRole, PublicServiceKind,
    sign_public_evidence_record,
};
use crate::types::{Address, Hash, hash_bytes};
#[cfg(test)]
use libp2p::PeerId;
use std::collections::BTreeSet;

mod arguments;
mod network_evidence;
mod network_observation;
mod node_evidence;
mod publication_evidence;
mod reports;
mod run_window_evidence;
mod service_evidence;
mod validation;

use arguments::{
    parse_hash_argument, parse_hash_list_argument, parse_hex_bytes_argument,
    parse_public_evidence_record_kind, parse_public_node_role, parse_public_service_kind,
    parse_u64, parse_usize, public_evidence_record_field_prefix, public_evidence_record_kind_tag,
    public_node_role_tag, public_service_kind_tag,
};
use network_evidence::{
    NetworkObservationEvidenceLine, network_observation_evidence_line,
    network_observation_evidence_line_from_service_log, network_observation_root_from_record_line,
};
#[cfg(test)]
use network_evidence::{network_observation_root, service_log_field};
#[cfg(test)]
use network_observation::network_observation_multiaddr_is_public;
#[cfg(test)]
use network_observation::{public_dns_host, public_dns_host_is_well_formed};
#[cfg(test)]
use node_evidence::node_heartbeat_observation_summary_from_file;
use node_evidence::{
    node_heartbeat_evidence_line, node_heartbeat_evidence_line_from_file,
    operator_identity_attestation_evidence_line,
};
use publication_evidence::{auditor_record_evidence_line, publication_evidence_lines};
pub use reports::{validate_public_evidence_manifest, validate_public_testnet_preflight_manifest};
#[cfg(test)]
use run_window_evidence::run_window_observation_summary_from_file;
use run_window_evidence::{run_window_evidence_line, run_window_evidence_line_from_file};
use service_evidence::{
    ServiceHealthEvidenceLine, service_content_evidence_line,
    service_content_evidence_line_from_bytes, service_health_evidence_line,
    service_health_evidence_line_from_file,
};
#[cfg(test)]
use service_evidence::{public_service_content_root, service_health_observation_summary_from_file};
use validation::{
    ensure_auth_token, ensure_data_dir, ensure_libp2p_multiaddr, ensure_listen_addr,
    ensure_minimum_stake, ensure_node_endpoint, json_escape, miner_device_readiness,
    wallet_address_hex,
};

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

#[cfg(test)]
mod tests;
