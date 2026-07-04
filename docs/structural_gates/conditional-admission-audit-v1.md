# Conditional Admission Audit V1

NANDA status: PASS.

This packet checks the claim boundary for the conditional admission audit and
the CPU operator catalog rerank caused by route-support exhaustion.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_conditional_hook_ready_rows | value | 63 | conditional admission audit field hook_ready_rows |
| metric_conditional_true_rows | value | 17 | conditional admission audit field label_true_rows |
| metric_conditional_false_rows | value | 46 | conditional admission audit field label_false_rows |
| metric_conditional_best_safe_true_accepts | value | 3 | conditional admission audit field best_safe_true_accepts |
| metric_conditional_best_safe_false_accepts | value | 0 | conditional safe candidate field false_accepts |
| metric_conditional_support_exhausted | value | true | catalog field conditional_admission_current_support_exhausted |
| metric_conditional_catalog_rank_after_audit | value | 7 | catalog conditional row priority_rank |
| metric_cpu80_unique_verified_accepts | value | 22 | feedback/catalog unique accepts |
| metric_market_claim_allowed | value | false | claim boundary |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_conditional_hook_ready_rows | value | 63 | conditional admission audit field hook_ready_rows |
| metric_conditional_true_rows | value | 17 | conditional admission audit field label_true_rows |
| metric_conditional_false_rows | value | 46 | conditional admission audit field label_false_rows |
| metric_conditional_best_safe_true_accepts | value | 3 | conditional admission audit field best_safe_true_accepts |
| metric_conditional_best_safe_false_accepts | value | 0 | conditional safe candidate field false_accepts |
| metric_conditional_support_exhausted | value | true | catalog field conditional_admission_current_support_exhausted |
| metric_conditional_catalog_rank_after_audit | value | 7 | catalog conditional row priority_rank |
| metric_cpu80_unique_verified_accepts | value | 22 | feedback/catalog unique accepts |
| metric_market_claim_allowed | value | false | claim boundary |
