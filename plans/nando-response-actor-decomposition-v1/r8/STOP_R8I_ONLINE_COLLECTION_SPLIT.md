# STOP-R8I: Online Collection Owner Split

Status: `PASS / MOVE_ONLY / AUTHORITY_FALSE`

Date: 2026-07-21

## Boundary

```text
online_collection.rs             public contracts and manifest digests
online_collection/ingest.rs      observation and replay ingestion
online_collection/admission.rs   candidate/freeze assembly and persistence
online_collection/migration.rs   checkpoint and evidence compaction
online_collection/subcenter.rs   subcenters and active witnesses
online_collection/authority.rs   independent receipts and authority proof
online_collection/status.rs      status, layouts, blockers, bounded IO
```

Sibling bridges are `pub(super)` only. They do not extend the crate API, and
the candidate module still cannot activate a package without external
admission.

## File Budget

```text
before online_collection.rs          6674
largest file after split             1538
hard production violations              0
```

## Proof

```text
AST functions and methods               134/134
nando-response-actor frozen fingerprint PASS
compile                                 PASS
new remote background builds               0
execution authority                    false
deploy/restart                         not run
```

Machine receipt: `R8I_ONLINE_COLLECTION_SPLIT_STOP.json`.

No threshold, Wave, verifier, checkpoint schema, or admission behavior changed.
