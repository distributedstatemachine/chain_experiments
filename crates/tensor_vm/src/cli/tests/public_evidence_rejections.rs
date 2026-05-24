use super::*;

#[test]
fn execute_public_evidence_rejects_invalid_args() {
    let service_endpoint_id = manifest_hash(b"rpc-service");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "service",
            "health",
            "--kind",
            "archive",
            "--endpoint-id",
            &service_endpoint_id,
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
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "service",
            "health",
            "--kind",
            "rpc",
            "--endpoint-id",
            "12",
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
        .is_err()
    );
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    let record_root = manifest_hash(b"network-runtime-root");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .is_err()
    );
    let invalid_hash = "g".repeat(64);
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &invalid_hash,
            "--manifest-signer",
            &manifest_signer,
            "--record-root",
            &record_root,
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        execute_node_heartbeat(
            [0; 32],
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            10
        )
        .is_err()
    );
    assert!(execute_node_heartbeat(address(b"miner-a"), [0; 32], 0, 9, 10).is_err());
    assert!(
        execute_node_heartbeat(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            10,
            9,
            10,
        )
        .is_err()
    );
    assert!(
        execute_node_heartbeat(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            0,
            9,
            0,
        )
        .is_err()
    );
    let miner_address_hex = manifest_address(b"miner-a");
    let miner_operator_hex = manifest_hash(b"miner-a-operator");
    let heartbeat_summary = node_heartbeat_observation_summary_from_file(
            PublicNodeRole::Miner,
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            &format!(
                "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},1\n"
            ),
        )
        .unwrap();
    assert_eq!(heartbeat_summary.first_seen_block, 0);
    assert_eq!(heartbeat_summary.last_seen_block, 1);
    assert_eq!(heartbeat_summary.signed_heartbeat_count, 2);
    for invalid_heartbeat_observations in [
        "# no observations\n\n".to_owned(),
        format!(" node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!(
            "node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},0\nnode_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex},2\n"
        ),
        format!(
            "node_heartbeat_observation=validator,{miner_address_hex},{miner_operator_hex},0\n"
        ),
        format!("node_heartbeat_observation=observer,{miner_address_hex},{miner_operator_hex},0\n"),
        format!(
            "node_heartbeat_observation=miner,{},{} ,0\n",
            miner_address_hex, miner_operator_hex
        ),
        format!("node_heartbeat_observation=miner,{miner_address_hex},{miner_operator_hex}\n"),
        "service_health_observation=0,reachable\n".to_owned(),
    ] {
        assert!(
            node_heartbeat_observation_summary_from_file(
                PublicNodeRole::Miner,
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                &invalid_heartbeat_observations,
            )
            .is_err()
        );
    }
    assert!(
        execute_node_heartbeat_file(std::env::temp_dir().join(format!(
            "missing-tensor-vm-node-heartbeat-{}.records",
            std::process::id()
        )))
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            [0; 32],
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            [0; 32],
            "https://operators.tensorvm.net/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://localhost/miner-a",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_operator_attestation(
            address(b"miner-a"),
            hash_bytes(b"test", &[b"miner-a-operator"]),
            "https://operators.tensorvm.net/miner-a",
            0,
        )
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact",
            "--kind",
            "operator-identity",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-root",
            &manifest_hash(b"network-runtime-root"),
            "--record-count",
            "4",
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            "",
        ])
        .is_err()
    );
    let root_a = manifest_hash(b"network-observation-a");
    let root_b = manifest_hash(b"network-observation-b");
    let padded_roots = format!("{root_a}, {root_b}");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "summary-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--record-roots",
            &padded_roots,
        ])
        .is_err()
    );
    let empty_root_entry = format!("{root_a},,{root_b}");
    assert!(
        parse_test_cli(&[
            "public",
            "evidence",
            "record",
            "artifact-roots",
            "--kind",
            "network-runtime",
            "--bundle-id",
            &manifest_hash(b"public-evidence-bundle"),
            "--manifest-signer",
            &manifest_address(b"public-evidence-publisher"),
            "--artifact-uri",
            "https://evidence.tensorvm.net/network-runtime.json",
            "--record-roots",
            &empty_root_entry,
        ])
        .is_err()
    );
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let manifest_signer = address(b"public-evidence-publisher");
    let record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    let artifact_uri = "https://evidence.tensorvm.net/network-runtime.json";
    assert!(execute_record_summary(bundle_id, manifest_signer, record_root, 4).is_ok());
    assert!(
        execute_record_artifact(bundle_id, manifest_signer, artifact_uri, record_root, 4).is_ok()
    );
    assert!(execute_record_summary([0; 32], manifest_signer, record_root, 4).is_err());
    assert!(execute_record_summary(bundle_id, [0; 32], record_root, 4).is_err());
    assert!(execute_record_summary(bundle_id, manifest_signer, [0; 32], 4).is_err());
    assert!(execute_record_summary(bundle_id, manifest_signer, record_root, 0).is_err());
    assert!(
        execute_record_artifact([0; 32], manifest_signer, artifact_uri, record_root, 4).is_err()
    );
    assert!(execute_record_artifact(bundle_id, [0; 32], artifact_uri, record_root, 4).is_err());
    assert!(
        execute_record_artifact(
            bundle_id,
            manifest_signer,
            "https://localhost/network-runtime.json",
            record_root,
            4,
        )
        .is_err()
    );
    assert!(
        execute_record_artifact(
            bundle_id,
            manifest_signer,
            "https://evidence.tensorvm.net/",
            record_root,
            4,
        )
        .is_err()
    );
    assert!(execute_record_artifact(bundle_id, manifest_signer, artifact_uri, [0; 32], 4).is_err());
    assert!(
        execute_record_artifact(bundle_id, manifest_signer, artifact_uri, record_root, 0).is_err()
    );
    assert!(execute_record_summary_roots(Vec::new()).is_err());
    assert!(execute_record_summary_roots(vec![[0; 32]]).is_err());
    let duplicate_record_root = hash_bytes(b"test", &[b"network-runtime-root"]);
    assert!(
        execute_record_summary_roots(vec![duplicate_record_root, duplicate_record_root]).is_err()
    );
    assert!(execute_record_artifact_roots(Vec::new()).is_err());
    assert!(
        execute_record_artifact_roots(vec![duplicate_record_root, duplicate_record_root]).is_err()
    );
}

