use super::frozen_support_contains;

#[test]
fn contract_gap_never_enters_frozen_support() {
    assert!(frozen_support_contains(29_931, 29_931));
    assert!(!frozen_support_contains(35_084, 29_931));
}
