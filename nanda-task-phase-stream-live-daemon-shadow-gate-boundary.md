# NANDA Task: phase-stream-live-daemon-shadow-gate-boundary

## query

Check that the live daemon shadow gate remains a scoped shadow proof only: it
does not enable product local accept, mutate runtime, promote the package, or
allow market money claims.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_daemon_shadow_gate | scopes_to | action_family_tool_status | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#coverage_scope |
| product_local_accept_enabled_flag | equals | false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#local_accept_enabled |
| live_daemon_shadow_gate | reports | product_runtime_changed_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#product_runtime_changed |
| live_daemon_shadow_gate | reports | serving_runtime_changed_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#serving_runtime_changed |
| live_daemon_shadow_gate | reports | promoted_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#promoted |
| live_daemon_shadow_gate | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#market_money_claim_allowed |
| forbidden_flags | remain | clear | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#forbidden_flags |
| legacy_nwrb_backend | remains | forbidden_boundary_text_only | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#boundary |
| fallback_probe | keeps_scope | synthetic_reversed_vector_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_kind |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_daemon_shadow_gate | scopes_to | action_family_tool_status | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#coverage_scope |
| product_local_accept_enabled_flag | equals | false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#local_accept_enabled |
| live_daemon_shadow_gate | reports | product_runtime_changed_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#product_runtime_changed |
| live_daemon_shadow_gate | reports | serving_runtime_changed_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#serving_runtime_changed |
| live_daemon_shadow_gate | reports | promoted_false | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#promoted |
| live_daemon_shadow_gate | keeps_disabled | market_money_claim | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#market_money_claim_allowed |
| forbidden_flags | remain | clear | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#forbidden_flags |
| legacy_nwrb_backend | remains | forbidden_boundary_text_only | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#boundary |
| fallback_probe | keeps_scope | synthetic_reversed_vector_probe | target/nando-wave/streaming/phase-atom-live-daemon-shadow-gate-v1.report.json#fallback_probe.probe_kind |
