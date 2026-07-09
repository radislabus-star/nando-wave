# NANDA Task: Phase Stream Frontier Union

## Query

Verify that the frontier union is only an accounting union over safe
phase-center shadow reports, including the run_check time-split audit, without
runtime promotion, local accept, market money claim, unsafe input inclusion, or
legacy `.nwrb` revival.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| frontier-union | entrypoint | phase-stream-real-traffic-frontier-union-v1 | crates/nando-cli/src/main.rs |
| frontier-union | implementation | run_phase_stream_real_traffic_frontier_union_v1 | crates/nando-cli/src/phase_streaming_cmd.rs |
| frontier-union | supported_report_kind_set | generic_online_plus_guarded_split_plus_run_check_audit | report_kind branches |
| frontier-union | run_check_gate | promotion_candidate_true_product_false_package_match | audit branch |
| frontier-union | safety_gate | zero_false_accepts_no_local_accept_no_runtime_change | stream/audit filters |
| frontier-union | claim_gate | no_market_money_claim_no_cpu10_claim | report boundary |
| frontier-union | legacy_gate | forbidden_flags_all_false | forbidden_flags_value_all_false |
| frontier-union | dedupe_key | request_fingerprint | union implementation |
| frontier-report | safe_inputs | 5 | target report |
| frontier-report | excluded_inputs | 0 | target report |
| frontier-report | combined_accepts | 68 | target report |
| frontier-report | combined_tokens | 55626 | target report |
| frontier-report | combined_cost_microusd | 60098 | target report |
| frontier-report | duplicate_fingerprints | 1 | target report |
| frontier-report | evidence_missing_events | 0 | target report |
| frontier-boundary | status | accounting_only_not_cpu10_complete | docs/EXECUTOR_REVIEW_NOTES.md |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| frontier-union | enables | product_local_accept | negative-contract:local_accept_false |
| frontier-union | promotes | serving_runtime | negative-contract:serving_runtime_false |
| frontier-union | revives | nwrb_role_binding_backend | negative-contract:legacy_backend_false |
| frontier-union | includes | unsafe_false_accept_report | negative-contract:false_accept_filter |
| frontier-union | double_counts | request_fingerprint | negative-contract:dedupe_key |
| frontier-report | claims | market_money_proof | negative-contract:market_claim_false |
| frontier-report | claims | cpu10_complete | negative-contract:68_not_500 |
