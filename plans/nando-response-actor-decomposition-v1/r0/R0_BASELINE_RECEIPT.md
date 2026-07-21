# STOP-R0 Baseline And Ownership Receipt

Status: `PASS`

Base HEAD: `3d7fbefe070d66e64870b4387870a843de697804`

Authority: `false`

F5-B: `NOT_STARTED`

## Accounting

```text
visible Rust files                         95 / 95
visible Rust lines                       103,389
tracked Rust files                         94
tracked Rust lines                       102,989
mixed-owner files explicitly mapped        46 / 46
root public re-export statements            47 / 47
public re-exported symbols                  699 / 699
external Rust call sites                    176
external caller files                        29
unknown external callers                      0
side-effect/authority candidate calls      1,788
files containing those candidates             64
unowned side-effect files                      0
schema constants                              99
tracked JSON files                            203
```

The side-effect inventory is deliberately conservative: it includes
filesystem, process, environment, clock, checkpoint, authority, and ACTIVE
tokens. False-positive search rows remain in the receipt; every containing
file nevertheless has an explicit owner and movement policy.

## Frozen Behavior

```text
full lib tests                         502 PASS / 26 known FAIL
known failure-set SHA-256              9ada408b39863f06a8cbfddd626bf26e84428277dc0728f484c2afa3b56d9508
Clippy library diagnostics             12
Clippy test-only diagnostics             8
Clippy total                            20
Clippy set SHA-256                      63c9f1b7339470bb141ad2086e8cc8607b29c54fedf430be95610d53f058c451
```

Known debt is an exact fingerprint, not a permissive baseline. A new failure
or diagnostic fails a move; a disappearing one must be explained rather than
silently counted as refactor success.

## Live Boundary

```text
structural gate                         PASS
wave causal gate                        PASS
transition runtime admission            PASS
response runtime                        VETO
response ACTIVE packages                   0
response false accepts                     0
response parity failures                  0
M3                                      WATCH
eligible_for_local_accept               false
```

The VETO/WATCH state is frozen as pre-refactor product truth. The decomposition
must not convert it into authority.

## Structural Review

The combined cross-owner worksheet returned the expected VETO. After splitting
it into owner-local invariants, all `15/15` routes returned structural `PASS`
with `authority_ready=false`. Graphify independently surfaced the same current
mixed ownership across EffectLaw, ProtocolMode, learner, runtime, proof, and
admission modules.

## Verdict

```text
source files accounted                         100%
public symbols accounted                       100%
mixed-owner files explicitly split in map      100%
unowned side effects                              0
unknown external callers                          0
baseline artifacts canonical                  PASS
authority                                      false
STOP-R0                                        PASS
```

R1 may create build tooling. No operator crate movement is authorized before
STOP-R1.
