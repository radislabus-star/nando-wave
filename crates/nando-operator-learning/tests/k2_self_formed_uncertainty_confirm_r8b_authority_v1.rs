use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use nando_operator_learning::{
    K2CompositionAuthorityBoundaryV1, K2UncertaintyConfirmDataMountV1,
    K2UncertaintyConfirmGuestExecutableV1, K2UncertaintyConfirmMountTargetV1,
    K2UncertaintyR8BAuthorizationReceiptV2, K2UncertaintyR8BAuthorizationRequestV2,
    K2UncertaintyR8BProcessLedgerV2, K2UncertaintyR8BPublicationReceiptV2,
    K2UncertaintyR8BPublicationRequestV2, composition_root_v1, composition_sha256_file_v1,
    run_self_formed_confirm_sandbox_v1, uncertainty_bytes_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const S05_SELECTOR_V2: &str = "r8b_v7_s05_authority_aggregate";

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_s05_authority_aggregate() {
    begin_suite_request_from_stdin_v2("S05_AUTHORITY_PUBLICATION", S05_SELECTOR_V2);
    r8b_publisher_rejects_no_clobber_and_symlink_boundaries();
    publish_suite_measurements_v2(vec![SuiteMeasurementV2 {
        relative_path: "suites/s05/aggregate-publication-faults.json",
        kind: nando_operator_learning::K2UncertaintyR8BEvidenceKindV2::AggregatePublicationFaults,
        source_roots_sha256: vec![root_v1("s05-existing"), root_v1("s05-symlink")],
        observed: 2,
        metrics: std::collections::BTreeMap::new(),
    }]);
}

#[test]
fn r8b_authorizer_requires_a_closed_actual_packet() {
    let environment = TestEnvironmentV1::new("authority-closed-packet");
    let packet = environment.private_child("packet");
    write_new_read_only_v2(
        &packet.join("aggregate-manifest.json"),
        b"{\"not\":\"a canonical R8B manifest\"}",
    );
    freeze_directory_tree_v2(&packet);
    let authorizer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-r8b-authorizer"));
    let authorizer_sha = composition_sha256_file_v1(&authorizer).expect("M25 SHA-256");
    let request = K2UncertaintyR8BAuthorizationRequestV2::seal(
        root_v1("closed-packet-route"),
        root_v1("closed-packet-manifest"),
        authorizer_sha.clone(),
    )
    .expect("M25 request");
    let mount = [K2UncertaintyConfirmDataMountV1 {
        host_path: &packet,
        target: K2UncertaintyConfirmMountTargetV1::AggregateEvidence,
        writable: false,
    }];
    assert!(
        run_self_formed_confirm_sandbox_v1(
            K2UncertaintyConfirmGuestExecutableV1::R8BAggregateAuthorizer,
            &authorizer,
            &authorizer_sha,
            &mount,
            &uncertainty_bytes_v1(&request).expect("M25 request bytes"),
            60,
        )
        .is_err()
    );
    assert!(!packet.join("R8B_RECEIPT_V2.json").exists());
}

#[test]
fn r8b_process_ledger_rejects_started_without_finished() {
    let environment = TestEnvironmentV1::new("authority-ledger");
    let binaries = LinkedBinariesV2::from_cargo();
    let route = root_v1("incomplete-ledger-route");
    let binary = binaries.get("M02_GENERATOR");
    let mut durable = DurableProcessLedgerV2::create(
        &environment.root.join("ledger"),
        route.clone(),
        binaries.get("M24_LINKED_RUNNER"),
        &[binary],
    );
    let started = durable.start(
        "C02",
        None,
        None,
        binary,
        root_v1("generator-request"),
        root_v1("generator-stdin"),
    );
    assert!(K2UncertaintyR8BProcessLedgerV2::seal(route, vec![started]).is_err());
}

#[test]
fn r8b_publisher_writes_exact_authorization_bytes_once() {
    let environment = TestEnvironmentV1::new("authority-publisher");
    let publication = environment.private_child("publication");
    let publisher = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-r8b-evidence-publisher"
    ));
    let publisher_sha = composition_sha256_file_v1(&publisher).expect("M26 SHA-256");
    let authorization = component_authorization_v2(&publisher_sha);
    let request = K2UncertaintyR8BPublicationRequestV2::seal(
        publication.to_string_lossy().into_owned(),
        authorization.clone(),
    )
    .expect("M26 request");
    let receipt: K2UncertaintyR8BPublicationReceiptV2 =
        run_process_success_v1(&publisher, &request);
    receipt.validate().expect("M26 receipt");
    let final_path = publication.join("R8B_RECEIPT_V2.json");
    assert_eq!(
        fs::read(&final_path).expect("published authorization bytes"),
        uncertainty_bytes_v1(&authorization).expect("expected authorization bytes")
    );
    assert_eq!(
        fs::metadata(final_path)
            .expect("published authorization metadata")
            .permissions()
            .mode()
            & 0o7777,
        0o400
    );
}

#[test]
fn r8b_publisher_rejects_no_clobber_and_symlink_boundaries() {
    for boundary in ["existing", "symlink"] {
        let environment = TestEnvironmentV1::new(boundary);
        let publication = environment.private_child("publication");
        let final_path = publication.join("R8B_RECEIPT_V2.json");
        if boundary == "existing" {
            write_new_read_only_v2(&final_path, b"existing authority bytes");
        } else {
            let outside = environment.root.join("outside.json");
            fs::write(&outside, b"outside").expect("outside bytes");
            std::os::unix::fs::symlink(&outside, &final_path).expect("publication symlink");
        }
        let publisher = PathBuf::from(env!(
            "CARGO_BIN_EXE_nando-k2-self-formed-r8b-evidence-publisher"
        ));
        let publisher_sha = composition_sha256_file_v1(&publisher).expect("M26 SHA-256");
        let request = K2UncertaintyR8BPublicationRequestV2::seal(
            publication.to_string_lossy().into_owned(),
            component_authorization_v2(&publisher_sha),
        )
        .expect("M26 boundary request");
        assert!(!run_process_v1(&publisher, &request).status.success());
        assert!(fs::symlink_metadata(final_path).is_ok());
    }
}

fn component_authorization_v2(publisher_sha: &str) -> K2UncertaintyR8BAuthorizationReceiptV2 {
    let mut value = K2UncertaintyR8BAuthorizationReceiptV2 {
        schema: "nando.k2-self-formed-r8b-authorization-receipt.v2".to_owned(),
        request_root_sha256: root_v1("component-request"),
        tested_commit_sha256: root_v1("component-commit"),
        route_id_sha256: root_v1("component-route"),
        manifest_root_sha256: root_v1("component-manifest"),
        linked_manifest_root_sha256: root_v1("component-linked-manifest"),
        suite_manifest_root_sha256: root_v1("component-suite-manifest"),
        process_ledger_root_sha256: root_v1("component-process-ledger"),
        entry_roots_sha256: vec![root_v1("component-entry")],
        publisher_executable_sha256: publisher_sha.to_owned(),
        disposition: "R8B_FROZEN".to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    value.receipt_root_sha256 = composition_root_v1(&value).expect("component auth root");
    value.validate().expect("component authorization bytes");
    value
}
