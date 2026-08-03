use super::*;

fn root(value: char) -> String {
    value.to_string().repeat(64)
}

#[test]
fn collection_program_evidence_is_part_of_the_outcome_identity() {
    let receipt = K1FutureOutcomeReceiptV1::seal_with_program_evidence(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        7,
        true,
        true,
    )
    .expect("receipt");
    assert_eq!(receipt.schema, K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2);
    let mut changed = receipt.clone();
    changed.program_evidence_root_sha256 = Some(root('7'));
    assert_eq!(changed.validate(), Err("k1_future_outcome_receipt_invalid"));
}

#[test]
fn typed_contradiction_cannot_be_an_independent_pass() {
    let receipt = K1FutureOutcomeReceiptV1::seal_with_typed_consequence(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        root('7'),
        8,
        true,
    )
    .expect("typed outcome");
    assert!(!receipt.program_consistent);
    assert!(!receipt.independent_verifier_pass);
    assert!(receipt.validate().is_ok());
}
