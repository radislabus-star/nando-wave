# Planning Next-Step Artifact Progress V1 Structural Gate

## Claim

Planning-next-step artifact progress V1 attaches tool-call fingerprints and
allows a true verifier label only for successful nando-wave project-progress
tool evidence. It does not enable local accepts or market savings.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| artifact progress command | reads | planning output evidence trace | target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.trace.jsonl |
| artifact progress command | reads | local Codex rollout tool events | /home/ubu/.codex/sessions |
| artifact progress command | writes | planning artifact-progress trace | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.trace.jsonl |
| artifact progress command | writes | tool-call fingerprints | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.trace.jsonl#tool_call_fingerprints |
| artifact progress command | does_not_write | raw prompt text | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#raw_prompt_text_written |
| artifact progress command | does_not_write | raw response text | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#raw_response_text_written |
| artifact progress command | does_not_write | raw tool outputs | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#tool_outputs_written |
| artifact progress command | does_not_use | target labels | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#target_labels_used |
| artifact progress command | does_not_use | proof labels | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#proof_labels_used |
| artifact progress verifier | requires | successful nando-wave project-progress tool | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| artifact progress verifier | allows | 1 true label | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#verified_true_events |
| artifact progress verifier | assigns | 6 false labels | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#verified_false_events |
| artifact progress verifier | indexes | 1 successful tool event | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#tool_events_indexed |
| shadow report | reports | 0 nando shadow accepts | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.shadow-report.json#nando_shadow_accepts |
| shadow report | reports | 0 verified safe accepts | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.shadow-report.json#verified_safe_accepts |
| shadow report | reports | 0 false accepts | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.shadow-report.json#false_accepts |
| audit report | reports | 7 verification hook ready events | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json#verification_hook_ready_events |
| audit report | reports | 0 verified CPU accept eligible events | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json#verified_cpu_accept_eligible_events |
| market claim | remains | false | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json#market_claim_allowed |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| tool-backed planning label | means | one real project-progress turn was verified | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json#verified_true_events |
| tool-backed planning label | does_not_mean | CPU local accept | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.shadow-report.json#nando_shadow_accepts |
| tool-backed planning label | does_not_mean | market savings | target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json#market_claim_allowed |
| safe-policy calibration | remains | next engineering debt | docs/EXECUTOR_REVIEW_NOTES.md |
