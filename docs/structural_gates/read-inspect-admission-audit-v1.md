# read-inspect-admission-audit-v1

## query

Check whether the read_context_path / read_inspect route has a safe request-side
admission policy after output evidence was attached.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| read_inspect_admission_audit | has_report | read-inspect-admission-audit-v1.report.json | admission audit report |
| read_inspect_admission_audit | reads | read-inspect-output-evidence-v1.trace.jsonl | evidence trace |
| read_inspect_admission_audit | reads | codex_history_fingerprints_only | /home/ubu/.codex/history.jsonl |
| read_inspect_admission_audit | has_hook_ready_rows | 9 | report |
| read_inspect_admission_audit | has_rows_with_prompt_features | 9 | report |
| read_inspect_admission_audit | has_label_true_rows | 1 | report |
| read_inspect_admission_audit | has_label_false_rows | 8 | report |
| read_inspect_admission_audit | has_minimum_true_support | 3 | report |
| read_inspect_admission_audit | has_robust_safe_policy_found | false | report |
| read_inspect_admission_audit | has_singleton_safe_policy_found | false | report |
| read_inspect_admission_audit | has_best_robust_true_accepts | 0 | report |
| read_inspect_admission_audit | has_best_singleton_true_accepts | 0 | report |
| read_inspect_admission_audit | writes_raw_prompt_text | false | report |
| read_inspect_admission_audit | writes_raw_response_text | false | report |
| read_inspect_admission_audit | uses_response_text_for_features | false | report |
| read_inspect_admission_audit | uses_target_labels_for_runtime | false | report |
| read_inspect_admission_audit | uses_proof_labels_for_runtime | false | report |
| read_inspect_admission_audit | enables_local_accepts | false | report |
| read_inspect_admission_audit | allows_market_claim | false | report |
| read_context_path | source_route | read_inspect | agent-loop-profile-registry-v1.report.json |
| read_context_path | current_unique_verified_accepts | 0 | agent-loop-profile-registry-v1.report.json |
| cpu80_state | current_verified_cpu_accepts | 26 | agent-loop-profile-registry-v1.report.json |
| cpu80_state | verified_gap_to_80 | 774 | agent-loop-profile-registry-v1.report.json |
| read_context_path | has_request_side_safe_admission | false | robust_safe_policy_found=false |
| read_context_path | should_lower_threshold | false | no safe policy and 8 false rows |
| read_context_path | should_promote_local_accept | false | local_accepts_enabled=false |
| read_context_path | should_split_or_capture_state | true | next_engineering_debt |
| market_claim | remains_blocked | true | market_claim_allowed=false |
| broad_answer_or_explain | remains_quarantined | true | executor notes |

## claim_boundary

```text
This packet records a negative admission result, not a new savings claim.
It proves only that current read_inspect request-side features do not separate
verified true rows from false rows. Any future local accept path must use a
separate promoted shadow artifact with provider cost, false_accepts=0,
unverified_shadow_accepts=0, unique attribution over exact cache, and rollback.
```

## nanda_structural_gate

```text
report:
  target/nando-wave/real-traffic-shadow/read-inspect-admission-audit-v1.nanda.json

verdict:
  PASS

complexity_score:
  51

stable_triads:
  29

weak_triads:
  0

conflicts:
  0

evidence_gaps:
  0

interpretation:
  negative_admission_boundary_recorded
```
