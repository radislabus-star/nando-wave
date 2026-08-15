# K2 Self-Formed Uncertainty Preregistration V4

Status: `FROZEN AFTER ADVERSARIAL CRITIQUE / IMPLEMENTATION NOT YET AUTHORIZED`

Date: `2026-08-15`

Authority: `FALSE`

## 1. Scope And Precedence

V2 and V3 remain canonical except where V4 explicitly supersedes single-probe
closure for factorized `2 x 2` uncertainty. The R7 discrepancy is preserved at:

```text
K2_SELF_FORMED_UNCERTAINTY_R7_DISCREPANCY_2026-08-15.md
```

V4 does not reopen the language, learner, four-model semantic quotient, raw
probe denominator, complete scorer-sufficient frontier, predecessor selector
source, authority boundary, nonce chronology, or one-attempt rule.

The adversarial critique is preserved at:

```text
K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V3.md
```

## 2. Corrected Question

The experiment asks:

> Can Nanda induce a complete four-class semantic model set from incomplete
> transitions, detect when the unchanged single-probe scorer cannot close that
> uncertainty, mechanically form the smallest bounded public probe plan needed
> to close it, freeze the entire plan before outcomes, and reduce every case to
> the private true semantic class through independently observed effects?

The permitted plan length is `1..=2`. A two-probe plan is not called a learned
strategy, Natural K2, open-ended planning, or Wave-causal grokking.

## 3. Preserved First Selection

For every case, the first probe remains exactly the V3 tournament winner:

```text
complete representative set
-> exact eight-probe tournament requests
-> byte-identical predecessor selector
-> direct complete-frontier winner parity
-> first probe root
```

Selector source SHA-256 remains:

```text
733b9b59fdfd7e2b5ed68461da89a27c84f04ade2e4e51ae5243dbb7175ef390
```

No V4 adapter may change first-probe eligibility, predictions, minimax score,
pair separation, risk, cost, stable tie-break, or winner.

## 4. Closure Need

Before any outcome exists, the planner derives the first winner's partition
from its four exact precommitted predictions.

```text
largest partition == 1     plan length = 1
largest partition > 1      completion census required
```

The branch is determined entirely from public prediction equality. Topology
family, matched-pair index, private mapping, expected outcome, and true class
are forbidden inputs.

## 5. Complete Completion Census

When completion is required, the first probe remains fixed. Every other member
of the complete V3 representative set becomes exactly one second-probe
candidate. No candidate is sampled, beam-pruned, named, or hand-selected.

For each unordered model pair `i,j`, joint equality is:

```text
joint_equal[i,j]
= first_equal[i,j] AND second_equal[i,j]
```

This is sufficient because two models produce the same ordered observation
vector exactly when they agree on both component outcomes. Exact manifests and
absolute outcome roots remain retained as witnesses but are not class labels.

Each completion candidate binds:

```text
first probe root
second probe root
first and second prediction roots
joint six-bit equality matrix
joint partition sizes
joint minimax eliminated
joint pair separation
checked cumulative risk
checked cumulative cost
candidate root
authority false
```

Duplicate probe roots and any candidate not derived from two complete frontier
members are rejected.

`K2UncertaintyClosureCensusV1` is frozen for every case, including cases that
cannot close. It binds:

```text
case root
frontier root and canonical sorted representative roots
representative count
first tournament root and first probe root
first equality matrix and first partition
completion-required bit
canonical sorted second-probe candidate roots
candidate count, exactly representative count - 1 when required, else zero
one-to-one representative-to-candidate membership receipt
all completion-candidate roots and denominator root
disposition: SINGLE_PROBE | TWO_PROBE | CLOSURE_UNAVAILABLE
selected second root, optional
selected joint partition, optional
planner executable root
authority false
census root
```

For `SINGLE_PROBE`, the first partition is exactly `[1,1,1,1]`, the completion
candidate set is canonically empty, and the second root is absent. For
`TWO_PROBE`, completion is required, every representative other than the first
appears exactly once, and the selected joint partition is `[1,1,1,1]`.
`CLOSURE_UNAVAILABLE` retains the complete candidate census and has no plan or
dispatch authority.

The planner request contains only the frozen four public models, complete
representative set, first winner, and exact public prediction witnesses.
Topology family, matched-pair index, private mapping, private truth, expected
outcome, observed outcome, and private safety resolution are schema-forbidden.

## 6. Completion Ranking

