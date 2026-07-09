# NANDA Task: Phase Stream Action-Family Time-Split Promotion Audit

## Query

Verify the narrow route: action-family phase atoms enter the frontier only
through offline `.nwpc` promotion-candidate audits, not through raw discovery
and not as product local accept.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| phase-atom-trace | entrypoint | phase-stream-real-traffic-phase-atom-trace-v1 | main-dispatch-trace |
| phase-atom-trace | output_ready_rows | 374 | trace-report-ready-rows |
| action-family-discovery | entrypoint | phase-stream-phase-atom-action-family-time-split-discovery-v1 | main-dispatch-discovery |
| action-family-discovery | split_policy | older_train_newer_shadow | discovery-code-split-policy |
| action-family-discovery | package | phase_center_nwpc_candidate | discovery-report-package |
| metrics-candidate | unique_accepts_over_exact_cache | 11 | metrics-audit-report |
| metrics-candidate | false_accepts | 0 | metrics-audit-report |
| metrics-candidate | promotion_candidate_allowed | true | metrics-audit-report |
| planning-candidate | unique_accepts_over_exact_cache | 19 | planning-audit-report |
| planning-candidate | false_accepts | 0 | planning-audit-report |
| planning-candidate | promotion_candidate_allowed | true | planning-audit-report |
| action-family-audit | entrypoint | phase-stream-phase-atom-action-family-time-split-promotion-audit-v1 | main-dispatch-audit |
| action-family-audit | gate | package_match_false_accepts_zero_parity_zero | audit-code-gate |
| frontier-union | supported_input | phase_atom_action_family_time_split_promotion_audit_v1 | union-supported-kind-branch |
| frontier-union | unsupported_input | raw_action_family_discovery_report | union-unsupported-raw-discovery |
| frontier-report | safe_inputs | 7 | frontier-report-safe-inputs |
| frontier-report | combined_accepts | 98 | frontier-report-accepts |
| cpu10-gap | remaining_accept_gap | 402 | cpu10-gap-report |
| cpu10-gap | scoring_only_can_reach_cpu10 | false | cpu10-gap-report |
| boundary | product_local_accept | false | notes-boundary |
| boundary | serving_promotion | false | notes-boundary |
| boundary | market_money_claim | false | notes-boundary |
| boundary | legacy_nwrb_backend | false | notes-boundary |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| raw_action_family_discovery_report | enters | frontier-union | negative-audit-required |
| action-family-audit | enables | product_local_accept | negative-local-accept |
| action-family-audit | promotes | serving_profile | negative-serving-promotion |
| action-family-audit | claims | market_money_proof | negative-market-claim |
| action-family-audit | revives | nwrb_role_binding_backend | negative-legacy-backend |
| frontier-report | claims | cpu10_complete | negative-98-not-500 |
