use super::*;

#[test]
fn execute_public_service_health_evidence_rejects_invalid_args() {
    for public_url in [
        "http://127.0.0.1/health",
        "https://rpc.example.test/health",
        "https://rpc.tensorvm.net/",
        "https://rpc.tensorvm.net/health?probe=1",
        "https://rpc.tensorvm.net/health#probe",
        "https://rpc.tensorvm.net/wrong",
    ] {
        let mut args = valid_service_health_args();
        args.endpoint.public_url = public_url.to_owned();
        assert!(
            execute_service_health(args).is_err(),
            "public URL {public_url:?} should be rejected"
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
    let mut args = valid_service_health_args();
    args.endpoint.endpoint_id = hash_arg([0; 32]);
    assert!(
        execute_service_health(args).is_err(),
        "zero endpoint id should be rejected"
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
}

fn valid_service_health_args() -> ServiceHealthArgs {
    ServiceHealthArgs {
        endpoint: valid_service_health_endpoint_args(),
        health_path: "/health".to_owned(),
        first_block: 0,
        last_block: 9,
        reachable_count: 10,
        signed_health_check_count: 10,
    }
}

fn valid_service_health_file_args() -> ServiceHealthFromFileArgs {
    ServiceHealthFromFileArgs {
        endpoint: valid_service_health_endpoint_args(),
        health_path: "/health".to_owned(),
        observation_file: missing_temp_file("unused-service-health", "records"),
    }
}

fn valid_service_health_endpoint_args() -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs {
        kind: service_kind_arg(PublicServiceKind::Rpc),
        endpoint_id: hash_arg(hash_bytes(b"test", &[b"rpc-service"])),
        public_url: "https://rpc.tensorvm.net/health".to_owned(),
    }
}

fn execute_service_health(args: ServiceHealthArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::Health(args))
}

fn execute_service_health_file(args: ServiceHealthFromFileArgs) -> crate::error::Result<String> {
    execute_service_command(EvidenceServiceCommand::HealthFile(args))
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
