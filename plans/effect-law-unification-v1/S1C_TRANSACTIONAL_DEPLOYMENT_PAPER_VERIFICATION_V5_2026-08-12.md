# S1C Transactional Deployment Paper Verification V5 2026-08-12

Status: `PASS / ONE S1C-3 V5 ATTEMPT AUTHORIZED / PRODUCTION UNCHANGED`

## Verdict

V4 is terminal and is not retried. V5 repairs only the pre-measurement
environment selector and the durability of its negative evidence.

```text
V4 fresh candidate builds                  PASS
V4 oracle ownership                       PASS
V4 fixed CPU 4 quiescence                 TIMEOUT after 1,800 seconds
V4 resource measurement                   not reached
V4 production mutation                    none
V5 candidate                              unchanged
V5 config                                 unchanged
V5 resource thresholds                    unchanged
V5 remote attempts                        exactly one
```

## Frozen Selection Contract

```text
representative CPU pool                    [4,6]
physical core 4 siblings                   [4,5]
physical core 6 siblings                   [6,7]
maximum frequency class                    5,400,000 kHz
observation                                simultaneous /proc/stat boundaries
passing window                             30 trailing intervals
interval duration                          0.90 .. 1.50 seconds
per-logical-CPU busy                       <= 20%
per-logical-CPU window mean                <= 5%
IO some avg10                              <= 0.20
IO full avg10                              <= 0.05
forbidden build process                    none
selection tie-break                        lowest representative CPU
```

Both logical siblings must pass. Selection occurs before any hot, durability,
idle, RSS, or parity metric. After selection, every resource metric runs on the
same selected CPU. Production affinity is immutable.

## Durable Negative Evidence

`quiescence-receipt.json` is mandatory on both `PASS` and `TIMEOUT`. The
independent verifier must recompute topology, interval rows, sliding windows,
blocker census, roots, selected CPU, and verdict. A timeout is terminal
non-authority and cannot reach resource measurement or deployment.

## Candidate Identity

```text
candidate commit
  03e3dd00c90206e2f705371318c50dd50537d6d8

candidate tree
  06a9df51797dffc127fec41672bddae29c38bb92

production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316

Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1

candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

## Resource Claim Boundary

The scientific and resource thresholds are unchanged. In particular:

```text
single-ledger p99                         <= 5,000,000 ns
precommit p99                             <= 5,000,000 ns
settlement p99                            <= 5,000,000 ns
each durability hard max                  <= 20,000,000 ns
aggregate episode hard max                <= 20,000,000 ns
```

A small observed deviation is evidence for the frozen pass/fail rule, not a
reason to ask for a new threshold or to adapt the attempt.

## Structural Verification

```text
NANDA self-check                          PASS
NANDA doctor                              healthy
structural verdict                        PASS
authority_ready                           false
weak details                              0
repair queue                              0
safe_to_edit                              true
```

The structural result proves route coherence only. Transaction authority
belongs to the V5 independent verifier and terminal receipts.

## Production Baseline

V4 terminal evidence records production unchanged:

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
V4 terminal report SHA-256
  81ab2b4e5f82b5168e39fac28387cb7a7087e3d4b6bdd4e001158eaf17aa5497

V5 preregistration SHA-256
  30dd4c6500e392c51379283bccb7deafe99b68a393b5e19ec7297b45c65cd5c6

V5 critique SHA-256
  a093dff8c5379260c1bf7f2f5b5a33b6c075689b76372a02c1e9a2204287f88b

V5 structural result SHA-256
  38ac0720cb10e86e6d8df89353eeefb0f97a15f3bc62c031155e11e309ea752b

candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

Exactly one V5 transaction is authorized after this packet is committed,
manifested, and the executor/verifier fault-injection gates pass. A successful
S1C-3 deployment installs operational capture only. It does not prove a
natural decision episode, grounded meaning, S1C-4, or K2.
