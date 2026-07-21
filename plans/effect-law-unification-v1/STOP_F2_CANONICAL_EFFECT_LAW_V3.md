# Effect Law Unification: Canonical STOP-F2 V3

Date: 2026-07-21 Europe/Tallinn

Status: **F2 COMPLETE / CANONICAL EFFECT LAW V3 PASS / F3 NOT STARTED /
AUTHORITY FALSE**

This receipt supersedes the rejected F2 and the F2R2-F2R4 candidates. It
closes only the shadow canonical-law and trusted-restart contract. It does not
change grouping, generation, runtime selection, admission, deployment, or
execution authority.

## Final Route

```text
externally pinned generation manifest root
-> trusted capture + parity + observed-state set
-> real actor execution
-> independent verifier
-> observed delta == teacher claim
-> source-neutral quotient over independent surfaces
-> CanonicalEffectLawV3
-> deterministic restart bundle
-> externally pinned TrustedEffectLawBundleRootV3
-> root validation before bundle decoding
-> manifest/dictionary/hypothesis/version rebinding
-> trusted byte-identical restart
```

External admission remains the only production authority.

## Final Restart Repair

F2R4 still allowed a caller to alter an internally valid law, mapping, or
delta root and recompute the unkeyed proof and bundle hashes. Evidence
membership remained valid, so that route proved integrity but not provenance.

Canonical F2 adds an opaque `TrustedEffectLawBundleRootV3` with private fields
and no production constructor. Its digest binds:

```text
exact bundle bytes
generation manifest root
dictionary root
quotient hypothesis root
canonicalizer version
```

The restart loader validates this capability before deserializing bundle
bytes, then rechecks the parsed law identity against the pinned context. The
only pin helper is under `#[cfg(test)]`; F2 therefore cannot create its own
production trust.

## Mandatory Forgery Matrix

```text
original trusted bundle                                  PASS, byte-identical
different generation manifest                           REJECT
valid mapping permutation + all hashes recomputed       REJECT InvalidTrustRoot
changed law + action root + bundle hash recomputed      REJECT InvalidTrustRoot
changed delta roots + proof and bundle hashes recomputed REJECT InvalidTrustRoot
candidate constructs bundle capability in production   IMPOSSIBLE through safe API
```

## Canonical F2 Matrix

```text
external manifest provenance                            PASS
real actor + independent verifier                       PASS
observed delta versus teacher claim                     PASS
physical names isolated in ProtocolFacetV3              PASS
alpha/wire rename invariance                            PASS
effect-significant state separation                     PASS
three-dimensional evidence independence                 PASS
ambiguous action equivalence                            ABSTAIN
deterministic canonical bytes                           PASS
trusted restart provenance                              PASS
production callers                                     0
execution authority                                     false
```

## Verification

```text
effect_law_v3::tests                    28 PASS / 0 FAIL
historical effect_law::tests            15 PASS / 0 FAIL
cargo check nando-response-actor        PASS
F2-aware Clippy                         PASS
semantic baseline                       22 PASS / 3 known FAIL
git diff --check                        PASS
```

The three semantic failures are the unchanged `online_collection` baseline:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

Full `-D warnings` remains blocked by the same 12 pre-existing diagnostics
outside F2. Allowing only those nine known lint classes produces a clean F2
Clippy result.

## Structural And Live Gates

The first combined worksheet mixed restart provenance and authority owners and
remains preserved as `VETO`. After splitting by decision owner:

```text
f2r5-bundle-root-owner          PASS
f2r5-bundle-authority-isolation PASS
```

The repository-local composite gate passed without mutation:

```text
/home/ubu/projects/nando-wave/ops/phase-center-test-server/bin/nando-live-transition-gate
verdict                         PASS
active false accepts            0
runtime parity mismatches       0
response M3                     WATCH
```

The shared PATH wrapper was also invoked once, resolved `project_root=/home`,
and returned `ERROR`; that result was not relabelled. The canonical
project-local command above resolved the correct repository and passed.

## Source Commitments

```text
effect_law_v3.rs             620 lines  283c83d041b437207828b60a0b8147e294e5c7dc1cfb83c6df4fc6fb5c758bae
effect_law_v3/trust.rs       365 lines  4bac14e6d385a2d17125055de7be1f0081765c58b17e23fc54fff2e9b924d6d7
effect_law_v3/canonical.rs   820 lines  3be74e029c41920e024eebecb6759d8c246dccc83e071721ac53994b56a70223
effect_law_v3/evidence.rs    895 lines  e63655b54292e2aef78c6bdb3a3d30d18d57c9fc660080bd01b1f0d19dfababb
effect_law_v3_tests.rs      1062 lines  e9d8b770d29e828573df670cd500d0f0d61c36dca121f402a88de628616dae69
```

## Stop Boundary

```text
Canonical F2                    COMPLETE
F3 dual classification          UNLOCKED / NOT STARTED
production root provider        NOT INTEGRATED
production receipt corpus       NOT CLAIMED
B1 binding evidence             INSUFFICIENT_BINDING_EVIDENCE
ProtocolMode compiler           BLOCKED until B1
runtime/admission wiring        UNCHANGED
execution authority             false
commit/push/deploy/restart       NO
```

F3 may compare legacy signatures and V3 law IDs over the same trusted rows in
shadow. It may not use restored law data as authority, alter grouping, or
change runtime/admission behavior.
