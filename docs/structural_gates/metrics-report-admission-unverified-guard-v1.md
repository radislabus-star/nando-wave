# Metrics Report Admission Unverified Guard V1

## query

Verify the metrics-report admission correction: request-side policy selection
must treat unverified rows as unsafe. The p99 metrics split is a discovery
signal, not a promoted CPU savings path.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| metrics_report_admission | input_window | current5k_non_synthetic_codex_trace | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_admission | hook_ready_rows | 63 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_admission | label_true_rows | 31 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_admission | label_false_rows | 20 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_admission | unverified_rows | 12 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| p99_terms_not_concise | accepts | 14 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| p99_terms_not_concise | true_accepts | 12 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| p99_terms_not_concise | unsafe_false_or_unverified_accepts | 2 | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_admission | robust_safe_policy_found | false | `target/nando-wave/real-traffic-shadow/metrics-report-admission-calibration-v1-5k.report.json` |
| metrics_report_p99_split | promotion_policy | watch_no_safe_policy | `docs/CPU_CALL_CATALOG.md#metrics-report-p99-split-finding` |
| unknown_output_evidence | cpu_accept_policy | unsafe_until_verified | `docs/CPU_CALL_CATALOG.md#metrics-report-p99-split-finding` |
| cpu80_counter | unchanged_incremental_unique_accepts | 104 | `target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json` |
