# K2 Self-Formed Uncertainty V4 Adversarial Critique

Status: `COMPLETE / REPAIRS REQUIRED BELOW ARE INCORPORATED IN V4`

Date: `2026-08-15`

Authority: `FALSE`

Target: `K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md`

## Verdict

The V4 closure idea is the smallest honest repair for the R7 discrepancy. It
preserves the complete V3 frontier and byte-identical first winner, adds no
outcome-conditioned choice, and can distinguish the factorized `2 x 2` model
set with at most two independently executed probes.

The draft was not implementation-ready. It left one terminal route, several
denominator identities, and crash chronology implicit. Those gaps could allow
an implementation to omit a difficult case, select a second probe from an
incomplete census, or lose the distinction between execution and observation.

## P0 Findings

| ID | Attack | Required closure |
|---|---|---|
| P0-1 | `CLOSURE_UNAVAILABLE` had no immutable artifact or position in the all-case barrier. A failed case could disappear instead of failing the batch. | Freeze a `K2UncertaintyClosureCensusV1` for every case. It has exactly one of `SINGLE_PROBE`, `TWO_PROBE`, or `CLOSURE_UNAVAILABLE`; all sixteen census roots enter the batch barrier and any unavailable disposition terminates before dispatch. |
| P0-2 | A completion denominator root alone does not prove membership completeness. | Bind the canonical sorted representative roots, excluded first root, exact candidate count `representative_count - 1`, every candidate root, and a checked one-to-one membership receipt. Reject omissions, additions, duplicates, and foreign roots. |
| P0-3 | A completion planner could reuse outcome bytes or private topology through an auxiliary request even if the ranking tuple is public. | Give the planner one public request schema containing only frozen models, complete representatives, first winner, and exact prediction witnesses. Reject private mapping, family, matched pair, expected outcome, observed outcome, or safety resolution fields. |
| P0-4 | The draft did not require the selected second probe to be the direct global completion winner in an implementation-independent process. | Independent preverification reconstructs every completion candidate, ranking tuple, closing disposition, and selected second root without importing planner logic. Both census roots and winner roots must match before the batch barrier. |
| P0-5 | A two-probe plan could dispatch the second probe only after inspecting the first outcome. The identity would be frozen, but the execution decision would still be outcome-conditioned. | Durably dispatch the whole ordered plan before any observer result is accepted. Workers use separate immutable workspaces. Observation may be collected in ordinal order, but no worker dispatch occurs after the first observation. |
| P0-6 | `same-identity redispatch forbidden` plus a crash after execution but before observation had no explicit terminal. Retrying could duplicate an intervention; continuing could invent an outcome. | Model every legal journal prefix. A dispatched probe without a durable matching observation becomes `INDETERMINATE_EXECUTION` and permanently fails the case. No redispatch, elimination, or cleanup-before-terminal is allowed. |
| P0-7 | The final verifier could share the closure planner and reproduce the same AND, ranking, or denominator bug. | Final verification imports canonical schemas and hashing only. It independently rebuilds raw predictions, quotient membership, first selection, completion census, ordered vector, and joint elimination. |

## P1 Findings

| ID | Risk | Required closure |
|---|---|---|
| P1-1 | `joint equality` could be tampered into a non-equivalence relation. | Recompute each of six bits as `first_equal AND second_equal`, derive connected equivalence classes independently, and reject any stored partition mismatch. |
| P1-2 | Single-probe cases had an underspecified completion root. | Freeze the canonical empty completion-candidate set, zero candidate count, absent second root, and first partition `[1,1,1,1]`. |
| P1-3 | Two-probe candidates could silently exceed aggregate safety budgets. | Use checked addition for cumulative risk and cost, bind both component accounting receipts, and reject overflow or totals above `20`. |
| P1-4 | Plan roots could be cyclic or ambiguous because census, preverification, dispatch, and journal roots were not layered. | Freeze roots in a strict DAG: frontier -> first tournament -> completion candidates -> census -> independent preverification -> closure plan -> all-case barrier -> dispatch -> observations -> elimination -> cleanup. |
| P1-5 | A swapped ordinal could still carry valid probe and observation roots. | Bind ordinal, probe root, initial-manifest root, worker request root, observer request root, observation root, and vector index in every receipt. |
| P1-6 | Separate workspaces were stated but not made a byte identity. | Derive one fresh workspace identity per `(case root, plan root, ordinal)` and reject shared paths, post-state carry-over, or initial-manifest mismatch. |
| P1-7 | The draft deferred protocol-size discovery until implementation. | Measure the largest development request before code freeze. Keep the `1,048,576` byte limit; use root-addressed immutable artifacts if the direct request would exceed it. |
| P1-8 | Existing J1-J10 controls did not cover unavailable closure, direct-winner mismatch, checked budgets, or crash prefixes. | Extend the control set to J1-J16 and require exact named error dispositions. |

## Minimality Check

For four models, the first predecessor winner is already globally minimax over
the complete representative set. If any one probe has partition
`[1,1,1,1]`, that probe outranks every non-singleton partition, so V4 uses one
probe. Otherwise at least two probes are necessary. V4 enumerates every second
representative and accepts only an exact joint singleton partition, so a
successful two-probe plan is minimal within the frozen language.

This is plan closure, not learned strategy. The outer algorithm is a fixed
bounded enumerator introduced by the experimenter after the V2/V3
impossibility was exposed.

## Required Control Delta

Retain V2 `32/32`, V3 `T1-T4`, and V4 `J1-J10`. Add:

```text
J11 unavailable closure omitted from all-case barrier rejected
J12 wrong completion count or membership root rejected
J13 non-global second winner or wrong disposition rejected
J14 cumulative risk/cost overflow or budget excess rejected
J15 invalid crash prefix, redispatch, or observation-before-plan rejected
J16 stored joint partition inconsistent with independently recomputed classes rejected
```

Generic parse failure or an unrelated error does not count.

## Claim Boundary

Passing V4 can establish only bounded generated capability:

```text
complete induced four-model uncertainty
-> complete public probe frontier
-> immutable outcome-blind closure plan of length one or two
-> isolated interventions and independent observations
-> exact one-class elimination
```

It cannot establish Natural K2, a learned strategy, open-ended experiment
design, natural-traffic transfer, Wave-causal grokking, K1 admission, product
authority, or deployment readiness.

## Final Assessment

V4 may proceed to owner-bounded structural gates and implementation preflight
only after P0-1 through P0-7 and P1-1 through P1-8 are present in the frozen
paper contract. Structural PASS remains coherence-only and authority remains
false.
