# Effect Law Unification: STOP-F2R3 Candidate

Date: 2026-07-21 Europe/Tallinn

Status: **F2R3 IMPLEMENTED / SAFETY PASS / CANONICAL REVIEW REQUIRED / F3 NOT STARTED**

F2R3 replaces the rejected F2R2 proof boundary. It remains a shadow-only,
uncommitted candidate and has no generation, grouping, runtime, verifier, or
admission caller.

## Git And Authority

```text
HEAD                         32ce298799b331db32a311654c070ad5c393a00e
origin/main                  23c04b728999716c53c988b0e67f03df034cefe5
commit created               NO
push                         NO
production deployed          NO
services restarted           NO
F3 started                   NO
execution_authority          false
live service                 active; original PID 263278
live composite status gate   PASS
```

Live authority receipt:

```text
/var/lib/nando-wave/transition/response-online-miner-report.json
schema                nando.embedded-response-online-miner.v1
execution_authority   false
checkpoint_restored   true
tail_follow_active    true
```

The live gate was run read-only. The host `/tmp` had a stale per-user quota
failure, so the unchanged gate ran inside `bwrap` with an isolated empty
`/tmp`. All sections passed: structural, wave causal, runtime admission,
response runtime, deployment, and the not-required expression shadow.

## Repaired Route

```text
TeacherTransition
|
+-- ObservationCandidateV3
|   `-- complete exact EffectDeltaContractV3
|
+-- ImmutableEffectReceiptResolverV3
|   +-- CaptureCommitmentIndex membership
|   `-- sealed DurableRuntimeParityReceipt
|
+-- SealedEffectObservationV3
|   +-- episode lineage
|   +-- physical surface root
|   +-- physical program ID
|   +-- capture / parity / verifier roots
|   `-- preserved physical graph and constants
|
+-- multidimensional independence
|   +-- >= 2 episode lineages
|   +-- >= 2 surface roots
|   `-- >= 2 physical program IDs
|
+-- bounded physical-adapter quotient
|   +-- project transport carrier atoms only
|   +-- retain all other effect atoms
|   +-- canonicalize topology and relation program jointly
|   `-- one action-equivalence class or ABSTAIN
|
`-- CanonicalEffectLawV3 candidate
    `-- restart bundle: law + mappings + proof roots
