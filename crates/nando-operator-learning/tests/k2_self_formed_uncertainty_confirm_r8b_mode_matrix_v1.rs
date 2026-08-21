use std::fs::{self, File};
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2UncertaintyConfirmGeneratorResponseV1, K2UncertaintyConfirmOwnerReceiptV1,
    K2UncertaintyConfirmSplitReceiptV1, K2UncertaintyControlScopeV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    K2UncertaintyDevelopmentRehearsalTerminalRequestV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalProcessRequestV1, composition_sha256_file_v1,
    load_development_rehearsal_owner_full_v1, load_development_rehearsal_owner_metadata_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const S03_SELECTOR_V2: &str = "r8b_v7_s03_mode_aggregate";

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_s03_mode_aggregate() {
    begin_suite_request_from_stdin_v2("S03_MODE_MATRIX", S03_SELECTOR_V2);
    r8b_mode_and_legacy_matrix_rejects_x01_through_x20();
    publish_suite_measurements_v2(vec![SuiteMeasurementV2 {
        relative_path: "suites/s03/mode-matrix.json",
        kind: nando_operator_learning::K2UncertaintyR8BEvidenceKindV2::ModeMatrix,
        source_roots_sha256: vec![root_v1("s03-x01-x20-pass")],
        observed: 20,
        metrics: std::collections::BTreeMap::new(),
    }]);
}

