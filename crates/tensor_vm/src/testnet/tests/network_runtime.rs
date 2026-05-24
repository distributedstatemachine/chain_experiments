use super::*;

#[test]
fn public_operator_matching_rejects_missing_operator_candidates() {
    let mut address_to_operator = BTreeMap::new();
    let mut seen_addresses = BTreeSet::new();
    assert!(!match_public_operator_address(
        hash_bytes(b"test", &[b"missing-public-operator"]),
        &BTreeMap::new(),
        &mut address_to_operator,
        &mut seen_addresses,
    ));
    assert!(address_to_operator.is_empty());
    assert!(seen_addresses.is_empty());
}

#[test]
fn network_runtime_observation_helpers_reject_bad_roots_addresses_and_counts() {
    let root_a = hash_bytes(b"test", &[b"network-observation-a"]);
    assert!(
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &[]
        )
        .is_err()
    );
    assert!(
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &[[0; 32]]
        )
        .is_err()
    );
    assert!(
        aggregate_public_evidence_record_roots(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            &[root_a, root_a]
        )
        .is_err()
    );

    let public_ipv4: Multiaddr = "/ip4/8.8.8.8/tcp/4001".parse().unwrap();
    let private_ipv4: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
    let public_ipv6: Multiaddr = "/ip6/2606:4700:4700::1111/tcp/4001".parse().unwrap();
    let local_ipv6: Multiaddr = "/ip6/::1/tcp/4001".parse().unwrap();
    let special_dns: Multiaddr = "/dns/example.test/tcp/4001".parse().unwrap();
    let zero_tcp_port: Multiaddr = "/ip4/8.8.8.8/tcp/0".parse().unwrap();
    let ignored_protocol: Multiaddr = "/ip4/8.8.8.8/udp/4001/tcp/4001".parse().unwrap();
    assert!(public_network_runtime_multiaddr_is_external(&public_ipv4));
    assert!(!public_network_runtime_multiaddr_is_external(&private_ipv4));
    assert!(public_network_runtime_multiaddr_is_external(&public_ipv6));
    assert!(!public_network_runtime_multiaddr_is_external(&local_ipv6));
    assert!(!public_network_runtime_multiaddr_is_external(&special_dns));
    assert!(!public_network_runtime_multiaddr_is_external(
        &zero_tcp_port
    ));
    assert!(public_network_runtime_multiaddr_is_external(
        &ignored_protocol
    ));

    let mut bundle = complete_public_evidence_bundle();
    bundle.run.nodes[0].signed_heartbeat_count = 0;
    let criteria = PublicTestnetCriteria {
        min_miners: 2,
        min_validators: 1,
        duration_days: 0,
        min_finality_rate_bps: 9_000,
        min_data_availability_bps: 9_500,
        min_invalid_work_rejections: 1,
        min_reward_settlement_records: 1,
    };
    let (miner_operators, validator_operators) = bundle
        .run
        .matched_independent_public_operators_for_criteria(&criteria);
    assert!(
        !bundle.has_network_runtime_observation_records_for_public_operators(
            3,
            &miner_operators,
            &validator_operators
        )
    );
}
