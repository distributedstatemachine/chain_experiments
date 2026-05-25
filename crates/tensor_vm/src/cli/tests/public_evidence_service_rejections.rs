use super::*;

#[test]
fn execute_public_service_evidence_rejects_invalid_args() {
    for public_url in [
        "https://localhost/chain/head",
        "https://rpc.tensorvm.net/",
        "https://rpc.tensorvm.net/chain/head?height=1",
        "https://rpc.tensorvm.net/chain/head#latest",
        "https://rpc.tensorvm.net/wrong",
    ] {
        assert!(
            execute_service_content(service_content_args(
                service_content_target_args(public_url, "/chain/head", 1_700_000_000),
                hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
                64,
            ))
            .is_err(),
            "public URL {public_url:?} should be rejected"
        );
    }
    assert!(
        execute_service_content(service_content_args(
            service_content_target_args(
                "https://rpc.tensorvm.net/chain/head",
                "chain/head",
                1_700_000_000
            ),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            64,
        ))
        .is_err(),
        "relative content path should be rejected"
    );
    assert!(
        execute_service_content(service_content_args(
            service_content_target_args("https://rpc.tensorvm.net/wrong", "/wrong", 1_700_000_000),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            64,
        ))
        .is_err(),
        "wrong content endpoint should be rejected"
    );
    assert!(
        execute_service_content(service_content_args(
            valid_service_content_target_args(),
            [0; 32],
            64,
        ))
        .is_err()
    );
    assert!(
        execute_service_content(service_content_args(
            service_content_target_args("https://rpc.tensorvm.net/chain/head", "/chain/head", 0),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            64,
        ))
        .is_err(),
        "zero observation timestamp should be rejected"
    );
    assert!(
        execute_service_content(service_content_args(
            valid_service_content_target_args(),
            hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            63,
        ))
        .is_err()
    );

    assert!(
        execute_service_content_bytes(ServiceContentFromBytesArgs::new(
            valid_service_content_target_args(),
            HexBytesArg::new(vec![1_u8; 63]),
        ))
        .is_err()
    );
    assert!(
        execute_service_content_file(ServiceContentFromFileArgs::new(
            valid_service_content_target_args(),
            missing_temp_file("service-content", "body"),
        ))
        .is_err()
    );
}

fn valid_service_content_target_args() -> ServiceContentTargetArgs {
    service_content_target_args(
        "https://rpc.tensorvm.net/chain/head",
        "/chain/head",
        1_700_000_000,
    )
}

fn service_content_target_args(
    public_url: &str,
    content_path: &str,
    observed_at: u64,
) -> ServiceContentTargetArgs {
    ServiceContentTargetArgs::new(
        service_endpoint_args(public_url),
        content_path,
        observation_timestamp_args(observed_at),
    )
}

fn service_endpoint_args(public_url: &str) -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs::new(
        service_kind_arg(PublicServiceKind::Rpc),
        hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url,
    )
}

fn service_content_args(
    target: ServiceContentTargetArgs,
    content_root: [u8; 32],
    min_content_bytes: u64,
) -> ServiceContentArgs {
    ServiceContentArgs::new(target, hash_arg(content_root), min_content_bytes)
}

fn execute_service_content(args: ServiceContentArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::Content(args))
}

fn execute_service_content_bytes(
    args: ServiceContentFromBytesArgs,
) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::ContentBytes(args))
}

fn execute_service_content_file(args: ServiceContentFromFileArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::ContentFile(args))
}

fn execute_service_command(command: EvidenceServiceCommand) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Service(command))
}

fn missing_temp_file(stem: &str, extension: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "missing-tensor-vm-{stem}-{}.{extension}",
        std::process::id()
    ))
}
