# Catalog Exhaustion Rerank V1

NANDA status: pending.

This packet checks one route-selection claim: exhausted existing-profile routes
are downranked, so planning_next_step becomes the next catalog target.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_metrics_report_incremental_unique | value | 3 | metrics_report_row_incremental_unique |
| metric_metrics_report_best_robust_safe | value | 3 | metrics_report_row_best_robust_safe |
| metric_metrics_report_support_exhausted | value | true | metrics_report_row_support_exhausted |
| metric_serving_ops_incremental_unique | value | 3 | serving_ops_row_incremental_unique |
| metric_serving_ops_best_safe | value | 3 | serving_ops_row_best_safe |
| metric_serving_ops_support_exhausted | value | true | serving_ops_row_support_exhausted |
| metric_edit_incremental_unique | value | 1 | edit_row_incremental_unique |
| metric_edit_best_safe | value | 1 | edit_row_best_safe |
| metric_edit_support_exhausted | value | true | edit_row_support_exhausted |
| metric_catalog_top_row | value | planning_next_step_route_gap_family | catalog_first_row_after_rerank |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_metrics_report_incremental_unique | value | 3 | metrics_report_row_incremental_unique |
| metric_metrics_report_best_robust_safe | value | 3 | metrics_report_row_best_robust_safe |
| metric_metrics_report_support_exhausted | value | true | metrics_report_row_support_exhausted |
| metric_serving_ops_incremental_unique | value | 3 | serving_ops_row_incremental_unique |
| metric_serving_ops_best_safe | value | 3 | serving_ops_row_best_safe |
| metric_serving_ops_support_exhausted | value | true | serving_ops_row_support_exhausted |
| metric_edit_incremental_unique | value | 1 | edit_row_incremental_unique |
| metric_edit_best_safe | value | 1 | edit_row_best_safe |
| metric_edit_support_exhausted | value | true | edit_row_support_exhausted |
| metric_catalog_top_row | value | planning_next_step_route_gap_family | catalog_first_row_after_rerank |
