# S1C Pre-Action Decision Owner Critique V1

Status: `ADVERSARIAL REVIEW / IMPROVEMENTS APPLIED / SPLIT GATE 7 OF 7 PASS`

Date: 2026-08-11

Reviewed artifact: `S1C_PRE_ACTION_DECISION_OWNER_PREREGISTRATION_V1.md`

## 1. Verdict Before Repair

The route was directionally correct but not initially implementation-safe. It
closed the missing owner without inventing goals, yet several details could
still have produced false K2 alternatives, a second serving decision, a torn
authority snapshot, or an ambiguous terminal verdict.

The review therefore rejected a direct jump to code. The repaired plan is
acceptable for fail-closed structural verification only. It still grants no
runtime or scientific authority.

## 2. Findings And Repairs

| Severity | Finding | Failure if ignored | Applied repair |
|---|---|---|---|
| P0 | Two ACTIVE product packages could be mistaken for two K1 actions. | A legacy or partial-law package would manufacture a meaningful alternative and a false K2 vocabulary. | Available actions now require the latest anchored `k1_unit_eligible=true` certification entry in the same frozen admitted snapshot. Current truth is one K1 action, not two. |
| P0 | A package-specific root could masquerade as action semantics. | Renaming or repackaging one operator would look like a new action. | Added package-neutral `K1ActionContractProjectionV1`; package/bundle/admission bindings remain opaque execution joins. |
| P0 | A second applicability evaluator was an easy implementation shortcut. | Evidence and serving could disagree while each path looked locally correct. | Frozen one `PreparedResponseEvaluation`, consumed once by `execute_prepared`; compatibility `execute` uses the same route. |
| P0 | The parent wording required persistence before ranking, while complete applicability and current top-8 ranking share one evaluator. | Implementers would either duplicate the evaluator or silently violate the temporal contract. | Goal freezes before evaluation; ranking stays private; durable precommit must precede selected-action publication and execution. No ranking information reaches the binder or ledger. |
| P0 | A precommit cannot include the physical receipt created by writing that same precommit without a circular root. | The design would require a second loosely bound record or unverifiable self-reference. | Durability receipt is now a deterministic recovery projection from the synced framed record coordinates and precommit root. |
| P0 | The initial `PASS` condition overlapped `EMPTY_ALTERNATIVE_SURFACE`. | One-action episodes could receive two terminal interpretations. | Terminal precedence is now `VETO -> EMPTY_GOAL -> EMPTY_ALTERNATIVE -> INSUFFICIENT_LINEAGES -> PASS`; PASS requires alternative-bearing episodes from at least two independent lineages. |
| P0 | Goal evidence could be a hash of free text. | Hashing hides the leak but does not remove free-text semantic authority. | Free text, embeddings, and LLM classification are forbidden even when hashed. Only exact typed protocol facts or mechanically reproducible bounded fields are eligible. |
| P1 | Registry, admission, and certification files could be read across different revisions. | A package might be applicable under one snapshot and epistemically eligible under another. | S1C requires one immutable `DecisionAuthoritySnapshotV1`, built off-path with before/after fingerprints and published atomically beside `ResponseExecutor`. Torn refreshes are rejected. |
| P1 | Reading and validating the certification ledger on every request would add IO and race exposure. | Hot latency and idle CPU would regress; filesystem failure could influence serving. | Certification projection is prevalidated during cache refresh. Request evaluation reads only the immutable in-memory snapshot. |
| P1 | The parent schema had no temporal selected-action binding. | A valid selected root could be attached to the wrong precommit after the fact. | Added required `SelectedActionBindingReceiptV1` referencing precommit, source-neutral action root, opaque execution binding, runtime receipt, and post-precommit sequence. |
| P1 | The 256-root action cap was not linked to unchanged top-8 serving. | Capacity pressure could suppress or alter a valid local response. | `CAPACITY_EXHAUSTED` censors only K2 evidence; the private legacy top-8 result remains executable. |
| P1 | `AvailableActionContractsV1` rejects an empty action vector. | A no-action request could be serialized as a misleading ABSTAIN-only decision. | No applicable certified action now yields `NO_APPLICABLE_K1_ACTION` and no decision precommit. It remains in denominator counters. |
| P1 | The horizon could expand into an arbitrary multi-turn timeout. | Later outcomes could be selected to satisfy an earlier vague goal. | S1C V1 is restricted to the same-request terminal receipt. Multi-step horizons require a new preregistration. |
| P1 | The 10,000-row window did not define which request boundary owned the denominator. | Gateway ingress, actor-eligible requests, and persisted rows could be mixed. | Added separate counters and defined the terminal denominator as ordinary requests that reach the pre-action response decision boundary after the watermark. Total ingress remains a separate context denominator. |
| P1 | A framed journal without quota and active-reference retention could grow forever or delete unsettled evidence. | Server disk or scientific lineage would be lost. | Added 32 KiB record, 64 MiB segment, 2 GiB quota, append cursor, anchored checkpoint requirement, and fail-closed capture disablement. |
| P1 | Sync latency could be averaged into cheap no-goal requests. | A slow durability path would look performant under a mostly empty goal surface. | Split no-goal and eligible-sync latency denominators, each with immutable p99 and hard ceilings. |
| P1 | The existing exact-Wave precommit already performs package-scoped control evaluations. | Reordering it could contaminate goal binding or change mechanism evidence. | S1C precommit occurs first; existing exact-Wave precommit remains unchanged and still precedes actual execution. It is not an S1C input. |
| P2 | Dashboard package compatibility rows currently label both ACTIVE packages as legacy while the anchored K1 gate reports one certified law. | UI summaries could be used as certification authority. | The anchored certification ledger is frozen as authority; dashboard compatibility rows are observational only. UI repair, if needed, is a separate later slice. |
| P2 | With one K1 law, the expected near-term result is probably no meaningful alternative. | The team could interpret an honest empty result as another indefinite blocker. | Added a 72-hour/10,000-decision-surface terminal and explicit `EMPTY_ALTERNATIVE_SURFACE`; waiting cannot continue indefinitely. |

