use super::parser_support::{address_arg, hash_arg, multiaddr, path};
use super::{
    EvidenceCommand, EvidenceNetworkCommand, EvidenceRecordCommand, NetworkObservationArgs,
    NetworkObservationFromServiceLogArgs, PublicCommand, PublicEvidenceManifestArgs,
    PublicEvidenceRecordKindArg, PublicTestnetManifestArgs, RecordArtifactArgs,
    RecordArtifactFromFileArgs, RecordArtifactFromRootsArgs, RecordSummaryArgs,
    RecordSummaryFromFileArgs, RecordSummaryFromRootsArgs, TvmdCommand, manifest_address,
    manifest_hash, parse_test_cli,
};
use crate::types::{address, hash_bytes};
use libp2p::PeerId;

#[test]
fn parses_documented_public_commands() {
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
                peer_id: peer_id.parse().expect("test peer ID must parse"),
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::Summary(RecordSummaryArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                record_root: hash_arg(hash_bytes(b"test", &[b"network-runtime-root"])),
                record_count: 4,
            }),
        )))
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::Artifact(RecordArtifactArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_root: hash_arg(hash_bytes(b"test", &[b"network-runtime-root"])),
                record_count: 4,
            }),
        )))
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                record_roots: vec![
                    hash_arg(hash_bytes(b"test", &[b"network-observation-a"])),
                    hash_arg(hash_bytes(b"test", &[b"network-observation-b"])),
                ],
            }),
        )))
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_roots: vec![
                    hash_arg(hash_bytes(b"test", &[b"network-observation-a"])),
                    hash_arg(hash_bytes(b"test", &[b"network-observation-b"])),
                ],
            }),
        )))
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::SummaryFile(RecordSummaryFromFileArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                record_file: path("artifacts/network-runtime.records"),
            }),
        )))
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
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Record(
            EvidenceRecordCommand::ArtifactFile(RecordArtifactFromFileArgs {
                kind: PublicEvidenceRecordKindArg::NetworkRuntime,
                bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
                record_file: path("artifacts/network-runtime.records"),
            }),
        )))
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
