# STOP-B1A: Pre-Action Binding Evidence

Date: 2026-07-21 Europe/Tallinn

```text
verdict                         INSUFFICIENT_BINDING_EVIDENCE
frozen denominator              129 / 129
exceptional rows                 10 / 10
selector selected                NO
ProtocolMode F4                  NOT STARTED / BLOCKED
execution authority             false
B1A checkpoint commit            fb23013
push / deploy                    NO
service restart                  NO
```

## Scope

B1A closes only the exact version-space baseline for source-role binding. It
does not compile a selector or `ProtocolMode` and does not change runtime,
admission, Wave, generation, thresholds, or authority.

The evidence route is deliberately split at the label boundary:

```text
pre-action request + provider surface + structural context
-> PreActionBindingSurfaceV1
-> label-blind candidate enumeration
-> CandidateRelationGraphV1
-> immutable FrozenCandidateRelationGraphV1

all graphs frozen
-> expected action-equivalence receipt joined afterward
-> bounded exact version-space evaluation
-> identifiable action class OR INSUFFICIENT_BINDING_EVIDENCE
```

`binding_evidence.rs` depends only on `std`, `serde`, `serde_json`, and
`sha2`. It has no crate-local, runtime, admission, Wave, generation, or
ProtocolMode dependency.

## Machine Artifact

```text
plans/effect-law-unification-v1/STOP_B1A_BINDING_EVIDENCE.json
schema        nando.binding-version-space-report.v1.r1
file sha256   e2b887fbb8569afc1e702b9132b6b45c6e77c0d60353c1586d1fd8ef09783b73
report sha256 cd9a6bdddd64cf8ad75f7be9e9c9c149030f7ff599716abd81fb63836bfed64b
```

The checked-in file is byte-identical to the generated artifact at:

```text
/home/ubu/tmp/nando-b1b0/STOP_B1A_BINDING_EVIDENCE.json
```

## Frozen Accounting

```text
frozen rows                         129
positive rows                       129
expected value observable           129
baseline EXACT                      119
baseline WRONG                        3
baseline ABSTAIN                      7
baseline VERIFY_FAILED                0
exceptional rows accounted           10 / 10
censored unknown                    723
applicability-negative rows           0
```

The ten exceptional rows are exactly the three historical WRONG and seven
historical ABSTAIN rows. Censored parity evidence remains unknown; it is never
converted into a negative.

## Candidate And Search Budgets

```text
candidates enumerated             136488
maximum candidates in one row       3976 / 8192
candidate budget exhaustion            0
relation-edge budget exhaustion        0
hypotheses evaluated                2441 / 2441
hypothesis budget exhaustion       false
complete hypotheses                    0
complete action-equivalence classes    0
wrong bindings                         0
negative accepts                       0 / denominator 0
```

Values are represented only by opaque action-equivalence commitments. Value,
path hash, ordinal, prefix, field name, and function name are not available as
hypothesis predicates. Search budgets are hard and fail closed.

The pre-action temporal envelope is 32 events. A smaller envelope censored an
exceptional receipt whose relevant candidate was at distance 16; the accepted
bound covers that row without turning distance into a hidden answer rule.

## Version-Space Result

No hypothesis covers every positive row with one unambiguous action class.
The strongest incomplete hypothesis is:

```text
shared opaque call lineage
+ temporal distance 0
+ string value type

covered positive rows        63
uncovered positive rows      15
ambiguous rows               51
wrong bindings                0
negative accepts              0
```

Therefore B1A does not select a selector. A deterministic tie-break, latest
item, ordinal, prefix, path, or known value would be an unsupported lookup.

## Ties And Distinguishing Probes

```text
unresolved row/action-class ties    86 / 86
distinguishing probes emitted       86 / 86
tie-report budget exhaustion        false
```

All 86 ties prove the same relation-neutral missing distinction:

```text
expected action class vs competing action classes
```

Canonical machine probe:

```text
Acquire a pre-action observable feature that separates the expected
action-equivalence class from every competing class while holding the
currently shared features constant.
```

The existence of a missing discriminator is proven. Which causal relation
provides it is unknown. `parent_action_to_capability_instance` is candidate
hypothesis H1, not a B1A result. The null hypothesis H0 remains
`relation_not_observable`; both are unproven until B1B acquisition.

