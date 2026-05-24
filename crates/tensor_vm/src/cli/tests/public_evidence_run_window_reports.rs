use super::*;

#[test]
fn execute_run_window_evidence_reports_outputs() {
    let run_window = execute_public_evidence_command(&EvidenceCommand::Run(
        EvidenceRunCommand::Window(RunWindowArgs {
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            started_at: 1_700_000_000,
            ended_at: 1_700_000_060,
            observed_blocks: 10,
        }),
    ))
    .unwrap();
    assert_eq!(
        run_window,
        format!(
            "run_started_at_unix_seconds=1700000000\nrun_ended_at_unix_seconds=1700000060\nrun_window_signature={}\nobserved_blocks=10",
            hex(&manifest_bundle().run_window_signature)
        )
    );
    let run_window_observation_file = std::env::temp_dir().join(format!(
        "tensor-vm-run-window-{}.records",
        std::process::id()
    ));
    let run_window_observations = (0..10)
        .map(|block| {
            let timestamp = if block == 9 {
                1_700_000_060
            } else {
                1_700_000_000 + block * 6
            };
            format!("run_window_observation={block},{timestamp}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&run_window_observation_file, run_window_observations).unwrap();
    let run_window_from_file = execute_public_evidence_command(&EvidenceCommand::Run(
        EvidenceRunCommand::WindowFile(RunWindowFromFileArgs {
            bundle_id: hash_arg(hash_bytes(b"test", &[b"public-evidence-bundle"])),
            manifest_signer: address_arg(address(b"public-evidence-publisher")),
            block_observation_file: run_window_observation_file.clone(),
        }),
    ))
    .unwrap();
    std::fs::remove_file(&run_window_observation_file).unwrap();
    assert_eq!(run_window_from_file, run_window);
}
