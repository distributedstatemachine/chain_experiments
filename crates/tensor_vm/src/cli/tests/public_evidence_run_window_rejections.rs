use super::*;

#[test]
fn direct_run_window_evidence_rejects_invalid_args() {
    assert!(
        execute_run_window(
            [0; 32],
            address(b"public-evidence-publisher"),
            1_700_000_000,
            1_700_000_060,
            10
        )
        .is_err()
    );
    assert!(
        execute_run_window(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            [0; 32],
            1_700_000_000,
            1_700_000_060,
            10,
        )
        .is_err()
    );
    assert!(
        execute_run_window(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            address(b"public-evidence-publisher"),
            1_700_000_060,
            1_700_000_000,
            10,
        )
        .is_err()
    );
    assert!(
        execute_run_window(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            address(b"public-evidence-publisher"),
            1_700_000_000,
            1_700_000_060,
            0,
        )
        .is_err()
    );
    let run_window_summary = run_window_observation_summary_from_file(
        "run_window_observation=7,1700000000\nrun_window_observation=8,1700000006\n",
    )
    .unwrap();
    assert_eq!(
        run_window_summary.run_started_at_unix_seconds,
        1_700_000_000
    );
    assert_eq!(run_window_summary.run_ended_at_unix_seconds, 1_700_000_006);
    assert_eq!(run_window_summary.observed_blocks, 2);
    for invalid_run_window_observations in [
        "# no observations\n\n",
        " run_window_observation=0,1700000000\n",
        "run_window_observation=0,1700000000\nrun_window_observation=0,1700000001\n",
        "run_window_observation=0,1700000000\nrun_window_observation=2,1700000012\n",
        "run_window_observation=0,1700000006\nrun_window_observation=1,1700000000\n",
        "run_window_observation=0,0\n",
        "run_window_observation=0, 1700000000\n",
        "run_window_observation=0\n",
        "service_health_observation=0,reachable\n",
    ] {
        assert!(run_window_observation_summary_from_file(invalid_run_window_observations).is_err());
    }
    assert!(
        execute_run_window_file(std::env::temp_dir().join(format!(
            "missing-tensor-vm-run-window-{}.records",
            std::process::id()
        )))
        .is_err()
    );
}

fn execute_run_window(
    bundle_id: [u8; 32],
    manifest_signer: [u8; 32],
    started_at: u64,
    ended_at: u64,
    observed_blocks: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Run(EvidenceRunCommand::Window(
        RunWindowArgs::new(
            run_window_context_args(bundle_id, manifest_signer),
            started_at,
            ended_at,
            observed_blocks,
        ),
    )))
}

fn execute_run_window_file(
    block_observation_file: std::path::PathBuf,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Run(EvidenceRunCommand::WindowFile(
        RunWindowFromFileArgs::new(
            run_window_context_args(
                hash_bytes(b"test", &[b"public-evidence-bundle"]),
                address(b"public-evidence-publisher"),
            ),
            block_observation_file,
        ),
    )))
}
