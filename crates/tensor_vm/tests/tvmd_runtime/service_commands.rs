use super::*;
use tensor_vm::{
    app::{execute_tvmd_command, init_service_store},
    cli::{
        EvidenceCommand, PublicCommand, PublicEvidenceManifestArgs, PublicTestnetManifestArgs,
        TvmdCommand,
    },
};

fn workspace_manifest_path(relative_path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn docs_public_testnet_preflight_command_reports_pending_status() {
    let report = execute_tvmd_command(&TvmdCommand::Public(PublicCommand::Preflight(
        PublicTestnetManifestArgs::new(
            workspace_manifest_path("docs/tensorvm/public-testnet.preflight").into(),
        ),
    )))
    .unwrap();

    assert_eq!(
        report_field(&report, "public_testnet_preflight_ready"),
        "false"
    );
    assert_eq!(report_field(&report, "local_shape_ready"), "true");
    assert_eq!(report_field(&report, "deployment_plan_ready"), "false");
    assert_eq!(report_u64(&report, "miners"), 10);
    assert_eq!(report_u64(&report, "validators"), 5);
    assert_eq!(report_field(&report, "production_libp2p_runtime"), "true");
    assert_eq!(report_field(&report, "public_services_planned"), "false");
}

#[test]
fn docs_public_testnet_evidence_command_reports_non_full_spec_status() {
    let report = execute_tvmd_command(&TvmdCommand::Public(PublicCommand::Evidence(
        EvidenceCommand::Validate(PublicEvidenceManifestArgs::new(
            workspace_manifest_path("docs/tensorvm/public-testnet.evidence").into(),
        )),
    )))
    .unwrap();

    assert_eq!(report_field(&report, "public_evidence_full_spec"), "false");
    assert_eq!(report_field(&report, "public_criterion"), "false");
    assert_eq!(report_field(&report, "independently_checkable"), "false");
    assert_eq!(report_field(&report, "published_evidence_bundle"), "false");
    assert_eq!(report_field(&report, "signed_run_window"), "true");
    assert_eq!(
        report_field(&report, "supporting_record_artifacts"),
        "false"
    );
    assert_eq!(
        report_field(&report, "deployed_public_service_content"),
        "false"
    );
    assert_eq!(report_field(&report, "required_run_duration"), "false");
    assert_eq!(report_field(&report, "required_block_count"), "false");
}

#[test]
fn service_init_recovers_torn_snapshot_and_block_log_from_chain_state() {
    let data_dir = std::env::temp_dir().join(format!(
        "tensor-vm-service-init-recovery-{}",
        std::process::id()
    ));
    let data_dir_text = data_dir.to_string_lossy().into_owned();
    let store = NodeStore::open(data_dir.clone());
    let mut chain = Chain::new(hash_bytes(b"test", &[b"service-init-recovery"]));
    let miner = address(b"service-init-recovery-miner");
    register_miner(&mut chain, miner);
    register_validator(&mut chain, miner);
    produce_block(&mut chain, miner, 1_000);
    produce_block(&mut chain, miner, 1_006);
    store.persist_chain(&chain).unwrap();

    let mut stale_snapshot = ChainSnapshot::from_chain(&chain);
    stale_snapshot.block_count = stale_snapshot.block_count.saturating_sub(1);
    store.snapshot_store().save(&stale_snapshot).unwrap();

    let report = init_service_store(&data_dir_text).unwrap();
    assert_eq!(report_field(&report, "command"), "service_init");
    assert_eq!(report_field(&report, "existing_store"), "true");
    assert_eq!(report_field(&report, "recovered_store"), "true");
    assert_eq!(report_field(&report, "recovery_source"), "chain_state");
    assert_eq!(report_u64(&report, "block_count"), 2);
    assert_eq!(store.load_chain().unwrap(), chain);

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
