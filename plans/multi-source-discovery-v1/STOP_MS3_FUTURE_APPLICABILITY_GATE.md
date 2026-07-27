# STOP-MS3 Future Applicability Gate

Date: 2026-07-26

## Verdict

```text
Version space complete             PASS
Semantic classes                   1
Unique law frozen                  PASS
Future applicability gate          LIVE / COLLECTING
Independent topologies             1 / 256
Structurally not applicable        1
Predictions committed              0
Independent future                 NOT_EVALUATED
Authority                          false
Phase mutation                     false
```

This STOP closes the bounded acquisition mechanism. It does not close MS3
future transfer and does not authorize a package.

## Signal Route

```text
Frozen natural-law candidate
  contract 23f002035fb5090827a7815fc24d2fdc633df2126a32c978b5347ae69410ba81
  |
  +-> support-lineage reuse                         excluded
  |
  +-> independent topology
       |
       +-> structurally not applicable              denominator only
       |
       +-> applicable
            -> prediction bytes persisted
            -> fsync + atomic rename
            -> durable prediction event
            -> terminal outcome join
                 |
                 +-> terminal <= durable prediction
                 |      PRECOMMITTED_PREDICTION_MISSING
                 |
                 +-> terminal > durable prediction
                        independent verifier
                        -> FUTURE_PASS | CONTRADICTION
```

## Frozen Gate

```text
schema                 nando.ms3-future-applicability-contract.v1
contract root          781b6b6018ee54e83f1fe97222a40f98e060ca1bb083d397555f065a2f5dfedd
opened sequence        13611
prediction min         13612
opened unix            1785096275
deadline unix          1785182675
max topologies         256
authority              false
phase mutation         false
```

The denominator is not a support/future row threshold. It is a bounded search
budget for observing whether the frozen applicability scope occurs again.

## Live Receipt

```text
verdict                         collecting
blocker                         applicable_independent_topology_pending
independent topologies          1
independent lineages            1
structurally not applicable     1
predictions committed           0
active predictions              0
precommitted missing            0
ledger root                     58d393e3a2941c0578baedb2b9199439e73a9e76edc67acb1720e4ad13319823
```

The ordinary post-open diagnostic had 292 topology rows at the recorded
snapshot: 291 reused the support lineage and were excluded; one independent
topology was structurally not applicable.

## Restart Proof

The cold learner was restarted after the gate opened. The immutable bytes were
identical before and after restart:

```text
frozen contract CBOR
  cfeade1afa5c82c03765b917b05399493e11a7dfa03a4bfe0edab5efe62733c1

prediction ledger CBOR
  addaef06dcee22405b48bdb0707406c2bff627368a23d34fc634f815ba3dc292

applicability ledger CBOR
  bcd49c266489dd1095431a2435ee14507ff156874f5009ef66017b5e50f1031c
```

The report root changes with `generated_at_unix`; it is a content-addressed
snapshot, not the immutable ledger identity. The ledger root and all semantic
counts remained unchanged.

The hot serving invocation remained:

```text
aec3a81260c648ed84538066131715c6
```

Only `nando-response-learning.service` was restarted.

## Verification

```text
nando-operator-learning multi_source     47 / 47 PASS
nando-transition-serving lib             133 PASS / 1 ignored
isolated extractor hot budget             PASS
strict Clippy for changed crates          PASS
rustfmt and diff check                     PASS
false accepts                              0
runtime parity failures                    0
```

The full serving suite once observed the extractor benchmark at 723 us while
the live cold learner and parallel Rust checks were contending for CPU. The
same exact benchmark passed in isolation. This is recorded as environmental
load sensitivity, not hidden as a clean full-suite timing result.

## Fail-Closed Rules

- No post-action or teacher output may create a runtime prediction.
- A terminal outcome older than the durable prediction receipt is excluded.
- A non-applicable topology is not a contradiction or anti-center.
- Gate exhaustion cannot widen the applicability guard.
- A report, learner artifact or BundleV4 cannot grant its own lease.
- M3 uses a separate live-traffic denominator and is not closed by this gate.

## Only Unlocked Step

```text
fresh independent applicable topology
-> durable pre-action prediction
-> later verified terminal outcome
-> FUTURE_PASS or CONTRADICTION
```

`CanonicalOperatorIR`, BundleV4 and External Admission remain locked until
`FUTURE_PASS`.

## Missing Completed Frame Lifecycle Repair

Live evaluation exposed a lifecycle state that the original deadline-only gate
did not distinguish:

```text
durable prediction
+ durable terminal receipt
+ no completed RelationFrame
+ later durable topology from the same lineage
-> CENSORED_MISSING_COMPLETED_FRAME
```

The later topology is a capture fence. Its root, request identity, lineage,
sequence and capture time are committed into the censored receipt. A global
watermark or elapsed delay is not sufficient. Without this same-lineage fence,
the prediction remains pending.

The censored outcome:

```text
creates anti-evidence       false
updates phase memory        false
grants authority            false
counts as contradiction     false
closes current prediction   true
reopens acquisition         true
```

The first live prediction had a terminal receipt but no completed frame. Its
terminal preceded the durable prediction by 37 ms, while a later request in
the same lineage proved capture had advanced. The repaired evaluator emitted
the censored receipt immediately instead of waiting for the 24-hour emergency
deadline.

After deployment and a cold restart, a second pending frame-less prediction
was independently censored and acquisition returned to `COLLECTING` again:

```text
predictions committed                 2
censored missing completed frame      2
active predictions                    0
verdict                               collecting
blocker                               applicable_independent_topology_pending
authority ready                       false
phase mutation allowed                false
```

The hot serving PID remained unchanged and its restart count stayed zero.
The installed cold binary SHA-256 is
`4dc1f7f0dfb276014400e9bf6c9f9da62cd5a5a0b2afc9bfa99145eb4cbee0aa`.
The implementation commits are `5a3112e` and `5793ee0`.

Verification after the final source synchronization:

```text
nando-operator-learning               298 / 298 PASS
nando-transition-serving              138 PASS / 2 ignored
strict Clippy for changed crates       PASS
NANDA composite gate                  PASS
false accepts                         0
runtime parity failures               0
```
