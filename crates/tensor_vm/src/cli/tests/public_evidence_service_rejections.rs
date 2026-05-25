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
        let mut args = valid_service_content_args();
        args.target.endpoint.public_url = public_url.to_owned();
        assert!(
            execute_service_content(args).is_err(),
            "public URL {public_url:?} should be rejected"
        );
    }
    let mut args = valid_service_content_args();
    args.target.content_path = "chain/head".to_owned();
    assert!(
        execute_service_content(args).is_err(),
        "relative content path should be rejected"
    );
    let mut args = valid_service_content_args();
    args.target.endpoint.public_url = "https://rpc.tensorvm.net/wrong".to_owned();
    args.target.content_path = "/wrong".to_owned();
    assert!(
        execute_service_content(args).is_err(),
        "wrong content endpoint should be rejected"
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            content_root: hash_arg([0; 32]),
            ..valid_service_content_args()
        })
        .is_err()
    );
    let mut args = valid_service_content_args();
    args.target.observed_at = 0;
    assert!(
        execute_service_content(args).is_err(),
        "zero observation timestamp should be rejected"
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            min_content_bytes: 63,
            ..valid_service_content_args()
        })
        .is_err()
    );

    assert!(
        execute_service_content_bytes(ServiceContentFromBytesArgs {
            content: HexBytesArg::new(vec![1_u8; 63]),
            ..valid_service_content_bytes_args()
        })
        .is_err()
    );
    assert!(
        execute_service_content_file(ServiceContentFromFileArgs {
            content_file: missing_temp_file("service-content", "body"),
            ..valid_service_content_file_args()
        })
        .is_err()
    );
}

fn valid_service_content_args() -> ServiceContentArgs {
    ServiceContentArgs {
        target: valid_service_content_target_args(),
        content_root: hash_arg(hash_bytes(b"test", &[b"rpc-service", b"content-root"])),
        min_content_bytes: 64,
    }
}

fn valid_service_content_bytes_args() -> ServiceContentFromBytesArgs {
    ServiceContentFromBytesArgs {
        target: valid_service_content_target_args(),
        content: HexBytesArg::new(vec![1_u8; 64]),
    }
}

fn valid_service_content_file_args() -> ServiceContentFromFileArgs {
    ServiceContentFromFileArgs {
        target: valid_service_content_target_args(),
        content_file: missing_temp_file("unused-service-content", "body"),
    }
}

fn valid_service_content_target_args() -> ServiceContentTargetArgs {
    ServiceContentTargetArgs {
        endpoint: valid_service_endpoint_args(),
        content_path: "/chain/head".to_owned(),
        observed_at: 1_700_000_000,
    }
}

fn valid_service_endpoint_args() -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
    }
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
