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
    durable_predictions: usize,
    verified_future_outcomes: usize,
) -> DeadlineClassification {
    if surviving_semantic_class_roots_sha256.len() == 1 && !durable_prediction_contract {
        return DeadlineClassification {
            verdict: K1GenerationVerdictClassV1::AcquisitionFail,
            runtime_state: K1NaturalSchedulerRuntimeStateV1::TerminalAcquisitionFail,
            blocker: "independent_future_prediction_contract_missing",
        };
    }
    if surviving_semantic_class_roots_sha256.len() == 1 {
        if durable_predictions == 0 && future_eligible_rows == 0 {
            return DeadlineClassification {
                verdict: K1GenerationVerdictClassV1::IndependentFutureNotObserved,
                runtime_state:
                    K1NaturalSchedulerRuntimeStateV1::TerminalIndependentFutureNotObserved,
                blocker: "independent_future_not_observed",
            };
        }
        let blocker = if durable_predictions == 0 {
            "independent_future_not_precommitted"
        } else if verified_future_outcomes < durable_predictions {
            "durable_future_prediction_unsettled"
        } else {
            "independent_future_verification_incomplete"
        };
        return DeadlineClassification {
            verdict: K1GenerationVerdictClassV1::AcquisitionFail,
            runtime_state: K1NaturalSchedulerRuntimeStateV1::TerminalAcquisitionFail,
            blocker,
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
        let deadline = classify_deadline(&["a".repeat(64)], 0, true, 0, 0);

        assert_eq!(
            deadline.verdict,
            K1GenerationVerdictClassV1::IndependentFutureNotObserved
        );
        assert_eq!(deadline.blocker, "independent_future_not_observed");
    }

    #[test]
    fn unresolved_version_space_remains_probe_exhausted() {
        let deadline = classify_deadline(&["a".repeat(64), "b".repeat(64)], 0, true, 0, 0);

        assert_eq!(deadline.verdict, K1GenerationVerdictClassV1::ProbeExhausted);
    }

    #[test]
    fn predicted_future_that_never_settled_is_acquisition_fail() {
        let deadline = classify_deadline(&["a".repeat(64)], 1, true, 1, 0);

        assert_eq!(
            deadline.verdict,
            K1GenerationVerdictClassV1::AcquisitionFail
        );
        assert_eq!(deadline.blocker, "durable_future_prediction_unsettled");
    }

    #[test]
    fn unpredicted_future_is_acquisition_fail() {
        let deadline = classify_deadline(&["a".repeat(64)], 1, true, 0, 0);

        assert_eq!(
            deadline.verdict,
            K1GenerationVerdictClassV1::AcquisitionFail
        );
        assert_eq!(deadline.blocker, "independent_future_not_precommitted");
    }

    #[test]
    fn unique_class_without_durable_prediction_contract_is_acquisition_fail() {
        let deadline = classify_deadline(&["a".repeat(64)], 0, false, 0, 0);

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
