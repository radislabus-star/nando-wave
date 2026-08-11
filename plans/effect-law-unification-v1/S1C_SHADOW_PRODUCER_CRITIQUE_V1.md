# S1C Shadow Producer Adversarial Critique V1

Status: `FINAL CRITIQUE / REPAIRS ACCEPTED INTO PREREGISTRATION / NO CODE`

Date: 2026-08-11

Reviewed artifact: `S1C_SHADOW_PRODUCER_PREREGISTRATION_V1.md`

Parent commit: `d43fc8cd4fcb73e6fb15bcde143a68960272425e`

## 1. Review Objective

This review tries to falsify the S1C-2 paper route before code. It focuses on
ways a shadow producer could quietly become post-hoc goal inference, run a
second evaluator, combine torn authority snapshots, report asynchronous writes
as durable, omit terminal receipts, or acquire deployment and training
authority.

The review does not evaluate natural K2 evidence. None exists yet.

## 2. Findings And Accepted Repairs

| Severity | Finding | Failure mode | Accepted repair in final preregistration |
|---|---|---|---|
| P0 | A goal root could be a hash of request text. | Free-text semantic inference would be hidden behind a content address. | Forbidden request text, embeddings, and classification even when hashed; limited ingress to three exact pre-action typed classes. |
| P0 | The goal could be created after ranking from the selected package or margin. | The evidence would explain the decision it had already observed. | Goal freeze precedes `evaluate_pre_action`; package, rank, margin, selected action, actor/verifier output, and future rows are absolute denylist inputs. |
| P0 | Capture could call `evaluate_pre_action`, then serving could call `execute`. | Evidence and product would be based on two evaluations and could diverge. | Frozen one-evaluator route consumes the exact same `PreparedResponseEvaluation`; after preparation every failure still calls `execute_prepared` once. |
| P0 | Executor and K1 index could be loaded or refreshed separately. | Available actions could be certified under a different registry/admission epoch from the executed package. | One off-path `ResponseDecisionSnapshotV1`, double fingerprint read, one cache swap, one request-thread clone; request threads read no authority files. |
| P0 | An append call could be called durable before `sync`. | Crash recovery could lose the pre-action fact and leave only post-action evidence. | Durability begins only after framed append plus sync; selected action and satisfaction also require sync and restart validation. |
| P0 | Only the precommit might be persisted. | The project could count intended decisions without durable selected action or terminal truth. | Added separate selected-action-binding and goal-satisfaction ledger prefixes, ordered joins, recovery tests, and explicit censors. |
| P0 | HTTP success or actor verification could be treated as goal satisfaction. | Operational success would be mislabeled as semantic goal success. | Reproduce the frozen exact predicate over the independently observed consequence; exact false is durable negative evidence. |
| P0 | A shadow error could fall back to `executor.execute`. | Persistence failure would trigger a second evaluator and perhaps a different response. | After preparation, all capture failures consume the same prepared object; compatibility execution is allowed only before evidence evaluation. |
| P1 | A missing natural goal source could be concealed by a default goal. | S1C-4 could falsely report a populated decision surface. | Exact `MISSING_EXACT_GOAL`, no evaluation/write on no-goal fast path, and explicit allowance for terminal `EMPTY_GOAL_SURFACE`. |
| P1 | K1 package identity could leak into public action semantics. | K2 would learn package/version IDs rather than transferable action contracts. | Public evidence carries source-neutral action and binding roots; package identity stays inside the opaque execution join. |
| P1 | Censor events could be cited as durable evidence after the journal failed. | Diagnostic telemetry would be promoted into a scientific denominator. | Censors are diagnostic only; failed persistence cannot yield a durable episode, and S1C-4 owns the exact append-cursor denominator. |
| P1 | Enabling capture could open local-accept or certification authority. | An observational feature flag could alter product or epistemic authority. | One false-by-default flag; explicit false training, phase, certification, admission, and deployment authority. |
| P1 | Existing exact-Wave capture could be reclassified as S1C evidence. | Mechanism evidence and grounded-decision evidence would be merged. | Existing exact-Wave precommit remains byte/behavior unchanged and has no S1C goal, action, or satisfaction authority. |
| P1 | New public accessors could expand into a second runtime API. | Implementation would redesign response actor under a narrow shadow task. | Allowed only capture evidence needed for the selected K1 join; any other public API or schema outside the file list requires a paper revision. |
| P1 | Per-request censor fsync could violate the no-goal latency budget. | Shadow observation could slow all ordinary traffic. | No-goal path performs no S1C evaluation and no synchronous journal write; S1C-4 uses the existing source denominator. |
| P1 | Separate ledger quotas could each consume 2 GiB. | The slice could multiply the frozen disk budget. | The 2 GiB quota applies to all grounded-decision prefixes combined. |
| P2 | A source PASS could be reported as live capture PASS. | Uninstalled code could be mistaken for production evidence. | Terminal name is `S1C2_SOURCE_PASS`; activation and deployment remain false until S1C-3. |
| P2 | Status-only plan repair could rewrite old receipt meaning. | Historical authority could be altered after the fact. | Only canonical current-state pointers change; immutable receipts and frozen criteria remain untouched. |

## 3. Rejected Alternatives

```text
infer goal from ordinary request prose
  rejected: semantic authority comes from the model, not observed typed truth

use package output as the expected goal
  rejected: post-hoc tautology

run evaluator once for evidence and once for serving
  rejected: no shared decision identity

load certification ledger on each request
  rejected: latency and torn-authority route

append selected action asynchronously and call it durable
  rejected: crash can erase the evidence after HTTP completion

count HTTP 200 as satisfaction
  rejected: operational outcome is not the frozen goal predicate

write every missing-goal censor synchronously
  rejected: violates no-goal budget and adds no scientific episode

activate capture during source verification
  rejected: deployment belongs to S1C-3
```

## 4. Residual Scientific Risks

The paper contract cannot guarantee that ordinary traffic exposes an exact
typed goal. The likely first natural census result may be
`EMPTY_GOAL_SURFACE`. That would falsify the current traffic source as a K2
training surface without falsifying the shadow implementation.

One K1-certified action currently cannot provide a meaningful multi-action
composition claim. ABSTAIN is a control action, not a second K1 law. S2 remains
blocked until the canonical S1C exit criteria and independent-lineage criteria
are met.

The S1C-2 source candidate will still require resource measurements. Paper
coherence cannot predict fsync tail latency on the mini-PC.

## 5. Review Verdict

All P0/P1 findings have an explicit repair in the final preregistration. The
remaining uncertainty is observable and terminal rather than hidden by a
permissive fallback.

```text
goal ingress boundary                 READY FOR STRUCTURAL GATE
one-evaluator route                   READY FOR STRUCTURAL GATE
atomic authority snapshot             READY FOR STRUCTURAL GATE
persistence and serving independence  READY FOR STRUCTURAL GATE
selected action and satisfaction      READY FOR STRUCTURAL GATE
slice authority boundary              READY FOR STRUCTURAL GATE
code started                          no
runtime changed                       no
deployment allowed                    false
authority_ready                       false
```
