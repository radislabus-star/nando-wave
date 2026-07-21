# STOP-B1B-F0: Future Acquisition Freeze

Date: 2026-07-21 Europe/Tallinn

```text
B1B-S support freeze                 COMPLETE / PINNED
B1B-F0 future protocol              COMPLETE / FROZEN
future rows captured                0
expected labels joined              false
H0 / H1                             UNPROVEN / UNPROVEN
B1B-F                               NOT OPENED
F4                                  BLOCKED / NOT STARTED
execution authority                 false
```

## Scope

STOP-B1B-F0 freezes the future-only acquisition mechanism before the first
future event. It does not run acquisition, join a label, adjudicate H0/H1,
compile a selector or `ProtocolMode`, modify Wave, or grant runtime authority.

The future owner is pinned to commit
`a3f8f05f5b8d8928c1044ed38eb107fb8be27e5b`, the exact B1B-S freeze bytes,
freeze receipt, watermark bytes, capture-index root, and
`next_sequence = 12`. A locally recomputed replacement is rejected.

## Frozen Route

```text
checked-in B1B-S freeze + watermark
-> exact SHA-256 and receipt validation
-> exact capture-index prefix extension
-> sequences 12..23 only
-> 12 fixed label-blind slots
-> I1-I6, two rows each
-> four future-only session partitions
-> pre-action payload/graph parity
-> unseen structural challenges
-> future freeze capability
       |
       X  STOP-B1B-F0: capability implemented, acquisition not run
       |
       +-> B1B-F physical capture       NOT OPENED
       +-> trusted label resolver       NOT STARTED
       +-> H0/H1 adjudication           NOT STARTED
       +-> F4                            BLOCKED
```

`BindingFutureCaptureOwnerV1` has no label, teacher, post-action, hypothesis,
compiler, admission, or authority input. It rebuilds the candidate graph from
the same raw pre-action payload whose canonical payload digest appears in the
evidence record; it does not accept a caller-supplied frozen graph.

## Frozen Future Design

```text
planned rows                         12
session slots                         4
rows per intervention I1-I6           2
minimum distinct wire shapes          3
future shapes reused from support      0 allowed
future field names reused from support 0 allowed
ordinal/layout trap pairs             >=1
candidate action set across trap      identical
candidate order authority             false
raw payload persisted                 false
expected labels available             false
teacher/post-action available          false
```

The ordinal/layout trap requires two surfaces for one intervention to expose
the same action-equivalence set through different wire shapes. This tests a
structural transfer law rather than the current number of extractor feature
splits.

## Event-Time And Lineage Rules

The later capture index must contain the entire B1B-S prefix byte-for-byte and
exactly append sequences 12 through 23. The first future record must chain from
the last support record; every later record must chain from its immediate
predecessor.

Each frozen session slot maps to exactly one session lineage and different
slots must map to different lineages. Every future lineage must be absent from
the support set. A slot is single-use.

Restart is not authorized by the future bundle's local digest alone.
Reconstruction requires the exact pinned support-freeze and watermark bytes
plus a future receipt persisted outside the bundle, then repeats prefix,
capture-record membership, lineage, shape, field-name, and ordinal-trap
validation.

## Attack Matrix

```text
recomputed protocol checksum after mutation       REJECTED
replaced support freeze                           REJECTED
replaced watermark                                REJECTED
non-prefix capture index                          REJECTED
restart with recomputed foreign support prefix    REJECTED
restart without exact pinned support artifact     REJECTED
restart with another row's capture receipt        REJECTED
recomputed derived graph under original root      REJECTED
historical/support session reused as future       REJECTED
payload not matching canonical pre-action graph   REJECTED
duplicate future slot                             REJECTED
missing slot                                      REJECTED
one session slot split across lineages            REJECTED
repeated support-like layout                      REJECTED
no ordinal/layout trap                            REJECTED
exact bounded proof fixture                       PASS / NOT EVIDENCE
restart reconstruction                            BYTE-IDENTICAL
```

## Artifacts

```text
crates/nando-response-actor/src/binding_evidence_future_capture.rs
crates/nando-response-actor/src/binding_evidence_future_capture_tests.rs
plans/effect-law-unification-v1/STOP_B1B_F0_FUTURE_ACQUISITION_PROTOCOL.json
```

The JSON protocol is generated from Rust and checked byte-for-byte in tests.
Its receipt SHA-256 is:

```text
9b167a72feefe5a80d619f345a69995ccf4a045fa09c7cc69425dc02e33dbd51
```

## Verification

```text
F0 causal and attack tests            22 / 22 PASS
complete binding route tests           65 / 65 PASS
cargo check                                      PASS
rustfmt                                           PASS
owner-local NANDA routes                  7 / 7 PASS
live composite structural routes          3 / 3 PASS
future evidence rows                   0
production callers                     0
expected labels joined             false
H0 / H1                         UNPROVEN
F4                               BLOCKED
execution authority                 false
```

The full response-actor library baseline is `466 PASS / 26 known FAIL` across
492 tests. All 22 F0 tests pass; none of the 26 failures names an F0 module.
Clippy reports the same 12 pre-existing errors outside the F0 files and no F0
diagnostic. These unrelated debts are not converted into F0 failures or
represented as repaired.

The read-only live composite gate passes when invoked with the explicit project
root. It still reports `eligible_for_local_accept = false`, zero active response
packages, zero false accepts, zero runtime parity failures, and `M3 = WATCH`.
The learning and serving InvocationIDs remain respectively
`8e59505eb1b943778601c9b3bacbd607` and
`74ac3080f80b4fe387de2a94380e3657`; neither service was restarted.

Graphify final update: `24,222 nodes / 55,313 edges / 1,035 communities`,
PASS.

## STOP-B1B-F0

```text
future protocol                   SEALED
future owner                      IMPLEMENTED / UNUSED
proof-only future fixtures        PASS / EXCLUDED FROM EVIDENCE
physical future capture           NOT OPENED
trusted labels                    NOT JOINED
H0 / H1                           UNPROVEN
F4                                BLOCKED
runtime / services / authority    UNCHANGED
```

Work stops here. The next permissible stage is a separate B1B-F run of this
exact protocol. It must first freeze all 12 future candidate graphs and stop
again before trusted labels or adjudication are joined.
