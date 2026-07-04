# Mixed Safe Policy V3

NANDA status: pending.

This packet checks one narrow structural route: mixed-map v3 promotion keeps
route-local verified accepts safe.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_mixed_v3_true_accepts | value | 7 | field policy_accept_verified_true_rows |
| metric_mixed_v3_false_accepts | value | 0 | field policy_accept_verified_false_rows |
| metric_mixed_v3_unverified_accepts | value | 0 | field policy_accept_unverified_rows |
| metric_mixed_v3_shadow_accepts | value | 7 | field nando_shadow_accepts |
| metric_mixed_v3_shadow_false_accepts | value | 0 | field false_accepts |
| metric_cpu80_unique_verified_accepts | value | 26 | field verified_cpu_accept_unique_request_fingerprints |
| metric_cpu80_unique_gap_to_80 | value | 774 | field unique_verified_gap_to_80_calls |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| metric_mixed_v3_true_accepts | value | 7 | field policy_accept_verified_true_rows |
| metric_mixed_v3_false_accepts | value | 0 | field policy_accept_verified_false_rows |
| metric_mixed_v3_unverified_accepts | value | 0 | field policy_accept_unverified_rows |
| metric_mixed_v3_shadow_accepts | value | 7 | field nando_shadow_accepts |
| metric_mixed_v3_shadow_false_accepts | value | 0 | field false_accepts |
| metric_cpu80_unique_verified_accepts | value | 26 | field verified_cpu_accept_unique_request_fingerprints |
| metric_cpu80_unique_gap_to_80 | value | 774 | field unique_verified_gap_to_80_calls |
