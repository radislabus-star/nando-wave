# Metrics-Report Admission Calibration V1

NANDA status: pending.

This packet checks one narrow claim: metrics_report now has a request-side
admission calibration candidate with 3 true accepts and 0 false accepts, and
the CPU80 feedback ladder records it as a review-only safe-policy candidate.
It is not promoted CPU savings and not a market claim.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metrics-report-admission | route | metrics_report_readout | metrics-report-admission-calibration-v1 report |
| metrics-report-admission | input evidence | metrics-report-output-evidence trace plus codex history fingerprints | report writes fingerprints/features/counts only |
| metrics-report-admission | calibration labels | 18 true / 14 false | metrics-report-admission-calibration-v1 report |
| metrics-report-admission | robust policy | false_accept_terms_no_failure = 3 true / 0 false | support gate is 3 |
| metrics-report-feedback | stage | local_accept_calibration_safe_policy_candidate | cpu-route-feedback-loop-v1 metrics_report row |
| metrics-report-feedback | verified accepts | 0 | cpu-route-feedback-loop-v1 metrics_report row |
| metrics-report-admission | local accepts | disabled | metrics-report-admission-calibration-v1 report |
| metrics-report-admission | market claim | disallowed | metrics-report-admission-calibration-v1 report |
| CPU80 | status | not achieved | cpu-route-feedback-loop-v1 unique verified accepts 16 and gap 784 |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metrics-report-admission | route | metrics_report_readout | candidate claim |
| metrics-report-admission | input evidence | metrics-report-output-evidence trace plus codex history fingerprints | candidate claim |
| metrics-report-admission | calibration labels | 18 true / 14 false | candidate claim |
| metrics-report-admission | robust policy | false_accept_terms_no_failure = 3 true / 0 false | candidate claim |
| metrics-report-feedback | stage | local_accept_calibration_safe_policy_candidate | candidate claim |
| metrics-report-feedback | verified accepts | 0 | candidate claim |
| metrics-report-admission | local accepts | disabled | candidate claim |
| metrics-report-admission | market claim | disallowed | candidate claim |
| CPU80 | status | not achieved | candidate claim |
