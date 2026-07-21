# Effect Law Unification: STOP-F2R4 Trusted Candidate

> **Historical candidate only.** Canonical F2 is closed by
> `STOP_F2_CANONICAL_EFFECT_LAW_V3.md`. F2R4 did not yet bind restored law bytes
> to an external bundle capability.

Date: 2026-07-21 Europe/Tallinn

Status: **F2R4 IMPLEMENTED / SAFETY PASS / CANONICALIZATION CORE PASS /
CANONICAL F2 REVIEW REQUIRED / F3 NOT STARTED**

F2R4 repairs the final trust, effect-delta, source-neutrality, and restart
findings from the F2R3 review. It remains an uncommitted shadow-only candidate.
It has no generation, grouping, runtime, admission, or authority caller.

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
```

Read-only live receipt:

```text
/var/lib/nando-wave/transition/response-online-miner-report.json
mtime                  2026-07-21 03:20:38 Europe/Tallinn
schema                 nando.embedded-response-online-miner.v1
execution_authority    false
checkpoint_restored    true
tail_follow_active     true
```

The service was inspected only. It was not restarted or reconfigured.

## Final Route

```text
[generation manifest root owner]
  |
  +-- externally pinned opaque root capability
  |     `-- no production constructor in F2R4
  |
  `-- [trusted evidence resolver]
        |
        +-- canonical manifest bytes == pinned root
        +-- capture index membership
        +-- sealed parity receipt membership
        +-- independently committed observed state
        |
        `-- TrustedEffectEvidenceSetV3
              |
              `-- [observation sealer]
                    |
                    +-- execute_response(raw request, raw payload)
                    +-- independent verifier(actor response)
                    +-- observed delta != teacher claim -> REJECT
                    |
                    `-- VerifiedEffectDeltaReceiptV3
                          |
                          +-- [ProtocolFacet]
                          |     `-- names, fields, selectors, labels
                          |
                          `-- [canonical law normalizer]
                                +-- canonical nodes + argument ordinals
                                +-- bounded alpha-renaming
                                +-- effect-significant invariants retained
                                `-- ambiguous action classes -> ABSTAIN
                                      |
                                      `-- CanonicalEffectLawV3 candidate
                                            |
                                            +-- [restart loader]
                                            |     `-- byte-identical bundle
                                            |
                                            `-- [restart trust validator]
                                                  +-- proof-root rebinding
                                                  +-- episode independence
                                                  +-- surface independence
                                                  `-- physical-program independence
```

External admission remains outside this tree and remains the sole owner of
runtime response authority.

## P0 Repairs

### Externally rooted trust

`TrustedGenerationManifestRootV3` is an opaque capability with a private
field. Evidence producers cannot construct it. Manifest decoding starts only
after the supplied bytes match the independently supplied root. Recomputing a
forged capture index, parity receipt, and manifest therefore does not replace
the original trust root.

The only root pin helper is under `#[cfg(test)]`. That is deliberate for this
shadow-only F2 slice: integrating a production generation-root provider would
be F3 behavior and is not hidden inside the canonicalization core.

### Independently checked effect delta

Teacher atoms are claims, not observed truth. Sealing now:

```text
executes the actual ResponseProgram
-> verifies the actual response through the independent verifier
-> binds it to a separately committed observed state
-> reconstructs the delta from observed effect atoms
-> rejects disagreement with the teacher-declared delta
```

The receipt commits the trust-manifest, observed-state, actor, verifier,
teacher-claim, and delta-verifier roots.

### Source-neutral law identity

Physical argument names, selectors, projection fields, temporal labels, and
cardinality labels are retained in `ProtocolFacetV3`. Canonical law clauses use
canonical node IDs and argument ordinals. Free relation labels undergo bounded
alpha-canonicalization.

The quotient removes only the explicitly declared physical-surface class.
Completion state, response shape, output status, renderer, status mapping,
temporal and cardinality relations, typed constants, and preserved-frame
relations remain effect-significant.

### Trusted restart

Restart requires the opaque trusted evidence set. It reconstructs law bytes,
physical-to-canonical mappings, and proof roots, then rebinds every proof to
trusted transition, capture, parity, verifier, observer, resolver, manifest,
and delta-verifier roots. Episode, surface, and physical-program independence
are recomputed instead of restored as caller-provided counters.

## Mandatory Matrix

```text
fully recomputed fake evidence vs original external root    REJECT
renamed arguments / fields / selectors / role labels       SAME LAW ID
teacher claim vs observed postcondition mismatch            REJECT
restart with original trusted evidence                      BYTE IDENTICAL
restart with different trusted generation                   REJECT
restart 3D independence                                     RECOMPUTED
real execute_response + independent verifier route          PASS
wait vs terminate                                           NO COMMON LAW
symmetric non-equivalent bindings                           ABSTAIN
```

