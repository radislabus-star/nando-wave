use nando_operator_learning::multi_source::{
    K1ConsequenceTypeV1, K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1,
};

use super::{frozen_support_contains, frozen_support_manifest, selected_shape_is_compatible};

fn root(value: u64) -> String {
    format!("{value:064x}")
}

fn evidence_row(evidence: u64, lineage: u64) -> K1NaturalEvidenceRowV1 {
    K1NaturalEvidenceRowV1::seal(
        root(evidence),
        root(9),
        root(10),
        root(11),
        root(12),
        root(lineage),
        K1ConsequenceTypeV1::Scalar,
        K1NaturalEvidenceClassV1::NaturalLive,
        29_931,
        29_931,
        1,
        true,
        true,
        false,
    )
    .expect("evidence row")
}

#[test]
fn contract_gap_never_enters_frozen_support() {
    assert!(frozen_support_contains(29_931, 29_931));
    assert!(!frozen_support_contains(35_084, 29_931));
}

#[test]
fn support_manifest_is_canonical_across_join_order() {
    let first = evidence_row(1, 20);
    let second = evidence_row(2, 21);
    let forward = frozen_support_manifest([&first, &second]).expect("forward manifest");
    let reversed = frozen_support_manifest([&second, &first]).expect("reversed manifest");
    assert_eq!(forward, reversed);
}

#[test]
fn empty_identification_can_reach_a_terminal_verdict() {
    let frozen = root(30);
    assert!(selected_shape_is_compatible(None, &frozen));
    assert!(selected_shape_is_compatible(Some(&frozen), &frozen));
    assert!(!selected_shape_is_compatible(Some(&root(31)), &frozen));
}
