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
        assert!(
            execute_service_health(service_health_args(
                service_health_endpoint_args(public_url),
                service_health_path_args("/health"),
                block_height_window_args(0, 9),
                10,
                10,
            ))
            .is_err(),
            "public URL {public_url:?} should be rejected"
        );
    }
    assert!(
        execute_service_health(service_health_args(
            valid_service_health_endpoint_args(),
            service_health_path_args("health"),
            block_height_window_args(0, 9),
            10,
            10,
        ))
        .is_err()
    );
    assert!(
        execute_service_health(service_health_args(
            valid_service_health_endpoint_args(),
            service_health_path_args("/health"),
            block_height_window_args(10, 9),
            10,
            10,
        ))
        .is_err()
    );
    assert!(
        execute_service_health(service_health_args(
            service_health_endpoint_args_from_id([0; 32]),
            service_health_path_args("/health"),
            block_height_window_args(0, 9),
            10,
            10,
        ))
        .is_err(),
        "zero endpoint id should be rejected"
    );
    assert!(
        execute_service_health(service_health_args(
            valid_service_health_endpoint_args(),
            service_health_path_args("/health"),
            block_height_window_args(0, 9),
            0,
            10,
        ))
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
        execute_service_health_file(ServiceHealthFromFileArgs::new(
            valid_service_health_endpoint_args(),
            service_health_path_args("/health"),
            missing_temp_file("service-health", "records"),
        ))
        .is_err()
    );
}

fn valid_service_health_endpoint_args() -> PublicServiceEndpointArgs {
    service_health_endpoint_args("https://rpc.tensorvm.net/health")
}

fn service_health_endpoint_args(public_url: &str) -> PublicServiceEndpointArgs {
    service_health_endpoint_args_from(hash_bytes(b"test", &[b"rpc-service"]), public_url)
}

fn service_health_endpoint_args_from_id(endpoint_id: [u8; 32]) -> PublicServiceEndpointArgs {
    service_health_endpoint_args_from(endpoint_id, "https://rpc.tensorvm.net/health")
}

fn service_health_endpoint_args_from(
    endpoint_id: [u8; 32],
    public_url: &str,
) -> PublicServiceEndpointArgs {
    PublicServiceEndpointArgs::new(
        service_kind_arg(PublicServiceKind::Rpc),
        hash_arg(endpoint_id),
        public_url,
    )
}

fn service_health_args(
    endpoint: PublicServiceEndpointArgs,
    health: ServiceHealthPathArgs,
    window: BlockHeightWindowArgs,
    reachable_count: u64,
    signed_health_check_count: u64,
) -> ServiceHealthArgs {
    ServiceHealthArgs::new(
        endpoint,
        health,
        window,
        reachable_count,
        signed_health_check_count,
    )
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
