use super::parser_support::{hash_arg, path};
use super::{
    EvidenceCommand, EvidenceServiceCommand, HexBytesArg, PublicCommand, PublicServiceEndpointArgs,
    PublicServiceKindArg, ServiceContentArgs, ServiceContentFromBytesArgs,
    ServiceContentFromFileArgs, ServiceContentTargetArgs, ServiceHealthArgs,
    ServiceHealthFromFileArgs, ServiceHealthPathArgs, TvmdCommand, block_height_window_args,
    manifest_hash, observation_timestamp_args, parse_test_cli,
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
            EvidenceServiceCommand::Health(ServiceHealthArgs::new(
                service_endpoint_args("https://rpc.tensorvm.net/health"),
                service_health_path_args("/health"),
                block_height_window_args(0, 9),
                10,
                10,
            )),
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
            EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs::new(
                service_endpoint_args("https://rpc.tensorvm.net/health"),
                service_health_path_args("/health"),
                path("artifacts/rpc-health.records"),
            )),
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
            EvidenceServiceCommand::Content(ServiceContentArgs::new(
                service_content_target_args("https://rpc.tensorvm.net/chain/head", "/chain/head",),
                hash_arg(hash_bytes(b"test", &[b"rpc-service-content"])),
                64,
            )),
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
            EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs::new(
                service_content_target_args("https://rpc.tensorvm.net/chain/head", "/chain/head",),
                HexBytesArg::new(vec![42_u8; 64]),
            )),
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
            EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs::new(
                service_content_target_args("https://rpc.tensorvm.net/chain/head", "/chain/head",),
                path("artifacts/rpc-chain-head.body"),
            )),
        )))
    );
}

fn service_endpoint_args(public_url: &str) -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs::new(
        PublicServiceKindArg::Rpc,
        hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url,
    )
}

fn service_health_path_args(health_path: &str) -> ServiceHealthPathArgs {
    ServiceHealthPathArgs::new(health_path)
}

fn service_content_target_args(public_url: &str, content_path: &str) -> ServiceContentTargetArgs {
    ServiceContentTargetArgs::new(
        service_endpoint_args(public_url),
        content_path,
        observation_timestamp_args(1_700_000_000),
    )
}