#[test]
fn r8b_mode_and_legacy_matrix_rejects_x01_through_x20() {
    let environment = TestEnvironmentV1::new("mode-matrix");
    let lab = environment.private_child("lab");
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let request = development_owner_request_v1(&lab, "attempt", &owner, &generator);
    let receipt: K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 =
        run_process_success_v1(&owner, &request);
    let attempt = lab.join("attempt");
    let (durable_owner, full) =
        load_development_rehearsal_owner_full_v1(&attempt, &request).expect("full owner route");
    assert_eq!(receipt, durable_owner);
    load_development_rehearsal_owner_metadata_v1(&attempt, &request)
        .expect("positive metadata route");

    let fixtures = confirm_fixture_root_v1();
    let confirm_split_bytes =
        fs::read(fixtures.join("confirm-split-receipt.json")).expect("Confirm split fixture");
    let confirm_owner_bytes = fs::read(fixtures.join("historical-development-owner-receipt.json"))
        .expect("historical owner fixture");
    let confirm_response_bytes = fs::read(fixtures.join("confirm-generator-response.json"))
        .expect("Confirm response fixture");
    let development_split_bytes =
        uncertainty_bytes_v1(&full.split).expect("Development split bytes");
    let development_owner_bytes =
        uncertainty_bytes_v1(&durable_owner).expect("Development owner bytes");
    let development_response_bytes =
        uncertainty_bytes_v1(&full.generator_response).expect("Development response bytes");

    let mut rejected = Vec::new();
    reject_decode_v1::<K2UncertaintyDevelopmentRehearsalSplitReceiptV1>(
        "X01",
        &confirm_split_bytes,
        &mut rejected,
    );
    reject_decode_v1::<K2UncertaintyConfirmSplitReceiptV1>(
        "X02",
        &development_split_bytes,
        &mut rejected,
    );
    reject_decode_v1::<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1>(
        "X03",
        &confirm_owner_bytes,
        &mut rejected,
    );
    reject_decode_v1::<K2UncertaintyConfirmOwnerReceiptV1>(
        "X04",
        &development_owner_bytes,
        &mut rejected,
    );
    let confirm_as_development: K2UncertaintyGeneratorResponseV1 =
        uncertainty_decode_v1(&confirm_response_bytes).expect("canonical Confirm response shape");
    assert!(confirm_as_development.validate().is_err(), "X05 accepted");
    rejected.push("X05");
    let development_as_confirm: K2UncertaintyConfirmGeneratorResponseV1 =
        uncertainty_decode_v1(&development_response_bytes)
            .expect("canonical Development response shape");
    assert!(development_as_confirm.validate().is_err(), "X06 accepted");
    rejected.push("X06");

    reject_invalid_split_v1(
        "X07",
        &full.split,
        |value| value.public_batch_root_sha256 = root_v1("confirm-public-batch"),
        &mut rejected,
    );
    reject_invalid_split_v1(
        "X08",
        &full.split,
        |value| value.public_denominator_root_sha256 = root_v1("confirm-denominator"),
        &mut rejected,
    );
    reject_invalid_split_v1(
        "X09",
        &full.split,
        |value| value.pipe_receipt.generator_request_root_sha256 = root_v1("confirm-pipe"),
        &mut rejected,
    );
    reject_invalid_split_v1(
        "X10",
        &full.split,
        |value| value.development_seed_commitment_sha256 = root_v1("non-development-seed"),
        &mut rejected,
    );
    let missing_split = environment.private_child("missing-split");
    assert!(load_development_rehearsal_owner_metadata_v1(&missing_split, &request).is_err());
    rejected.push("X11");
    reject_invalid_owner_v1(
        "X12",
        &durable_owner,
        |value| value.split_receipt_root_sha256 = root_v1("substituted-split"),
        &mut rejected,
    );
    reject_invalid_owner_v1(
        "X13",
        &durable_owner,
        |value| value.owner_request_root_sha256 = root_v1("mismatched-owner-request"),
        &mut rejected,
    );
    reject_invalid_owner_v1(
        "X14",
        &durable_owner,
        |value| value.attempt_root_sha256 = root_v1("mismatched-attempt"),
        &mut rejected,
    );
    reject_invalid_owner_v1(
        "X15",
        &durable_owner,
        |value| value.generator_response_root_sha256 = root_v1("mismatched-response"),
        &mut rejected,
    );
    reject_invalid_split_v1(
        "X16",
        &full.split,
        |value| value.public_denominator_root_sha256 = root_v1("mismatched-public-root"),
        &mut rejected,
    );
    reject_invalid_split_v1(
        "X17",
        &full.split,
        |value| value.artifacts[2].semantic_root_sha256 = root_v1("wrong-private-mount"),
        &mut rejected,
    );
    reject_terminal_scope_substitution_v1();
    rejected.push("X18");
    let historical: K2UncertaintyConfirmOwnerReceiptV1 =
        uncertainty_decode_v1(&confirm_owner_bytes).expect("historical receipt remains decodable");
    historical
        .validate()
        .expect("historical receipt remains valid");
    assert_ne!(
        historical.generator_response_root_sha256,
        full.split.split_receipt_root_sha256
    );
    rejected.push("X19");

    let contender_lab = environment.private_child("contender-lab");
    let contender_request =
        development_owner_request_v1(&contender_lab, "attempt", &owner, &generator);
    let lock = File::open(&contender_lab).expect("open contender lab lock");
    lock.try_lock().expect("hold contender lab lock");
    let before = tree_snapshot_v1(&contender_lab);
    assert!(!run_process_v1(&owner, &contender_request).status.success());
    assert_eq!(tree_snapshot_v1(&contender_lab), before);
    assert!(!contender_lab.join("attempt").exists());
    rejected.push("X20");

    assert_eq!(
        rejected,
        (1..=20)
            .map(|value| format!("X{value:02}"))
            .collect::<Vec<_>>()
    );
}

