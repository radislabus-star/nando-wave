# S1C Transactional Deployment Paper Verification V4 2026-08-12

Status: `PASS / ONE S1C-3 V4 ATTEMPT AUTHORIZED / PRODUCTION UNCHANGED`

## Verdict

V3 is terminal and is not retried. V4 corrects only the resource-test
denominator so that it matches the two synchronous stages in production.

```text
V3 ownership route                       PASS
V3 quiescence                            PASS
V3 contamination                         false
V3 aggregate durability p99              5,767,585 ns FAIL
V4 precommit p99 bound                   5,000,000 ns unchanged
V4 settlement p99 bound                  5,000,000 ns unchanged
V4 aggregate hard max                    20,000,000 ns unchanged
V4 candidate                             03e3dd00c90206e2f705371318c50dd50537d6d8
V4 candidate tree                        06a9df51797dffc127fec41672bddae29c38bb92
V4 attempts                              exactly one
```

## Candidate Identity

The only crates diff against candidate base `a3ea27a` is inside the
`#[cfg(test)]` module of `grounded_decision_capture.rs`.

```text
changed runtime files                    0
production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
base projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
release binary size, base and V4          62,751,656 bytes
.text size, base and V4                   0x21b4132
.rodata size, base and V4                 0x579b40
.eh_frame size, base and V4               0x2dec28
.data.rel.ro size, base and V4             0x07a6b8
.data size, base and V4                    0x003330
```

Whole-file hashes differ because Rust binds crate source fingerprints into ELF
metadata. The final V4 binary hash is therefore frozen after the fresh
transaction build and bound by all downstream receipts, rather than being used
as a false test-only source equivalence claim.

## Focused Verification

```text
grounded decision tests                  6 PASS, 2 ignored resource gates
local stage-correct release gate         PASS
precommit p99                            107,565 ns
settlement p99                           313,372 ns
aggregate hard max                       430,084 ns
strict Clippy                            PASS
NANDA structural gate                    PASS
authority_ready                          false
repair queue                             0
safe_to_edit                             true
```

The local timing is sanity evidence only. Resource authority belongs to the
fresh remote V4 transaction.

## Production Baseline

```text
transition-serving          PID 165670   restarts 0   active
response-learning           PID 369456   restarts 0   active
gateway-control            PID 1035203  restarts 0   active
certification authority     PID 164668   restarts 0   active
transport / Nginx           PID 682430   restarts 0   active
local connector             PID 2919     restarts 0   active
route receipt failures      0
grounded-decision journal   ABSENT
```

```text
installed binary SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58

installed config SHA-256
  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5
```

## Paper Identity

```text
V3 terminal report SHA-256
  249b82497967f68fbd9ca6fd84e7b2205dc69d9c8652cdf30e89ed55859b60d8

V4 preregistration SHA-256
  cc388638f989048aa8b9ced30f4304d2af19bcd0919419bd1ddb6829c406de17

V4 critique SHA-256
  6cbf9a41284bc0d5329b2363bd843374bbb126d02650a8732ef2abaadc07aeea

V4 structural result SHA-256
  6720cc5a97af413f58d22f7153597cd0067bbb5dd65670e640e04e8ecd3e0ce1

candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

Exactly one V4 transaction is authorized after the paper packet is committed,
manifested, and its implementation/verifier fault-injection gates pass. No
S1C-4 or K2 claim is permitted before verified deployment PASS.
