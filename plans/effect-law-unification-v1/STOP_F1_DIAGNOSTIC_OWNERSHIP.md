# Effect Law Unification: STOP-F1

Date: 2026-07-20 Europe/Tallinn

Status: **F1 COMPLETE / STOP / F2 NOT STARTED**

This receipt closes diagnostic ownership extraction only. It does not change
signatures, semantic grouping, selectors, replay decisions, runtime actions,
generation identity, verifier behavior, admission, or authority.

## Source State

```text
HEAD                         23c04b728999716c53c988b0e67f03df034cefe5
branch                       main
commit created               NO
production deployed          NO
services restarted           NO
execution authority          false
top-level miner false accepts 0
```

The worktree contains the accepted uncommitted F0/R1 slice. F1 preserves those
changes and does not claim ownership of unrelated dirty files.

## Ownership Map

```text
online_state.rs
  owns evidence storage, CEGIS state, generations, parity reservoirs
  exposes the existing immutable audit method through an inherent impl
          |
          v
online_diagnostics.rs
  owns audit schemas, replay matrix, hash provenance, candidate diagnostics
  receives &StreamingSelfTrainingState
  cannot mutate state or authority
          |
          v
runtime.rs
  owns bounded neutral parsing primitives
  no longer owns selector candidate count/provenance diagnostics
```

Dependency direction:

```text
online_state::diagnostics -> online_state private read-only state
online_state::diagnostics -> runtime neutral parser primitives
online_state              -X-> diagnostic replay helpers
runtime                   -X-> diagnostic candidate ranking
diagnostics               -X-> generation/admission/authority mutation
```

The child-module placement is deliberate: Rust privacy permits immutable
inspection of private miner storage without making storage fields or CEGIS
internals `pub(crate)`. The public crate API remains available through the same
`online_state` and crate-root re-exports.

## File Ownership

```text
crates/nando-response-actor/src/online_diagnostics.rs  NEW diagnostic owner
crates/nando-response-actor/src/online_state.rs        storage owner only
crates/nando-response-actor/src/runtime.rs             neutral parser owner
crates/nando-response-actor/src/lib.rs                 unchanged public exports
crates/nando-response-actor/src/online.rs              unchanged caller API
```

Line budgets after extraction:

```text
online_state.rs        4089
online_diagnostics.rs   840
runtime.rs             3839
```

F1 is a move-only ownership cut. It does not claim that the remaining large
production files have completed their broader spectral-budget refactor.

## Frozen Parity Input

One read-only source snapshot was used for both binaries:

```text
/home/ubu/tmp/nando-f1-baseline/frames.jsonl
sha256 015efbc76d5b92e2d2a8e114a26c4a209081edb91eb323c06cec9d7b45453e83

/home/ubu/tmp/nando-f1-baseline/checkpoint.cbor
sha256 556b64ee2465039dd5ac1ef53000b15d755cff2573749c98801cba1dfc540a5c
```

Accepted F0 source binary:

```text
/home/ubu/tmp/nando-r1/nando-online-response-diagnose-stop-f0-debug
sha256 c31b8759ec0fb5c46d7057e1e415aae5f0770d12d36b065db74c505223898cbb
```

The earlier non-debug `stop-f0` binary predates the final
`selector_candidates` field and is not the accepted byte-parity baseline. Its
rejection is provenance reconciliation, not a relaxation of the parity gate.

## Byte-Identical Audit

```text
F0 output /home/ubu/tmp/nando-f1-baseline/old-debug.json
F1 output /home/ubu/tmp/nando-f1-baseline/new.json

F0 sha256 df40930ddcc61795b356a98f5307a1a744dfc374f8381dc0a9b28b58a9d74aa8
F1 sha256 df40930ddcc61795b356a98f5307a1a744dfc374f8381dc0a9b28b58a9d74aa8
cmp         PASS
```

Preserved invariants:

```text
rows                         129
actors                         6
reasoned actor cells         774
write_stdin projection   24/3/6/0
schema nando.semantic-law-evidence-audit.v1
```

The accepted STOP-A and STOP-F0 artifacts also remain unchanged:

```text
STOP-A e9e43513bca355a0ec77588d995c1a77c11188d59d8b1b5fc7dea8b9b1f9e9d0
STOP-F0 ec254a037ebf0e7bfb84af1d73ede142e28bf5d56ef74dbd54a212e26b63f08c
```

## Tests

```text
cargo +1.97.0 check diagnostic binary                 PASS
focused online_state semantic tests                    3/3 PASS
broad semantic_ baseline                              22 PASS / 3 FAIL
git diff --check                                       PASS
```

The same three pre-existing `online_collection` tests still fail for the same
functional reasons and were not edited:

```text
semantic_program_pool_survives_field_renames_and_collects_future
  -> portable multi-output package missing

semantic_count_inside_teacher_prose_reaches_external_admission
  -> "Total records: 3" != "3"

multi_output_semantic_program_reaches_external_admission
  -> support_phase_adapter_unproven
```

The first test attempt used the nearly full `/tmp` tmpfs and produced storage
errors. It was discarded. The accepted baseline rerun used
`TMPDIR=/home/ubu/tmp/nando-f1-test-tmp` and reproduced the original three
functional failures exactly.

Clippy with `-D warnings` remains blocked by 12 pre-existing warnings in
`online.rs`, `online_collection.rs`, `online_state.rs`, `operator_vm.rs`,
`runtime.rs`, and `semantic_alias.rs`. No warning points to
`online_diagnostics.rs`. F1 does not fix or suppress that unrelated debt.

## Structural Gates

The first combined ownership worksheet returned VETO because it mixed three
decision owners. The corrected owner-local gates are:

```text
diagnostic owner       PASS
online state owner     PASS
runtime owner          PASS
byte parity            PASS
authority/lifecycle    PASS
```

Trace directory:

```text
/home/ubu/tmp/nando-f1-baseline/nanda-tmp/nanda-structural-gate/
```

The initial VETO traces remain in that directory; they were not relabeled.

## Resource Metrics

```text
F0 baseline replay   2:01.72  max RSS 644812 KiB
F1 candidate replay  2:03.09  max RSS 647900 KiB
RSS delta                         +3088 KiB
```

These are debug replay measurements over the 112 MiB frames and 131 MiB
checkpoint snapshot. They are proof-path metrics, not production latency.

## Authority Boundary

Live verification after F1:

```text
execution_authority          false
top-level miner false accepts 0
service start                2026-07-20 05:38:30 EEST
service restart by F1        NO
```

No checkpoint, generation, package, threshold, registry, service, or live route
was modified.

## Unresolved

1. `INSUFFICIENT_BINDING_EVIDENCE` remains unchanged.
2. B1 must collect label-free causal relations before F4.
3. The three `online_collection` baseline failures remain separate debt.
4. Global Clippy remains red on the recorded pre-existing warnings.
5. F2 and F3 have not started.

## Stop

F1 is complete. Work stops here. No `CanonicalEffectLawV2`, shadow grouping,
BindingEvidencePackage, ProtocolMode, selector, runtime, verifier, generation,
or admission implementation is authorized by this receipt.
