# STOP-MS2 Core And V4 Evidence Ledger

Status: CORE PASS / LIVE FRESH EPOCH PENDING / AUTHORITY FALSE

## Implemented Route

```text
provider request
-> LearningRequestStructureV2
   -> provider-bound TurnIntentId
   -> RequestEventId
   -> source-neutral pre-action topology
-> LearningStructureRecordV3
-> request-learning checkpoint V4
   -> immutable rows keyed by commitment root
   -> repeated events in one turn are preserved
-> completed RelationFrame
-> MultiSourceJoinLedgerV1
   -> blind-then-reveal order proof
   -> exact identity/token/session checks
   -> ambiguous or invalid rows are censored
-> source-neutral factorizer
-> disjoint marginal token ledger
-> CoverageOpportunitySnapshotV1
```

The runtime lookup remains keyed by the latest turn state. Proof evidence is a
separate append-only commitment ledger. A repeated `TurnIntentId` can no longer
overwrite an earlier pre-action observation.

## Compatibility Boundary

V2 and V3 checkpoints still decode. Their topology rows lack the physical
capture provenance introduced by V4 and are restored with
`physical_order_proven=false`. They remain visible for diagnostics but cannot
join, identify an operator, satisfy future evidence, or acquire authority.

## Gates

```text
same turn, two request commitments retained       PASS
checkpoint restart retains both commitments       PASS
post-action topology cannot join                   PASS
pre-action applicability separated from effect    PASS
one intent buys marginal token mass once           PASS
ACTIVE overlap subtraction                         PASS (unit)
restart report byte parity                         PASS
raw request/teacher text in joined ledger          ABSENT BY TYPE
operator-learning multi-source tests               6/6 PASS
transition-serving bridge tests                    3/3 PASS
learning + serving Clippy -D warnings              PASS
execution authority                                false
```

The live checkpoint at the time of implementation was V3:

```text
stored turns       85
stored topologies   1
joined rows         0
censored legacy     1
```

That row was not promoted or relabelled. A new V4 deployment and ordinary
post-deployment traffic are required for `STOP-MS2 LIVE`.

## Wide Baseline

The complete `nando-operator-learning` package run reached 211 PASS and 30
failures in the existing checked-in B1B support/future artifact family. The
new multi-source tests and all focused owners pass. This document does not
reclassify or repair that independent historical artifact debt.
