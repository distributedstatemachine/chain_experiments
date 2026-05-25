use super::*;

#[test]
fn execute_service_evidence_reports_outputs() {
    let service_health = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::Health(ServiceHealthArgs::new(
            service_endpoint_args(
                PublicServiceKind::Rpc,
                b"rpc-service",
                "https://rpc.tensorvm.net/health",
            ),
            service_health_path_args("/health"),
            block_height_window_args(0, 9),
            10,
            10,
        )),
    ))
    .unwrap();
    let rpc_service_id = manifest_hash(b"rpc-service");
    let rpc_service_signature = manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service");
    assert_eq!(
        comma_record_fields(&service_health, "service=", 9),
        [
            "rpc",
            rpc_service_id.as_str(),
            "https://rpc.tensorvm.net/health",
            "/health",
            "0",
            "9",
            "10",
            "10",
            rpc_service_signature.as_str(),
        ]
    );
    let health_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-health-{}-{}.records",
        std::process::id(),
        manifest_hash(b"rpc-service").as_bytes()[0]
    ));
    let health_observations = (0..10)
        .map(|block| format!("service_health_observation={block},reachable"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&health_observation_file, health_observations).unwrap();
    let service_health_from_file = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::HealthFile(ServiceHealthFromFileArgs::new(
            service_endpoint_args(
                PublicServiceKind::Rpc,
                b"rpc-service",
                "https://rpc.tensorvm.net/health",
            ),
            service_health_path_args("/health"),
            health_observation_file.clone(),
        )),
    ))
    .unwrap();
    std::fs::remove_file(&health_observation_file).unwrap();
    assert_eq!(service_health_from_file, service_health);
    let additional_service_cases: [(PublicServiceKind, &[u8], &str); 3] = [
        (PublicServiceKind::Explorer, b"explorer-service", "explorer"),
        (PublicServiceKind::Faucet, b"faucet-service", "faucet"),
        (
            PublicServiceKind::Telemetry,
            b"telemetry-service",
            "telemetry",
        ),
    ];
    for (kind, label, tag) in additional_service_cases {
        let line = execute_public_evidence_command(&EvidenceCommand::Service(
            EvidenceServiceCommand::Health(ServiceHealthArgs::new(
                service_endpoint_args(kind, label, public_service_url(kind)),
                service_health_path_args("/health"),
                block_height_window_args(0, 9),
                10,
                10,
            )),
        ))
        .unwrap();
        let endpoint_id = manifest_hash(label);
        let service_signature = manifest_service_signature(kind, label);
        assert_eq!(
            comma_record_fields(&line, "service=", 9),
            [
                tag,
                endpoint_id.as_str(),
                public_service_url(kind),
                "/health",
                "0",
                "9",
                "10",
                "10",
                service_signature.as_str(),
            ]
        );
    }

    let service_content = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::Content(ServiceContentArgs {
            target: service_content_target_args(PublicServiceKind::Rpc, b"rpc-service"),
            content_root: hash_arg(hash_bytes(b"test", &[b"rpc-service", b"content-root"])),
            min_content_bytes: 64,
        }),
    ))
    .unwrap();
    let rpc_content_root = hex(&hash_bytes(b"test", &[b"rpc-service", b"content-root"]));
    let rpc_content_signature =
        public_service_content(PublicServiceKind::Rpc, b"rpc-service").content_signature;
    let rpc_content_signature = hex(&rpc_content_signature);
    assert_eq!(
        comma_record_fields(&service_content, "service_content=", 8),
        [
            "rpc",
            rpc_service_id.as_str(),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            rpc_content_root.as_str(),
            "1700000000",
            "64",
            rpc_content_signature.as_str(),
        ]
    );
    assert_eq!(
        service_content,
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service")
    );
    let observed_content = vec![7_u8; 80];
    let observed_content_root = public_service_content_root(&observed_content);
    let service_content_from_bytes = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::ContentBytes(ServiceContentFromBytesArgs {
            target: service_content_target_args(PublicServiceKind::Rpc, b"rpc-service"),
            content: HexBytesArg::new(observed_content.clone()),
        }),
    ))
    .unwrap();
    let observed_content_root_hex = hex(&observed_content_root);
    let service_content_from_bytes_fields =
        comma_record_fields(&service_content_from_bytes, "service_content=", 8);
    assert_eq!(
        service_content_from_bytes_fields[..7],
        [
            "rpc",
            rpc_service_id.as_str(),
            public_service_content_url(PublicServiceKind::Rpc),
            public_service_content_path(PublicServiceKind::Rpc),
            observed_content_root_hex.as_str(),
            "1700000000",
            "80",
        ]
    );
    let content_file = std::env::temp_dir().join(format!(
        "tensor-vm-service-content-{}-{}.body",
        std::process::id(),
        observed_content_root[0]
    ));
    std::fs::write(&content_file, &observed_content).unwrap();
    let service_content_from_file = execute_public_evidence_command(&EvidenceCommand::Service(
        EvidenceServiceCommand::ContentFile(ServiceContentFromFileArgs {
            target: service_content_target_args(PublicServiceKind::Rpc, b"rpc-service"),
            content_file: content_file.clone(),
        }),
    ))
    .unwrap();
    std::fs::remove_file(&content_file).unwrap();
    assert_eq!(service_content_from_file, service_content_from_bytes);
}

fn service_endpoint_args(
    kind: PublicServiceKind,
    label: &[u8],
    public_url: &str,
) -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs {
        kind: service_kind_arg(kind),
        endpoint_id: hash_arg(hash_bytes(b"test", &[label])),
        public_url: public_url.to_owned(),
    }
}

fn service_content_target_args(kind: PublicServiceKind, label: &[u8]) -> ServiceContentTargetArgs {
    ServiceContentTargetArgs {
        endpoint: service_endpoint_args(kind, label, public_service_content_url(kind)),
        content_path: public_service_content_path(kind).to_owned(),
        observation: observation_timestamp_args(1_700_000_000),
    }
}
