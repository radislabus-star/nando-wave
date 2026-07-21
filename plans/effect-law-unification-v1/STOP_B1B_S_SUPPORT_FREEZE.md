# STOP-B1B-S: Label-Blind Support Freeze

Date: 2026-07-21 Europe/Tallinn

```text
B1B0R preregistration                COMPLETE
B1B-S controlled support capture    COMPLETE
support rows                        12
support session lineages             4
I1-I6 rows                           2 each
expected labels joined              false
support label denominators          PENDING_TRUSTED_RESOLVER
H0 / H1                             UNPROVEN / UNPROVEN
future                              NOT_OPENED
F4                                  BLOCKED / NOT STARTED
execution authority                 false
```

## Scope

STOP-B1B-S implements and exercises the support-only half of the B1B
acquisition protocol. It introduces an independent capture-owner boundary,
collects a controlled label-blind causal corpus, and seals an immutable capture
prefix and watermark before any future row exists.

This STOP does not resolve trusted labels, classify the planned positive and
applicability-negative slots, adjudicate H0 or H1, compile a selector or
`ProtocolMode`, modify Wave, or grant runtime authority.

The 12 rows are a controlled causal acquisition corpus produced by the bounded
B1B-S acquisition entrypoint. They are not claimed to be production traffic.
The six planned applicable slots and six planned negative-or-ambiguous slots
remain unlabelled until the later trusted resolver joins independently sealed
labels.

## Frozen Signal Path

```text
controlled pre-action interventions I1-I6
-> canonical EvidenceLedgerRecord
-> CaptureCommitmentIndex
-> label-blind PreActionBindingSurfaceV1
-> frozen CandidateRelationGraph
-> BindingSupportCaptureBatchV1
       |
       v
independent support capture-owner process
-> canonical evidence-record digest validation
-> capture receipt/index membership validation
-> exact capture-index byte recheck
-> BindingSupportFreezeV1
-> immutable support watermark at next_sequence=12
       |
       X  STOP-B1B-S
       |
       +-> B1B-F future capture             NOT OPENED
       +-> trusted label resolver           NOT STARTED
       +-> H0/H1 adjudication               NOT STARTED
       +-> F4 ProtocolMode compiler         BLOCKED
```

The capture owner has no future API, no expected-label input, and no
adjudication route. `freeze(self)` consumes the owner. The filesystem entrypoint
re-reads the capture-index bytes immediately before output and rejects any
concurrent change instead of moving the support/future boundary.

All three durable outputs are created with `create_new`; an existing freeze,
watermark, or report cannot be overwritten in place.

## Support Evidence

```text
support rows                         12
distinct session lineages            4
watermark next_sequence              12
I1 reorder, preserve linkage          2
I2 change linkage, preserve order     2
I3 add same-type decoy                2
I4 completed parent                   2
I5 two active parents                 2
I6 output without matching parent     2
candidate extraction budgets      bounded
relation extraction budgets       bounded
expected labels joined              false
teacher/post-action input              0
```

The owner derives the pre-action wire root and session lineage from the
canonical normalized evidence record. A row cannot supply those roots as
self-declared strings. Every row is re-frozen and checked against the concrete
capture receipt and the capture-index record with the same sequence and digest.

One session may contribute multiple interventions, but it may not contribute a
duplicate `(intervention, session lineage)` support vote. At least three
independent support sessions and every preregistered intervention are required
before freeze.

## Fail-Closed Matrix

```text
foreign capture receipt                    REJECTED
record absent from capture index            REJECTED
recomputed invalid ledger record            REJECTED
forged frozen graph root                    REJECTED
duplicate row or evidence reference         REJECTED
duplicate lineage/intervention vote         REJECTED
missing I1-I6 denominator                   REJECTED
fewer than three session lineages           REJECTED
capture index changes during freeze         REJECTED
non-canonical restart bytes                 REJECTED
future_opened tampering                     REJECTED
row-order shuffle                           BYTE-IDENTICAL
restart reconstruction                      BYTE-IDENTICAL
```

