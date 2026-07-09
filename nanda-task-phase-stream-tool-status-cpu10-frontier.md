# NANDA Task: Tool-Status Phase Stream CPU10 Frontier

## Query

Verify the narrow route: real Codex session tool-status events become an
offline verifier-bound `.nwpc` promotion candidate, push the combined frontier
past CPU10, and still do not enable product local accept or forbidden backends.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| tool-status-trace | entrypoint | phase-stream-codex-session-tool-status-verifier-trace-v1 | main-dispatch-tool-status |
| tool-status-trace | source | codex_session_exec_command_end | trace-report-source |
| tool-status-trace | rows_written | 10000 | trace-report |
| tool-status-trace | negative_rows | 388 | trace-report |
| tool-status-trace | raw_text_written | false | trace-report-boundary |
| tool-status-discovery | action_family | action_family:tool_status | discovery-report |
| tool-status-discovery | split | older_train_newer_shadow | discovery-report |
| tool-status-discovery | heldout_events | 2000 | discovery-report |
| tool-status-discovery | heldout_accuracy_milli | 1000 | discovery-report |
| tool-status-discovery | false_accepts | 0 | discovery-report |
| tool-status-audit | report_kind | phase_atom_action_family_time_split_promotion_audit_v1 | audit-report |
| tool-status-audit | promotion_candidate_allowed | true | audit-report |
| tool-status-audit | product_promotion_allowed | false | audit-report |
| tool-status-audit | unique_accepts_over_exact_cache | 1641 | audit-report |
| frontier-union | safe_inputs | 8 | frontier-report |
| frontier-union | combined_accepts | 1739 | frontier-report |
| cpu10-gap | target_accepts | 500 | cpu10-report |
| cpu10-gap | remaining_accept_gap | 0 | cpu10-report |
| cpu10-gap | frontier_reaches_target | true | cpu10-report |
| cpu10-gap | old_trace_pool_shortfall | 393 | cpu10-report |
| boundary | local_accept_serving_market_legacy | false | notes-boundary |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| tool-status-audit | enables | product_local_accept | negative-local-accept |
| tool-status-audit | promotes | serving_profile | negative-serving-promotion |
| tool-status-audit | claims | market_money_proof | negative-market-claim |
| tool-status-audit | uses | forbidden_lookup_or_target_authority | negative-forbidden-boundary |
| tool-status-audit | revives | nwrb_role_binding_backend | negative-legacy-backend |
