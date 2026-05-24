use super::{
    CommandFixture, manifest_address, manifest_auditor_uri, manifest_hash, parse_test_cli,
};
use crate::hash::hex;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{address, hash_bytes};
use libp2p::PeerId;

#[test]
fn parses_documented_miner_commands() {
    assert_eq!(
        parse_test_cli(&["miner", "register", "--stake", "100"]).unwrap(),
        CommandFixture::MinerRegister { stake: 100 }
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
        CommandFixture::MinerStart {
            wallet: "miner.key".to_owned(),
            device: "cpu".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["miner", "status"]).unwrap(),
        CommandFixture::MinerStatus
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
        CommandFixture::MinerRun {
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
        CommandFixture::MinerRun {
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
        CommandFixture::ValidatorRegister { stake: 10_000 }
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
        CommandFixture::ValidatorStart {
            wallet: "validator.key".to_owned(),
            node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["validator", "status"]).unwrap(),
        CommandFixture::ValidatorStatus
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
        CommandFixture::ValidatorRun {
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
        CommandFixture::ValidatorRun {
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
        parse_test_cli(&["testnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        CommandFixture::LocalTestnetSeed {
            data_dir: "/var/lib/tensorvm".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "validate",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .unwrap(),
        CommandFixture::PublicEvidenceValidate {
            manifest: "docs/tensorvm/public-testnet.evidence".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "testnet",
            "preflight",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .unwrap(),
        CommandFixture::PublicTestnetPreflight {
            manifest: "docs/tensorvm/public-testnet.preflight".to_owned(),
        }
    );
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "publish",
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
        CommandFixture::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "audit",
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
        CommandFixture::PublicEvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_060,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "run",
            "window",
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
        CommandFixture::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "run",
            "window-file",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--block-observation-file",
            "artifacts/block-observations.records",
        ])
        .unwrap(),
        CommandFixture::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: "artifacts/block-observations.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "node",
            "heartbeat",
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
        CommandFixture::PublicEvidenceNodeHeartbeat {
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
            "evidence",
            "node",
            "heartbeat-file",
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
        CommandFixture::PublicEvidenceNodeHeartbeatFromFile {
            role: PublicNodeRole::Miner,
            address: address(b"miner-a"),
            operator_id: hash_bytes(b"test", &[b"miner-a-operator"]),
            heartbeat_file: "artifacts/miner-a-heartbeats.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "node",
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
        CommandFixture::PublicEvidenceOperatorAttestation {
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
            "evidence",
            "service",
            "health",
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
        CommandFixture::PublicEvidenceServiceHealth {
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
            "evidence",
            "service",
            "health-file",
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
        CommandFixture::PublicEvidenceServiceHealthFromFile {
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
            "evidence",
            "service",
            "content",
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
        CommandFixture::PublicEvidenceServiceContent {
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
            "evidence",
            "service",
            "content-bytes",
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
        CommandFixture::PublicEvidenceServiceContentFromBytes {
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
            "evidence",
            "service",
            "content-file",
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
        CommandFixture::PublicEvidenceServiceContentFromFile {
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
            "evidence",
            "network",
            "observation",
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
        CommandFixture::PublicEvidenceNetworkObservation {
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
            "evidence",
            "network",
            "from-service-log",
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
        CommandFixture::PublicEvidenceNetworkObservationFromServiceLog {
            operator_id: hash_bytes(b"test", &[b"network-operator"]),
            listen_address: "/dns/node-a.tensorvm.net/tcp/4001".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            service_log: "artifacts/node-a-service.log".to_owned(),
        }
    );
    let record_root = manifest_hash(b"network-runtime-root");
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "record",
            "summary",
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
        CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "record",
            "artifact",
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
        CommandFixture::PublicEvidenceRecordArtifact {
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
            "evidence",
            "record",
            "summary-roots",
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
        CommandFixture::PublicEvidenceRecordSummaryFromRoots {
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
            "evidence",
            "record",
            "artifact-roots",
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
        CommandFixture::PublicEvidenceRecordArtifactFromRoots {
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
            "evidence",
            "record",
            "summary-file",
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
        CommandFixture::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "evidence",
            "record",
            "artifact-file",
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
        CommandFixture::PublicEvidenceRecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["service", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        CommandFixture::ServiceInit {
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
        CommandFixture::ServicePeerAdd {
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
        CommandFixture::ServiceReadiness {
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
        CommandFixture::ServiceReadiness {
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
        CommandFixture::ServiceServe {
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
        CommandFixture::ServiceServe {
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
        CommandFixture::ServiceStatus {
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
        CommandFixture::ServiceBlock {
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
        CommandFixture::ProposerRun {
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
        CommandFixture::ProposerRun {
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
        CommandFixture::MinerStart {
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
        CommandFixture::MinerRun {
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
        CommandFixture::ServiceServe {
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
        CommandFixture::ServiceInit {
            data_dir: ".tensorvm".to_owned(),
        }
    );
}