fn reject_terminal_scope_substitution_v1() {
    let experiment_root = root_v1("x18-terminal-experiment");
    let freeze_root = root_v1("x18-terminal-freeze");
    let attempt_root = root_v1("x18-terminal-attempt");
    let terminal = PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-terminal-evaluator"
    ));
    let terminal_sha256 =
        composition_sha256_file_v1(&terminal).expect("terminal evaluator SHA-256");
    let (oracle_batch, routes, resources) = r7j_terminal_evidence_v1();
    let controls = vec![
        control_receipt_v1(
            K2UncertaintyControlScopeV1::SuccessorStaticLegacy,
            &experiment_root,
            None,
            None,
        ),
        control_receipt_v1(
            K2UncertaintyControlScopeV1::SuccessorStaticV3,
            &experiment_root,
            None,
            None,
        ),
        control_receipt_v1(
            K2UncertaintyControlScopeV1::SuccessorStaticV4,
            &experiment_root,
            None,
            None,
        ),
        control_receipt_v1(
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5,
            &experiment_root,
            Some(&freeze_root),
            None,
        ),
    ];
    let baseline = evaluate_development_terminal_v1(
        &terminal,
        &terminal_sha256,
        K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
            experiment_root.clone(),
            oracle_batch.clone(),
            controls.clone(),
            routes.clone(),
            resources.clone(),
            terminal_sha256.clone(),
        )
        .expect("baseline Development terminal request"),
    );
    assert_eq!(
        baseline.disposition,
        K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass
    );
    assert_eq!(baseline.reason, "development_component_routes_complete");

    let sealed_control = control_receipt_v1(
        K2UncertaintyControlScopeV1::SealedAttemptV5,
        &experiment_root,
        Some(&freeze_root),
        Some(&attempt_root),
    );
    for (foreign_scope, foreign_control) in [
        (
            K2UncertaintyControlScopeV1::SuccessorStaticV4,
            controls[2].clone(),
        ),
        (K2UncertaintyControlScopeV1::SealedAttemptV5, sealed_control),
    ] {
        let mut substituted = controls.clone();
        substituted[3] = foreign_control;
        let receipt = evaluate_development_terminal_v1(
            &terminal,
            &terminal_sha256,
            K2UncertaintyDevelopmentRehearsalTerminalRequestV1::seal(
                experiment_root.clone(),
                oracle_batch.clone(),
                substituted,
                routes.clone(),
                resources.clone(),
                terminal_sha256.clone(),
            )
            .expect("substituted Development terminal request"),
        );
        assert_eq!(
            receipt.disposition,
            K2UncertaintyTerminalDispositionV1::InfrastructureFail,
            "X18 accepted {foreign_scope:?}"
        );
        assert_eq!(receipt.reason, "development_evidence_invalid");
    }
}

fn evaluate_development_terminal_v1(
    terminal: &Path,
    terminal_sha256: &str,
    request: K2UncertaintyDevelopmentRehearsalTerminalRequestV1,
) -> K2UncertaintyTerminalEvaluationReceiptV1 {
    assert_eq!(
        request.terminal_evaluator_executable_sha256,
        terminal_sha256
    );
    run_process_success_v1(
        terminal,
        &K2UncertaintyTerminalProcessRequestV1::Development { request },
    )
}

fn reject_decode_v1<T>(label: &'static str, bytes: &[u8], rejected: &mut Vec<&'static str>)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    assert!(
        uncertainty_decode_v1::<T>(bytes).is_err(),
        "{label} accepted"
    );
    rejected.push(label);
}

fn reject_invalid_split_v1(
    label: &'static str,
    source: &K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    mutate: impl FnOnce(&mut K2UncertaintyDevelopmentRehearsalSplitReceiptV1),
    rejected: &mut Vec<&'static str>,
) {
    let mut value = source.clone();
    mutate(&mut value);
    assert!(value.validate().is_err(), "{label} accepted");
    rejected.push(label);
}

fn reject_invalid_owner_v1(
    label: &'static str,
    source: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    mutate: impl FnOnce(&mut K2UncertaintyDevelopmentRehearsalOwnerReceiptV1),
    rejected: &mut Vec<&'static str>,
) {
    let mut value = source.clone();
    mutate(&mut value);
    assert!(value.validate().is_err(), "{label} accepted");
    rejected.push(label);
}

fn confirm_fixture_root_v1() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("plans/effect-law-unification-v1/evidence")
        .join("K2_SELF_FORMED_UNCERTAINTY_V5_R8B_PREFLIGHT_V2")
        .join("preimplementation-confirm-fixtures")
}
