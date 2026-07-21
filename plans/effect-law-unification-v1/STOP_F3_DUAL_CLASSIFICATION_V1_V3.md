# STOP-F3: Shadow Dual Classification V1/V3

Date: 2026-07-21

```text
STOP-F3R                        PASS (trusted fixture-shadow scope)
F4 ProtocolMode compiler       NOT STARTED
execution authority            false
commit / push / deploy         NO
service restart                NO
```

## Scope

F3 now runs two independent read-only classifications over the same trusted
row:

```text
TeacherTransition + SealedEffectObservationV3
|
+-- V1: as_training_relation_frame()
|       -> teacher_semantic_law_signature()
|
`-- V3: trusted evidence membership
        -> private single-row canonical grouping key
        -> 3D-independent search_quotient()
        -> EffectLawIdV3
```

The private grouping key is never reported as an EffectLawIdV3. A public V3
law ID is emitted only after the quotient has at least two independent episode
lineages, surfaces, and physical programs. V1 is not a label, seed, filter, or
grouping input for V3.

This STOP report uses a deterministic 12-row structured fixture corpus. Every
row is sealed through the real response actor and independent verifier route,
then joined to one trusted generation manifest. It is not a claim that a live
production receipt corpus has already been dual-classified.

## Machine Report

Canonical artifact:

```text
plans/effect-law-unification-v1/STOP_F3_DUAL_CLASSIFICATION_V1_V3.json
schema      nando.effect-law-dual-classification-report.v1-v3.r1
file sha256 c1de712eb4b1f43e40e38d092fca6565202e5ef6a625cdf8c54eeff254f3880c
```

```text
denominator                         12
accounted rows                      12
trusted rows                        12
V1 attempted / classified           12 / 12
V3 attempted / classified           12 / 12
legacy V1 cohorts                    3
canonical V3 laws                    6
unknown / censored                   0
trust failures                       0
unexplained merges                   0
unexplained splits                   0
pairwise discrepancies expected      6
pairwise discrepancies accounted     6
execution authority                  false
verdict                              PASS
```

Each of the six V3 laws has:

```text
observations                         2
independent episode lineages         2
independent surfaces                 2
independent physical programs        2
```

## Merge And Split Map

```text
V1 cohort 53e83e9f... -> V3 512e0c9a...                         2 rows
V1 cohort 85e35cac... -> V3 4156a164... / 90b551cc... /
                         V3 97e9e09f... / a0ffb7d5...            8 rows
V1 cohort ff85e24d... -> V3 abf5af12...                         2 rows
```

The one V1-to-V3 split expands to six V3-law pairs. Every pair carries its own
`DiscrepancyWitness`; all six are explained by concrete committed V3 facet
differences drawn from:

```text
typed_constants
temporal_cardinality
preserved_frame
```

Expected surface-only V3 merges are zero in this corpus. Direct and wrapped,
renamed physical surfaces still converge to one V3 law in the focused test;
the corresponding V1 fixture rows already share one legacy signature, so this
is agreement rather than a V1/V3 discrepancy.

No label-free fixture currently produces two distinct V1 signatures that are
proven to differ only by protocol surface while sharing one V3 law. Therefore
`PROTOCOL_ONLY_MERGE_FIXTURE_PROVEN_V3` remains false and every future merge is
WATCH even when pairwise effect roots match and protocol roots differ.

## Accounting Contract

Every input transition digest and observation digest must be unique. A
duplicate fails closed before a report is produced. Trusted rows receive
exactly one of:

```text
trusted_classified
trusted_legacy_unknown
trusted_v3_censored
trusted_dual_censored
trust_failure
```

Censored rows remain reporting outcomes and never become negative evidence.
Trust failures, censored rows, unexplained merges, or unexplained splits force
the report verdict to WATCH.

Merge and split discrepancies are never explained at group level. For `N`
classes the report requires exactly `N * (N - 1) / 2` pairwise witnesses. A
missing or unexplained pair forces WATCH.

## Verification

```text
F3R focused tests                       12 PASS / 0 FAIL
F2 effect_law_v3::tests                 28 PASS / 0 FAIL
historical effect_law::tests            15 PASS / 0 FAIL
cargo check nando-response-actor        PASS
F3-aware Clippy                         PASS
semantic_ baseline                      22 PASS / 3 known FAIL
git diff --check                        PASS
row-order shuffle                       byte-identical report
foreign / tampered trust                rejected and accounted
aggregate protocol-root false proof     WATCH
protocol-only merge without fixture     WATCH
checked-in JSON canonical parity        PASS
production callers                      0
```

Full `-D warnings` still reports the same 12 pre-existing diagnostics in nine
known lint classes outside F3. Allowing only those nine established classes
produces a clean Clippy result. The unchanged semantic failures remain:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

## Structural Gates

The routes were checked separately so evidence ownership, accounting, and
authority could not be merged under one synthetic owner:

```text
f3-dual-evidence-owner          PASS
f3-dual-accounting-owner        PASS
f3-dual-authority-isolation     PASS
```

The first worksheet drafts returned VETO because their owner metadata mixed
the classifier with helper functions. The worksheets were repaired to model
each local decision under `EffectLawDualClassifierV3`; no code or candidate
triad was weakened to obtain PASS.

Graphify was updated after the code change. The graph contains the new
classifier/report community and no production caller route.

## Diff Ownership

F3 owns only:

```text
effect_law_v3/dual_classifier.rs       classifier and report schema
effect_law_v3/canonical.rs             private single-row grouping facets
effect_law_v3.rs                       shadow module/export boundary
effect_law_dual_classifier_v3_tests.rs focused F3 corpus and tests
effect_law_v3_tests.rs                 test-only fixture exposure
lib.rs                                 public diagnostic exports
STOP_F3_*.json / STOP_F3_*.md          machine and human receipts
graphify-out/                          generated graph update
```

No F3 code imports or mutates `SemanticAliasGraph`, generation ownership,
miner state, Wave, thresholds, selectors, runtime execution, checkpoint,
admission, or ACTIVE authority.

## Repository State

```text
HEAD         32ce298799b331db32a311654c070ad5c393a00e
origin/main  23c04b728999716c53c988b0e67f03df034cefe5
branch       main, ahead 2 before F3
```

The existing dirty F0-F2 slice and unrelated untracked diagnostic file were
preserved. No commit was created and nothing was pushed.

## Stop Boundary

```text
F3 implementation and fixture-shadow gate   COMPLETE
production dual-run coverage                 NOT CLAIMED
B1 binding evidence                          INSUFFICIENT_BINDING_EVIDENCE
F4 ProtocolMode compiler                     BLOCKED until B1
runtime / admission / ACTIVE wiring          UNCHANGED
```

Work stops here at STOP-F3.
