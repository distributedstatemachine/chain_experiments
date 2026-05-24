use super::*;
use tensor_vm::{
    app::{execute_tvmd_command, init_service_store},
    cli::{
        EvidenceCommand, PublicEvidenceManifestArgs, PublicTestnetManifestArgs, TestnetCommand,
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
    let report = execute_tvmd_command(&TvmdCommand::Testnet(TestnetCommand::Preflight(
        PublicTestnetManifestArgs {
            manifest: workspace_manifest_path("docs/tensorvm/public-testnet.preflight").into(),
        },
    )))
    .unwrap();

    assert!(report.contains("public_testnet_preflight_ready=false"));
    assert!(report.contains("local_shape_ready=true"));
    assert!(report.contains("deployment_plan_ready=false"));
    assert!(report.contains("miners=10"));
    assert!(report.contains("validators=5"));
    assert!(report.contains("production_libp2p_runtime=true"));
    assert!(report.contains("public_services_planned=false"));
}

#[test]
fn docs_public_testnet_evidence_command_reports_non_full_spec_status() {
    let report = execute_tvmd_command(&TvmdCommand::Evidence(EvidenceCommand::Validate(
        PublicEvidenceManifestArgs {
            manifest: workspace_manifest_path("docs/tensorvm/public-testnet.evidence").into(),
        },
    )))
    .unwrap();

    assert!(report.contains("public_evidence_full_spec=false"));
    assert!(report.contains("public_criterion=false"));
    assert!(report.contains("independently_checkable=false"));
    assert!(report.contains("published_evidence_bundle=false"));
    assert!(report.contains("signed_run_window=true"));
    assert!(report.contains("supporting_record_artifacts=false"));
    assert!(report.contains("deployed_public_service_content=false"));
    assert!(report.contains("required_run_duration=false"));
    assert!(report.contains("required_block_count=false"));
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
    chain.produce_block(miner, 1_000).unwrap();
    chain.produce_block(miner, 1_006).unwrap();
    store.persist_chain(&chain).unwrap();

    let mut stale_snapshot = ChainSnapshot::from_chain(&chain);
    stale_snapshot.block_count = stale_snapshot.block_count.saturating_sub(1);
    store.snapshot_store().save(&stale_snapshot).unwrap();

    let report = init_service_store(&data_dir_text).unwrap();
    assert!(report.contains("command=service_init"));
    assert!(report.contains("existing_store=true"));
    assert!(report.contains("recovered_store=true"));
    assert!(report.contains("recovery_source=chain_state"));
    assert!(report.contains("block_count=2"));
    assert_eq!(store.load_chain().unwrap(), chain);

    std::fs::remove_dir_all(data_dir).expect("test dir must be removed");
}
