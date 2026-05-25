use super::*;

#[test]
fn direct_publication_evidence_rejects_invalid_args() {
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://evidence.tensorvm.example/public-evidence.json",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "http://127.0.0.1/public-evidence.json",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            " https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json ",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json?download=1",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            [0; 32],
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-publisher"),
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json",
            [0; 32],
            1,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-publisher"),
            0,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-publisher"),
            2,
            1,
        )
        .is_err()
    );
    assert!(
        execute_publication(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            "https://tensorvm.net/tensorvm/public-evidence.json",
            address(b"public-evidence-publisher"),
            1,
            0,
        )
        .is_err()
    );
    assert!(execute_auditor([0; 32], manifest_auditor_uri(), 1_700_000_000).is_err());
    assert!(
        execute_auditor(
            address(b"public-evidence-auditor-0"),
            "https://localhost/audit.json",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_auditor(
            address(b"public-evidence-auditor-0"),
            "https://auditor.tensorvm.net/",
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_auditor(
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            0
        )
        .is_err()
    );
    assert!(
        execute_auditor_with_public_uri(
            "https://localhost/public-evidence.json",
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            1_700_000_000,
        )
        .is_err()
    );
    assert!(
        execute_auditor_with_public_uri(
            "https://tensorvm.net/",
            address(b"public-evidence-auditor-0"),
            manifest_auditor_uri(),
            1_700_000_000,
        )
        .is_err()
    );
}

fn execute_publication(
    bundle_id: [u8; 32],
    public_uri: &str,
    manifest_signer: [u8; 32],
    manifest_signature_count: u64,
    independent_auditor_count: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Publish(PublicationArgs {
        bundle: publication_bundle_args(bundle_id, public_uri),
        signer: manifest_signer_args(manifest_signer),
        manifest_signature_count,
        independent_auditor_count,
    }))
}

fn execute_auditor(
    auditor_id: [u8; 32],
    audit_uri: impl Into<String>,
    observed_at: u64,
) -> crate::error::Result<String> {
    execute_auditor_with_public_uri(
        "https://tensorvm.net/tensorvm/public-evidence.json",
        auditor_id,
        audit_uri,
        observed_at,
    )
}

fn execute_auditor_with_public_uri(
    public_uri: &str,
    auditor_id: [u8; 32],
    audit_uri: impl Into<String>,
    observed_at: u64,
) -> crate::error::Result<String> {
    execute_public_evidence_command(&EvidenceCommand::Audit(AuditorRecordArgs {
        bundle: publication_bundle_args(
            hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri,
        ),
        auditor_id: address_arg(auditor_id),
        audit_uri: audit_uri.into(),
        observation: observation_timestamp_args(observed_at),
    }))
}
