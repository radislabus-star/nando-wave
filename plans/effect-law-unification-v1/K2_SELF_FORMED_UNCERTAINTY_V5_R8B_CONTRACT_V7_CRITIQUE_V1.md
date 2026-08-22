# K2 Self-Formed Uncertainty V5 R8B Contract V7 Critique V1

Status: `CRITIQUE COMPLETE / V7 DRAFT VETO / REPAIR REQUIRED`

Date: `2026-08-21`

## Verdict

V7 correctly removes the impossible M24-only nested ledger and the recursive
M25/M26 denominator. It is not ready for structural gates. Two P0 and five P1
defects remain in the first draft.

## Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | The canonical receipt channel is assigned only to S01-S05, but the M24 child is also a libtest process. | Its actual stdout cannot equal the canonical linked-route receipt required by the current M25 binding. | Give M24 child the same closed canonical output channel and keep libtest stdout/stderr separate. |
| P0 | One finished event has one decoded receipt root, while S01 produces three receipts and M24 child produces at least the linked-route receipt plus the in-process Oracle batch. | Multiple packet entries cannot bind to their actual producer without fabricating events, repeating process runs or dropping evidence. | Replace the scalar receipt binding with a bounded, unique list of produced-receipt descriptors. Each descriptor binds relative path, bytes, hash, schema and semantic root. |
| P1 | The Oracle batch is constructed by M24 child from sixteen M16 case receipts, but the draft does not name it as an M24 child output. | M25 would look for a process stdout that never existed. | Publish the canonical Oracle batch in the M24 child output channel and bind its sixteen M16 source roots. |
| P1 | `FrozenControlScopes` is retained as a control-receipt evidence kind even though no M17 invocation emits a four-scope receipt. | A single scope receipt could be relabelled as observed `4`, or the same bytes could be duplicated under two denominators. | Make it an explicit derived measured receipt emitted by M24 child from the four distinct M17 receipts. State that it is a coverage census, not an independent additive result. |
| P1 | Shared-ledger writers receive only route ID and path. The draft does not bind each writer to an allowed child-role set. | A compromised or buggy nested owner could append a row claiming another manifest role. | Verify current executable SHA and enforce `M01 -> M02 only`, `M10 -> M03-M09 only`, and `M24 -> direct children only` at append time. |
| P1 | Suite output custody does not define the closed path set, hard-link count or whether the producer may emit extra files. | A valid receipt could coexist with an unclassified side output or be substituted through a second link. | Freeze an exact per-invocation output manifest; require regular `0400`, `nlink=1`, canonical bytes, no symlinks and no extra paths. |
| P1 | The post-authorization audit is named but its authority is ambiguous. | It could be misreported as an input proof or as a second authorization decision. | Define it as append-only diagnostics only. The final authority artifact is exact M25 bytes published by M26 plus M26's concrete publication receipt; P09 cannot alter either disposition. |

## Scalar Binding Failure

The first V7 draft assumes:

```text
one process invocation -> one canonical receipt
```

The real routes include:

```text
S01 invocation
  -> Confirm canonical bytes receipt
  -> Development known-answer receipt
  -> publication-boundary receipt

M24 child invocation
  -> linked-route receipt
  -> Oracle batch
  -> four-scope coverage census
```

Repeating S01 three times would avoid its local multiplicity but would not solve
M24 child or Oracle provenance. The general process model therefore needs a
bounded output descriptor set rather than another special case.

## Required Model

Each finished event must retain actual process diagnostics and separately bind
zero or more canonical authority outputs:

```text
stdout length/hash
stderr length/hash
produced_receipts[]
  relative path
  byte length
  mode
  content SHA-256
  receipt schema
  semantic root
```

For one-shot protocol processes the list has one descriptor and its bytes equal
stdout. For S01-S05 and M24 child the list describes a closed immutable output
directory. M25 must match each non-parent packet entry to exactly one descriptor
from its named producer event.

## Next Legal Action

Repair the V7 contract in place, then run a second adversarial critique. No
implementation or R8B execution is authorized by this critique.
