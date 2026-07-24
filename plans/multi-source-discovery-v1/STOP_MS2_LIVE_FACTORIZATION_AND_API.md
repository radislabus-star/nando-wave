# STOP-MS2 Live Factorization And API

Date: 2026-07-24 Europe/Tallinn.

## Verdict

```text
MS2-A blind-then-reveal join              PASS
MS2-B source-neutral factorizer           PASS
MS2-C disjoint marginal token ledger      PASS
MS2-D compact live API                    PASS
operator authority                        false
```

## Implemented Route

```text
OpportunityIntentAuditRowV1
+ PreActionTopologyCommitV1
+ completed RelationFrame
-> MultiSourceJoinLedgerV1
-> FactorizedMultiSourceRowV1
-> CoverageOpportunitySnapshotV1
-> LiveMultiSourceDiscoverySnapshotV2
-> GET /v2/multi-source/report
```

The endpoint reads a precomputed immutable snapshot. It does not lock the
miner and does not read a learner checkpoint from the HTTP path.

## Measured Live Boundary

The last pre-MS3 deployment reported:

```text
pre-action topology rows             452
completed relevant relation frames     0
joined transitions                     0
authority                              false
blocker              no_completed_relation_frame
endpoint latency              about 0.3 ms
```

This is an evidence boundary, not a Wave or threshold failure. Topology is
captured before action reveal, but no matching completed frame had yet entered
the retained live snapshot.

## Proof

```text
token conservation                   PASS
duplicate marginal purchase             0
ACTIVE overlap subtraction           PASS
input-order byte parity              PASS
snapshot validation                  PASS
snapshot budget                      PASS
hot serving overhead                    0
false accepts                           0
runtime parity failures                 0
```

MS2 issues no candidate and no authority. MS3 consumes only its immutable
joined rows.
