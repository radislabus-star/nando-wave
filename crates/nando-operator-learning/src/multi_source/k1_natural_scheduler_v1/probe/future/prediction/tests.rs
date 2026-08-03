use super::*;

fn root(value: char) -> String {
    value.to_string().repeat(64)
}

#[test]
fn scalar_v1_root_remains_the_legacy_tuple_commitment() {
    let receipt = K1FuturePredictionReceiptV1::seal(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        root('7'),
        root('8'),
        &root('9'),
        10,
        11,
        12_000_000,
    )
    .expect("v1 receipt");
    let expected = canonical_json_sha256(&(
        K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1,
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        root('7'),
        root('8'),
        receipt.predicted_symbolic_action_root_sha256.as_str(),
        10_u64,
        11_u64,
        12_000_000_u64,
        false,
        false,
    ))
    .expect("legacy root");
    assert_eq!(receipt.prediction_root_sha256, expected);
    assert!(!receipt.has_typed_consequence_precommit());
    assert_eq!(
        serde_json::from_slice::<K1FuturePredictionReceiptV1>(
            &serde_json::to_vec(&receipt).expect("encode")
        )
        .expect("decode"),
        receipt
    );
}

#[test]
fn typed_prediction_binds_execution_and_consequence_roots() {
    let receipt = K1FuturePredictionReceiptV1::seal_typed(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        root('7'),
        root('8'),
        &root('9'),
        root('a'),
        root('b'),
        root('c'),
        10,
        11,
        11_500_000,
        12_000_000,
    )
    .expect("v2 receipt");
    assert!(receipt.has_typed_consequence_precommit());
    let mut tampered = receipt;
    tampered.predicted_typed_consequence_root_sha256 = Some(root('d'));
    assert_eq!(
        tampered.validate(),
        Err("k1_future_prediction_receipt_invalid")
    );
}