## Machine Artifacts

```text
plans/effect-law-unification-v1/STOP_B1B_S_FREEZE.json
SHA-256  8a856f4abc9f56b3c618c78acb0d38e9a93b3b10a0f0a7cf1dc0248f38bb2a5f

plans/effect-law-unification-v1/STOP_B1B_S_WATERMARK.json
SHA-256  4eb5160744dfdf0ffd3717fbe91a918cadab797532b3bfa733172b3926dcbaa5

plans/effect-law-unification-v1/STOP_B1B_S_SUPPORT_FREEZE.json
SHA-256  e46e8c4dc3857ce00f35ad139d9db558a413620182df8af2c0dc47f0e85770b9
```

The checked-in test reconstructs the freeze from canonical bytes, derives its
report, and compares it with the machine report. It also checks that the
watermark bytes and both artifact hashes match.

Ephemeral acquisition receipts remain at:

```text
/home/ubu/tmp/nando-b1b-s.FkkoJ8/
capture-index.cbor       0d46b9bd6d6f60069c7887cf079d3140d8fbc5534e44d3fa4ee1986675fb5925
support-batch.json       559b9068713f83f7b3476830420df91c08b944cc11f80cafe75106d283f98ff7
acquisition-report.json  63bc52e77393e63637c7ec16fbae5720b7303fae3efb1507e24f32d2fae8c653
```

A privacy scan found no raw expected labels, requests, provider payloads, raw
handle values, or absolute paths in the checked-in STOP artifacts.

## Ownership

```text
controlled corpus producer     nando-binding-support-acquire
support capture authority      nando-binding-support-capture-owner
support validation/domain      binding_evidence_capture_owner.rs
future collection              absent from this slice
trusted label resolution       binding_evidence_preregistration.rs / later B1B
H0/H1 adjudication             absent from this slice
runtime execution              unchanged
external admission             unchanged
```

The support module depends on frozen binding graphs, capture provenance, and
the preregistered watermark schema. Runtime, Wave, generation, admission, and
authority modules do not call the B1B-S owner.

## Verification

```text
B1B-S capture-owner tests               14 / 14 PASS
binding-filter library route            43 / 43 PASS
cargo check -p nando-response-actor            PASS
scoped rustfmt                                 PASS
new Clippy findings in B1B-S files                0
pre-existing Clippy findings, library target       12
additional pre-existing test-target findings        8
golden report/freeze/watermark parity            PASS
NANDA ownership/trust/stage routes          3 / 3 PASS
Graphify update                                  PASS
git diff --check                                PASS
```

The live `binding_` filter completed in 0.86 seconds with 36,796 KiB maximum
RSS. `cargo clippy --lib -D warnings` reproduced the 12 previously recorded
library debts in unrelated legacy files. `--all-targets` additionally exposed
eight existing test-only findings outside the B1B-S module. None are repaired
or represented as new B1B-S debt in this slice.

Read-only service state remained unchanged:

```text
nando-response-learning InvocationID    8e59505eb1b943778601c9b3bacbd607
nando-transition-serving InvocationID   74ac3080f80b4fe387de2a94380e3657
service restart                         NOT PERFORMED
deployment                              NOT PERFORMED
execution authority                     false
```

## STOP-B1B-S

```text
support capture owner               IMPLEMENTED
controlled support corpus           FROZEN
physical watermark                  SEALED
trusted labels                      NOT JOINED
positive/negative classification    PENDING
H0 / H1                             UNPROVEN
B1B-F                               NOT OPENED
B1B adjudication                    NOT STARTED
F4                                  BLOCKED / NOT STARTED
runtime / services / authority      UNCHANGED
```

Work stops here. The next permissible stage is B1B-F through a separate
post-freeze acquisition process that proves exact prefix extension and uses
session lineages disjoint from this support set. Resolver/adjudication may run
only after the future candidate graphs are frozen. F4 remains forbidden until
STOP-B1B establishes a unique action-equivalence class with zero wrong
bindings, or returns `INSUFFICIENT_BINDING_EVIDENCE`.
