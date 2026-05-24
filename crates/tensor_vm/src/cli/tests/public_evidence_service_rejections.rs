use super::*;

#[test]
fn execute_public_service_evidence_rejects_invalid_args() {
    for public_url in [
        "http://127.0.0.1/health",
        "https://rpc.example.test/health",
        "https://rpc.tensorvm.net/",
        "https://rpc.tensorvm.net/health?probe=1",
        "https://rpc.tensorvm.net/health#probe",
        "https://rpc.tensorvm.net/wrong",
    ] {
        assert!(
            execute_service_health(ServiceHealthArgs {
                public_url: public_url.to_owned(),
                ..valid_service_health_args()
            })
            .is_err()
        );
    }
    assert!(
        execute_service_health(ServiceHealthArgs {
            health_path: "health".to_owned(),
            ..valid_service_health_args()
        })
        .is_err()
    );
    assert!(
        execute_service_health(ServiceHealthArgs {
            first_block: 10,
            ..valid_service_health_args()
        })
        .is_err()
    );
    assert!(
        execute_service_health(ServiceHealthArgs {
            endpoint_id: hash_arg([0; 32]),
            ..valid_service_health_args()
        })
        .is_err()
    );
    assert!(
        execute_service_health(ServiceHealthArgs {
            reachable_count: 0,
            ..valid_service_health_args()
        })
        .is_err()
    );

    let partial_health = service_health_observation_summary_from_file(
        "service_health_observation=0,reachable\nservice_health_observation=1,unreachable\n",
    )
    .unwrap();
    assert_eq!(partial_health.first_seen_block, 0);
    assert_eq!(partial_health.last_seen_block, 1);
    assert_eq!(partial_health.reachable_observation_count, 1);
    assert_eq!(partial_health.signed_health_check_count, 2);
    for invalid_health_observations in [
        "# no observations\n\n",
        " service_health_observation=0,reachable\n",
        "service_health_observation=0,reachable\nservice_health_observation=0,reachable\n",
        "service_health_observation=0,reachable\nservice_health_observation=2,reachable\n",
        "service_health_observation=0,ok\n",
        "service_health_observation=0, reachable\n",
        "service_health_observation=0\n",
        "record_root=00\n",
    ] {
        assert!(service_health_observation_summary_from_file(invalid_health_observations).is_err());
    }
    assert!(
        execute_service_health_file(ServiceHealthFromFileArgs {
            observation_file: missing_temp_file("service-health", "records"),
            ..valid_service_health_file_args()
        })
        .is_err()
    );

    for public_url in [
        "https://localhost/chain/head",
        "https://rpc.tensorvm.net/",
        "https://rpc.tensorvm.net/chain/head?height=1",
        "https://rpc.tensorvm.net/chain/head#latest",
        "https://rpc.tensorvm.net/wrong",
    ] {
        assert!(
            execute_service_content(ServiceContentArgs {
                public_url: public_url.to_owned(),
                ..valid_service_content_args()
            })
            .is_err()
        );
    }
    assert!(
        execute_service_content(ServiceContentArgs {
            content_path: "chain/head".to_owned(),
            ..valid_service_content_args()
        })
        .is_err()
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            content_path: "/wrong".to_owned(),
            ..valid_service_content_args()
        })
        .is_err()
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            content_root: hash_arg([0; 32]),
            ..valid_service_content_args()
        })
        .is_err()
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            observed_at: 0,
            ..valid_service_content_args()
        })
        .is_err()
    );
    assert!(
        execute_service_content(ServiceContentArgs {
            min_content_bytes: 63,
            ..valid_service_content_args()
        })
        .is_err()
    );

    let endpoint_id = hex(&hash_bytes(b"test", &[b"rpc-service"]));
    for content_hex in ["zz", "abc"] {
        assert!(
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
                content_hex,
            ])
            .is_err()
        );
    }
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

fn valid_service_health_args() -> ServiceHealthArgs {
    ServiceHealthArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/health".to_owned(),
        health_path: "/health".to_owned(),
        first_block: 0,
        last_block: 9,
        reachable_count: 10,
        signed_health_check_count: 10,
    }
}

fn valid_service_health_file_args() -> ServiceHealthFromFileArgs {
    ServiceHealthFromFileArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/health".to_owned(),
        health_path: "/health".to_owned(),
        observation_file: missing_temp_file("unused-service-health", "records"),
    }
}

fn valid_service_content_args() -> ServiceContentArgs {
    ServiceContentArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
        content_path: "/chain/head".to_owned(),
        content_root: hash_arg(hash_bytes(b"test", &[b"rpc-service", b"content-root"])),
        observed_at: 1_700_000_000,
        min_content_bytes: 64,
    }
}

fn valid_service_content_bytes_args() -> ServiceContentFromBytesArgs {
    ServiceContentFromBytesArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
        content_path: "/chain/head".to_owned(),
        observed_at: 1_700_000_000,
        content: HexBytesArg::new(vec![1_u8; 64]),
    }
}

fn valid_service_content_file_args() -> ServiceContentFromFileArgs {
    ServiceContentFromFileArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
        content_path: "/chain/head".to_owned(),
        observed_at: 1_700_000_000,
        content_file: missing_temp_file("unused-service-content", "body"),
    }
}

fn execute_service_health(args: ServiceHealthArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::Health(args))
}

fn execute_service_health_file(args: ServiceHealthFromFileArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::HealthFile(args))
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
