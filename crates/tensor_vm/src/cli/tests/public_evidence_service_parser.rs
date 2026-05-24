use super::parser_support::{hash_arg, path};
use super::{
    EvidenceCommand, EvidenceServiceCommand, HexBytesArg, PublicCommand, PublicServiceKindArg,
    ServiceContentArgs, ServiceContentFromBytesArgs, ServiceContentFromFileArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs, TvmdCommand, manifest_hash, parse_test_cli,
};
use crate::hash::hex;
use crate::types::hash_bytes;

#[test]
fn parses_service_evidence_commands() {
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
}
