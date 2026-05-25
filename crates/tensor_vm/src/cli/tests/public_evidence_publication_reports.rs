use super::*;

#[test]
fn execute_publication_evidence_reports_outputs() {
    let publication = execute_public_evidence_command(&EvidenceCommand::Publish(PublicationArgs {
        bundle: publication_bundle_args(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json",
        ),
        manifest_signer: address_arg(address(b"public-evidence-publisher")),
        manifest_signature_count: 1,
        independent_auditor_count: 1,
    }))
    .unwrap();
    let bundle_id = manifest_hash(b"public-evidence-bundle");
    let manifest_signer = manifest_address(b"public-evidence-publisher");
    let manifest_signature = manifest_publication_signature();
    assert_report_fields(
        &publication,
        &[
            ("bundle_id", bundle_id.as_str()),
            (
                "public_uri",
                "https://tensorvm.net/tensorvm/public-evidence.json",
            ),
            ("manifest_signer", manifest_signer.as_str()),
            ("manifest_signature", manifest_signature.as_str()),
            ("manifest_signature_count", "1"),
            ("independent_auditor_count", "1"),
        ],
    );

    let auditor_record =
        execute_public_evidence_command(&EvidenceCommand::Audit(AuditorRecordArgs {
            bundle: publication_bundle_args(
                hash_bytes(b"test", &[b"public-evidence-bundle"]),
                "https://tensorvm.net/tensorvm/public-evidence.json",
            ),
            auditor_id: address_arg(address(b"public-evidence-auditor-0")),
            audit_uri: manifest_auditor_uri(),
            observed_at: 1_700_000_060,
        }))
        .unwrap();
    assert_eq!(
        auditor_record,
        format!(
            "auditor={},{},1700000060,{}",
            manifest_address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            manifest_auditor_signature()
        )
    );
}
