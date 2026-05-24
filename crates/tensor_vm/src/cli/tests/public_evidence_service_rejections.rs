use super::*;

#[test]
fn execute_evidence_fixture_rejects_invalid_public_service_evidence_args() {
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "http://127.0.0.1/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.example.test/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health?probe=1".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health#probe".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 10,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: [0; 32],
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 10,
            signed_health_check_count: 10,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceHealth {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            first_seen_block: 0,
            last_seen_block: 9,
            reachable_observation_count: 0,
            signed_health_check_count: 10,
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
        execute_evidence_fixture(&EvidenceFixture::ServiceHealthFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/health".to_owned(),
            health_path: "/health".to_owned(),
            observation_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-service-health-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://localhost/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head?height=1".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head#latest".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/wrong".to_owned(),
            content_path: "/wrong".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: [0; 32],
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 0,
            min_content_bytes: 64,
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContent {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            content_root: hash_bytes(b"test", &[b"rpc-service", b"content-root"]),
            observed_at_unix_seconds: 1_700_000_000,
            min_content_bytes: 63,
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
        execute_evidence_fixture(&EvidenceFixture::ServiceContentFromBytes {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_bytes: vec![1_u8; 63],
        })
        .is_err()
    );
    assert!(
        execute_evidence_fixture(&EvidenceFixture::ServiceContentFromFile {
            kind: PublicServiceKind::Rpc,
            endpoint_id: hash_bytes(b"test", &[b"rpc-service"]),
            public_url: "https://rpc.tensorvm.net/chain/head".to_owned(),
            content_path: "/chain/head".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
            content_file: std::env::temp_dir()
                .join("tensor-vm-missing-service-content-file.body")
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
}
