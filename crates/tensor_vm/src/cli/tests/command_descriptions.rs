use super::{
    CommandFixture, describe_command_fixture, manifest_address, manifest_auditor_uri,
    parse_test_cli,
};
use crate::hash::hex;
use crate::testnet::{PublicEvidenceRecordKind, PublicNodeRole, PublicServiceKind};
use crate::types::{address, hash_bytes};
use libp2p::PeerId;

#[test]
fn clap_cli_parses_and_describes_commands() {
    let command = parse_test_cli(&["miner", "register", "--stake", "250"]).unwrap();
    assert_eq!(command, CommandFixture::MinerRegister { stake: 250 });
    let bootstrap_peer = PeerId::random().to_string();

    let commands = [
        (
            CommandFixture::MinerRegister { stake: 1 },
            "register miner with stake 1",
        ),
        (
            CommandFixture::MinerStart {
                wallet: "miner.key".to_owned(),
                device: "cpu".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            },
            "start miner wallet=miner.key device=cpu node=/ip4/127.0.0.1/tcp/4001",
        ),
        (
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
            },
            "run miner role wallet=miner.key device=cpu node=/ip4/127.0.0.1/tcp/4001 listen=127.0.0.1:8545 p2p_listen=/ip4/127.0.0.1/tcp/0 data_dir=/var/lib/tensorvm max_requests=7 max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
        ),
        (CommandFixture::MinerStatus, "show miner status"),
        (
            CommandFixture::ValidatorRegister { stake: 10 },
            "register validator with stake 10",
        ),
        (
            CommandFixture::ValidatorStart {
                wallet: "validator.key".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
            },
            "start validator wallet=validator.key node=/ip4/127.0.0.1/tcp/4001",
        ),
        (
            CommandFixture::ValidatorRun {
                wallet: "validator.key".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                listen: "127.0.0.1:8545".to_owned(),
                p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                identity_seed: None,
                auth_token: "secret".to_owned(),
                max_requests: 7,
            },
            "run validator role wallet=validator.key node=/ip4/127.0.0.1/tcp/4001 listen=127.0.0.1:8545 p2p_listen=/ip4/127.0.0.1/tcp/0 data_dir=/var/lib/tensorvm max_requests=7 max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
        ),
        (CommandFixture::ValidatorStatus, "show validator status"),
        (
            CommandFixture::ProposerRun {
                wallet: "proposer.key".to_owned(),
                node: "/ip4/127.0.0.1/tcp/4001".to_owned(),
                listen: "127.0.0.1:8545".to_owned(),
                p2p_listen: "/ip4/127.0.0.1/tcp/0".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                identity_seed: None,
                auth_token: "secret".to_owned(),
                max_requests: 7,
            },
            "run proposer role wallet=proposer.key node=/ip4/127.0.0.1/tcp/4001 listen=127.0.0.1:8545 p2p_listen=/ip4/127.0.0.1/tcp/0 data_dir=/var/lib/tensorvm max_requests=7 max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
        ),
        (
            CommandFixture::ServiceInit {
                data_dir: "/var/lib/tensorvm".to_owned(),
            },
            "initialize service node store data_dir=/var/lib/tensorvm",
        ),
        (
            CommandFixture::ServicePeerAdd {
                data_dir: "/var/lib/tensorvm".to_owned(),
                peer_id: bootstrap_peer.clone(),
                address: "/dns/bootstrap.tensorvm.net/tcp/4001".to_owned(),
            },
            "add libp2p bootstrap peer data_dir=/var/lib/tensorvm peer_id=",
        ),
        (
            CommandFixture::ServiceReadiness {
                p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                identity_seed: None,
            },
            "check mandatory libp2p service readiness p2p_listen=/ip4/0.0.0.0/tcp/4001 data_dir=/var/lib/tensorvm max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
        ),
        (
            CommandFixture::ServiceServe {
                listen: "0.0.0.0:8545".to_owned(),
                p2p_listen: "/ip4/0.0.0.0/tcp/4001".to_owned(),
                data_dir: "/var/lib/tensorvm".to_owned(),
                identity_seed: None,
                auth_token: "secret".to_owned(),
                max_requests: 0,
            },
            "serve RPC explorer faucet telemetry over mandatory libp2p listen=0.0.0.0:8545 p2p_listen=/ip4/0.0.0.0/tcp/4001 data_dir=/var/lib/tensorvm max_requests=0 max_transmit_bytes=1048576 request_timeout_seconds=10 max_concurrent_streams=128 idle_timeout_seconds=60",
        ),
        (
            CommandFixture::ServiceStatus {
                data_dir: "/var/lib/tensorvm".to_owned(),
            },
            "show service node store status data_dir=/var/lib/tensorvm",
        ),
        (
            CommandFixture::ServiceBlock {
                data_dir: "/var/lib/tensorvm".to_owned(),
                height: 3,
            },
            "show service node store block data_dir=/var/lib/tensorvm height=3",
        ),
        (
            CommandFixture::LocalTestnetSeed {
                data_dir: "/var/lib/tensorvm".to_owned(),
            },
            "seed local CPU testnet data_dir=/var/lib/tensorvm",
        ),
        (
            CommandFixture::PublicEvidenceValidate {
                manifest: "evidence.txt".to_owned(),
            },
            "validate public evidence manifest evidence.txt",
        ),
        (
            CommandFixture::PublicTestnetPreflight {
                manifest: "preflight.txt".to_owned(),
            },
            "run public testnet preflight manifest preflight.txt",
        ),
    ];
    for (command, description) in commands {
        let actual = describe_command_fixture(&command);
        if matches!(command, CommandFixture::ServicePeerAdd { .. }) {
            assert!(actual.starts_with(description));
            assert!(actual.contains("address=/dns/bootstrap.tensorvm.net/tcp/4001"));
        } else {
            assert_eq!(actual, description);
        }
    }

    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceServiceHealth {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: "artifacts/rpc-health.records".to_owned(),
        }),
        "generate rpc service health evidence from captured observations observation_file=artifacts/rpc-health.records public_url=https://rpc.tensorvm.net/health health_path=/health"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceServiceContent {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceServiceContentFromBytes {
            kind: PublicServiceKind::Faucet,
            endpoint_id: hash_bytes(b"test", &[b"faucet-service"]),
            public_url: "https://faucet.tensorvm.net/faucet/page".to_owned(),
            content_path: "/faucet/page".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_bytes: vec![1_u8; 64],
        }),
        "generate faucet service content evidence from observed bytes public_url=https://faucet.tensorvm.net/faucet/page content_path=/faucet/page"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceServiceContentFromFile {
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
        describe_command_fixture(&CommandFixture::PublicEvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        }),
        "generate public evidence publication signature public_uri=https://tensorvm.net/tensorvm/public-evidence.json"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        }),
        "generate public evidence run window started=1700000000 ended=1700000060 observed_blocks=10"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: "artifacts/block-observations.records".to_owned(),
        }),
        "generate public evidence run window from captured block observations block_observation_file=artifacts/block-observations.records"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceAuditorRecord {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordSummaryFromRoots {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::InvalidWorkRejections,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"invalid-work-root"]),
            record_count: 1,
        }),
        "generate invalid-work public evidence record summary records=1"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
            kind: PublicEvidenceRecordKind::RewardSettlements,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
            record_count: 1,
        }),
        "generate reward-settlement public evidence record summary records=1"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordArtifact {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordArtifactFromRoots {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordSummaryFromFile {
            kind: PublicEvidenceRecordKind::NetworkRuntimeObservations,
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            record_file: "artifacts/network-runtime.records".to_owned(),
        }),
        "generate network-runtime public evidence record summary from record file record_file=artifacts/network-runtime.records"
    );
    assert_eq!(
        describe_command_fixture(&CommandFixture::PublicEvidenceRecordArtifactFromFile {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceNetworkObservation {
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
        }),
        format!(
            "generate signed libp2p network observation peer_id={peer_id} listen_address=/dns/node-a.tensorvm.net/tcp/4001"
        )
    );
    assert_eq!(
        describe_command_fixture(
            &CommandFixture::PublicEvidenceNetworkObservationFromServiceLog {
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
            describe_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeat {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceNodeHeartbeatFromFile {
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
        describe_command_fixture(&CommandFixture::PublicEvidenceOperatorAttestation {
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
            describe_command_fixture(&CommandFixture::PublicEvidenceRecordSummary {
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
