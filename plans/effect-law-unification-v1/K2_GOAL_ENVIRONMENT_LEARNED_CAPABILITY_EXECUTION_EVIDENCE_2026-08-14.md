# K2 Goal Environment Learned Capability Execution Evidence

Date: 2026-08-14

## Verdict

```text
K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS
authority                                              false
natural K2 claim                                      not made
LawCertificate / K1 registry / package activation     unchanged
production / dashboard / deployment                   unchanged
```

The isolated capability route learned two hidden filesystem effects from
support outcomes, predicted both effects on an unseen target, selected the one
that satisfied a preregistered exact goal, executed it in the existing Law Lab,
and passed a separately implemented exact oracle. This proves the bounded
generated-fixture capability only.

## Terminal Roots

```text
learned capability outcome   9b5d8ddbf1429c3390ac7079117b81b9c127ecb215920c1461ee78b0dcb15428
learned capability seal      84053f07611ee97c8874a748bc140437f87cfc5e5afcbe60a94d248070932b6e
learned law set              18c57e662b973acc66a54e0b1f2490eb0878edf1991e90638158a1c826ace9e3
target prediction set        0f906847ca765b67ae5f516ba47d1731ab2ecd03e4a1ff1171f734f839897d16
V1 target outcome            46a437257bd174c1ccb33adfdf317343c8f50c0a799c6af7dd5b3831428108cb
V1 target seal               c2b146342ad7bfe5d8c082aa9b211a8b66b1b621edba0d2685bdc181c2ac0b23
ablation receipt             75bc1d59664a8682e754d640b78fe81b9660a40c2cc1ce44d5d4c7ee2ad970d6
```

## Executed Route

```text
support worlds                         3
opaque actions                         2
support Law Lab executions             6 / 6 exact
main external learner processes        2 / 2
learned effects                        2 / 2
unseen target predictions              2 / 2
independently verified support laws    2 / 2
independently verified predictions     2 / 2
selected target actions                1 / 2
target Law Lab execution               PASS
target exact oracle                    PASS
wrong predictions                      0
journal events                         22 / 22
authority bits                         all false
```

The learner, selector test process, Law Lab worker, and exact oracle had four
pairwise-distinct executable hashes. The external learner ran under bwrap with
cleared environment, no network, no repository/private/target mounts, and the
frozen CPU, memory, process, wall-time, stdout, and stderr limits.

## Negative Controls

All 13 preregistered controls produced their exact required result. The first
six used source-bound mutated manifests and the real external learner process;
the wrong-action control used one additional real Law Lab execution and one
additional exact-oracle process.

```text
support count                 INSUFFICIENT_SUPPORT
action identity shuffle       NON_TRANSFERABLE_DELTA
ambiguous copy source         AMBIGUOUS_SOURCE_MATCH
constant output               NON_TRANSFERABLE_DELTA
outcome dependence            NON_TRANSFERABLE_DELTA
dynamic opaque IDs            TRANSFERABLE_WITH_DYNAMIC_IDS
holdout alias                 TARGET_NOT_INDEPENDENT
support provenance mismatch   SUPPORT_EVIDENCE_INVALID
target goal leakage           LEARNER_REQUEST_PRIVATE_FIELD_REJECTED
prediction tamper             TARGET_PREDICTION_ROOT_MISMATCH
wrong action exact oracle     EXACT_GOAL_UNSATISFIED
cross-experiment replay       CROSS_EXPERIMENT_REPLAY
authority tamper              AUTHORITY_BOUNDARY_VIOLATED
```

```text
ablation learner processes    6
ablation sandbox probes       1
ablation oracle invocations   1
```

The dynamic-ID control used a second harness commitment, two disjoint opaque
IDs, and learned the same two effect bodies. The outcome-dependence control
removed all post-action deltas while preserving IDs and therefore learned no
law. The wrong-action control executed the non-selected hidden effect against
the same target goal; the external oracle returned `goal_satisfied=false`, and
both the unchanged V1 outcome constructor and the learned-capability guard
rejected PASS.

## Durability And Replay

Restart projection was byte-identical at every legal prefix from 0 through 22
events. Tests rejected a gap, duplicate, tamper, wrong order, and foreign
experiment identity. A restart after a published support dispatch prohibited
same-identity continuation. Fault injection proved both boundaries:

```text
fault after temp fsync                         no event published
fault after publish before directory fsync    published event recovered
published dispatch without observation        INDETERMINATE; no rerun
```

Event 19 projects to `LEARNED_TO_V1_BINDING_FROZEN`; only event 20 projects to
`TARGET_EPISODE_COMPLETE`. Terminal replay compares both request roots, every
V1 evidence root, every support root, and every outcome counter. The learned
terminal outcome, terminal journal entry, final projection, and post-event seal
remain acyclic.

## Regression And Gates

All Rust work ran on the mini-PC with `CARGO_BUILD_JOBS=20` and
`CARGO_TARGET_DIR=/home/e/.cache/nando-wave-k2-goal-target`.

```text
cargo test -p nando-operator-learning --all-targets     449 PASS / 5 ignored
learned real-process integration                         1 PASS
unchanged V1 real-process integration                    1 PASS
real Law Lab integrations                                3 PASS
cargo fmt --all -- --check                               PASS
cargo clippy -p nando-operator-learning --all-targets
  -- -D warnings                                         PASS
NANDA execution / holdout / provenance packets           PASS / authority_ready=false
implementation preflight                                 READY_TO_IMPLEMENT / blockers=0
```

All 12 frozen file hashes in the preflight receipt remained byte-identical.
The two preserved directory fences remained outside the scoped change:
`graphify-out/` and the S1C3 evidence directories. Every disposable K2 and Law
Lab fixture path was absent after the final run, and no learner, worker, or
oracle process remained alive.

## Claim Boundary

This result demonstrates a small but real machine-created law route:

```text
hidden action effects
-> observed support transitions
-> external bounded induction
-> unseen target prediction
-> goal-conditioned choice
-> isolated execution
-> independent exact verification
```

It does not demonstrate natural traffic acquisition, natural K2 composition,
open-ended semantics, production safety, economic coverage, or authority. Any
such promotion requires a separate preregistration and independent evidence.
