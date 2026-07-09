# NANDA Triad Worksheet

task_id: phase-stream-nwpc-quarantine
domain: code
query: verify quarantine .nwpc candidate is not promoted serving/local accept

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | phase_stream_report | mode | shadow_only | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:5 | 1.0 | report | mode | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t2 | phase_stream_report | local_accept_enabled | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:7 | 1.0 | report | safety_flag | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t3 | candidate_package | package_kind | quarantine_candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:56 | 1.0 | package | boundary_label | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t4 | candidate_package | quarantine_only | true | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:67 | 1.0 | package | safety_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t5 | candidate_package | serving_profile_artifact | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:68 | 1.0 | package | safety_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t6 | candidate_package | promoted | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:69 | 1.0 | package | promotion_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t7 | phase_shadow | false_accepts | 0 | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:75 | 1.0 | shadow_eval | safety_metric | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| t8 | forbidden_backend_guard | rejects | role_binding_commands | crates/nando-cli/src/main.rs:466-469 | 1.0 | guard | forbidden_route | legacy_guard | legacy_boundary | CLI | nando-cli | main | forbidden_guard | crates/nando-cli/src/main.rs | current |
| t9 | executor_notes | states | candidate_not_product_serving | docs/EXECUTOR_REVIEW_NOTES.md:92-104 | 1.0 | review_log | boundary | phase_stream | review_boundary | docs | nando-wave | EXECUTOR_REVIEW_NOTES | boundary_record | docs/EXECUTOR_REVIEW_NOTES.md | current |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | phase_stream_report | mode | shadow_only | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:5 | 1.0 | report | mode | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c2 | phase_stream_report | local_accept_enabled | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:7 | 1.0 | report | safety_flag | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c3 | candidate_package | package_kind | quarantine_candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:56 | 1.0 | package | boundary_label | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c4 | candidate_package | quarantine_only | true | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:67 | 1.0 | package | safety_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c5 | candidate_package | serving_profile_artifact | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:68 | 1.0 | package | safety_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c6 | candidate_package | promoted | false | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:69 | 1.0 | package | promotion_flag | phase_stream | package_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | candidate_package | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c7 | phase_shadow | false_accepts | 0 | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json:75 | 1.0 | shadow_eval | safety_metric | phase_stream | shadow_boundary | CLI | nando-cli | phase-stream-test-output-parse-v1 | shadow_report | target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json | current |
| c8 | forbidden_backend_guard | rejects | role_binding_commands | crates/nando-cli/src/main.rs:466-469 | 1.0 | guard | forbidden_route | legacy_guard | legacy_boundary | CLI | nando-cli | main | forbidden_guard | crates/nando-cli/src/main.rs | current |
| c9 | executor_notes | states | candidate_not_product_serving | docs/EXECUTOR_REVIEW_NOTES.md:92-104 | 1.0 | review_log | boundary | phase_stream | review_boundary | docs | nando-wave | EXECUTOR_REVIEW_NOTES | boundary_record | docs/EXECUTOR_REVIEW_NOTES.md | current |

## notes

- Boundary-only gate: quarantine candidate package must not be confused with promoted serving/local accept.
- Package parity remains covered by command output and JSON report, not by this structural route gate.