The actor/verifier test calls the production execution and independent
verification functions. Its input evidence is still a structured fixture, not
a production receipt. F2R4 therefore proves the route contract, not live-data
coverage.

## Verification

```text
cargo +1.97.0 test -p nando-response-actor --lib effect_law_v3::tests
  28 / 28 PASS                                              0.43 s

cargo +1.97.0 test -p nando-response-actor --lib effect_law::tests
  15 / 15 PASS                                              0.17 s

cargo +1.97.0 test -p nando-response-actor --lib semantic_
  22 PASS / 3 known FAIL                                   17.69 s

cargo +1.97.0 check -p nando-response-actor --lib
  PASS                                                       0.16 s

cargo +1.97.0 clippy -p nando-response-actor --lib -- -D warnings
  12 pre-existing diagnostics outside F2R4

same Clippy run with only those nine known lint classes allowed
  PASS                                                      19.32 s

git diff --check
  PASS
```

The unchanged semantic failures are:

```text
online_collection::semantic_program_pool_survives_field_renames_and_collects_future
online_collection::semantic_count_inside_teacher_prose_reaches_external_admission
online_collection::multi_output_semantic_program_reaches_external_admission
```

After stale Cargo targets were removed from `/tmp`, these failures reproduced
with their real baseline assertions. The earlier disk-quota message was not
used as their explanation.

The 12 full-Clippy diagnostics remain in `online.rs`, `online_collection.rs`,
`online_state.rs`, `operator_vm.rs`, `runtime.rs`, and `semantic_alias.rs`. No
diagnostic points to an F2R4 file. Those unrelated warnings were not edited.

## Structural Gates

The first five combined F2R4 worksheets remain preserved as VETO receipts.
They mixed multiple decision owners and one test helper was labelled as a
runtime owner. No VETO was reclassified or weakened.

The repaired proof tree has one decision owner per local route:

```text
f2r4-manifest-root-owner           PASS  conflicts 0  gaps 0
f2r4-trusted-evidence-resolver     PASS  conflicts 0  gaps 0
f2r4-actor-verifier-delta-route    PASS  conflicts 0  gaps 0
f2r4-observed-state-owner          PASS  conflicts 0  gaps 0
f2r4-protocol-facet-extraction     PASS  conflicts 0  gaps 0
f2r4-canonical-law-normalization   PASS  conflicts 0  gaps 0
f2r4-restart-loader                PASS  conflicts 0  gaps 0
f2r4-restart-trust-validator       PASS  conflicts 0  gaps 0
f2r4-authority-isolation           PASS  conflicts 0  gaps 0
```

All nine reports also have `foreign_pull=0`, `repair_count=0`, and one owner
with gravity `1.0`.

Receipts:

```text
/home/ubu/projects/nando-wave/target/f2r4-tmp/nanda-structural-gate/
```

Graphify was updated after the implementation:

```text
nodes          23,673
edges          53,664
communities     1,023
```

The shortest trusted restart path is two hops through
`TrustedEffectEvidenceSetV3`. Caller inventory shows no F2R4 edge into external
admission or runtime authority.

## Source Ownership

```text
effect_law_v3.rs             607 lines  16207065ea1bed904968c74f1c4c0810dd8971550f04f21867fc52890344e907
effect_law_v3/evidence.rs    895 lines  e63655b54292e2aef78c6bdb3a3d30d18d57c9fc660080bd01b1f0d19dfababb
effect_law_v3/canonical.rs   817 lines  bd4e77df4b003ac5fa189c5fc1491e333f52acc58938c1d01985fa657dcb3fa8
effect_law_v3/trust.rs       282 lines  5b2499dfcc525065504095959c666bc21d3147356c1e8c6fada3aa521a1a8d9a
effect_law_v3_tests.rs      1028 lines  c420dab56878249d449ba89e4b77512890564b5053e424b55a57ddd73790b6fc
```

`admission_bundle.rs` remains the owner of durable parity receipt sealing and
validation. `online_admission.rs` consumes that owner. Neither module grants
F2R4 authority.

Unrelated untracked
`crates/nando-response-actor/src/nando-online-response-diagnose.rs` was not
modified.

## Stop Boundary

F2R4 stops at canonical F2 review. The remaining boundaries are explicit:

```text
production trust-root provider    NOT INTEGRATED; belongs to later dual-run path
production receipt corpus         NOT CLAIMED; current proof uses structured fixtures
B1 binding evidence               INSUFFICIENT_BINDING_EVIDENCE remains open
F3 dual classification            NOT STARTED
ProtocolMode compiler             BLOCKED until B1
runtime role binding              BLOCKED until F4
generation/admission wiring       FORBIDDEN in this slice
authority                         OFF
```

No threshold, selector, semantic grouping, legacy signature, runtime behavior,
or authority policy was changed to obtain this result.
