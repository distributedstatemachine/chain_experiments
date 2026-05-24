use super::super::{validate_public_evidence_manifest, validate_public_testnet_preflight_manifest};
use super::{evidence_manifest, preflight_manifest};

#[test]
fn validate_public_evidence_manifest_reports_default_criteria_status() {
    let report = validate_public_evidence_manifest(&evidence_manifest()).unwrap();
    assert!(report.contains("public_evidence_full_spec=false"));
    assert!(report.contains("public_criterion=false"));
    assert!(report.contains("independently_checkable=true"));
    assert!(report.contains("published_evidence_bundle=true"));
    assert!(report.contains("independent_auditor_records=true"));
    assert!(report.contains("signed_run_window=true"));
    assert!(report.contains("block_history=true"));
    assert!(report.contains("finality_history=true"));
    assert!(report.contains("operator_identity_attestations=true"));
    assert!(report.contains("network_runtime_observations=true"));
    assert!(report.contains("data_availability_measurements=true"));
    assert!(report.contains("signed_invalid_work_rejection_records=true"));
    assert!(report.contains("signed_reward_settlement_records=true"));
    assert!(report.contains("supporting_record_artifacts=true"));
    assert!(report.contains("miners=2"));
    assert!(report.contains("validators=1"));
    assert!(report.contains("run_started_at_unix_seconds=1700000000"));
    assert!(report.contains("run_ended_at_unix_seconds=1700000060"));
    assert!(report.contains("observed_duration_seconds=60"));
    assert!(report.contains("required_duration_seconds=604800"));
    assert!(report.contains("observed_blocks=10"));
    assert!(report.contains("required_blocks=100800"));
    assert!(report.contains("finality_rate_bps=10000"));
    assert!(report.contains("data_availability_bps=9500"));
    assert!(report.contains("invalid_receipts_submitted=1"));
    assert!(report.contains("invalid_receipts_rejected=1"));
    assert!(report.contains("invalid_work_rejection_rate_bps=10000"));
    assert!(report.contains("reward_settlement_records=1"));
    assert!(report.contains("external_operator_evidence=true"));
    assert!(report.contains("required_miners=false"));
    assert!(report.contains("required_validators=false"));
    assert!(report.contains("required_run_duration=false"));
    assert!(report.contains("required_block_count=false"));
    assert!(report.contains("required_finality=true"));
    assert!(report.contains("required_data_availability=true"));
    assert!(report.contains("invalid_work_rejection_evidence=true"));
    assert!(report.contains("reward_settlement_evidence=true"));
    assert!(report.contains("production_libp2p_runtime=true"));
    assert!(report.contains("deployed_rpc_service=true"));
    assert!(report.contains("deployed_explorer_service=true"));
    assert!(report.contains("deployed_faucet_service=true"));
    assert!(report.contains("deployed_telemetry_service=true"));
    assert!(report.contains("deployed_public_service_content=true"));
    assert!(report.contains("deployed_public_services=true"));

    let insufficient_operator_records = evidence_manifest().replace(
        "operator_identity_attestation_records=3",
        "operator_identity_attestation_records=2",
    );
    let insufficient_operator_report =
        validate_public_evidence_manifest(&insufficient_operator_records).unwrap();
    assert!(insufficient_operator_report.contains("operator_identity_attestations=false"));
    assert!(insufficient_operator_report.contains("external_operator_evidence=false"));
    assert!(insufficient_operator_report.contains("public_criterion=false"));

    let missing_auditor_records = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("auditor="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_auditor_report =
        validate_public_evidence_manifest(&missing_auditor_records).unwrap();
    assert!(missing_auditor_report.contains("published_evidence_bundle=true"));
    assert!(missing_auditor_report.contains("independent_auditor_records=false"));
    assert!(missing_auditor_report.contains("independently_checkable=false"));

    let missing_artifacts = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("record_artifact="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_artifacts_report = validate_public_evidence_manifest(&missing_artifacts).unwrap();
    assert!(missing_artifacts_report.contains("supporting_record_artifacts=false"));
    assert!(missing_artifacts_report.contains("independently_checkable=false"));

    let missing_service_content = evidence_manifest()
        .lines()
        .filter(|line| !line.starts_with("service_content="))
        .collect::<Vec<_>>()
        .join("\n");
    let missing_service_content_report =
        validate_public_evidence_manifest(&missing_service_content).unwrap();
    assert!(missing_service_content_report.contains("deployed_public_service_content=false"));
    assert!(missing_service_content_report.contains("deployed_public_services=false"));

    assert!(validate_public_evidence_manifest("bad-manifest").is_err());
}

#[test]
fn validate_public_testnet_preflight_manifest_reports_launch_readiness() {
    let report = validate_public_testnet_preflight_manifest(&preflight_manifest()).unwrap();
    assert!(report.contains("public_testnet_preflight_ready=true"));
    assert!(report.contains("local_shape_ready=true"));
    assert!(report.contains("deployment_plan_ready=true"));
    assert!(report.contains("miners=10"));
    assert!(report.contains("validators=5"));
    assert!(report.contains("required_blocks=100800"));
    assert!(report.contains("required_miners=true"));
    assert!(report.contains("required_validators=true"));
    assert!(report.contains("positive_stakes=true"));
    assert!(report.contains("funded_faucet=true"));
    assert!(report.contains("cuda_kernels_available=true"));
    assert!(report.contains("cuda_ready_miner_count=10"));
    assert!(report.contains("cuda_ready_miners=true"));
    assert!(report.contains("libp2p_ready_node_count=15"));
    assert!(report.contains("libp2p_ready_nodes=true"));
    assert!(report.contains("production_libp2p_runtime=true"));
    assert!(report.contains("rpc_service_plan=true"));
    assert!(report.contains("explorer_service_plan=true"));
    assert!(report.contains("faucet_service_plan=true"));
    assert!(report.contains("telemetry_service_plan=true"));
    assert!(report.contains("public_service_content_planned=true"));
    assert!(report.contains("public_services_planned=true"));

    assert!(validate_public_testnet_preflight_manifest("bad-manifest").is_err());
}
