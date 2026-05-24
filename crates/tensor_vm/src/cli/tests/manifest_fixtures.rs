use super::*;
use crate::hash::hex;
use crate::testnet::{
    PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION, PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION,
    PublicEvidenceAuditorRecord, PublicEvidencePublication, PublicEvidenceRecordKind,
    PublicEvidenceRecordSummaries, PublicNetworkRuntimeEvidence, PublicNodeEvidence,
    PublicNodeRole, PublicOperatorIdentityAttestation, PublicServiceContentEvidence,
    PublicServiceEndpoint, PublicServiceEvidence, PublicServiceKind, PublicTestnetEvidenceBundle,
    PublicTestnetRunEvidence, aggregate_public_evidence_record_roots,
    public_network_runtime_observations_for_run,
};
use crate::types::{Hash, address, hash_bytes};

pub(super) fn manifest_hash(label: &[u8]) -> String {
    hex(&hash_bytes(b"test", &[label]))
}

pub(super) fn manifest_address(label: &[u8]) -> String {
    hex(&address(label))
}

pub(super) fn manifest_node_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let node = match role {
        PublicNodeRole::Miner => PublicNodeEvidence::miner(node_address, operator_id, 0, 9, 10),
        PublicNodeRole::Validator => {
            PublicNodeEvidence::validator(node_address, operator_id, 0, 9, 10)
        }
    };
    hex(&node.heartbeat_signature)
}

pub(super) fn manifest_operator_identity_uri(operator_id: &Hash) -> String {
    format!("https://operators.tensorvm.net/{}", hex(operator_id))
}

pub(super) fn manifest_operator_signature(
    role: PublicNodeRole,
    address_label: &[u8],
    operator_label: &[u8],
) -> String {
    let node_address = address(address_label);
    let operator_id = hash_bytes(b"test", &[operator_label]);
    let attestation = PublicOperatorIdentityAttestation::new(
        role,
        node_address,
        operator_id,
        manifest_operator_identity_uri(&operator_id),
        1_700_000_000,
    );
    hex(&attestation.operator_signature)
}

pub(super) fn public_service_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/health",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/health",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/health",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/health",
    }
}

pub(super) fn manifest_service_signature(kind: PublicServiceKind, label: &[u8]) -> String {
    let service = PublicServiceEvidence::new(
        kind,
        PublicServiceEndpoint::new(
            hash_bytes(b"test", &[label]),
            public_service_url(kind),
            "/health",
        ),
        0,
        9,
        10,
        10,
    );
    hex(&service.health_check_signature)
}

pub(super) fn public_service_content_url(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "https://rpc.tensorvm.net/chain/head",
        PublicServiceKind::Explorer => "https://explorer.tensorvm.net/explorer",
        PublicServiceKind::Faucet => "https://faucet.tensorvm.net/faucet/page",
        PublicServiceKind::Telemetry => "https://telemetry.tensorvm.net/telemetry/dashboard",
    }
}

pub(super) fn public_service_content_path(kind: PublicServiceKind) -> &'static str {
    match kind {
        PublicServiceKind::Rpc => "/chain/head",
        PublicServiceKind::Explorer => "/explorer",
        PublicServiceKind::Faucet => "/faucet/page",
        PublicServiceKind::Telemetry => "/telemetry/dashboard",
    }
}

pub(super) fn public_service_content(
    kind: PublicServiceKind,
    label: &[u8],
) -> PublicServiceContentEvidence {
    PublicServiceContentEvidence::new(
        kind,
        hash_bytes(b"test", &[label]),
        public_service_content_url(kind),
        public_service_content_path(kind),
        hash_bytes(b"test", &[label, b"content-root"]),
        1_700_000_000,
        64,
    )
}

pub(super) fn manifest_service_content_line(kind: PublicServiceKind, label: &[u8]) -> String {
    let content = public_service_content(kind, label);
    format!(
        "service_content={},{},{},{},{},{},{},{}",
        public_service_kind_tag(kind),
        hex(&content.endpoint_id),
        content.public_url,
        content.content_path,
        hex(&content.content_root),
        content.observed_at_unix_seconds,
        content.min_content_bytes,
        hex(&content.content_signature)
    )
}

pub(super) fn manifest_publication_signature() -> String {
    let publication = PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    );
    hex(&publication.manifest_signature)
}

