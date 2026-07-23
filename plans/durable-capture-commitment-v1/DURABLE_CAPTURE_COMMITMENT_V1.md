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

`live_scalar_generation_version=1` preserves Wave and self-training state but
starts the live scalar generation with empty support and future. It is persisted
separately from the general bucket strategy so historical strategy migrations
cannot skip or repeat this rotation. This is required because pre-archive
receipts cannot be retroactively placed under archive provenance. New traffic
must build both partitions after the durable boundary.

## Status

```text
archive unit tests                 PASS
streaming ledger restart tests    PASS
fresh generation migration test  PASS
Clippy -D warnings                PASS
production deployment            NOT RUN
fresh support/future              NOT EVALUATED
authority                         false
```
