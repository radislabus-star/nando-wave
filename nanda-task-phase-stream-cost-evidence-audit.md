# NANDA Task: Phase Stream Cost Evidence Audit

## Query

Check that `phase-stream-real-traffic-cost-evidence-audit-v1` is an audit-only
real-traffic phase-center helper. It may rank non-legacy buckets by verifier
and token/cost evidence and point to a money-ready `git_control` shadow run,
but it must not compile/promote packages, enable local accept, revive `.nwrb`,
or claim token savings when token counts are missing.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| cost audit command | entrypoint | phase-stream-real-traffic-cost-evidence-audit-v1 | crates/nando-cli/src/main.rs:484 |
| cost audit command | implementation | run_phase_stream_real_traffic_cost_evidence_audit_v1 | crates/nando-cli/src/phase_streaming_cmd.rs:2034 |
| cost audit command | human help | ranks non-legacy real-traffic buckets by verifier plus token/cost evidence | crates/nando-cli/src/help.rs:122 |
| cost audit report | kind | generic_real_traffic_cost_evidence_audit_v1 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:report_kind |
| cost audit report | mode | shadow_trace_cost_evidence_audit_only | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:mode |
| cost audit report | shadow_request_rows | 514 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:shadow_request_rows |
| cost audit report | nonlegacy_candidate_rows | 514 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:nonlegacy_candidate_rows |
| cost audit report | skipped_legacy_profile_events | 0 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:skipped_legacy_profile_events |
| cost audit report | verifier_bound_token_or_cost_events | 109 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:verifier_bound_token_or_cost_events |
| cost audit report | compile_ready_bucket_count | 6 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:compile_ready_bucket_count |
| cost audit report | money_proof_candidate_bucket_count | 1 | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:money_proof_candidate_bucket_count |
| test_output_parse bucket | blocker | add_verified_safe_negative_evidence | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:buckets[0].recommended_next_action |
| git_control bucket | readiness | can_measure_money true | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:buckets[1].can_measure_money |
| agent_continue_execute bucket | blocker | enrich_trace_with_token_cost_evidence | target/nando-wave/streaming/real-traffic-cost-evidence-audit-v1.report.json:buckets[2].recommended_next_action |
| git_control phase report | accepted_bucket_count | 1 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:accepted_bucket_count |
| git_control phase report | stream_false_accepts | 0 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:stream_false_accepts |
| git_control phase report | unique CPU accepts over exact cache | 1 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:total_unique_cpu_accepts_over_exact_cache |
| git_control phase report | cost saved microusd | 100 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:total_nando_cpu_cost_saved_microusd |
| git_control phase report | token saved | 0 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:total_nando_cpu_tokens_saved |
| git_control phase report | token evidence missing events | 1 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:token_evidence_missing_events |
| git_control phase report | cost evidence missing events | 0 | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:cost_evidence_missing_events |
| git_control phase report | local_accept_enabled | false | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:local_accept_enabled |
| git_control phase report | market_money_claim_allowed | false | target/nando-wave/streaming/git-control-cost-phase-center-online-discovery-v1.report.json:market_money_claim_allowed |
| legacy guard | blocks | role-binding command prefix | crates/nando-cli/src/main.rs:496 |
| legacy skip helper | rejects | role_binding/nwrb profile names | crates/nando-cli/src/phase_streaming_cmd.rs:3659 |
| executor notes | records | first cost-bearing phase-center shadow accept and token proof gap | docs/EXECUTOR_REVIEW_NOTES.md:1 |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| cost audit command | compiles | `.nwpc` package | negative-contract:audit_only_no_compile |
| cost audit command | promotes | phase-center candidate | negative-contract:audit_only_no_promote |
| cost audit command | enables | product local accept | negative-contract:local_accept_must_remain_false |
| cost audit command | revives | `.nwrb` role-binding backend | negative-contract:legacy_backend_forbidden |
| git_control phase report | claims | token savings | negative-contract:token_evidence_missing_events_is_1_and_tokens_saved_is_0 |
| git_control phase report | claims | market money proof | negative-contract:market_money_claim_allowed_false |
