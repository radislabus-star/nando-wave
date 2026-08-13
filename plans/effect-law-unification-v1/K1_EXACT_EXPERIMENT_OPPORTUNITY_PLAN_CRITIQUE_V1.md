# K1 Exact Experiment Opportunity Plan Critique V1

Status: SUPERSEDED by
`K1_EXACT_EXPERIMENT_OPPORTUNITY_PLAN_CRITIQUE_V2.md`. The original critique
found the causal-root defect but did not sufficiently split structural
authority, implementation stages, or executor handoff criteria.

Date: 2026-08-13.

## Verdict

The exact-opportunity direction is substantially safer than the rejected
coarse-family quotient, but its first draft still had authority and identity
holes. It was not ready for implementation until the P0 findings below were
resolved on paper.

## Findings And Repairs

| Priority | Finding | Why it breaks the plan | Incorporated repair |
|---|---|---|---|
| P0 | Current Raw Phase receives the full candidate freeze root | Timestamp, queue, or generation changes would alter a real identifier input while OpportunityRoot claimed they were irrelevant | V8 passes OpportunityRoot as the frozen Raw Phase and identifier evidence-domain root; full freeze root is provenance only |
| P0 | Terminal diagnostic was learner-produced | A compromised or buggy client could falsely label a root deterministic and suppress it forever | Dedicated authority route independently restores frozen inputs, reruns the shared pure evaluator, owns timestamp, and appends diagnostic plus verdict idempotently |
| P0 | Diagnostic root and causal result root were merged | Provenance metadata would make equal causal results appear different | Separate `IdentifierResultRoot` from `TerminalDiagnosticRoot` |
| P0 | Authority could reconstruct a queue from a client-supplied incomplete catalog | A client could omit the true leading candidate without violating queue parity over its own catalog | Freeze evidence-source prefixes; authority independently rebuilds join, motif, catalog, queue, and selected causal manifest |
| P1 | Missing collection artifacts had no explicit causal value | `None` could ambiguously mean no artifact, unread artifact, or implementation omission | Canonical empty relevant-artifact projection binds builder schema and exact support identities |
| P1 | Budget said terminal cooldown but used freeze timestamps | The stated policy could not be reconstructed from authority-owned fields | Policy is now a minimum interval between authority-sealed V8 freezes plus a trailing-24-hour count |
| P1 | Existing legacy generation at Phase B was unspecified | Enabling V8 could mutate or abandon an immutable V1-V7 generation | Legacy active generation finishes unchanged; V8 selection starts only when no legacy generation is active |
| P1 | Full checkpoint root could wake or perturb unrelated candidates | Unrelated collection activity could create false novel experiments | Checkpoint change wakes bounded reevaluation, but only exact relevant projection enters OpportunityRoot |
| P1 | Exact dedup could still spend unbounded work on always-new roots | Correct identity alone does not control research cost | One freeze per wake, five-minute freeze interval, 48 per trailing day, 256 rows per wake, explicit cooldown state |
| P2 | Old `211 -> 4` result was still psychologically attractive | It encourages treating consequence buckets as scientific families | Explicitly forbidden as an acceptance metric; coarse groups are dashboard-only |

## Remaining Risks That Tests Must Decide

### Exact roots may rarely repeat

Natural support manifests can be unique even when failures share a human
description. In that case exact dedup saves little. The diagnostic and research
budget still prevent blind burning, but the change earns deployment only if
production-copy replay shows either real duplicate savings or classified repair
information.

### Authority replay may be expensive

Independent catalog and identifier reconstruction strengthens authority but may
increase freeze latency and memory. Current and 10x replay measurements are a
deployment gate. Caching may optimize a source-root-stable result, but cache
bytes never become authority.

### Stable rejection reasons can hide detail

A closed enum is needed for deterministic roots, but an overly broad
`internal_unclassified` would recreate opaque failures. Such a result is
operational and cannot suppress an opportunity. Deployment is vetoed if
unclassified diagnostics dominate replay.

### This does not create Law #2

The repair can make search finite and explainable. It cannot guarantee a new
semantic class in existing traffic. Any plan that calls exact dedup Law #2 is
rejected.

## Final Recommendation

Proceed only in the phase order in
`K1_EXACT_EXPERIMENT_OPPORTUNITY_EXECUTION_PLAN_V1.md`. Do not salvage the
coarse-family ranking policy. Do not code through a paper `WATCH` or preflight
block. Do not deploy without the production-copy value gate and two-phase
reader/writer rollback fence.
