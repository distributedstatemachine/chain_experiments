use super::*;

#[test]
fn execute_node_evidence_reports_outputs() {
    let node_cases = [
        (
            PublicNodeRole::Miner,
            b"miner-a".as_slice(),
            b"miner-a-operator".as_slice(),
            "miner",
        ),
        (
            PublicNodeRole::Validator,
            b"validator-a".as_slice(),
            b"validator-a-operator".as_slice(),
            "validator",
        ),
    ];
    for (role, address_label, operator_label, tag) in node_cases {
        let node = execute_public_evidence_command(&EvidenceCommand::Node(
            EvidenceNodeCommand::Heartbeat(NodeHeartbeatArgs {
                node: public_node_identity_args(role, address_label, operator_label),
                window: block_height_window_args(0, 9),
                heartbeat_count: 10,
            }),
        ))
        .unwrap();
        let node_address = hex(&address(address_label));
        let operator_id = hex(&hash_bytes(b"test", &[operator_label]));
        let node_signature = manifest_node_signature(role, address_label, operator_label);
        assert_eq!(
            comma_record_fields(&node, "node=", 7),
            [
                tag,
                node_address.as_str(),
                operator_id.as_str(),
                "0",
                "9",
                "10",
                node_signature.as_str(),
            ]
        );
        let heartbeat_file = std::env::temp_dir().join(format!(
            "tensor-vm-node-heartbeat-{}-{}.records",
            std::process::id(),
            tag
        ));
        let heartbeat_records = (0..10)
            .map(|block| {
                format!(
                    "node_heartbeat_observation={tag},{},{},{}",
                    node_address, operator_id, block
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&heartbeat_file, heartbeat_records).unwrap();
        let node_from_file = execute_public_evidence_command(&EvidenceCommand::Node(
            EvidenceNodeCommand::HeartbeatFile(NodeHeartbeatFromFileArgs {
                node: public_node_identity_args(role, address_label, operator_label),
                heartbeat_file: heartbeat_file.clone(),
            }),
        ))
        .unwrap();
        std::fs::remove_file(&heartbeat_file).unwrap();
        assert_eq!(node_from_file, node);
    }

    let operator_id = hash_bytes(b"test", &[b"miner-a-operator"]);
    let operator_identity_uri = manifest_operator_identity_uri(&operator_id);
    let operator_attestation = execute_public_evidence_command(&EvidenceCommand::Node(
        EvidenceNodeCommand::OperatorAttestation(OperatorAttestationArgs {
            node: public_node_identity_args(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
            identity_uri: operator_identity_uri.clone(),
            observed_at: 1_700_000_000,
        }),
    ))
    .unwrap();
    assert_eq!(
        operator_attestation,
        format!(
            "operator=miner,{},{},{operator_identity_uri},1700000000,{}",
            manifest_address(b"miner-a"),
            manifest_hash(b"miner-a-operator"),
            manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator")
        )
    );
}

fn public_node_identity_args(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> PublicNodeIdentityArgs {
    PublicNodeIdentityArgs {
        role: node_role_arg(role),
        address: address_arg(address(address_label)),
        operator_id: hash_arg(hash_bytes(b"test", &[operator_label])),
    }
}
