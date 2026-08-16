# Critique Of K2 Self-Formed Uncertainty Preregistration V5

Status: `V5 DRAFT REQUIRES REPAIR / NO CODE AUTHORITY`

Date: `2026-08-16`

Target: `K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md`

## Verdict

V5 repairs the missing confirm route and the one-probe oracle contradiction,
but its first draft still contains two impossible parity requirements and four
under-specified owner boundaries. Code must not start from this draft.

## Findings

| Severity | Finding | Consequence | Repair |
|---|---|---|---|
| P0 | V5 adds Confirm fields to `K2UncertaintyGeneratorRequestV1` while requiring Development response byte identity. | Adding serialized fields changes canonical Development request bytes even when values are absent. | Keep the existing Development request/response schemas byte-exact. Add separate Confirm request and Confirm response schemas. The generator process accepts either canonical schema without a shared untagged envelope. |
| P0 | V5 requires the complete Development response SHA-256 to remain unchanged, but the request binds the generator executable SHA-256. | Any repaired generator binary has a new hash, so request root and outer response root must change. The requirement is mathematically impossible. | Freeze public batch root, private batch root and denominator root as exact behavior oracles. Permit only mechanically recomputed request/outer-response identity rebinding to the successor executable hash. |
| P0 | Existing 32 + 4 + 16 controls are test-owned, while the sealed terminal requires executable control receipts. | R9B could again freeze logs without a manifest-bound runtime control owner. | Add a distinct control evaluator executable. It reruns all named controls against successor schemas and binds exact codes; K1-K12 additionally bind actual attempt roots where applicable. |
| P0 | Bounded oracle and baseline evaluation have no executable owner. | The terminal evaluator could calculate the comparator it is supposed to verify independently. | Add an oracle/baseline evaluator executable with no closure-planner or final-verifier elimination imports. Terminal evaluator consumes its receipt only. |
| P0 | The private truth is one batch file, yet the final verifier may mount only one case. | Some coordinator-side process must read and split truth after observation, reopening the answer-leak route. | Confirm owner must publish sixteen separately rooted final-truth case files before selection. Final verifier receives only the matching case file after that case observation vector freezes. |
| P1 | Public expected-denominator commitment transport is implicit. | Coordinator could receive the resolver or truth root as a shortcut. | Publish a dedicated public commitment receipt containing only experiment ID, public batch root and private expected-denominator commitment. |
| P1 | Attempt-directory existence is the only one-shot marker. | Manual deletion or partial directory failure could permit a second nonce. | Publish an append-only authorization-slot ledger outside the attempt directory. `SLOT_CLAIMED` is fsynced before attempt-directory mutation and permanently consumes the receipt root. |
| P1 | Crash between nonce file publication and `NONCE_COMMITTED` is unspecified. | Restart might generate a replacement nonce. | Project this prefix as `NONCE_CREATED_UNCOMMITTED` and terminal `INDETERMINATE`; retained nonce is hashed for diagnosis but never dispatched. No replacement is allowed. |
| P1 | R9B dry-run bypasses the confirm owner. | CSPRNG-adjacent pipe, environment clearing and output split remain unexercised. | Add `DevelopmentRehearsal` mode to the same owner executable. It uses the frozen development seed, the exact pipe and split topology, but creates no nonce and no Confirm request. |
| P1 | Resolver table still contains every action mapping for every case. | A resolver process can leak alternatives unless output is exact and independently checked. | Run one resolver invocation per selected plan item with one case table mounted. Validate that output contains exactly one preselected action/effect and no other mapping bytes. |
| P1 | Control timing is mixed. | Build-time static controls and attempt-bound dynamic controls could be double-counted or omitted. | Separate successor static controls `32+4+16` at R8B from attempt-bound V5 controls `K1-K12`; terminal binds both receipts with distinct denominators. |
| P2 | The executable topology omits authorization-slot, control and oracle owners. | Manifest completeness cannot be checked. | Add explicit slot-ledger, control-evaluator and oracle/baseline-evaluator owners, or assign their functions to named distinct executables without role sharing. |

## Acceptance For Repaired V5

```text
separate Development and Confirm wire schemas
identity-rebinding parity instead of impossible full-response parity
per-case resolver and final-truth artifacts
external one-shot slot ledger
explicit uncommitted-nonce crash terminal
same-binary Development rehearsal
distinct oracle/baseline and control evaluators
static versus attempt-bound control denominators separated
```

Only after these repairs may owner-bounded structural gates and implementation
preflight decide whether code can start.