## 3. Repaired Route

```text
pre-action capture roots
-> exact goal binder
-> immutable goal contract
-> atomically published DecisionAuthoritySnapshotV1
   |- admitted ResponseExecutor
   `- latest anchored K1 action index
-> one PreparedResponseEvaluation
-> complete package-neutral action quotient
-> synced DecisionContractPrecommitV1
-> existing exact-Wave precommit, unchanged
-> consume exact prepared evaluation
-> SelectedActionBindingReceiptV1
-> independent terminal consequence verification
-> finite append-cursor census
```

## 4. Strongest Remaining Risks

These are expected experiment outcomes, not reasons to weaken the plan:

1. Ordinary traffic may expose no exact typed goal. The terminal result is
   `EMPTY_GOAL_SURFACE`.
2. K1 currently has one eligible action. Exact goals may exist while every
   action set lacks a meaningful alternative. The result is
   `EMPTY_ALTERNATIVE_SURFACE`.
3. Honest per-record durability may miss the 5 ms p99 or 20 ms hard sync-path
   budget. The result is VETO and capture remains off.
4. The current product package may lack one of the roots required to reproduce
   `ProgramSemanticClassDescriptorV1`. It is censored as
   `ACTION_PROJECTION_INCOMPLETE`; missing identity fields are not synthesized.
5. A second K1 law may arrive during the bounded window. The authority snapshot
   changes only at a new atomic refresh; existing precommits remain bound to
   their original certification root.

## 5. Explicitly Rejected Alternatives

```text
LLM goal extraction from the prompt
goal inferred from selected package or successful output
all ACTIVE packages treated as K1 actions
package ID used as semantic action identity
shadow evaluator beside the serving evaluator
re-evaluate after persistence instead of consuming prepared state
asynchronous best-effort precommit called durable
synthetic request used to populate the natural denominator
unbounded waiting for a second law or exact goal
raising latency, disk, action-count, or time budgets after seeing data
```

## 6. Decision

The repaired S1C-0 plan passed seven coherent structural routes after the
initial over-grouped packets correctly returned VETO and the aggregate packets
correctly returned size WATCH. Final conflicts, weak triads, mixed ownership,
and repair queues are empty. This coherence-only result allows the documentation
checkpoint and S1C-1 as the next slice; authority remains false.
