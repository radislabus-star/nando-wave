use std::collections::BTreeMap;

use nando_operator_learning::{
    K2_UNCERTAINTY_ACTIONS_V1, K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_CONTENTS_V1,
    K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1, K2_UNCERTAINTY_PATHS_V1,
    K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2_UNCERTAINTY_RISK_COST_SCHEMA_V1, K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1,
    K2CompositionTreeManifestV1, K2UncertaintyBudgetV1, K2UncertaintyContentAtomV1,
    K2UncertaintyDomainVocabularyV1, K2UncertaintyEligibilityDispositionV1,
    K2UncertaintyPathAtomV1, K2UncertaintyProbeEquivalenceKeyV1, K2UncertaintyRiskCostV1,
    K2UncertaintySafetyDispositionV1, K2UncertaintySplitV1, K2UncertaintySupportObservationV1,
    K2UncertaintySupportOutcomeV1, K2UncertaintySupportSetV1, K2UncertaintyTransitionReasonV1,
    composition_root_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[test]
fn r1_vocabulary_and_budget_are_canonical_and_authority_free() {
    let vocabulary = vocabulary();
    vocabulary.validate().expect("valid vocabulary");
    assert_eq!(
        vocabulary.budget.action_count,
        K2_UNCERTAINTY_ACTIONS_V1 as u64
    );
    assert_eq!(vocabulary.budget.path_count, K2_UNCERTAINTY_PATHS_V1 as u64);
    assert_eq!(
        vocabulary.budget.content_count,
        K2_UNCERTAINTY_CONTENTS_V1 as u64
    );
    assert_eq!(
        vocabulary.budget.raw_probe_count,
        K2_UNCERTAINTY_RAW_PROBES_V1 as u64
    );
    assert_eq!(
        vocabulary.budget.maximum_selector_requests,
        K2_UNCERTAINTY_MAX_SELECTOR_REQUESTS_V1 as u64
    );
    assert_eq!(
        vocabulary.budget.confirm_cases,
        K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
    );

    let bytes = uncertainty_bytes_v1(&vocabulary).expect("canonical bytes");
    let decoded: K2UncertaintyDomainVocabularyV1 =
        uncertainty_decode_v1(&bytes).expect("canonical decode");
    assert_eq!(decoded, vocabulary);

    let mut tampered = decoded;
    tampered.authority.product_authority = true;
    assert!(tampered.validate().is_err());
}

#[test]
fn r1_support_requires_exact_three_rows_for_each_opaque_action() {
    let vocabulary = vocabulary();
    let mut observations = Vec::new();
    let mut sequence = 0_u64;
    for action in &vocabulary.opaque_action_roots_sha256 {
        for slot in 0..K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1 {
            let pre = manifest(slot);
            let outcome = K2UncertaintySupportOutcomeV1::seal(
                K2UncertaintyTransitionReasonV1::Applied,
                pre.clone(),
            )
            .expect("support outcome");
            observations.push(
                K2UncertaintySupportObservationV1::seal(
                    vocabulary.case_id_sha256.clone(),
                    sequence,
                    pre,
                    action.clone(),
                    outcome,
                )
                .expect("support row"),
            );
            sequence += 1;
        }
    }
    let support = K2UncertaintySupportSetV1::seal(
        vocabulary.case_id_sha256.clone(),
        vocabulary.vocabulary_root_sha256.clone(),
        observations,
    )
    .expect("support set");
    support.validate().expect("valid support set");

    let mut missing = support;
    missing.observations.pop();
    assert!(missing.validate().is_err());
}

#[test]
fn r1_risk_cost_and_v3_quotient_key_are_exact() {
    let mut accounting = K2UncertaintyRiskCostV1 {
        schema: K2_UNCERTAINTY_RISK_COST_SCHEMA_V1.to_owned(),
        read_entries: 1,
        written_or_removed_entries: 1,
        overwritten_existing_entries: 1,
        removed_existing_entries: 0,
        overwritten_bytes: 4097,
        removed_bytes: 0,
        touched_bytes: 8192,
        risk_units: 0,
        cost_units: 0,
        accounting_root_sha256: String::new(),
    };
    accounting.reseal().expect("risk/cost receipt");
    assert_eq!(accounting.risk_units, 3);
    assert_eq!(accounting.cost_units, 5);

    let mut key = K2UncertaintyProbeEquivalenceKeyV1 {
        schema: K2_UNCERTAINTY_PROBE_EQUIVALENCE_KEY_SCHEMA_V1.to_owned(),
        pairwise_outcome_equal: [true, false, false, false, false, true],
        eligibility: K2UncertaintyEligibilityDispositionV1::Eligible,
        safety: K2UncertaintySafetyDispositionV1::Pass,
        risk_units: accounting.risk_units,
        cost_units: accounting.cost_units,
        applicability_hint: true,
        dependency_hint: false,
        cleanup_hint: false,
        key_root_sha256: String::new(),
    };
    key.reseal().expect("V3 quotient key");
    key.validate().expect("valid V3 quotient key");

    let mut changed = key;
    changed.pairwise_outcome_equal[0] = false;
    assert!(changed.validate().is_err());
}

#[test]
fn r1_frozen_budget_rejects_threshold_drift() {
    let mut budget = K2UncertaintyBudgetV1::frozen_v3().expect("frozen budget");
    budget.maximum_selector_requests += 1;
    assert!(budget.validate().is_err());
}

fn vocabulary() -> K2UncertaintyDomainVocabularyV1 {
    let actions = (0..K2_UNCERTAINTY_ACTIONS_V1)
        .map(|ordinal| root(&format!("action-{ordinal}")))
        .collect();
    let paths = (0..K2_UNCERTAINTY_PATHS_V1)
        .map(|ordinal| {
            K2UncertaintyPathAtomV1::seal(ordinal as u8, format!("p{ordinal}")).expect("path atom")
        })
        .collect();
    let contents = (0..K2_UNCERTAINTY_CONTENTS_V1)
        .map(|ordinal| {
            K2UncertaintyContentAtomV1::seal(
                ordinal as u8,
                format!("content-{ordinal}").into_bytes(),
            )
            .expect("content atom")
        })
        .collect();
    K2UncertaintyDomainVocabularyV1::seal(
        root("experiment"),
        root("case"),
        K2UncertaintySplitV1::Development,
        root("generator-schema"),
        actions,
        paths,
        contents,
    )
    .expect("vocabulary")
}

fn manifest(slot: usize) -> K2CompositionTreeManifestV1 {
    let mut files = BTreeMap::new();
    if slot > 0 {
        files.insert(
            format!("p{}", slot - 1),
            format!("value-{slot}").into_bytes(),
        );
    }
    K2CompositionTreeManifestV1::from_files(&files).expect("manifest")
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r1-test-root.v1", label)).expect("test root")
}
