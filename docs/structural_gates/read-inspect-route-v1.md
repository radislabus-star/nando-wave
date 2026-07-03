# read-inspect-route-v1

## query

Check whether the read_inspect real-traffic route/profile rung is recorded as
scoreable CPU route progress without being promoted to verified CPU savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| read_inspect_route | has_dry_run_report | read-inspect-payload-dry-run-v1.report.json | dry-run report |
| read_inspect_route | has_profile_report | read-inspect-profile-v1.report.json | profile report |
| read_inspect_route | has_registry | profile-registry-read-inspect-v1.json | profile report |
| read_inspect_dry_run | has_candidate_events | 27 | dry-run report |
| read_inspect_dry_run | has_payload_ready_events | 12 | dry-run report |
| read_inspect_dry_run | has_scoreable_payload_events | 12 | dry-run report |
| read_inspect_profile | has_edge_count | 8 | profile report |
| read_inspect_profile | has_runtime_bytes_estimate | 33000 | profile report |
| read_inspect_profile | has_disabled_threshold | i32_max | profile report |
| read_inspect_shadow | has_shadow_accepts | 0 | shadow report |
| read_inspect_audit | has_verification_hook_ready_events | 0 | verification audit |
| read_inspect_audit | has_verified_cpu_accept_eligible_events | 0 | verification audit |
| read_inspect_audit | has_false_accepts | 0 | verification audit |
| read_inspect_audit | missing_output_evidence | 12 | verification audit |
| read_inspect_audit | missing_explicit_verification | 12 | verification audit |
| read_inspect_claim | has_market_claim_allowed | false | reports |
| route_catalog_after_read | has_existing_route_candidate_events | 489 | route-gap catalog |
| route_catalog_after_read | has_no_candidate_events | 511 | route-gap catalog |
| route_gap_after_read | has_top_payload_ready_family | metrics_report_readout | route-gap readiness |
| feedback_after_read | has_verified_cpu_accepts | 9 | feedback report |
| historical_mixed_v2_feedback | has_verified_cpu_accepts | 17 | executor notes |
| numeric_boundary | forbids_mixing | 9_and_17_as_single_claim | executor notes |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| read_inspect_route | is_scoreable_route_progress | true | dry-run/profile/shadow/audit |
| read_inspect_route | is_verified_savings_progress | false | audit has zero verification hooks and zero verified accepts |
| read_inspect_route | requires_next_verifier | read_only_path_and_excerpt_verifier_v1 | executor notes |
| read_inspect_route | keeps_local_accepts_disabled | true | profile threshold i32_max |
| read_inspect_route | keeps_false_accepts_zero | true | shadow/audit |
| route_catalog_after_read | moves_read_inspect_from_gap_to_existing_route | true | route gap catalog after read registry |
| route_catalog_after_read | makes_metrics_report_readout_next_payload_ready_gap | true | route gap readiness after read registry |
| feedback_after_read | is_current_route_only_bundle | true | feedback report |
| historical_mixed_v2_feedback | remains_historical_stronger_verified_snapshot | true | executor notes |
| market_claim | remains_blocked | true | market_claim_allowed false |