```

There is deliberately no `VerifiedCanonicalEffectLawV3`. A canonical law
candidate can only be produced from private sealed observations, but it still
does not grant authority.

## Provenance Boundary

The candidate checksum is only an integrity hint. Sealing additionally
requires:

```text
capture receipt present
capture receipt is a member of the immutable index
parity receipt digest is valid
parity evidence_ref equals the transition frame
provider payload digest matches
teacher response digest matches
actor response equals teacher response
independent verifier root is valid
```

Candidate and sealed-observation fields are private. Neither type has a public
unchecked constructor. A recomputed candidate checksum with a forged frame ID
still fails immutable parity lookup.

## Complete Effect Delta

F2R3 no longer treats `EffectGraph` as the complete effect. The observation
keeps exact before/action atoms and separately commits:

```text
preconditions
action relations
CompletionState / ResponseShape / OutputStatus
renderer and status mapping
temporal relations
cardinality relations
typed constants and argument ownership
preserved-frame relations
```

Only `ActionFunction`, `ActionCustomTool`, and `ActionInnerTool` are classified
as projectable physical transport surface. Empty strings, `false`, integers,
operations, physical value types, postconditions, and mappings remain
effect-significant until a later evidence-backed protocol contract proves
otherwise.

Consequences established by tests:

```text
direct transport / wrapped transport, same full effect   one candidate law
wait / terminate, same role shape                        no common law
changed completion / response / status                   no common law
changed renderer / status mapping                        no common law
changed temporal / cardinality contract                  no common law
changed preserved frame                                  no common law
atom reordering, same effect                             same law
```

`wait` and `write_stdin(chars="")` are not declared equivalent by F2R3. That
protocol claim remains blocked until independent evidence supports it.

## Symmetry And Restart

Canonicalization enumerates a bounded set of color-preserving mappings.
Mappings that produce the same topology must collapse to one action-equivalence
root. Multiple roots return `AmbiguousActionEquivalence`; hash ordering never
creates authority.

Restart bytes include the canonical law, every physical-to-canonical mapping,
exact-delta roots, capture roots, parity roots, verifier roots, proof-set root,
and bundle root. Restore validates collection order, duplicates, node bounds,
mapping bijection, action-equivalence root, proof root, bundle root, and exact
canonical bytes.

## Focused Matrix

```text
F2R3 adversarial tests                          24 / 24 PASS
historical F2R2 tests                          15 / 15 PASS
cargo check -p nando-response-actor --lib      PASS
git diff --check                               PASS
semantic baseline                              22 PASS / 3 known FAIL
new semantic regressions                       0
```

The three unchanged baseline failures remain:

```text
online_collection::semantic_program_pool_survives_field_renames_and_collects_future
online_collection::semantic_count_inside_teacher_prose_reaches_external_admission
online_collection::multi_output_semantic_program_reaches_external_admission
```

Full `cargo clippy -- -D warnings` remains blocked by the same 12 pre-existing
diagnostics in `online.rs`, `online_collection.rs`, `online_state.rs`,
`operator_vm.rs`, `runtime.rs`, and `semantic_alias.rs`. No diagnostic points
to an F2R3 file. Those unrelated warnings were not changed in this slice.

## Structural Gates

Source triads were extracted from implementation lines. Candidate triads were
independently extracted from adversarial tests and the caller diff; they do not
use `candidate_diff` as both evidence sides.

```text
f2r3-provenance              PASS  complexity 12
f2r3-effect-delta            PASS  complexity 12
f2r3-quotient-boundary       PASS  complexity 10
f2r3-independence            PASS  complexity 12
f2r3-independence-negative   PASS  complexity  6
f2r3-symmetry                PASS  complexity 15
f2r3-restart                 PASS  complexity 17
f2r3-authority               PASS  complexity 14
```

Receipt directory:

```text
/home/ubu/projects/nando-wave/target/f2r3-tmp/nanda-structural-gate/
```

The first combined delta and independence packets returned VETO because they
mixed owners. They were repaired by splitting quotient projection from delta
ownership and negative evidence from independence counting. No threshold was
weakened.

## Source Ownership

```text
effect_law_v3.rs             507 lines  00dfb3cc4130f59f74ea965fd0f0edff21d23919058ed35057d00f33deb3ab43
effect_law_v3/evidence.rs    768 lines  f6cce30517d92e8e2c7dd8d99172e56270a9fd66af373230b3a535e435d92db1
effect_law_v3/canonical.rs   626 lines  a9a621152ba55f1fbd4036b7027bd3f78157e974a1c692430286fdbaaf00d1f1
effect_law_v3_tests.rs       699 lines  b49ab3be4cc12b396dc688cbf35ad42cd37e775d7a31a818d382b2781e54d44b
```

`admission_bundle.rs` now owns durable parity receipt sealing and validation;
`online_admission.rs` consumes that owner instead of carrying a duplicate
digest implementation.

Graphify update completed: 23,622 nodes, 53,483 edges, 1,022 communities.

## Remaining Boundary

F2R3 stops here. The next allowed action is architecture review of this
candidate. Until that review accepts canonical F2:

```text
F3 dual classification       FORBIDDEN
ProtocolMode compiler        FORBIDDEN
runtime role binding         FORBIDDEN
generation/admission wiring  FORBIDDEN
authority                    OFF
```

B1 binding evidence remains a separate unresolved route. F2R3 does not invent
a continuation selector, use raw prefixes as authority, or treat teacher
identity as binder input.
