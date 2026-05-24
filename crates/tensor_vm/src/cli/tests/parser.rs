use super::{
    AddressArg, AuditorRecordArgs, DataDirArgs, EvidenceCommand, EvidenceFixture,
    EvidenceNetworkCommand, EvidenceNodeCommand, EvidenceRunCommand, EvidenceServiceCommand,
    HashArg, HexBytesArg, LocalnetCommand, MinerCheckArgs, MinerCommand, MinerRunArgs,
    NetworkObservationArgs, NetworkObservationFromServiceLogArgs, NodeBlockArgs, NodeCheckArgs,
    NodeCommand, NodeHeartbeatArgs, NodeHeartbeatFromFileArgs, NodePeerAddArgs, NodePeerCommand,
    NodeRuntimeArgs, NodeServeArgs, OperatorAttestationArgs, ProposerCommand, PublicCommand,
    PublicEvidenceManifestArgs, PublicNodeRoleArg, PublicServiceKindArg, PublicTestnetManifestArgs,
    PublicationArgs, RoleRuntimeArgs, RunWindowArgs, RunWindowFromFileArgs, ServiceContentArgs,
    ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs, StakeArgs, TvmdCommand, ValidatorCheckArgs, ValidatorCommand,
    ValidatorRunArgs, manifest_address, manifest_auditor_uri, manifest_hash, parse_test_cli,
};
use crate::hash::hex;
use crate::testnet::PublicEvidenceRecordKind;
use crate::types::{address, hash_bytes};
use libp2p::PeerId;
use std::net::SocketAddr;
use std::path::PathBuf;

fn path(value: &str) -> PathBuf {
    value.into()
}

fn multiaddr(value: &str) -> libp2p::Multiaddr {
    value.parse().expect("fixture multiaddr must parse")
}

fn socket_addr(value: &str) -> SocketAddr {
    value.parse().expect("fixture socket address must parse")
}

fn data_dir_args(data_dir: &str) -> DataDirArgs {
    DataDirArgs {
        data_dir: path(data_dir),
    }
}

fn hash_arg(value: [u8; 32]) -> HashArg {
    HashArg::new(value)
}

fn address_arg(value: [u8; 32]) -> AddressArg {
    AddressArg::new(value)
}

fn node_runtime_args(
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> NodeRuntimeArgs {
    NodeRuntimeArgs {
        listen: socket_addr(listen),
        p2p_listen: multiaddr(p2p_listen),
        data_dir: path(data_dir),
        identity_seed: identity_seed.map(super::HashArg::new),
        auth_token: auth_token.to_owned(),
        max_requests,
    }
}

fn role_runtime_args(
    node: &str,
    listen: &str,
    p2p_listen: &str,
    data_dir: &str,
    identity_seed: Option<[u8; 32]>,
    auth_token: &str,
    max_requests: usize,
) -> RoleRuntimeArgs {
    RoleRuntimeArgs {
        node: multiaddr(node),
        node_runtime: node_runtime_args(
            listen,
            p2p_listen,
            data_dir,
            identity_seed,
            auth_token,
            max_requests,
        ),
    }
}

#[test]
fn parses_documented_miner_commands() {
    assert_eq!(
        parse_test_cli(&["miner", "register", "--stake", "100"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Register(StakeArgs { stake: 100 }))
    );
    assert_eq!(
        parse_test_cli(&[
            "miner",
            "check",
            "--wallet",
            "miner.key",
            "--device",
            "cpu",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
            wallet: path("miner.key"),
            device: "cpu".to_owned(),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
        }))
    );
    assert_eq!(
        parse_test_cli(&["miner", "status"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Status)
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
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: path("miner.key"),
            device: "cpu".to_owned(),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                None,
                "secret",
                7,
            ),
        }))
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
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: path("miner.key"),
            device: "cpu".to_owned(),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x11; 32]),
                "secret",
                7,
            ),
        }))
    );
}

