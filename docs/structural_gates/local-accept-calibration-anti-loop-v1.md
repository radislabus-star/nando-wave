# Local-Accept Calibration Anti-Loop V1

NANDA status: PASS.

This packet checks one route-selection claim: failed or low-support local-accept
calibrations are downranked and do not become CPU80 promotion claims.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_read_inspect_hook_ready_rows | value | 9 | read_inspect_calibration_report |
| metric_read_inspect_label_true_rows | value | 1 | read_inspect_calibration_report |
| metric_read_inspect_label_false_rows | value | 8 | read_inspect_calibration_report |
| metric_read_inspect_safe_policy_found | value | false | read_inspect_calibration_report |
| metric_read_inspect_catalog_rank | value | 15 | catalog_read_inspect_rank_field |
| metric_read_inspect_local_accept_failed | value | true | catalog_read_inspect_local_accept_failed_field |
| metric_planning_low_support | value | true | catalog_planning_low_support_field |
| metric_retrieval_low_support | value | true | catalog_retrieval_low_support_field |
| metric_cpu_verified_accepts | value | 26 | catalog_current_verified_cpu_accepts_field |
| metric_cpu_verified_gap_to_80 | value | 774 | catalog_verified_gap_to_80_calls_field |
| metric_market_claim_allowed | value | false | executor_notes_claim_boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_read_inspect_safe_policy_found | value | false | read_inspect_calibration_report |
| metric_read_inspect_catalog_rank | value | 15 | catalog_read_inspect_rank_field |
| metric_read_inspect_local_accept_failed | value | true | catalog_read_inspect_local_accept_failed_field |
| metric_planning_low_support | value | true | catalog_planning_low_support_field |
| metric_retrieval_low_support | value | true | catalog_retrieval_low_support_field |
| metric_cpu_verified_accepts | value | 26 | catalog_current_verified_cpu_accepts_field |
| metric_market_claim_allowed | value | false | executor_notes_claim_boundary |
