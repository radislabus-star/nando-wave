use super::*;

fn root(value: u64) -> String {
    format!("{value:064x}")
}

#[test]
fn only_precommitted_strict_subset_is_applied() {
    let previous = vec![root(1), root(2), root(3)];
    let next = vec![root(1), root(2)];
    assert_eq!(
        classify_probe_outcome(
            &previous,
            &next,
            MultiSourceT1IdentificationStateV1::Ambiguous,
            true,
            true,
        ),
        ProbeOutcomeDisposition::Applied
    );
}

#[test]
fn unchanged_version_space_is_the_only_censored_no_information_case() {
    let classes = vec![root(1), root(2)];
    assert_eq!(
        classify_probe_outcome(
            &classes,
            &classes,
            MultiSourceT1IdentificationStateV1::Ambiguous,
            true,
            false,
        ),
        ProbeOutcomeDisposition::NoInformation
    );
}

#[test]
fn counterevidence_cannot_disappear_from_the_terminal_denominator() {
    let previous = vec![root(1), root(2), root(3)];
    assert_eq!(
        classify_probe_outcome(
            &previous,
            &previous,
            MultiSourceT1IdentificationStateV1::FutureContradiction,
            true,
            false,
        ),
        ProbeOutcomeDisposition::Contradiction("probe_future_contradiction")
    );
    assert_eq!(
        classify_probe_outcome(
            &previous,
            &[root(1)],
            MultiSourceT1IdentificationStateV1::Ambiguous,
            true,
            false,
        ),
        ProbeOutcomeDisposition::Contradiction("probe_outcome_not_precommitted")
    );
    assert_eq!(
        classify_probe_outcome(
            &previous,
            &[root(1)],
            MultiSourceT1IdentificationStateV1::Ambiguous,
            false,
            true,
        ),
        ProbeOutcomeDisposition::Contradiction("probe_protocol_rebound")
    );
}