fn execute_node_heartbeat(
    address: [u8; 32],
    operator_id: [u8; 32],
    first_block: u64,
    last_block: u64,
    heartbeat_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::Heartbeat(
        NodeHeartbeatArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address),
            operator_id: hash_arg(operator_id),
            first_block,
            last_block,
            heartbeat_count,
        },
    )))
}

fn execute_node_heartbeat_file(heartbeat_file: std::path::PathBuf) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::HeartbeatFile(
        NodeHeartbeatFromFileArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address(b"miner-a")),
            operator_id: hash_arg(hash_bytes(b"test", &[b"miner-a-operator"])),
            heartbeat_file,
        },
    )))
}

fn execute_record_summary(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
    record_root: [u8; 32],
    record_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(EvidenceRecordCommand::Summary(
        RecordSummaryArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(bundle_id),
            manifest_signer: address_arg(manifest_signer),
            record_root: hash_arg(record_root),
            record_count,
        },
    )))
}

fn execute_record_artifact(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
    artifact_uri: &str,
    record_root: [u8; 32],
    record_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(EvidenceRecordCommand::Artifact(
        RecordArtifactArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(bundle_id),
            manifest_signer: address_arg(manifest_signer),
            artifact_uri: artifact_uri.to_owned(),
            record_root: hash_arg(record_root),
            record_count,
        },
    )))
}

fn execute_record_summary_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::SummaryRoots(RecordSummaryFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            record_roots: hash_args(record_roots),
        }),
    ))
}

fn execute_record_artifact_roots(record_roots: Vec<[u8; 32]>) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Record(
        EvidenceRecordCommand::ArtifactRoots(RecordArtifactFromRootsArgs {
            kind: record_kind_arg(PublicEvidenceRecordKind::NetworkRuntimeObservations),
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            artifact_uri: "https://evidence.tensorvm.net/network-runtime.json".to_owned(),
            record_roots: hash_args(record_roots),
        }),
    ))
}

fn execute_operator_attestation(
    address: [u8; 32],
    operator_id: [u8; 32],
    identity_uri: &str,
    observed_at: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(
        EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
            role: node_role_arg(PublicNodeRole::Miner),
            address: address_arg(address),
            operator_id: hash_arg(operator_id),
            identity_uri: identity_uri.to_owned(),
            observed_at,
        }),
    ))
}
