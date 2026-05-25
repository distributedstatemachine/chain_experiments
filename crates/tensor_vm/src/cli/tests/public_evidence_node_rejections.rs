use super::*;

#[test]
fn execute_public_node_evidence_rejects_invalid_args() {
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
}

fn execute_node_heartbeat(
    address: [u8; 32],
    operator_id: [u8; 32],
    first_block: u64,
    last_block: u64,
    heartbeat_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::Heartbeat(
        NodeHeartbeatArgs::new(
            public_node_identity_args(address, operator_id),
            block_height_window_args(first_block, last_block),
            heartbeat_count,
        ),
    )))
}

fn execute_node_heartbeat_file(heartbeat_file: std::path::PathBuf) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(EvidenceNodeCommand::HeartbeatFile(
        NodeHeartbeatFromFileArgs::new(
            public_node_identity_args(
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
            ),
            heartbeat_file,
        ),
    )))
}

fn execute_operator_attestation(
    address: [u8; 32],
    operator_id: [u8; 32],
    identity_uri: &str,
    observed_at: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Node(
        EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs::new(
            public_node_identity_args(address, operator_id),
            identity_uri,
            observation_timestamp_args(observed_at),
        )),
    ))
}

fn public_node_identity_args(address: [u8; 32], operator_id: [u8; 32]) -> PublicNodeIdentityArgs {
    PublicNodeIdentityArgs::new(
        node_role_arg(PublicNodeRole::Miner),
        address_arg(address),
        operator_id_args(operator_id),
    )
}
