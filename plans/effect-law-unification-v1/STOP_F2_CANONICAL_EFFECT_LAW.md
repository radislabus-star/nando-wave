# Effect Law Unification: STOP-F2 (REJECTED)

> **Historical receipt only. F2 was rejected by architecture review on
> 2026-07-21 and must not be treated as canonical.** The closed `EffectKindV2`
> and unique `EffectRoleKindV2` ontology could not represent Rich Operators
> without Rust changes. F2R was also rejected; the current uncommitted repair
> candidate is documented in `STOP_F2R2_EVIDENCE_BOUND_QUOTIENT_CANDIDATE.md`.
> F3 remains forbidden.

Date: 2026-07-21 Europe/Tallinn

Status: **F2 COMPLETE / STOP / F3 NOT STARTED**

This receipt closes the pure `CanonicalEffectLawV2` identity slice. It does
not switch grouping, generation, runtime, verifier, admission, or authority.

## Source State

```text
F2 parent                    ba150bfd258e8f636df118bbbc052b94ca97263a
branch                       main
production deployed          NO
services restarted           NO
execution authority          false
service active since         2026-07-20 05:38:30 EEST
```

The F2 commit SHA is reported after creating the checkpoint because a commit
cannot embed its own SHA. Excluded untracked diagnostics and `graphify-out/`
remain outside the checkpoint.

## Implemented Route

```text
complete EffectGraph topology
+ VerifiedSemanticFacetsV2
-> CanonicalEffectLawV2
-> deterministic versioned JSON bytes
-> SHA-256 EffectLawId
```

New owner:

```text
crates/nando-response-actor/src/effect_law.rs
  lines                       638
  source SHA-256              93c96ae8478b1d518ce2b52b70700f9b0a18b682091184cf194b2dc273b951e9
  schema                      nando.canonical-effect-law.v2
  golden polling EffectLawId  e47682b462e648fde1bcf896ad33fc0deaf780925cce93359468bdcc33448b7b
```

`effect_law.rs` imports only topology types, `AtomValueType`, Serde, SHA-256,
and standard collections/formatting. It has no dependency on diagnostics,
admission, runtime, verifier, generation, semantic aliases, or online state.

The canonical law and topology fields are private. Deserialization passes
through a private wire representation and the same bounded validator used by
fresh construction. Arbitrary bytes cannot directly manufacture a law ID.

## Bounded IR

```text
CanonicalEffectLawV2
|- CanonicalEffectTopologyV2
|- EffectKindV2
|- EffectRoleV2[]
|- SemanticConstantV2[]
|- EffectPredicateV2[]
|- EffectPostconditionV2[]
`- PreservedFrameContractV2
```

Limits:

```text
nodes          32
edges         256
roles          32
constants      32
preconditions  64
postconditions 64
```

Role and transport names do not enter canonical bytes. Roles reference
canonical topology node indices. Semantic constants accept only SHA-256
commitments and normalize hex case. All bounded collections are sorted and
duplicates are rejected before serialization.

## Identity Matrix

```text
wait(handle) == write_stdin(handle, chars="")       PASS
empty input != non-empty input                       PASS
continue != terminate                                PASS
direct transport == wrapped transport                PASS
renamed roles == same law                            PASS
changed preserved frame != same law                  PASS
ambiguous topology -> no EffectLawId                 PASS
incomplete topology -> no EffectLawId                PASS
restart serialization byte-identical                 PASS
facet ordering and digest case canonical             PASS
```

Focused result:

```text
cargo +1.97.0 test -p nando-response-actor --lib effect_law::tests
9 PASS / 0 FAIL
```

## Compatibility Proof

The F2 code diff adds only:

```text
crates/nando-response-actor/src/effect_law.rs
crates/nando-response-actor/src/lib.rs
plans/effect-law-unification-v1/STOP_F2_CANONICAL_EFFECT_LAW.md
```

Legacy and production owner files are byte-identical to `ba150bf`:

```text
teacher_join.rs       92d776a4c137fcb6016d8e93eea365d9637d7af81f439b428ae59abbf16a3b44
semantic_alias.rs     4fa94ce4e29f9814d89b7543710793b911b9d7ae03638a9283d0f0e19ae0724c
runtime.rs            ae2395289276a7350c5c57875d1fbff4d69c1e83054d7e9ad3fc6855c7ca0f54
verifier.rs           ace4df92814399cfc06525480ccb986cb661471dd229558ff2177d87711a7866
online_admission.rs   f3badde376edbff7ffe8945efd887ca05a29e7336b6167da2b6d0a44bd494b77
```

Therefore `teacher_semantic_law_signature`, `SemanticAliasGraph`, runtime
outputs, verifier decisions, admission, and authority have no F2 caller or
behavior change.

## Baseline And Tooling

```text
cargo +1.97.0 check -p nando-response-actor --lib        PASS
broad semantic_ baseline                                22 PASS / 3 FAIL
git diff --check                                         PASS
graphify update .                                        PASS
```

The same three accepted `online_collection` failures remain:

```text
semantic_program_pool_survives_field_renames_and_collects_future
semantic_count_inside_teacher_prose_reaches_external_admission
multi_output_semantic_program_reaches_external_admission
```

Clippy with `-D warnings` remains globally blocked by 12 pre-existing library
warnings in other owners. The final Clippy log contains zero diagnostics for
`effect_law.rs`; F2 neither fixes nor suppresses unrelated warnings.

The default `stable` rustup installation is incomplete on this host. Accepted
checks use the installed Rust 1.97.0 toolchain, which satisfies workspace
`rust-version = 1.95`.

## Structural Gates

The first mixed-owner worksheet returned `VETO` and remains preserved. After
splitting by decision owner:

```text
effect-law identity             PASS (mandatory, complexity 20)
domain import boundary          PASS
legacy grouping boundary        PASS
production authority boundary   PASS
B1/F4 lifecycle boundary        PASS
F2/F3 stop boundary             PASS
```

Trace directory:

```text
/home/ubu/tmp/nando-f2-gate/tmp/nanda-structural-gate/
```

## Authority Boundary

Live verification at STOP-F2:

```text
/var/lib/nando-wave/transition/response-online-miner-report.json
execution_authority false
nando-transition-serving.service active
service restart by F2 NO
```

No package, generation, registry, checkpoint, threshold, service, or live
route was changed.

## Unresolved

1. `INSUFFICIENT_BINDING_EVIDENCE` remains unchanged.
2. B1 still blocks F4 ProtocolMode and all new runtime binding.
3. F3 may compare legacy signatures and V2 IDs in shadow only.
4. The three `online_collection` baseline failures remain separate debt.
5. Global Clippy debt remains outside F2 ownership.

## Stop

F2 is complete. Work stops before F3. No dual grouping, selector,
ProtocolMode, runtime, verifier, generation, admission, deployment, or
authority implementation is part of this slice.
