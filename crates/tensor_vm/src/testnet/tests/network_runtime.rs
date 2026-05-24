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
    let public_ipv4_without_tcp: Multiaddr = "/ip4/8.8.8.8".parse().unwrap();
    let private_ipv4: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
    let public_ipv6: Multiaddr = "/ip6/2606:4700:4700::1111/tcp/4001".parse().unwrap();
    let local_ipv6: Multiaddr = "/ip6/::1/tcp/4001".parse().unwrap();
    let special_dns: Multiaddr = "/dns/example.test/tcp/4001".parse().unwrap();
    let zero_tcp_port: Multiaddr = "/ip4/8.8.8.8/tcp/0".parse().unwrap();
    let ignored_protocol: Multiaddr = "/ip4/8.8.8.8/udp/4001/tcp/4001".parse().unwrap();
    assert!(public_network_runtime_multiaddr_is_external(&public_ipv4));
    assert!(!public_network_runtime_multiaddr_is_external(
        &public_ipv4_without_tcp
    ));
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
    assert!(public_network_runtime_multiaddr_is_external(
        &"/dns/node.tensorvm.net/tcp/4001".parse().unwrap()
    ));
    for address in [
        "/ip4/0.0.0.0/tcp/4001",
        "/ip4/10.0.0.1/tcp/4001",
        "/ip4/100.64.0.1/tcp/4001",
        "/ip4/192.0.0.1/tcp/4001",
        "/ip4/192.0.2.10/tcp/4001",
        "/ip4/198.18.0.1/tcp/4001",
        "/ip4/198.51.100.10/tcp/4001",
        "/ip4/203.0.113.10/tcp/4001",
        "/ip4/224.0.0.1/tcp/4001",
        "/ip4/240.0.0.1/tcp/4001",
        "/ip4/255.255.255.255/tcp/4001",
        "/ip6/fc00::1/tcp/4001",
        "/ip6/fe80::1/tcp/4001",
        "/ip6/2001:db8::1/tcp/4001",
        "/ip6/ff02::1/tcp/4001",
        "/dns/node/tcp/4001",
        "/dns/bad_host.tensorvm.net/tcp/4001",
        "/dns/-bad.tensorvm.net/tcp/4001",
        "/dns/bad-.tensorvm.net/tcp/4001",
        "/dns/bad..tensorvm.net/tcp/4001",
        "/dns/node.tensorvm.example/tcp/4001",
        "/dns/example.com/tcp/4001",
        "/dns/123.456/tcp/4001",
        "/dns/localhost/tcp/4001",
        "/dns/node.local/tcp/4001",
        "/dns/203.0.113.10/tcp/4001",
        "/dns4/10.0.0.1/tcp/4001",
    ] {
        assert!(!public_network_runtime_multiaddr_is_external(
            &address.parse().unwrap()
        ));
    }
    assert!(!public_host_is_external("2001:db8::1"));
    assert!(public_host_is_external("2001:4860:4860::8888"));

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
