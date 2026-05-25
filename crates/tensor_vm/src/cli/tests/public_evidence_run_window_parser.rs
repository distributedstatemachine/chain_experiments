use super::parser_support::{path, run_window_context_args};
use super::{
    EvidenceCommand, EvidenceRunCommand, PublicCommand, RunWindowArgs, RunWindowFromFileArgs,
    TvmdCommand, manifest_address, manifest_hash, parse_test_cli,
};
use crate::types::{address, hash_bytes};

#[test]
fn parses_run_window_evidence_commands() {
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "run",
            "window",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--started-at",
            "1700000000",
            "--ended-at",
            "1700000060",
            "--observed-blocks",
            "10",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Run(
            EvidenceRunCommand::Window(RunWindowArgs::new(
                run_window_context_args(
                    hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    address(b"public-evidence-publisher"),
                ),
                1_700_000_000,
                1_700_000_060,
                10,
            )),
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "run",
            "window-file",
            "--bundle-id",
            &bundle_id,
            "--manifest-signer",
            &manifest_signer,
            "--block-observation-file",
            "artifacts/block-observations.records",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Run(
            EvidenceRunCommand::WindowFile(RunWindowFromFileArgs::new(
                run_window_context_args(
                    hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    address(b"public-evidence-publisher"),
                ),
                path("artifacts/block-observations.records"),
            )),
        )))
    );
}
