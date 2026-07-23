# Durable Capture Commitment V1

## Goal

Prevent verified operator evidence from becoming unverifiable when the bounded
rolling capture index evicts old records.

## Route

```text
streaming evidence ledger
-> append-only sequence + SHA-256 archive
-> atomic archive checkpoint
-> bounded rolling index
-> support/future capture receipts
-> crystallized candidate
-> external admission provenance
```

## Ownership

- `nando-operator-learning` owns the archive format and verification.
- `nando-transition-serving` appends and seals capture commitments.
- `nando-response-admission` reads commitments but cannot manufacture them.
- The rolling index remains a cache and carries no exclusive authority.

## Recovery Contract

- An append is durable only after the archive checkpoint is atomically sealed.
- An unsealed complete or partial tail is discarded on writer restart.
- Journal replay of an already sealed sequence is accepted only when its digest
  is byte-identical.
- Gaps, altered digests, truncated committed data, and invalid roots fail closed.
- Admission falls back to the archive only when a receipt is absent from the
  rolling index, never when the receipt or indexed digest is invalid.

## Fresh Generation

`live_scalar_generation_version=2` preserves Wave and self-training state but
starts the live scalar generation with empty support and future. It is persisted
separately from the general bucket strategy so historical strategy migrations
cannot skip or repeat this rotation. This is required because pre-archive
receipts cannot be retroactively placed under archive provenance. New traffic
must build both partitions after the durable boundary.

Version 2 also fixes the support-diversity deadlock found by fresh traffic. A
32-row support reservoir collected from fewer than three sessions now replaces
a repeated support row when a missing session first appears. It does not relabel
future evidence, and future collection starts only after support diversity is
valid.

## Status

```text
archive unit tests                 PASS
streaming ledger restart tests    PASS
fresh generation migration test  PASS
Clippy -D warnings                PASS
production deployment            PASS
fresh support/future              0/0 at corrective post-restart baseline
authority                         false
```

## Live Deployment Receipt

```text
commits                           297df6b + 104d121
serving binary SHA-256           75f242b0...39098
admission binary SHA-256         686f6d74...6dd11
durable archive                  632 KiB and growing
old scalar state                 219 support / 96 future
first incomplete rotation        25 support / 0 future
corrective fresh scalar state    0 support / 0 future
crystallized candidates          0
capture provenance blocker       cleared with fresh bundle
hot serving restart              none
hot serving InvocationID         2d1501b585a54be3bb315ca4fc42941e
cold learner restart count       0
execution authority              false
```

The first deployment exposed that a general bucket strategy version is not a
reliable owner for scalar-generation rotation. The corrective deployment added
the dedicated persisted generation version and the live state then rotated as
specified. This failed first live assertion was not treated as PASS.
