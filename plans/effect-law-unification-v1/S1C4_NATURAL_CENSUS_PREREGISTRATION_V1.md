# S1C-4 Natural Census Preregistration V1

Status: `FROZEN BEFORE IMPLEMENTATION / FINITE NATURAL CENSUS ONLY`

Date: `2026-08-13`

## 1. Plain-Language Question

S1C-4 asks one bounded question about ordinary traffic:

```text
Does Nanda naturally receive decisions that contain, before execution,
an exact goal and at least one applicable certified K1 action, and then
produce a selected action with an independently verified result?
```

S1C-4 does not learn a K2 law. It measures whether the evidence surface needed
for later grounded-meaning research exists at all.

## 2. Frozen Source Boundary

Only post-instrumentation ordinary requests are eligible. The denominator is
the durable `Request` suffix in `opportunity-bridge-v1`; its request identity
and bridge sequence are joined to exactly one durable S1C classification.

```text
durable opportunity Request at sequence q
+ one S1C classification bound to q and request identity
+ existing decision precommit / selected-action / satisfaction journals
-> one exact census row
```

Rows before the S1C-4 cursor are excluded. In-memory censor counters, archived
S1A transitions, generated traffic, synthetic fixtures, manually supplied
probes, post-action goal inference, and retrospective classification are
forbidden.

## 3. Frozen Window

The window opens only after the durable classification instrument is deployed
and its restart parity passes. The immutable start cursor binds:

```text
schema and implementation commit
deployment receipt root
opportunity bridge root and durable sequence
opportunity request-event count
classification ledger empty suffix cursor
three decision-journal append cursors
opened_at_unix
```

The natural denominator closes at the first of:

```text
1024 new durable ordinary Request events
24 hours after opened_at_unix
```

After closure, the collector waits up to 60 seconds for both durable writers to
cross the frozen end sequence. This quiescence allowance cannot add requests to
the closed denominator or repair a missing row.

## 4. Durable Classification Contract

Each eligible ordinary request receives exactly one terminal classification:

```text
DECISION_RECORDED
or one GroundedDecisionShadowCensorV1 reason
```

The row stores only bounded structural data:

```text
opportunity sequence
request-event identity root
session-lineage root
classification
optional decision-precommit root for DECISION_RECORDED
observed time
previous-row root and row root
```

Raw request text, response text, tool payloads, session payloads, inferred
goals, and model outputs are forbidden. Rows are append-only, canonically
encoded, hash chained, uniquely keyed by opportunity sequence and request
identity, and restart validated.

The request path may only enqueue a bounded row. A single background owner
appends, checkpoints, and syncs batches. Queue overflow, writer failure,
duplicate identity, duplicate sequence, non-monotonic sequence, malformed row,
or an unclassified denominator request is evidence loss and therefore `VETO`.
No such request may be silently removed from the denominator.

## 5. Exact Census Join

For every durable opportunity `Request` in the frozen suffix, the census
requires exactly one classification with the same request identity and
sequence. `DECISION_RECORDED` additionally requires a unique exact join across:

```text
DecisionContractPrecommitV1
DurableSelectedActionBindingV1
DurableGoalSatisfactionV1
```

All roots, request identity, authority snapshot, available-action set, selected
K1 action, independent verifier, temporal order, and goal satisfaction must
validate. A decision lineage is the pre-action session-lineage root stored by
the classification row; one lineage cannot count as two.

## 6. Frozen Verdict Function

The terminal verdict is deterministic and ordered:

```text
VETO
  any source drift, gap, duplicate, orphan, malformed row, journal mismatch,
  writer failure, false accept, parity failure, or forbidden provenance

EMPTY_GOAL_SURFACE
  complete denominator, zero valid pre-action exact goals, all non-VETO rows
  censored before goal binding as MISSING_EXACT_GOAL

EMPTY_ALTERNATIVE_SURFACE
  at least one valid exact goal, zero valid alternative-bearing precommits,
  and no VETO condition

INSUFFICIENT_LINEAGES
  at least one complete decision episode but fewer than two independent
  decision lineages

PASS
  at least two complete decision episodes in at least two independent
  lineages, each with a nonempty certified K1 alternative set and exact
  independently verified satisfaction
```

Mixed legitimate censor reasons that do not satisfy one of the exact empty
surface predicates remain `INSUFFICIENT_LINEAGES` only when at least one valid
decision episode exists; otherwise they are reported as `VETO` with
`heterogeneous_unresolved_surface`. No majority vote is permitted.

## 7. Safety And Claim Boundary

All of these remain false regardless of S1C-4 outcome:

```text
execution authority granted by census
model training allowed
phase mutation allowed
K2 law proved
Law #2 proved
automatic S2 entry
```

`PASS` means only that a natural, independently checkable decision surface
exists. `EMPTY_*` is a valid bounded negative result about this production
surface. `VETO` means the measurement is not interpretable.

## 8. Deployment And Rollback

The deployment unit is the transition-serving binary, its environment, the
classification ledger directory, the S1C-4 cursor/report sidecars, and the
gateway-control projection that reads the report. Nginx and the connector are
not restarted.

Before mutation, exact binary/config/sidecar bytes, service state, health,
opportunity checkpoint, journal prefixes, and ledger absence or prefix are
captured. Rollback restores binary/config/control-plane bytes while preserving
every naturally appended opportunity, decision, and classification suffix.
Rollback can close an uncompleted candidate window as `VETO`; it cannot delete,
truncate, relabel, or reopen that window.

## 9. Required Tests Before Deployment

```text
canonical row and hash-chain parity
exactly-one classification per request path
queue overflow -> visible denominator gap -> VETO
writer failure -> VETO
crash before append and crash after append
restart reconstruction and duplicate rejection
cursor immutability and frozen end boundary
decision-journal exact join and orphan rejection
verdict matrix for all five outcomes
raw-payload exclusion
capture-off serving parity
false accepts = 0 and runtime parity failures = 0
dashboard invalid-report rejection
transaction rollback preserving natural suffixes
```

Implementation may begin only after adversarial critique, NANDA structural
gate, and implementation preflight return `READY_TO_IMPLEMENT`.
