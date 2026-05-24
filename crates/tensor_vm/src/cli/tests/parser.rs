use super::{
    ExpectedCommand, manifest_address, manifest_auditor_uri, manifest_hash, parse_test_cli,
};
use crate::hash::hex;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{address, hash_bytes};
use libp2p::PeerId;

#[test]
fn parses_documented_miner_commands() {
    assert_eq!(
        parse_test_cli(&["miner", "register", "--stake", "100"]).unwrap(),
        ExpectedCommand::MinerRegister { stake: 100 }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["miner", "status"]).unwrap(),
        ExpectedCommand::MinerStatus
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
}

#[test]
fn parses_documented_validator_commands() {
    assert_eq!(
        parse_test_cli(&["validator", "register", "--stake", "10000"]).unwrap(),
        ExpectedCommand::ValidatorRegister { stake: 10_000 }
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "start",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        ExpectedCommand::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["validator", "status"]).unwrap(),
        ExpectedCommand::ValidatorStatus
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ValidatorRun {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "22".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "run",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ValidatorRun {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x22; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    assert_eq!(
        parse_test_cli(&["local-testnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::LocalTestnetSeed {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-evidence",
            "validate",
            "--manifest",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .unwrap(),
        ExpectedCommand::PublicEvidenceValidate {
            manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public-testnet",
            "preflight",
            "--manifest",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .unwrap(),
        ExpectedCommand::PublicTestnetPreflight {
            manifest: "docs/tensorvm/public-testnet.preflight".to_owned(),
        }
    );
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_060,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: "artifacts/block-observations.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceNodeHeartbeat {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            first_seen_block: 0,
            last_seen_block: 9,
            signed_heartbeat_count: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceNodeHeartbeatFromFile {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            heartbeat_file: "artifacts/miner-a-heartbeats.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceOperatorAttestation {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        }
    );
    let endpoint_id = manifest_hash(b"rpc-service");
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceServiceHealth {
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
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: "artifacts/rpc-health.records".to_owned(),
        }
    );
    let content_root = manifest_hash(b"rpc-service-content");
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceServiceContent {
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
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_hex,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceServiceContentFromFile {
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
        parse_test_cli(&[
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
            "4",
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
        ExpectedCommand::PublicEvidenceNetworkObservation {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            peer_id: peer_id.clone(),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            gossip_topic_count: 5,
            request_response_protocol_count: 4,
            bootstrap_peer_count: 2,
            max_transmit_bytes: 1_048_576,
            request_timeout_seconds: 10,
            max_concurrent_streams: 128,
            idle_connection_timeout_seconds: 60,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            service_log: "artifacts/node-a-service.log".to_owned(),
        }
    );
    let record_root = manifest_hash(b"network-runtime-root");
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordArtifact {
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
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordSummaryFromRoots {
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
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordArtifactFromRoots {
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
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::PublicEvidenceRecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::ServiceInit {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    let bootstrap_peer = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::ServicePeerAdd {
            data_dir: "/var/lib/tensorvm".to_owned(),
            peer_id: bootstrap_peer.clone(),
            address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "readiness",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .unwrap(),
        ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
        }
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "service",
            "readiness",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
        ])
        .unwrap(),
        ExpectedCommand::ServiceReadiness {
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
        }
    );
    assert_eq!(
        parse_test_cli(&[
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
        ExpectedCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "serve",
            "--listen",
            "0.0.0.0:8545",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "0",
        ])
        .unwrap(),
        ExpectedCommand::ServiceServe {
            listen: "0.0.0.0:8545".to_owned(),
            p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x11; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "status", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        ExpectedCommand::ServiceStatus {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "service",
            "block",
            "--data-dir",
            "/var/lib/tensorvm",
            "--height",
            "3"
        ])
        .unwrap(),
        ExpectedCommand::ServiceBlock {
            data_dir: "/var/lib/tensorvm".to_owned(),
            height: 3,
        }
    );
}

#[test]
fn parses_documented_proposer_commands() {
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ProposerRun {
            wallet: "proposer.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
    let identity_seed = "33".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "proposer",
            "run",
            "--wallet",
            "proposer.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001",
            "--listen",
            "127.0.0.1:8545",
            "--p2p-listen",
            "/ip4/127.0.0.1/tcp/0",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
            "--auth-token",
            "secret",
            "--max-requests",
            "7",
        ])
        .unwrap(),
        ExpectedCommand::ProposerRun {
            wallet: "proposer.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
            data_dir: "/var/lib/tensorvm".to_owned(),
            identity_seed: Some([0x33; 32]),
            auth_token: "secret".to_owned(),
            max_requests: 7,
        }
    );
}

#[test]
fn rejects_invalid_cli() {
    assert!(parse_test_cli(&["miner", "register"]).is_err());
    assert!(parse_test_cli(&["validator", "register", "--stake", "abc"]).is_err());
    assert!(
        parse_test_cli(&[
            "service",
            "serve",
            "--listen",
            "not-a-socket",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--node",
            "not-a-multiaddr",
            "--auth-token",
            "secret"
        ])
        .is_err()
    );
}

#[test]
fn clap_cli_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["miner", "start", "--wallet", "miner.key"]).unwrap(),
        ExpectedCommand::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "run",
            "--wallet",
            "miner.key",
            "--auth-token",
            "secret"
        ])
        .unwrap(),
        ExpectedCommand::MinerRun {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: ".tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "serve", "--auth-token", "secret"]).unwrap(),
        ExpectedCommand::ServiceServe {
            listen: "127.0.0.1:8545".to_owned(),
            p2p_listen: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            data_dir: ".tensorvm".to_owned(),
            identity_seed: None,
            auth_token: "secret".to_owned(),
            max_requests: 0,
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "init"]).unwrap(),
        ExpectedCommand::ServiceInit {
            data_dir: ".tensorvm".to_owned(),
        }
    );
}