pub(super) fn manifest_publication() -> PublicEvidencePublication {
    PublicEvidencePublication::new(
        hash_bytes(b"test", &[b"public-evidence-bundle"]),
        String::from("https://tensorvm.net/tensorvm/public-evidence.json"),
        address(b"public-evidence-publisher"),
        1,
        1,
    )
}

pub(super) fn manifest_auditor_uri() -> String {
    format!(
        "https://auditors.tensorvm.net/{}/0",
        manifest_hash(b"public-evidence-bundle")
    )
}

pub(super) fn manifest_auditor_signature() -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let record = PublicEvidenceAuditorRecord::new(
        &bundle_id,
        "https://tensorvm.net/tensorvm/public-evidence.json",
        address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        1_700_000_060,
    );
    hex(&record.auditor_signature)
}

pub(super) fn manifest_artifact_line(
    kind: PublicEvidenceRecordKind,
    root_label: &[u8],
    record_count: u64,
) -> String {
    manifest_artifact_line_for_root(kind, hash_bytes(b"test", &[root_label]), record_count)
}

pub(super) fn manifest_artifact_line_for_root(
    kind: PublicEvidenceRecordKind,
    record_root: Hash,
    record_count: u64,
) -> String {
    let bundle_id = hash_bytes(b"test", &[b"public-evidence-bundle"]);
    let artifact_uri = format!(
        "https://evidence.tensorvm.net/{}/{}.json",
        manifest_hash(b"public-evidence-bundle"),
        public_evidence_record_kind_tag(kind)
    );
    let signature = crate::testnet::sign_public_evidence_artifact(
        &address(b"public-evidence-publisher"),
        &bundle_id,
        kind,
        &artifact_uri,
        &record_root,
        record_count,
    );
    format!(
        "record_artifact={},{},{},{},{}",
        public_evidence_record_kind_tag(kind),
        artifact_uri,
        hex(&record_root),
        record_count,
        hex(&signature)
    )
}

pub(super) fn network_runtime_root_for_run(run: &PublicTestnetRunEvidence) -> Hash {
    let record_roots = public_network_runtime_observations_for_run(run)
        .iter()
        .map(|observation| observation.record_root)
        .collect::<Vec<_>>();
    aggregate_public_evidence_record_roots(
        PublicEvidenceRecordKind::NetworkRuntimeObservations,
        &record_roots,
    )
    .expect("generated network observation roots should aggregate")
}

