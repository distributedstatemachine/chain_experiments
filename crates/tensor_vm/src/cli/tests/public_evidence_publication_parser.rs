use super::parser_support::{address_arg, publication_bundle_args};
use super::{
    AuditorRecordArgs, EvidenceCommand, PublicCommand, PublicationArgs, TvmdCommand,
    manifest_address, manifest_auditor_uri, manifest_hash, parse_test_cli,
};
use crate::types::{address, hash_bytes};

#[test]
fn parses_publication_evidence_commands() {
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "publish",
            "--bundle-id",
            &bundle_id,
            "--public-uri",
            "https://tensorvm.net/tensorvm/public-evidence.json",
            "--manifest-signer",
            &manifest_signer,
            "--manifest-signature-count",
            "1",
            "--independent-auditor-count",
            "1",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Publish(
            PublicationArgs {
                bundle: publication_bundle_args(
                    hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    "https://tensorvm.net/tensorvm/public-evidence.json",
                ),
                manifest_signer: address_arg(address(b"public-evidence-publisher")),
                manifest_signature_count: 1,
                independent_auditor_count: 1,
            },
        )))
    );

    assert_eq!(
        parse_test_cli(&[
            "public",
            "evidence",
            "audit",
            "--bundle-id",
            &bundle_id,
            "--public-uri",
            "https://tensorvm.net/tensorvm/public-evidence.json",
            "--auditor-id",
            &manifest_address(b"public-evidence-auditor-0"),
            "--audit-uri",
            &manifest_auditor_uri(),
            "--observed-at",
            "1700000060",
        ])
        .unwrap(),
        TvmdCommand::Public(PublicCommand::Evidence(EvidenceCommand::Audit(
            AuditorRecordArgs {
                bundle: publication_bundle_args(
                    hash_bytes(b"test", &[b"public-evidence-bundle"]),
                    "https://tensorvm.net/tensorvm/public-evidence.json",
                ),
                auditor_id: address_arg(address(b"public-evidence-auditor-0")),
                audit_uri: manifest_auditor_uri(),
                observed_at: 1_700_000_060,
            },
        )))
    );
}
