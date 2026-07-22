# STOP-R8H: Response Miner Application Split

Status: `PASS / MOVE_ONLY / AUTHORITY_FALSE`

Date: 2026-07-21

## Boundary

```text
response_miner/app.rs                 stable binary entrypoint
response_miner/app/orchestration.rs   one bounded miner cycle
response_miner/app/collection.rs      cold collection compiler
response_miner/app/support.rs         support and evidence accounting
response_miner/app/parity.rs          independent runtime parity
response_miner/app/io.rs              proof loading and atomic writes
```

The binary name, CLI arguments, report schemas, proof checks, and authority
owner are unchanged.

## File Budget

```text
before app.rs                    3727
largest file after split         1706
hard production violations          0
```

## Proof

```text
AST functions and methods                56/56
nando-response-actor frozen fingerprint  PASS
compile                                  PASS
new remote background builds                0
execution authority                     false
deploy/restart                          not run
```

Machine receipt: `R8H_MINER_APP_SPLIT_STOP.json`.
