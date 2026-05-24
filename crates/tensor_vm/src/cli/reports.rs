use crate::chain::ChainParams;
use crate::error::Result;
use crate::testnet::{
    PublicTestnetCriteria, parse_public_testnet_evidence_manifest,
    parse_public_testnet_preflight_manifest,
};

pub fn validate_public_evidence_manifest(input: &str) -> Result<String> {
    let bundle = parse_public_testnet_evidence_manifest(input)?;
    let report = bundle.evaluate(
        &PublicTestnetCriteria::default(),
        ChainParams::default().block_time_seconds,
    );
    Ok(format!(
        "public_evidence_full_spec={}\npublic_criterion={}\nindependently_checkable={}\npublished_evidence_bundle={}\nindependent_auditor_records={}\nsigned_run_window={}\nblock_history={}\nfinality_history={}\noperator_identity_attestations={}\nnetwork_runtime_observations={}\ndata_availability_measurements={}\nsigned_invalid_work_rejection_records={}\nsigned_reward_settlement_records={}\nsupporting_record_artifacts={}\nminers={}\nvalidators={}\nrun_started_at_unix_seconds={}\nrun_ended_at_unix_seconds={}\nobserved_duration_seconds={}\nrequired_duration_seconds={}\nobserved_blocks={}\nrequired_blocks={}\nfinality_rate_bps={}\ndata_availability_bps={}\ninvalid_receipts_submitted={}\ninvalid_receipts_rejected={}\ninvalid_work_rejection_rate_bps={}\nreward_settlement_records={}\nexternal_operator_evidence={}\nrequired_miners={}\nrequired_validators={}\nrequired_run_duration={}\nrequired_block_count={}\nrequired_finality={}\nrequired_data_availability={}\ninvalid_work_rejection_evidence={}\nreward_settlement_evidence={}\nproduction_libp2p_runtime={}\ndeployed_rpc_service={}\ndeployed_explorer_service={}\ndeployed_faucet_service={}\ndeployed_telemetry_service={}\ndeployed_public_service_content={}\ndeployed_public_services={}",
        report.full_spec_evidence_met,
        report.run_evidence.public_criterion_met,
        report.independently_checkable,
        report.has_published_evidence_bundle,
        report.has_independent_auditor_records,
        report.has_signed_run_window,
        report.has_block_history,
        report.has_finality_history,
        report.has_operator_identity_attestations,
        report.has_network_runtime_observations,
        report.has_data_availability_measurements,
        report.has_invalid_work_rejection_records,
        report.has_reward_settlement_record_summary,
        report.has_public_supporting_record_artifacts,
        report.run_evidence.miner_count,
        report.run_evidence.validator_count,
        report.run_evidence.run_started_at_unix_seconds,
        report.run_evidence.run_ended_at_unix_seconds,
        report.run_evidence.observed_duration_seconds,
        report.run_evidence.required_duration_seconds,
        report.run_evidence.observed_blocks,
        report.run_evidence.required_blocks,
        report.run_evidence.finality_rate_bps,
        report.run_evidence.data_availability_bps,
        report.run_evidence.invalid_receipts_submitted,
        report.run_evidence.invalid_receipts_rejected,
        report.run_evidence.invalid_work_rejection_rate_bps,
        report.run_evidence.reward_settlement_records,
        report.run_evidence.external_operator_evidence,
        report.run_evidence.has_required_miners,
        report.run_evidence.has_required_validators,
        report.run_evidence.has_required_run_duration,
        report.run_evidence.has_required_block_count,
        report.run_evidence.has_required_finality,
        report.run_evidence.has_required_data_availability,
        report.run_evidence.has_invalid_work_rejection_evidence,
        report.run_evidence.has_reward_settlement_records,
        report.run_evidence.has_production_libp2p_runtime,
        report.run_evidence.has_deployed_rpc_service,
        report.run_evidence.has_deployed_explorer_service,
        report.run_evidence.has_deployed_faucet_service,
        report.run_evidence.has_deployed_telemetry_service,
        report.run_evidence.has_deployed_public_service_content,
        report.run_evidence.has_deployed_public_services,
    ))
}

pub fn validate_public_testnet_preflight_manifest(input: &str) -> Result<String> {
    let plan = parse_public_testnet_preflight_manifest(input)?;
    let report = plan.evaluate(ChainParams::default().block_time_seconds);
    Ok(format!(
        "public_testnet_preflight_ready={}\nlocal_shape_ready={}\ndeployment_plan_ready={}\nminers={}\nvalidators={}\nrequired_blocks={}\nrequired_miners={}\nrequired_validators={}\npositive_stakes={}\nfunded_faucet={}\ncuda_kernels_available={}\ncuda_ready_miner_count={}\ncuda_ready_miners={}\nlibp2p_ready_node_count={}\nlibp2p_ready_nodes={}\nproduction_libp2p_runtime={}\nrpc_service_plan={}\nexplorer_service_plan={}\nfaucet_service_plan={}\ntelemetry_service_plan={}\npublic_service_content_planned={}\npublic_services_planned={}",
        report.can_start_public_run,
        report.local_shape_ready,
        report.deployment_plan_ready,
        report.miner_count,
        report.validator_count,
        report.required_blocks,
        report.has_required_miners,
        report.has_required_validators,
        report.has_positive_stakes,
        report.has_funded_faucet,
        report.has_cuda_kernels_available,
        report.cuda_ready_miner_count,
        report.has_cuda_ready_miners,
        report.libp2p_ready_node_count,
        report.has_libp2p_ready_nodes,
        report.has_production_libp2p_runtime,
        report.has_rpc_service_plan,
        report.has_explorer_service_plan,
        report.has_faucet_service_plan,
        report.has_telemetry_service_plan,
        report.has_public_service_content_plan,
        report.has_public_service_plan,
    ))
}
