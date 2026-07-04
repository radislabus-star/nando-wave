# CPU Route Feedback Loop Integration Audit V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | feedback_loop_command_wiring | status | present | crates/nando-cli/src/role_binding_runtime_cmd.rs feedback_route_stage and feedback_route_next_action | 0.99 | integration item | status | integration | wiring |
| t2 | feedback_loop_cli_dispatch | status | present | crates/nando-cli/src/main.rs role-binding-real-traffic-feedback-loop-v1 dispatch | 0.99 | integration item | status | integration | wiring |
| t3 | feedback_loop_help_text | status | present | crates/nando-cli/src/help.rs role-binding-real-traffic-feedback-loop-v1 help text | 0.99 | integration item | status | integration | wiring |
| t4 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json | 0.99 | telemetry state | measured value | telemetry | current-state |
| t5 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json | 0.99 | telemetry state | measured value | telemetry | current-state |
| t6 | market_claim_boundary | requires | hook backed local accept plus false_accepts zero plus provider cost evidence plus non synthetic trace evidence | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.99 | claim boundary | required evidence | claim-boundary | boundary |
| t7 | candidate_or_scoreable_rows | are_not | verified CPU savings | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.99 | telemetry stage | forbidden claim | claim-boundary | boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | feedback_loop_command_wiring | status | present | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | integration item | status | integration | wiring |
| c2 | feedback_loop_cli_dispatch | status | present | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | integration item | status | integration | wiring |
| c3 | feedback_loop_help_text | status | present | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | integration item | status | integration | wiring |
| c4 | cpu_routability_current_state | verified_cpu_accept_eligible_events | 32 of 1000 | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | telemetry state | measured value | telemetry | current-state |
| c5 | cpu_routability_current_state | verified_gap_to_80_calls | 768 | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | telemetry state | measured value | telemetry | current-state |
| c6 | market_claim_boundary | requires | hook backed local accept plus false_accepts zero plus provider cost evidence plus non synthetic trace evidence | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | claim boundary | required evidence | claim-boundary | boundary |
| c7 | candidate_or_scoreable_rows | are_not | verified CPU savings | docs/EXECUTOR_REVIEW_NOTES.md CPU Route Feedback Loop Integration Audit | 0.95 | telemetry stage | forbidden claim | claim-boundary | boundary |

