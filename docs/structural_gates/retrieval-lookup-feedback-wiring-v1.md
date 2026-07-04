# Retrieval Lookup Feedback Wiring V1 Structural Gate

Query: verify that retrieval_lookup is wired into the CPU80 feedback loop as a
review-only route, not as verified CPU savings.

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | retrieval_lookup_route | uses_payload_artifact | retrieval_lookup_dry_run_trace | retrieval-lookup-payload-dry-run-v1: 25 candidates, 2 scoreable payloads | 1.0 | route | artifact | retrieval_lookup | retrieval_lookup |
| t2 | retrieval_lookup_route | uses_disabled_profile | retrieval_lookup_nwrb_profile | retrieval-lookup-profile-v1: threshold is disabled and local accepts are false | 1.0 | route | profile | retrieval_lookup | retrieval_lookup |
| t3 | retrieval_lookup_route | uses_verifier_evidence | source_path_or_url_presence_verifier | retrieval-lookup-output-evidence-v1 audit: 2 hook-ready rows | 1.0 | route | verifier | retrieval_lookup | retrieval_lookup |
| t4 | retrieval_lookup_route | calibration_result | support_insufficient | retrieval-lookup-local-accept-calibration-v1: 2 true rows below support gate 3 | 1.0 | route | gate | retrieval_lookup | retrieval_lookup |
| t5 | retrieval_lookup_route | feedback_stage | local_accept_calibration_support_insufficient | cpu-route-feedback-loop-v1 retrieval route row | 1.0 | route | stage | retrieval_lookup | retrieval_lookup |
| t6 | retrieval_lookup_route | verified_cpu_savings | zero_verified_accepts | cpu-route-feedback-loop-v1 retrieval route row: verified accepts 0 false accepts 0 | 1.0 | route | savings | retrieval_lookup | retrieval_lookup |
| t7 | retrieval_lookup_route | market_claim | disallowed | EXECUTOR_REVIEW_NOTES: candidate or scoreable rows alone are not savings | 1.0 | route | claim | retrieval_lookup | retrieval_lookup |
| t8 | feedback_scoreboard | counts_progress_by | unique_request_fingerprints | cpu-route-feedback-loop-v1: unique verified accepts remain 16 | 1.0 | scoreboard | rule | feedback | feedback |
| t9 | feedback_scoreboard | cpu80_status | not_achieved | cpu-route-feedback-loop-v1: unique gap to 80 is 784 calls | 1.0 | scoreboard | target | feedback | feedback |
| t10 | operator_catalog | ranks_retrieval_lookup_as | existing_profile_review_route | cpu-operator-catalog-v1: retrieval_lookup rank 15, verified accepts 0 | 1.0 | catalog | route | catalog | catalog |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| c1 | retrieval_lookup_route | uses_payload_artifact | retrieval_lookup_dry_run_trace | candidate answer | 1.0 | route | artifact | retrieval_lookup | retrieval_lookup |
| c2 | retrieval_lookup_route | uses_verifier_evidence | source_path_or_url_presence_verifier | candidate answer | 1.0 | route | verifier | retrieval_lookup | retrieval_lookup |
| c3 | retrieval_lookup_route | calibration_result | support_insufficient | candidate answer | 1.0 | route | gate | retrieval_lookup | retrieval_lookup |
| c4 | retrieval_lookup_route | feedback_stage | local_accept_calibration_support_insufficient | candidate answer | 1.0 | route | stage | retrieval_lookup | retrieval_lookup |
| c5 | retrieval_lookup_route | verified_cpu_savings | zero_verified_accepts | candidate answer | 1.0 | route | savings | retrieval_lookup | retrieval_lookup |
| c6 | retrieval_lookup_route | market_claim | disallowed | candidate answer | 1.0 | route | claim | retrieval_lookup | retrieval_lookup |
