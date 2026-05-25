use super::parser_support::path;
use super::{
    EvidenceCommand, PublicCommand, PublicEvidenceManifestArgs, PublicTestnetManifestArgs,
    TvmdCommand, parse_test_cli,
};

#[test]
fn parses_documented_public_commands() {
    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "validate",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Validate(
            PublicEvidenceManifestArgs::new(path("docs/tensorvm/public-testnet.evidence")),
        )))
    );
    assert_eq!(
        parse_test_cli(&[
            "public",
            "preflight",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Preflight(PublicTestnetManifestArgs::new(
            path("docs/tensorvm/public-testnet.preflight"),
        )))
    );
}

#[test]
fn rejects_retired_top_level_command_families() {
    assert!(parse_test_cli(&["role", "miner", "status"]).is_err());
    assert!(
        parse_test_cli(&[
            "public-evidence",
            "validate",
            "docs/tensorvm/public-testnet.evidence"
        ])
        .is_err()
    );
    assert!(
        parse_test_cli(&[
            "public-testnet",
            "preflight",
            "docs/tensorvm/public-testnet.preflight"
        ])
        .is_err()
    );
    assert!(parse_test_cli(&["local-testnet", "seed", "--data-dir", "/var/lib/tensorvm"]).is_err());
    assert!(parse_test_cli(&["local-cpu", "verify", "--json"]).is_err());
}
