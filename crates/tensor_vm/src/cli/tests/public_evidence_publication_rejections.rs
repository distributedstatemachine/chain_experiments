use super::*;

#[test]
fn execute_command_fixture_rejects_invalid_publication_evidence_args() {
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://evidence.tensorvm.example/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "http://127.0.0.1/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: " https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json ".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json?download=1".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: [0; 32],
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: [0; 32],
            manifest_signature_count: 1,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 0,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 2,
            independent_auditor_count: 1,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidencePublication {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            manifest_signer: address(b"public-evidence-publisher"),
            manifest_signature_count: 1,
            independent_auditor_count: 0,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: [0; 32],
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://localhost/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: "https://localhost/audit.json".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: "https://auditor.tensorvm.net/".to_owned(),
            observed_at_unix_seconds: 1_700_000_000,
        })
        .is_err()
    );
    assert!(
        execute_command_fixture(&CommandFixture::EvidenceAuditorRecord {
            bundle_id: hash_bytes(b"test", &[b"public-evidence-bundle"]),
            public_uri: "https://tensorvm.net/tensorvm/public-evidence.json".to_owned(),
            auditor_id: address(b"public-evidence-auditor-0"),
            audit_uri: manifest_auditor_uri(),
            observed_at_unix_seconds: 0,
        })
        .is_err()
    );
}
