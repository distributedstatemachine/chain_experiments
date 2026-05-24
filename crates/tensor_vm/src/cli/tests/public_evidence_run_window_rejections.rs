use super::*;

#[test]
fn execute_command_fixture_rejects_invalid_run_window_evidence_args() {
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceRunWindow {
            bundle_id: [0; 32],
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: [0; 32],
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_060,
            run_ended_at_unix_seconds: 1_700_000_000,
            observed_blocks: 10,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceRunWindow {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            run_started_at_unix_seconds: 1_700_000_000,
            run_ended_at_unix_seconds: 1_700_000_060,
            observed_blocks: 0,
        })
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
        execute_command_fixture(&CommandFixture::EvidenceRunWindowFromFile {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            manifest_signer: address(b"public-evidence-publisher"),
            block_observation_file: std::env::temp_dir()
                .join(format!(
                    "missing-tensor-vm-run-window-{}.records",
                    std::process::id()
                ))
                .to_string_lossy()
                .into_owned(),
        })
        .is_err()
    );
}
