use nando_operator_learning::multi_source::K1GenerationVerdictClassV1;

use super::K1NaturalSchedulerRuntimeStateV1;

pub(super) struct DeadlineClassification {
    pub(super) verdict: K1GenerationVerdictClassV1,
    pub(super) runtime_state: K1NaturalSchedulerRuntimeStateV1,
    pub(super) blocker: &'static str,
}

pub(super) fn classify_deadline(
    surviving_semantic_class_roots_sha256: &[String],
    future_eligible_rows: u64,
    durable_prediction_contract: bool,
) -> DeadlineClassification {
    if surviving_semantic_class_roots_sha256.len() == 1 && !durable_prediction_contract {
        return DeadlineClassification {
            verdict: K1GenerationVerdictClassV1::AcquisitionFail,
            runtime_state: K1NaturalSchedulerRuntimeStateV1::TerminalAcquisitionFail,
            blocker: "independent_future_prediction_contract_missing",
        };
    }
    if surviving_semantic_class_roots_sha256.len() == 1 && future_eligible_rows == 0 {
        return DeadlineClassification {
            verdict: K1GenerationVerdictClassV1::IndependentFutureNotObserved,
            runtime_state: K1NaturalSchedulerRuntimeStateV1::TerminalIndependentFutureNotObserved,
            blocker: "independent_future_not_observed",
        };
    }

    DeadlineClassification {
        verdict: K1GenerationVerdictClassV1::ProbeExhausted,
        runtime_state: K1NaturalSchedulerRuntimeStateV1::TerminalProbeExhausted,
        blocker: "generation_deadline_exhausted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_class_without_future_is_not_a_mechanism_failure() {
        let deadline = classify_deadline(&["a".repeat(64)], 0, true);

        assert_eq!(
            deadline.verdict,
            K1GenerationVerdictClassV1::IndependentFutureNotObserved
        );
        assert_eq!(deadline.blocker, "independent_future_not_observed");
    }

    #[test]
    fn unresolved_version_space_remains_probe_exhausted() {
        let deadline = classify_deadline(&["a".repeat(64), "b".repeat(64)], 0, true);

        assert_eq!(deadline.verdict, K1GenerationVerdictClassV1::ProbeExhausted);
    }

    #[test]
    fn observed_future_that_never_settled_is_not_not_observed() {
        let deadline = classify_deadline(&["a".repeat(64)], 1, true);

        assert_eq!(deadline.verdict, K1GenerationVerdictClassV1::ProbeExhausted);
    }

    #[test]
    fn unique_class_without_durable_prediction_contract_is_acquisition_fail() {
        let deadline = classify_deadline(&["a".repeat(64)], 0, false);

        assert_eq!(
            deadline.verdict,
            K1GenerationVerdictClassV1::AcquisitionFail
        );
        assert_eq!(
            deadline.blocker,
            "independent_future_prediction_contract_missing"
        );
    }
}