pub(super) fn manifest_network_observation_lines() -> String {
    public_network_runtime_observations_for_run(&manifest_bundle().run)
        .iter()
        .map(|observation| {
            format!(
                "network_runtime_observation={},{},{},{},{},{},{},{},{},{},{},{},{}",
                hex(&observation.operator_id),
                observation.peer_id,
                observation.listen_address,
                observation.observed_at_unix_seconds,
                observation.gossip_topic_count,
                observation.request_response_protocol_count,
                observation.bootstrap_peer_count,
                observation.max_transmit_bytes,
                observation.request_timeout_seconds,
                observation.max_concurrent_streams,
                observation.idle_connection_timeout_seconds,
                hex(&observation.record_root),
                hex(&observation.observation_signature)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn manifest_bundle() -> PublicTestnetEvidenceBundle {
    let run = PublicTestnetRunEvidence {
        nodes: vec![
            PublicNodeEvidence::miner(
                address(b"miner-a"),
                hash_bytes(b"test", &[b"miner-a-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::miner(
                address(b"miner-b"),
                hash_bytes(b"test", &[b"miner-b-operator"]),
                0,
                9,
                10,
            ),
            PublicNodeEvidence::validator(
                address(b"validator-a"),
                hash_bytes(b"test", &[b"validator-a-operator"]),
                0,
                9,
                10,
            ),
        ],
        network_runtime: PublicNetworkRuntimeEvidence {
            libp2p_runtime_used: true,
            peer_discovery_observed: true,
            gossip_propagation_observed: true,
            request_response_observed: true,
            dos_controls_enabled: true,
        },
        services: vec![
            PublicServiceEvidence::new(
                PublicServiceKind::Rpc,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"rpc-service"]),
                    public_service_url(PublicServiceKind::Rpc),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Explorer,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"explorer-service"]),
                    public_service_url(PublicServiceKind::Explorer),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Faucet,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"faucet-service"]),
                    public_service_url(PublicServiceKind::Faucet),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
            PublicServiceEvidence::new(
                PublicServiceKind::Telemetry,
                PublicServiceEndpoint::new(
                    hash_bytes(b"test", &[b"telemetry-service"]),
                    public_service_url(PublicServiceKind::Telemetry),
                    "/health",
                ),
                0,
                9,
                10,
                10,
            ),
        ],
        service_content: vec![
            public_service_content(PublicServiceKind::Rpc, b"rpc-service"),
            public_service_content(PublicServiceKind::Explorer, b"explorer-service"),
            public_service_content(PublicServiceKind::Faucet, b"faucet-service"),
            public_service_content(PublicServiceKind::Telemetry, b"telemetry-service"),
        ],
        run_started_at_unix_seconds: 1_700_000_000,
        run_ended_at_unix_seconds: 1_700_000_060,
        observed_blocks: 10,
        finalized_blocks: 10,
        checked_receipts: 20,
        available_receipts: 19,
        invalid_receipts_submitted: 1,
        invalid_receipts_rejected: 1,
        reward_settlement_records: 1,
    };
    let network_runtime_observation_root = network_runtime_root_for_run(&run);
    PublicTestnetEvidenceBundle::new(
        run,
        manifest_publication(),
        PublicEvidenceRecordSummaries {
            block_history_records: 10,
            block_history_root: hash_bytes(b"test", &[b"block-history-root"]),
            finality_history_records: 10,
            finality_history_root: hash_bytes(b"test", &[b"finality-history-root"]),
            operator_identity_attestation_records: 3,
            network_runtime_observation_records: 3,
            network_runtime_observation_root,
            data_availability_measurement_records: 20,
            data_availability_measurement_root: hash_bytes(b"test", &[b"data-availability-root"]),
            invalid_work_rejection_records: 1,
            invalid_work_rejection_root: hash_bytes(b"test", &[b"invalid-work-root"]),
            reward_settlement_root: hash_bytes(b"test", &[b"reward-settlement-root"]),
        },
    )
}

pub(super) fn evidence_manifest() -> String {
    format!(
        "\
version={PUBLIC_TESTNET_EVIDENCE_MANIFEST_VERSION}
bundle_id={}
public_uri=https://tensorvm.net/tensorvm/public-evidence.json
manifest_signer={}
manifest_signature={}
manifest_signature_count=1
independent_auditor_count=1
auditor={},{},1700000060,{}
{}
{}
{}
{}
{}
{}
block_history_records=10
block_history_root={}
block_history_signature={}
finality_history_records=10
finality_history_root={}
finality_history_signature={}
operator_identity_attestation_records=3
operator=miner,{},{},{},1700000000,{}
operator=miner,{},{},{},1700000000,{}
operator=validator,{},{},{},1700000000,{}
{}
network_runtime_observation_records=3
network_runtime_observation_root={}
network_runtime_observation_signature={}
data_availability_measurement_records=20
data_availability_measurement_root={}
data_availability_measurement_signature={}
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
run_started_at_unix_seconds=1700000000
run_ended_at_unix_seconds=1700000060
run_window_signature={}
observed_blocks=10
finalized_blocks=10
checked_receipts=20
available_receipts=19
invalid_receipts_submitted=1
invalid_receipts_rejected=1
invalid_work_rejection_records=1
invalid_work_rejection_root={}
invalid_work_rejection_signature={}
reward_settlement_records=1
reward_settlement_root={}
reward_settlement_signature={}
node=miner,{},{},0,9,10,{}
node=miner,{},{},0,9,10,{}
node=validator,{},{},0,9,10,{}
service=rpc,{},https://rpc.tensorvm.net/health,/health,0,9,10,10,{}
service=explorer,{},https://explorer.tensorvm.net/health,/health,0,9,10,10,{}
service=faucet,{},https://faucet.tensorvm.net/health,/health,0,9,10,10,{}
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,0,9,10,10,{}
{}
{}
{}
{}
",
        manifest_hash(b"public-evidence-bundle"),
        manifest_address(b"public-evidence-publisher"),
        manifest_publication_signature(),
        manifest_address(b"public-evidence-auditor-0"),
        manifest_auditor_uri(),
        manifest_auditor_signature(),
        manifest_artifact_line(
            PublicEvidenceRecordKind::BlockHistory,
            b"block-history-root",
            10
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::FinalityHistory,
            b"finality-history-root",
            10
        ),
        manifest_artifact_line_for_root(
            PublicEvidenceRecordKind::NetworkRuntimeObservations,
            manifest_bundle().network_runtime_observation_root,
            3
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::DataAvailabilityMeasurements,
            b"data-availability-root",
            20
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::InvalidWorkRejections,
            b"invalid-work-root",
            1
        ),
        manifest_artifact_line(
            PublicEvidenceRecordKind::RewardSettlements,
            b"reward-settlement-root",
            1
        ),
        manifest_hash(b"block-history-root"),
        hex(&manifest_bundle().block_history_signature),
        manifest_hash(b"finality-history-root"),
        hex(&manifest_bundle().finality_history_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-a-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"miner-b-operator"])),
        manifest_operator_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_operator_identity_uri(&hash_bytes(b"test", &[b"validator-a-operator"])),
        manifest_operator_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_network_observation_lines(),
        hex(&manifest_bundle().network_runtime_observation_root),
        hex(&manifest_bundle().network_runtime_observation_signature),
        manifest_hash(b"data-availability-root"),
        hex(&manifest_bundle().data_availability_measurement_signature),
        hex(&manifest_bundle().run_window_signature),
        manifest_hash(b"invalid-work-root"),
        hex(&manifest_bundle().invalid_work_rejection_signature),
        manifest_hash(b"reward-settlement-root"),
        hex(&manifest_bundle().reward_settlement_signature),
        manifest_address(b"miner-a"),
        manifest_hash(b"miner-a-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-a", b"miner-a-operator"),
        manifest_address(b"miner-b"),
        manifest_hash(b"miner-b-operator"),
        manifest_node_signature(PublicNodeRole::Miner, b"miner-b", b"miner-b-operator"),
        manifest_address(b"validator-a"),
        manifest_hash(b"validator-a-operator"),
        manifest_node_signature(
            PublicNodeRole::Validator,
            b"validator-a",
            b"validator-a-operator"
        ),
        manifest_hash(b"rpc-service"),
        manifest_service_signature(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_hash(b"explorer-service"),
        manifest_service_signature(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_hash(b"faucet-service"),
        manifest_service_signature(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_hash(b"telemetry-service"),
        manifest_service_signature(PublicServiceKind::Telemetry, b"telemetry-service"),
        manifest_service_content_line(PublicServiceKind::Rpc, b"rpc-service"),
        manifest_service_content_line(PublicServiceKind::Explorer, b"explorer-service"),
        manifest_service_content_line(PublicServiceKind::Faucet, b"faucet-service"),
        manifest_service_content_line(PublicServiceKind::Telemetry, b"telemetry-service"),
    )
}

pub(super) fn preflight_manifest() -> String {
    format!(
            "\
version={PUBLIC_TESTNET_PREFLIGHT_MANIFEST_VERSION}
miner_count=10
validator_count=5
miner_stake=100
validator_stake=10000
faucet_balance=1000000
faucet_drip=100
cuda_kernels_available=true
cuda_ready_miner_count=10
libp2p_ready_node_count=15
libp2p_runtime_used=true
peer_discovery_observed=true
gossip_propagation_observed=true
request_response_observed=true
dos_controls_enabled=true
service=rpc,{},https://rpc.tensorvm.net/health,/health,https://rpc.tensorvm.net/chain/head,/chain/head,true,true
service=explorer,{},https://explorer.tensorvm.net/health,/health,https://explorer.tensorvm.net/explorer,/explorer,true,true
service=faucet,{},https://faucet.tensorvm.net/health,/health,https://faucet.tensorvm.net/faucet/page,/faucet/page,true,true
service=telemetry,{},https://telemetry.tensorvm.net/health,/health,https://telemetry.tensorvm.net/telemetry/dashboard,/telemetry/dashboard,true,true
",
            manifest_hash(b"rpc-service"),
            manifest_hash(b"explorer-service"),
            manifest_hash(b"faucet-service"),
            manifest_hash(b"telemetry-service"),
        )
}
