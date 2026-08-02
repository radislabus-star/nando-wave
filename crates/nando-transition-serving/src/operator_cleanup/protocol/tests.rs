use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

static TEST_ID: AtomicU64 = AtomicU64::new(0);

fn root(value: u64) -> String {
    format!("{value:064x}")
}

fn test_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nando-cleanup-write-once-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn cleanup_request_rejects_source_receipt_rebinding() {
    let mut request = CleanupAuthorityRequestV1 {
        schema: CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        terminal_verdict_root_sha256: root(1),
        identification_report_root_sha256: root(2),
        package_id: "package-one".to_owned(),
        package_candidate_root_sha256: root(3),
        source_receipt_root_sha256: root(1),
        request_text: "status".to_owned(),
        provider_payload: serde_json::json!({"input": "status"}),
        expected_response_sha256: root(4),
    };
    request.validate().expect("valid request");

    request.source_receipt_root_sha256 = root(5);
    assert_eq!(
        request.validate(),
        Err("cleanup_authority_request_invalid".to_owned())
    );
}

#[test]
fn package_candidate_root_binds_every_cleanup_identity() {
    let baseline =
        k1_package_candidate_root(&root(10), &root(11), "package-one", &root(12), &root(13))
            .expect("baseline");
    let mutations = [
        k1_package_candidate_root(&root(20), &root(11), "package-one", &root(12), &root(13)),
        k1_package_candidate_root(&root(10), &root(21), "package-one", &root(12), &root(13)),
        k1_package_candidate_root(&root(10), &root(11), "package-two", &root(12), &root(13)),
        k1_package_candidate_root(&root(10), &root(11), "package-one", &root(22), &root(13)),
        k1_package_candidate_root(&root(10), &root(11), "package-one", &root(12), &root(23)),
    ];
    assert!(
        mutations
            .into_iter()
            .all(|candidate| candidate.is_ok_and(|root| root != baseline))
    );
}

#[test]
fn write_once_allows_idempotence_but_rejects_rebind() {
    let root = test_path();
    let path = root.join("receipt.json");
    write_once(&path, b"first").expect("first publication");
    write_once(&path, b"first").expect("idempotent publication");
    assert_eq!(
        write_once(&path, b"second"),
        Err("cleanup_verifier_output_rebind".to_owned())
    );
    assert_eq!(fs::read(&path).expect("receipt"), b"first");
    fs::remove_dir_all(root).expect("cleanup");
}