#[test]
fn parses_documented_validator_commands() {
    assert_eq!(
        parse_test_cli(&["validator", "register", "--stake", "10000"]).unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Register(StakeArgs { stake: 10_000 }))
    );
    assert_eq!(
        parse_test_cli(&[
            "validator",
            "check",
            "--wallet",
            "validator.key",
            "--node",
            "/ip4/127.0.0.1/tcp/4001"
        ])
        .unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Check(ValidatorCheckArgs {
            wallet: path("validator.key"),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
        }))
    );
    assert_eq!(
        parse_test_cli(&["validator", "status"]).unwrap(),
        TvmdCommand::Validator(ValidatorCommand::Status)
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
        TvmdCommand::Validator(ValidatorCommand::Run(ValidatorRunArgs {
            wallet: path("validator.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                None,
                "secret",
                7,
            ),
        }))
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
        TvmdCommand::Validator(ValidatorCommand::Run(ValidatorRunArgs {
            wallet: path("validator.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x22; 32]),
                "secret",
                7,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["localnet", "seed", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Localnet(LocalnetCommand::Seed(data_dir_args("/var/lib/tensorvm")))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "validate",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Validate(
            PublicEvidenceManifestArgs {
                manifest: path("docs/tensorvm/public-testnet.evidence"),
            },
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
            "preflight",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Preflight(PublicTestnetManifestArgs {
            manifest: path("docs/tensorvm/public-testnet.preflight"),
        }))
    );
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Publish(
            PublicationArgs {
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            },
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Audit(
            AuditorRecordArgs {
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
                auditor_id: address_arg(address(b"public-evidence-auditor-0")),
                audit_uri: manifest_auditor_uri(),
                observed_at: 1_700_000_060,
            },
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Run(
            EvidenceRunCommand::Window(RunWindowArgs {
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                started_at: 1_700_000_000,
                ended_at: 1_700_000_060,
                observed_blocks: 10,
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Run(
            EvidenceRunCommand::WindowFile(RunWindowFromFileArgs {
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                block_observation_file: path("artifacts/block-observations.records"),
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::Heartbeat(NodeHeartbeatArgs {
                role: PublicNodeRoleArg::Miner,
                address: address_arg(address(b"miner-a")),
                operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
                first_block: 0,
                last_block: 9,
                heartbeat_count: 10,
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::HeartbeatFile(NodeHeartbeatFromFileArgs {
                role: PublicNodeRoleArg::Miner,
                address: address_arg(address(b"miner-a")),
                operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
                heartbeat_file: path("artifacts/miner-a-heartbeats.records"),
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Node(
            EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
                role: PublicNodeRoleArg::Miner,
                address: address_arg(address(b"miner-a")),
                operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
                identity_uri: "https://operators.tensorvm.net/miner-a".to_owned(),
                observed_at: 1_700_000_000,
            }),
        )))
    );
    let endpoint_id = manifest_hash(b"rpc-service");
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Service(
            EvidenceServiceCommand::Health(ServiceHealthArgs {
                kind: PublicServiceKindArg::Rpc,
                endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                first_block: 0,
                last_block: 9,
                reachable_count: 10,
                signed_health_check_count: 10,
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Service(
            EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs {
                kind: PublicServiceKindArg::Rpc,
                endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
                public_url: "https://rpc.tensorvm.net/health".to_owned(),
                health_path: "/health".to_owned(),
                observation_file: path("artifacts/rpc-health.records"),
            }),
        )))
    );
    let content_root = manifest_hash(b"rpc-service-content");
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Service(
            EvidenceServiceCommand::Content(ServiceContentArgs {
                kind: PublicServiceKindArg::Rpc,
                endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                content_root: hash_arg(hash_bytes(b"test", &[b"rpc-service-content"])),
                observed_at: 1_700_000_000,
                min_content_bytes: 64,
            }),
        )))
    );
    let content_hex = hex(&[42_u8; 64]);
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Service(
            EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs {
                kind: PublicServiceKindArg::Rpc,
                endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at: 1_700_000_000,
                content: HexBytesArg::new(vec![42_u8; 64]),
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Service(
            EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs {
                kind: PublicServiceKindArg::Rpc,
                endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
                public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
                content_path: "/chain/head".to_owned(),
                observed_at: 1_700_000_000,
                content_file: path("artifacts/rpc-chain-head.body"),
            }),
        )))
    );
    let peer_id = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Network(
            EvidenceNetworkCommand::Observation(NetworkObservationArgs {
                operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
                peer_id: peer_id.parse().expect("fixture peer ID must parse"),
                listen_address: multiaddr("/dns/node-a.tensorvm.net/tcp/4001"),
                observed_at: 1_700_000_000,
                gossip_topics: 5,
                request_response_protocols: 4,
                bootstrap_peers: 2,
                max_transmit_bytes: 1_048_576,
                request_timeout_seconds: 10,
                max_concurrent_streams: 128,
                idle_timeout_seconds: 60,
            }),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Network(
            EvidenceNetworkCommand::FromServiceLog(NetworkObservationFromServiceLogArgs {
                operator_id: hash_arg(hash_bytes(b"test", &[b"network-operator"])),
                listen_address: multiaddr("/dns/node-a.tensorvm.net/tcp/4001"),
                observed_at: 1_700_000_000,
                service_log: path("artifacts/node-a-service.log"),
            }),
        )))
    );
    let record_root = manifest_hash(b"network-runtime-root");
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        EvidenceFixture::RecordSummary {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"network-runtime-root"]),
            record_count: 4,
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        EvidenceFixture::RecordArtifact {
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
            "public",
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
        EvidenceFixture::RecordSummaryFromRoots {
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
            "public",
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
        EvidenceFixture::RecordArtifactFromRoots {
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
            "public",
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
        EvidenceFixture::RecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
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
        EvidenceFixture::RecordArtifactFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }
    );
    assert_eq!(
        parse_test_cli(&["node", "init", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args("/var/lib/tensorvm")))
    );
    let bootstrap_peer = PeerId::random().to_string();
    assert_eq!(
        parse_test_cli(&[
            "node",
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
        TvmdCommand::Node(NodeCommand::Peer(NodePeerCommand::Add(NodePeerAddArgs {
            data_dir: path("/var/lib/tensorvm"),
            peer_id: bootstrap_peer.parse().expect("fixture peer ID must parse"),
            address: multiaddr("/dns/bootstrap.tensorvm.net/tcp/4001"),
        })))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: multiaddr("/ip4/0.0.0.0/tcp/4001"),
            data_dir: path("/var/lib/tensorvm"),
            identity_seed: None,
        }))
    );
    let identity_seed = "11".repeat(32);
    assert_eq!(
        parse_test_cli(&[
            "node",
            "check",
            "--p2p-listen",
            "/ip4/0.0.0.0/tcp/4001",
            "--data-dir",
            "/var/lib/tensorvm",
            "--identity-seed",
            &identity_seed,
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Check(NodeCheckArgs {
            p2p_listen: multiaddr("/ip4/0.0.0.0/tcp/4001"),
            data_dir: path("/var/lib/tensorvm"),
            identity_seed: Some(super::HashArg::new([0x11; 32])),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
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
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "0.0.0.0:8545",
                "/ip4/0.0.0.0/tcp/4001",
                "/var/lib/tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
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
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "0.0.0.0:8545",
                "/ip4/0.0.0.0/tcp/4001",
                "/var/lib/tensorvm",
                Some([0x11; 32]),
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["node", "status", "--data-dir", "/var/lib/tensorvm"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Status(data_dir_args("/var/lib/tensorvm")))
    );
    assert_eq!(
        parse_test_cli(&[
            "node",
            "block",
            "--data-dir",
            "/var/lib/tensorvm",
            "--height",
            "3"
        ])
        .unwrap(),
        TvmdCommand::Node(NodeCommand::Block(NodeBlockArgs {
            data_dir: path("/var/lib/tensorvm"),
            height: 3,
        }))
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
        TvmdCommand::Proposer(ProposerCommand::Run(ValidatorRunArgs {
            wallet: path("proposer.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                None,
                "secret",
                7,
            ),
        }))
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
        TvmdCommand::Proposer(ProposerCommand::Run(ValidatorRunArgs {
            wallet: path("proposer.key"),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/0",
                "/var/lib/tensorvm",
                Some([0x33; 32]),
                "secret",
                7,
            ),
        }))
    );
}

#[test]
fn rejects_invalid_cli() {
    assert!(parse_test_cli(&["miner", "register"]).is_err());
    assert!(parse_test_cli(&["validator", "register", "--stake", "abc"]).is_err());
    assert!(
        parse_test_cli(&[
            "node",
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
fn rejects_retired_top_level_command_families() {
    assert!(parse_test_cli(&["role", "miner", "status"]).is_err());
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "validate",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-testnet",
            "preflight",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .is_err()
    );
    assert!(parse_test_cli(&["local-testnet", "seed", "--data-dir", "/var/lib/tensorvm"]).is_err());
    assert!(parse_test_cli(&["local-cpu", "verify", "--json"]).is_err());
}

#[test]
fn clap_cli_defaults_runtime_arguments() {
    assert_eq!(
        parse_test_cli(&["miner", "check", "--wallet", "miner.key"]).unwrap(),
        TvmdCommand::Miner(MinerCommand::Check(MinerCheckArgs {
            wallet: path("miner.key"),
            device: "cpu".to_owned(),
            node: multiaddr("/ip4/127.0.0.1/tcp/4001"),
        }))
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
        TvmdCommand::Miner(MinerCommand::Run(MinerRunArgs {
            wallet: path("miner.key"),
            device: "cpu".to_owned(),
            runtime: role_runtime_args(
                "/ip4/127.0.0.1/tcp/4001",
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/4001",
                ".tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["node", "serve", "--auth-token", "secret"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Serve(NodeServeArgs {
            runtime: node_runtime_args(
                "127.0.0.1:8545",
                "/ip4/127.0.0.1/tcp/4001",
                ".tensorvm",
                None,
                "secret",
                0,
            ),
        }))
    );
    assert_eq!(
        parse_test_cli(&["node", "init"]).unwrap(),
        TvmdCommand::Node(NodeCommand::Init(data_dir_args(".tensorvm")))
    );
}
