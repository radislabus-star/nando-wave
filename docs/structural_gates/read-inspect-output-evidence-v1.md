# read-inspect-output-evidence-v1

## query

Check whether the read_inspect route now has deterministic output evidence
without being promoted to verified CPU savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| read_inspect_output_evidence | has_report | read-inspect-output-evidence-v1.report.json | output evidence report |
| read_inspect_output_evidence | has_trace | read-inspect-output-evidence-v1.trace.jsonl | output evidence report |
| read_inspect_output_evidence | input_trace | read-inspect-payload-dry-run-v1.trace.jsonl | output evidence report |
| read_inspect_output_evidence | has_output_evidence_matched_events | 9 | output evidence report |
| read_inspect_output_evidence | has_deterministic_verification_events | 8 | output evidence report |
| read_inspect_output_evidence | has_verifier_not_applicable_events | 1 | output evidence report |
| read_inspect_output_evidence | has_verified_true_events | 1 | output evidence report |
| read_inspect_output_evidence | has_verified_false_events | 8 | output evidence report |
| read_inspect_output_evidence | writes_raw_prompt_text | false | output evidence report |
| read_inspect_output_evidence | writes_raw_response_text | false | output evidence report |
| read_inspect_output_evidence | uses_response_text_at_analysis_time | true | output evidence report |
| read_inspect_output_evidence | uses_target_labels | false | output evidence report |
| read_inspect_output_evidence | uses_proof_labels | false | output evidence report |
| read_inspect_output_evidence | enables_local_accepts | false | output evidence report |
| read_inspect_output_evidence | allows_market_claim | false | output evidence report |
| read_inspect_shadow | has_report | read-inspect-output-evidence-v1.shadow-report.json | shadow report |
| read_inspect_shadow | has_total_llm_calls | 1000 | shadow report |
| read_inspect_shadow | has_exact_cache_hits | 53 | shadow report |
| read_inspect_shadow | has_nando_shadow_accepts | 0 | shadow report |
| read_inspect_shadow | has_verified_safe_accepts | 0 | shadow report |
| read_inspect_shadow | has_false_accepts | 0 | shadow report |
| read_inspect_shadow | has_incremental_reduction_vs_exact_cache_milli | 0 | shadow report |
| read_inspect_shadow | has_p99_shadow_score_latency_ns | 249929 | shadow report |
| read_inspect_audit | has_report | read-inspect-output-evidence-v1.verification-hook-audit.report.json | verification audit |
| read_inspect_audit | has_operator_candidate_calls | 12 | verification audit |
| read_inspect_audit | has_scoreable_candidate_calls | 12 | verification audit |
| read_inspect_audit | has_verification_hook_ready_events | 9 | verification audit |
| read_inspect_audit | has_verified_cpu_accept_eligible_events | 0 | verification audit |
| read_inspect_audit | has_candidates_missing_output_evidence | 3 | verification audit |
| read_inspect_audit | has_candidates_missing_explicit_verification | 3 | verification audit |
| read_inspect_audit | has_candidates_missing_provider_cost | 12 | verification audit |
| read_inspect_claim | has_market_claim_allowed | false | reports |
| feedback_after_read_output_evidence | has_report | cpu-route-feedback-loop-v1.report.json | feedback report |
| feedback_after_read_output_evidence | has_operator_candidate_calls | 314 | feedback report |
| feedback_after_read_output_evidence | has_scoreable_candidate_calls | 105 | feedback report |
| feedback_after_read_output_evidence | has_verification_hook_ready_events | 81 | feedback report |
| feedback_after_read_output_evidence | has_verified_cpu_accept_eligible_events | 8 | feedback report |
| feedback_after_read_output_evidence | has_verified_gap_to_80_calls | 792 | feedback report |
| historical_mixed_v2_feedback | has_verified_cpu_accepts | 17 | executor notes |
| numeric_boundary | forbids_mixing | 8_and_17_as_single_claim | executor notes |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| read_inspect_route | has_verifier_integration | true | output evidence command/report |
| read_inspect_route | is_verified_savings_progress | false | shadow accepts zero and local accepts disabled |
| read_inspect_route | has_local_accept_promotion_support | false | only one verifier-true event |
| read_inspect_route | keeps_local_accepts_disabled | true | output evidence/shadow/audit |
| read_inspect_route | keeps_false_accepts_zero | true | shadow/audit |
| read_inspect_route | requires_next_calibration | true | singleton support is not promotable |
| market_claim | remains_blocked | true | market_claim_allowed false and verified accepts zero for read_inspect |

## nanda_structural_gate

```text
report:
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.nanda.json

verdict:
  VETO

complexity_score:
  89

explanation:
  At least one candidate group has weak route coherence.
  At least one candidate triad has weak composite-mode support.
  Task exceeds target or hard limits and should be split.

weak_triads:
  7

evidence_gaps:
  0

interpretation:
  proof_shape_debt_not_runtime_failure
```

The JSON route/shadow/audit reports remain the numeric authority for this rung.
Do not cite this packet as a structural PASS.