`negative_accepts=0` has an applicability-negative denominator of zero in
B1A. It is exact accounting, not evidence that a binding hypothesis rejects
applicability negatives.

## Leakage And Adversarial Controls

```text
expected binding joins only after graph freeze             PASS
content-part reorder preserves graph/report                PASS
renamed fields and function names preserve relations       PASS
direct and singleton-wrapped surfaces agree                PASS
older active handle receives distinct temporal relation    PASS
duplicate same-prefix candidates remain a tie              PASS
values, paths, ordinals, prefixes absent from predicates   PASS
censored unknown is not an applicability negative          PASS
row-order shuffle gives byte-identical report              PASS
candidate and hypothesis budgets fail closed               PASS
identifiable fixture reports a class without a selector    PASS
```

## Verification

```text
B1A focused tests                    11 / 11 PASS
F3R dual-classifier tests            12 / 12 PASS
Canonical F2 V3 tests                28 / 28 PASS
Historical F2 tests                  15 / 15 PASS
cargo check nando-response-actor              PASS
F3-aware Clippy                               PASS
semantic baseline                     22 PASS / 3 known FAIL
```

Full Clippy still reproduces the 12 accepted legacy warnings. None points to
the B1A module. The same three pre-existing semantic failures remain unchanged:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

Accepted production-copy replay:

```text
elapsed     2:16.68
max RSS     454812 KiB
exit        0
```

## Structural Gates

Owner-local routes pass independently:

```text
b1a-pre-action-freeze-owner   PASS
b1a-version-space-owner       PASS
b1a-authority-owner           PASS
```

The first worksheet drafts returned `VETO` because contract and implementation
entities used unrelated lexical identities. Those traces are preserved. The
worksheets were repaired by binding the same canonical entities to source and
machine-artifact evidence; implementation, requirements, and candidate triads
were not weakened.

Trace directory:

```text
/home/ubu/tmp/nando-b1a/nanda-tmp/nanda-structural-gate/
```

## Diff Ownership

```text
binding_evidence.rs                    pure bounded evidence/version-space core
binding_evidence_tests.rs              leakage, adversarial, budget controls
online_diagnostics.rs                  frozen 129-row diagnostic adapter
online.rs                              read-only diagnostic forwarding method
nando-online-response-diagnose.rs      opt-in JSON report entrypoint
lib.rs                                 B1A diagnostic exports
STOP_B1A_BINDING_EVIDENCE.{json,md}    machine and human receipts
EFFECT_LAW_UNIFICATION_REFACTOR_V1.md  lifecycle status only
graphify-out/                          generated graph update
```

No B1A code imports or mutates runtime execution, admission, Wave, generation,
thresholds, selectors, checkpoints, registry authority, or ACTIVE packages.

## Repository And Authority State

```text
accepted B1A checkpoint  fb23013
branch                main
push / deploy         NO
service restart       NO
execution authority   false
ProtocolMode compiled false
F4 started            NO
```

Live read-only confirmation:

```text
learning service invocation  8e59505eb1b943778601c9b3bacbd607 (unchanged)
serving service invocation   74ac3080f80b4fe387de2a94380e3657 (unchanged)
serving /health              ok=true, executor_cache_ready=true
serving registry revision    0
transition false accepts     0
response false accepts       0
admission-controller report  BLOCK, active_packages=0
```

Systemd reports that both unit fragments have changed on disk since the active
processes were loaded. B1A did not run `daemon-reload`, restart either service,
or adopt those on-disk changes.

The existing dirty F0-F3R slice and unrelated untracked diagnostic file were
preserved.

## STOP-B1A

```text
B1A version-space baseline       COMPLETE
binding identifiable             NO
verdict                          INSUFFICIENT_BINDING_EVIDENCE
next architectural stage         B1B acquisition
missing discriminator            PROVEN
resolving causal relation        UNKNOWN
candidate relation H1            parent_action_to_capability_instance / UNPROVEN
F4 ProtocolMode compiler         BLOCKED
runtime / admission / authority  UNCHANGED
```

Work stops here. B1B and F4 are not started by this receipt.
