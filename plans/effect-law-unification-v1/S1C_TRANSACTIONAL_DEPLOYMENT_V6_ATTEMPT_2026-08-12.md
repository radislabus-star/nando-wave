# S1C Transactional Deployment V6 Attempt 2026-08-12

Status: `INVALID ENVIRONMENT / QUIESCENCE TIMEOUT / NO PRODUCTION MUTATION / V6 TERMINAL`

## Frozen Attempt

```text
paper commit             8c30a2cf2b122ab1a46b08e748c7d0ee6a08820b
implementation commit    b2ea24ccb3428368ba92b06b892b12c7d177e02c
candidate commit         03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree           06a9df51797dffc127fec41672bddae29c38bb92
transaction id           20260812T034827Z-8c30a2cf2b12-s1c3v6
remote attempts          exactly one
```

## Exact Result

V6 removed the accidental online Cargo route and built both fresh parity
oracles with the same frozen source, package, and lock. The independent V6
verifier accepted the terminal timeout receipt but did not grant deployment
authority:

```text
offline locked baseline oracle       PASS
offline locked candidate oracle      PASS
lock before/after both builds         byte-identical
oracle ownership                      PASS
quiescence                            TIMEOUT
independent verification              VALID / AUTHORITY FALSE
resource and latency measurements     NOT STARTED
parity execution                      NOT STARTED
predeployment authority               NOT ISSUED
systemctl stop                        NOT REACHED
production mutation                   no
```

This is not a latency result. The frozen `5 ms` p99 and `20 ms` hard-max gates
were never executed.

## Durable Quiescence Census

```text
deadline seconds                      1,800
attempted intervals                   1,787
required consecutive intervals       30
selected CPU                          null
longest eligible streak CPU 4         0
longest eligible streak CPU 6         0

process_observation_race              1,787
forbidden_build_process                 628
CPU 4 over per-interval bound           579
CPU 5 over per-interval bound           674
CPU 6 over per-interval bound           802
CPU 7 over per-interval bound           473
IO some over bound                       96
IO full over bound                      356
interval duration failures                0
```

The process detector made every interval globally ineligible. Each interval
reported between `526` and `563` process read errors, mean `544.59` across the
two endpoint snapshots.

## Exact Detector Defect

V6 read `/proc/<pid>/comm` and then `/proc/<pid>/exe`. It classified every
`FileNotFoundError`, `ProcessLookupError`, and other `OSError` as an unresolved
process race. Linux exposes no `/proc/<pid>/exe` target for kernel threads, so a
stable ordinary host has a permanent `ENOENT` population.

A post-terminal read-only diagnostic observed:

```text
observable user processes               67
ENOENT process rows                     272
other error classes                       0
stable no-exe rows sampled              kernel threads with empty cmdline
```

This diagnostic explains the code path; it is not substituted into the frozen
receipt. Receipt authority remains the durable fact that all `1,787` intervals
were rejected by `process_observation_race`.

The fix cannot be "ignore ENOENT". A process may exit or reuse a PID between
reads, and a user process whose main thread terminated may lack an `exe` link.
The next detector must prove a stable kernel thread, stable zombie, or vanished
PID before treating missing `exe` as non-executing. Every ambiguous case remains
fail-closed.

## Evidence

Remote directory:

```text
/var/lib/nando-wave/deployments/
  20260812T034827Z-8c30a2cf2b12-s1c3v6/
```

Local mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260812T034827Z-8c30a2cf2b12-s1c3v6/
```

```text
local evidence files                    21
local evidence listing root
  e767a99f770abcd5398d5f2ba615ee092ac285922225eb5e93d3070c1f6441af

oracle ownership embedded root
  9f4471e4c321815695fd5c6b72872d826a39e4542b8ffedc4677758d9de0d7cf
quiescence embedded root
  ffca5a5eb1b0e2644e530a3caeef59b7402ddd38b5abb9cf8e523eb26d350405
local verifier verdict
  INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT
```

Both authoritative remote receipts are root-owned mode `0400`.

## Production Preservation

```text
transition-serving          PID 165670   restarts 0   active
response-learning           PID 369456   restarts 0   active
gateway-control            PID 1035203  restarts 0   active
certification authority     PID 164668   restarts 0   active
transport gateway           PID 682430   restarts 0   active
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

V6 is terminal and is never retried. It neither installs S1C-3 nor authorizes
S1C-4, grounded meaning, K2, training, phase mutation, or a dashboard claim.
