use std::path::Path;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_tvmd(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_tvmd"))
        .args(args)
        .current_dir(workspace_root())
        .output()
        .expect("tvmd command must execute");

    assert!(
        output.status.success(),
        "tvmd failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("tvmd stdout must be utf8")
}

#[test]
fn documented_public_testnet_preflight_command_reports_pending_status() {
    let stdout = run_tvmd(&[
        "public-testnet",
        "preflight",
        "--manifest",
        "docs/tensorvm/public-testnet.preflight",
    ]);

    assert!(stdout.contains("public_testnet_preflight_ready=false"));
    assert!(stdout.contains("local_shape_ready=true"));
    assert!(stdout.contains("deployment_plan_ready=false"));
    assert!(stdout.contains("production_libp2p_runtime=true"));
    assert!(stdout.contains("public_services_planned=false"));
}

#[test]
fn documented_public_testnet_evidence_command_reports_non_full_spec_status() {
    let stdout = run_tvmd(&[
        "public-evidence",
        "validate",
        "--manifest",
        "docs/tensorvm/public-testnet.evidence",
    ]);

    assert!(stdout.contains("public_evidence_full_spec=false"));
    assert!(stdout.contains("public_criterion=false"));
    assert!(stdout.contains("independently_checkable=false"));
    assert!(stdout.contains("published_evidence_bundle=false"));
    assert!(stdout.contains("signed_run_window=true"));
    assert!(stdout.contains("supporting_record_artifacts=false"));
    assert!(stdout.contains("required_run_duration=false"));
    assert!(stdout.contains("required_block_count=false"));
}
