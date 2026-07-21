# STOP-B1B0R: Trusted Acquisition Boundary

Date: 2026-07-21 Europe/Tallinn

```text
original STOP-B1B0 freeze             PRESERVED in f13434d
H0 / H1                               UNCHANGED / UNPROVEN
I1-I6                                 UNCHANGED
capture event-time boundary           PASS
capture receipt/index join            PASS
support/future session separation     PASS
minimum sessions per partition        3 / ENFORCED
duplicate lineage vote                REJECTED
acquisition                            NOT RUN
F4                                     BLOCKED / NOT STARTED
execution authority                   false
```

## Scope

STOP-B1B0R repairs the evidence-admission boundary found during review of
STOP-B1B0. It does not collect a row, adjudicate H0/H1, compile a selector or
`ProtocolMode`, modify Wave, or grant runtime authority.

The original preregistration remains recoverable byte-for-byte from commit
`f13434d`. This follow-up changes the active schema to
`nando.binding-evidence-preregistration.v1.r1` before acquisition starts.

## Repaired Signal Path

```text
capture owner at support freeze
-> CaptureCommitmentIndex prefix
-> canonical UntrustedBindingCaptureWatermarkV1 bytes
-> opaque externally pinned watermark root
       |
       v
later support + future manifest
-> full CaptureCommitmentIndex
-> exact prefix-extension proof
-> evidence_ref -> concrete CaptureEvidenceReceipt
-> exact (capture_sequence, capture_record_sha256) join
-> support sequence < watermark.next_sequence
-> future sequence >= watermark.next_sequence
-> support/future session sets disjoint
-> at least three sessions in each partition
-> one vote per (partition, label, intervention, session)
-> TrustedBindingLabelSetV1
```

The old `captured_post_freeze` flag remains a consistency declaration, but it is
no longer sufficient evidence. Event-time authority comes from the separately
pinned watermark and append-only capture-index extension.

## Capture Provenance

Each label envelope now commits:

```text
capture receipt root
capture sequence
capture record SHA-256
pre-action wire root
session lineage SHA-256
partition
intervention
label
```

The manifest carries the concrete capture receipts and the complete capture
commitment index. Resolution fails unless every envelope joins exactly one
receipt, the receipt is present in the index, and its selected record matches
both sequence and digest.

The watermark has a separate opaque root. As with canonical F2 provenance, its
production constructor is intentionally absent at this STOP: the diagnostic or
acquisition producer cannot bless a watermark it just recomputed. The eventual
capture owner must provide that capability.

## Event-Time Split

The frozen watermark contains the capture-index prefix and its next sequence.
The later index must preserve that prefix byte-for-byte.

```text
support record  sequence <  freeze.next_sequence
future record   sequence >= freeze.next_sequence
```

Consequently, changing a historical envelope to `partition=future`, setting
`captured_post_freeze=true`, and recomputing its local checksum is rejected as
`InvalidCaptureChronology`.

## Independent Lineage Votes

The canonical architecture permits multiple future frames from the same
future-only session. It requires at least three distinct sessions and forbids
support/future session overlap. STOP-B1B0R implements that exact boundary; it
does not incorrectly strengthen row count into one session per row.

Repeated frames from one session cannot inflate one causal cell. The trusted
resolver accepts at most one vote for each:

```text
(partition, evaluation label, intervention ID, session lineage)
```

The existing row denominators remain frozen:

```text
positive rows per partition                 >= 6
applicability-negative rows per partition   >= 6
rows per intervention per partition         >= 1
distinct sessions per partition             >= 3
```

## Attack Matrix

```text
recomputed expected-action digest           REJECTED by manifest root
recomputed watermark chronology             REJECTED by watermark root
historical capture relabelled as future      REJECTED
capture receipt not joined to index          REJECTED
later capture index not extending prefix     REJECTED
support/future session overlap               REJECTED
fewer than three sessions                    REJECTED
duplicate session/intervention vote          REJECTED
missing positive denominator                 REJECTED
missing negative denominator                 REJECTED
missing intervention denominator             REJECTED
teacher/post-action observability             REJECTED
valid prefix extension and future split      PASS
```

## Machine Artifact

```text
plans/effect-law-unification-v1/STOP_B1B0R_PREREGISTRATION.json
schema       nando.binding-evidence-preregistration.v1.r1
SHA-256      c7e8346406f4131679de8c4a88d6fb14f7557399551139d70b8f1b86916dded8
```

The generated Rust preregistration is tested byte-for-byte against this JSON.

## Verification

```text
B1B0R preregistration and attack tests       17 / 17 PASS
B1A + B1B0R binding suite                    28 / 28 PASS
cargo check -p nando-response-actor                 PASS
rustfmt                                             PASS
new Clippy findings in B1B0R files                     0
accepted pre-existing Clippy findings                  12
full lib baseline                              430 / 456 PASS
full lib failures outside B1B0R route                  26
NANDA event-time / lineage / authority          3 / 3 PASS
live transition composite gate                       PASS
Graphify                               24,023 nodes / 54,655 edges
acquisition callers                                     0
execution authority                                 false
```

The broad library suite remains red in 26 existing `collection_synthesis`,
`online`, `online_collection`, `operator_live_shadow`, and legacy role-grounding
tests. No failing test references the B1B0R module; its complete 28-test route is
green. Those unrelated failures are not repaired or represented as PASS here.

Read-only live verification after the change:

```text
composite verdict                         PASS
eligible_for_local_accept                 false
response ACTIVE packages                  0
response false accepts                    0
response runtime parity failures          0
M3                                        WATCH
nando-response-learning InvocationID      8e59505eb1b943778601c9b3bacbd607
nando-transition-serving InvocationID     74ac3080f80b4fe387de2a94380e3657
```

Both invocation IDs are unchanged from STOP-B1B0. No deployment, restart,
`daemon-reload`, registry mutation, or service mutation occurred.

## STOP-B1B0R

```text
scientific contract                  SEALED / v1.r1
trusted event-time boundary          IMPLEMENTED
independent lineage denominator      IMPLEMENTED
H0 / H1                              UNPROVEN
binding relation selected            NO
acquisition                          NOT RUN
selector / ProtocolMode              NOT CREATED
F4                                   BLOCKED
runtime / services / authority       UNCHANGED
```

Work stops here. The next permissible stage is bounded B1B acquisition through
an independent capture owner. It must stop again after producing either trusted
causal evidence or `INSUFFICIENT_BINDING_EVIDENCE`; it may not enter F4 in the
same change.