The outcome-blind completion ranking tuple is frozen as:

```text
joint minimax eliminated       descending
joint pair separation          descending
cumulative risk                ascending
cumulative cost                ascending
second probe root              ascending
```

The first probe is fixed, so this ranking cannot rewrite or compete with the
predecessor winner. A direct complete completion census and an independent
preverification implementation must select the same second root.

The selected joint partition must be `[1,1,1,1]`. If the complete census has no
closing candidate, the case terminates `CLOSURE_UNAVAILABLE` before dispatch.
It may not execute a partial plan and may not weaken the final residual rule.

Independent preverification reconstructs the complete candidate set, every
joint equality bit, every ranking tuple, the disposition, and the selected
second root without importing planner logic. Its census root and selected root
must match the planner before the all-case barrier.

## 7. Immutable Plan

`K2UncertaintyClosurePlanV1` freezes before the all-case barrier:

```text
case root
frontier root
first tournament root
first probe root
first partition
completion-required bit
complete completion-candidate denominator root
closure census root and disposition
selected second probe root, optional
selected joint partition
plan length
ordered probe roots
ordered prediction roots
cumulative budgets
planner executable root
preverifier root
authority false
plan root
```

Roots are acyclic and ordered exactly:

```text
frontier
-> first tournament
-> completion candidates
-> closure census
-> independent preverification
-> closure plan
-> all-case barrier
-> dispatch
-> observations
-> elimination
-> cleanup
```

All sixteen census roots and dispositions join the existing
`ALL_CASES_PRECOMMITTED` batch root. Every successful census also contributes
exactly one plan root. Any `CLOSURE_UNAVAILABLE` disposition terminates the
batch before dispatch while retaining all sixteen census roots. No worker
starts until all sixteen census/plan and independent preverification receipts
are durably published.

`plan length`, `second root`, ranking, budget, and execution order cannot change
after the first outcome. There is no `PROBE_PENDING` adaptation in V4.

## 8. Execution Semantics

Every selected probe executes from its own immutable initial manifest in a
fresh disposable workspace:

```text
probe 0 initial manifest -> worker 0 -> observer 0
probe 1 initial manifest -> worker 1 -> observer 1, when present
```

Probe 1 never reads probe 0's workspace or post-state. The scientific evidence
is the ordered vector of independently observed exact outcome roots, not a
sequential filesystem effect.

Private safety resolves and verifies each selected action/effect separately
after public plan freeze. One plan dispatch binds all safety receipts, worker
requests, observer requests, probe ordinals, and executable roots before the
first worker starts.

The whole ordered plan is durably dispatched before any observer result is
accepted. Worker execution may be serialized for resource control, but probe 1
dispatch cannot depend on probe 0's outcome. Each workspace identity is derived
from `(case root, plan root, probe ordinal)` and binds its exact initial
manifest. Shared paths, state carry-over, and initial-manifest substitution are
rejected.

## 9. Crash And Journal Boundary

The outer batch journal preserves one case-level triplet:

```text
PLAN_DISPATCHED -> OBSERVATION_VECTOR_FROZEN -> MODELS_UPDATED
```

A nested append-only case execution journal binds the internal route:

```text
PLAN_FROZEN
-> PLAN_DISPATCHED
-> PROBE_0_EXECUTED
-> optional PROBE_1_EXECUTED
-> PROBE_0_OBSERVED
-> optional PROBE_1_OBSERVED
-> OBSERVATION_VECTOR_FROZEN
-> CASE_TERMINAL
-> CLEANUP_FROZEN
```

Every event uses temp, file fsync, rename, and directory fsync. Restart projects
every legal prefix exactly. A durable dispatch without its matching observation
is terminal `INDETERMINATE_EXECUTION`; same-identity redispatch, model
elimination, and invented observation are forbidden. Ordinal, probe root,
initial-manifest root, workspace identity, worker request, observer request,
and observation root are bound at every applicable event. Cleanup cannot occur
before the case terminal and the outer `MODELS_UPDATED` publication.

## 10. Independent Final Verification

The final verifier independently reconstructs:

```text
support consistency
four syntactic and semantic models
all 1,792 raw probes and 7,168 predictions
complete V3 quotient
unchanged first-probe tournament and direct winner
complete V4 completion census when required
closure-plan root and cumulative budgets
private safety for every selected probe
dispatch and fresh-workspace bindings
each worker/observer parity receipt
ordered observation vector
semantic elimination and private truth match
```

