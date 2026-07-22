# STOP-F8-C External Admission Reconstruction

Status: `PASS / PRODUCTION_CALLERS_0 / AUTHORITY_FALSE`

Date: `2026-07-22 Europe/Tallinn`

## Result

`nando-operator-admission` now reconstructs a candidate from immutable bytes:

```text
generation checkpoint
+ frozen generation capture index
+ live provider capture index
+ generation shadow ledger
+ phase-control receipt
+ resource receipt
-> independently reconstructed commitments
-> opaque ExternalGenerationAdmissionCandidateV3
```

The two capture indexes are intentionally distinct. Frozen support/future
evidence is joined through the generation index; post-freeze provider traffic
is joined through the provider index. Live lineages may not overlap frozen
support or future lineages.

## Independent Checks

```text
checkpoint and generation bundle decode       canonical
generation capture join                       exact
dispatch index reconstruction                 exact
provider capture join per live receipt        exact
F5 traffic generation/request/index roots     exact
F6 action/output/artifact/verifier roots       exact
support and future roots                      recomputed
phase controls vs live traffic set            exact
resource policy                               validated
submitted vs reconstructed commitments        byte-identical
```

Unknown schemas, missing inputs, root drift, traffic-set substitution,
support/future overlap and runtime parity mismatch fail closed. Equal causal
controls produce `WATCH_NO_CAUSAL_GAIN`, never `PASS`.

## STOP Matrix

```text
immutable reconstruction tests                5/5 PASS
submitted commitment tamper                   BLOCK
unknown schema                                BLOCK
missing independent input                     ABSTAIN
foreign control traffic set                   BLOCK
candidate authority                           false
verified submission authority                 false
production callers                            0
```

Full package verification shared with STOP-F8-B:

```text
nando-operator-admission                       7/7 PASS
nando-operator-learning                      217/217 PASS
nando-operator-persistence                    15/15 PASS
Clippy touched packages -D warnings               PASS
```

## Boundary

F8-C proves reconstruction and provenance. It does not prove that Wave caused
an improvement. `ShadowReady` is therefore still non-authoritative and cannot
cross admission until F8-D replaces caller-provided control counts with a
proof-owned matched-traffic evaluator.