A model survives only if every selected probe prediction equals the observation
at the same bound ordinal. PASS still requires exactly one survivor and exact
private true-class equality.

The final verifier imports canonical schemas and hashing only. It does not
import the closure planner, completion ranking, joint partition builder, or
process elimination implementation. It recomputes the six joint equality bits
as `first_equal AND second_equal`, reconstructs equivalence classes, uses
checked arithmetic for cumulative risk/cost, and rejects stored partition or
winner drift.

## 11. Frozen Denominators

Preserved per case:

```text
support consistency dispositions        336
materialized syntactic models              4
semantic classes                            4
raw probes                              1,792
raw predictions                        7,168
```

V4 derived denominators:

```text
first tournaments                          16
single-probe plans                           8 expected in development
two-probe plans                              8 expected in development
selected probe executions                   24 expected in development
completion candidates per two-probe case    representative_count - 1
completion candidate total                  derived and frozen
joint pairwise comparisons                  candidates * 6
```

Confirm counts are generated independently and reported exactly. PASS requires
all sixteen cases to close with plan length at most two; it does not require the
development split's `8/8` plan-length distribution in confirm.

## 12. Budgets

```text
probes per case                         <= 2
risk per probe                          <= 10
cost per probe                          <= 10
cumulative risk per plan                <= 20
cumulative cost per plan                <= 20
protocol bytes per owner message        <= 1,048,576
resident memory per process             <= 512 MiB
wall time per case                      <= 60 seconds
sealed batch wall time                  <= 20 minutes
```

No budget may be raised after nonce creation. If a two-probe final-verifier
request exceeds the existing protocol limit, implementation must bind compact
root-addressed artifacts before R9; raising the limit is not an allowed repair.
The largest development request must be measured before implementation freeze;
checked addition is mandatory for every cumulative budget.

## 13. V4 Controls

V2 controls `32/32` and V3 controls `T1-T4` remain mandatory. Add:

```text
J1 private mapping or topology in completion request rejected
J2 post-outcome second-probe selection rejected
J3 omitted completion candidate detected by denominator root
J4 duplicate or foreign second probe rejected
J5 wrong joint-equality AND matrix rejected
J6 changed first predecessor winner rejected
J7 swapped probe ordinal or observation rejected
J8 shared/carry-over workspace binding rejected
J9 missing second observation rejected for a two-probe plan
J10 cleanup before case terminal and outer model update rejected
J11 unavailable closure omitted from all-case barrier rejected
J12 wrong completion count or membership root rejected
J13 non-global second winner or wrong disposition rejected
J14 cumulative risk/cost overflow or budget excess rejected
J15 invalid crash prefix, redispatch, or observation-before-plan rejected
J16 stored joint partition inconsistent with independently recomputed classes rejected
```

Generic parse failure or a different error code does not count as control PASS.

## 14. Implementation Continuation

Completed R0-R6 remain preserved. R7 is repaired in these sub-slices only after
V4 critique, structural gate, and preflight delta pass:

```text
R7A  V4 closure-plan schemas and complete completion census
R7B  independent completion verifier and plan precommit
R7C  multi-probe dispatch bundle and nested durable case journal
R7D  independent joint elimination, J1-J16, full 16-case process route
R7E  durability, resource terminals, strict Clippy, commit and push
R8   full non-sealed suites and structural/quality gates
R9   source/executable/test freeze and confirm-read capability receipt
R10  STOP before confirm nonce creation, search, read, or sealed execution
```

## 15. Claim Boundary

The strongest permitted development PASS is:

> In a frozen generated filesystem language, Nanda independently induced four
> semantic world models, formed the complete single-probe frontier, preserved
> the byte-identical predecessor winner, detected when one probe could not close
> factorized uncertainty, mechanically froze a bounded one- or two-probe public
> closure plan before outcomes, and reduced every development case to the
> private true class through isolated execution and independent observation.

This is a bounded generated capability result. It does not prove Natural K2,
learned strategy, natural traffic transfer, Wave-causal grokking, K1 admission,
product authority, or deployment readiness.

## 16. Stop Rule

V4 grants no Rust edits until its adversarial critique, owner-bounded structural
gate, and implementation preflight delta all permit the exact repair. Confirm
nonce creation remains forbidden through R9. R10 requires a separate explicit
authorization and cannot be crossed by successful development tests alone.
