# S1C Transactional Deployment Preregistration V7

Status: `DESIGN REVIEW / PROCESS OBSERVATION REPAIR ONLY`

Date: `2026-08-12 Europe/Tallinn`

## 1. Exact Blocker

The single V6 transaction proved its offline oracle closure and then ended in
an independently verified quiescence timeout before any resource measurement
or production mutation. Its process detector classified missing
`/proc/<pid>/exe` for stable kernel threads as a process race in every one of
`1,787` intervals. A valid quiescence window was therefore impossible.

V7 repairs only process-snapshot classification. It does not change the Rust
candidate, runtime config, workloads, affinity, CPU/IO thresholds, latency
budgets, quiescence duration, deployment chronology, or scientific claims.

## 2. Immutable Candidate And Proof Inputs

```text
candidate commit
  03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree
  06a9df51797dffc127fec41672bddae29c38bb92
production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
candidate Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1
candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
oracle source SHA-256
  bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26
oracle Cargo.lock SHA-256
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
```

The complete V6 offline build contract remains mandatory:
`s1c3-parity-oracle 0.1.0`, common source and lock, fresh disjoint workspaces and
targets, `--offline --locked`, `CARGO_NET_OFFLINE=true`, and lock identity before
and after each build.

## 3. Frozen Process Observation Contract

Each PID observation is bound to `/proc/<pid>/stat` field `22` (`starttime`) so
PID reuse cannot merge two processes. The detector parses state and comm from
the same stat row and then classifies exactly one status:

```text
OBSERVABLE_USER_PROCESS
  stat starttime stable before/after exe read
  /proc/<pid>/exe resolved
  comm or executable basename compared with forbidden build names

PROVEN_NON_EXECUTING
  one of:
    PID vanished before the closing stat
    stable state Z zombie
    stable Kthread: 1 with empty cmdline and missing exe

UNRESOLVED_PROCESS_OBSERVATION
  every other case, including:
    permission denial
    malformed stat/status/cmdline
    starttime changed or PID was reused
    stable non-zombie user process with missing exe
    kernel-thread attributes not jointly proven
```

Only `UNRESOLVED_PROCESS_OBSERVATION` is a global quiescence blocker. A proven
vanished process, zombie, or kernel thread cannot execute `cargo`, `rustc`, a C
compiler, or a linker and therefore is counted but not blocked.

No PID is accepted from `comm` alone. A resolved user executable still compares
both `comm` and executable basename against the complete frozen forbidden-name
set. A build-process match remains a global blocker exactly as in V6.

The receipt records per endpoint:

```text
observable process count
proven vanished count
proven zombie count
proven kernel-thread count
unresolved rows with PID, reason, and available identities
matching forbidden processes
snapshot root over all classifications
```

The independent verifier recomputes each interval blocker from these receipt
rows. A summary count without the classified rows cannot authorize PASS.

## 4. Fault-Injection Requirements

Before a V7 transaction, tests must reject:

```text
kernel thread classified from ENOENT alone
nonempty cmdline classified as kernel thread
Kthread: 0 classified as kernel thread
zombie accepted without stable starttime
PID reuse accepted as one process
permission denial treated as vanished
malformed stat/status treated as non-executing
forbidden comm hidden by an innocent executable basename
forbidden executable hidden by an innocent comm
receipt summary not derivable from classified rows
old V6 process_observation_race schema
```

A deterministic synthetic `/proc` fixture validates classification logic only.
It does not provide quiescence or deployment evidence.

## 5. Inherited Frozen Bounds

```text
measurement representatives            [4,6]
physical siblings                       4:[4,5], 6:[6,7]
selection                               first simultaneous valid window
required intervals                      30
maximum wait                            1,800 seconds
per-logical-CPU busy                    <= 20%
per-logical-CPU window mean             <= 5%
IO some avg10                           <= 0.20
IO full avg10                           <= 0.05
single-ledger p99                       <= 5 ms, PASS 3/3
precommit p99                           <= 5 ms, PASS 3/3
settlement p99                          <= 5 ms, PASS 3/3
each durability hard max                <= 20 ms
aggregate episode hard max              <= 20 ms
false accepts                           0
runtime parity failures                 0
```

Small latency deviations receive the frozen PASS/VETO result. They do not cause
threshold adaptation or an approval request.

No unrelated mini-PC workload is stopped. Production affinity is unchanged.
If ordinary load prevents a valid window, V7 terminates with the complete
census and is not retried.

## 6. Attempt And Claim Boundary

V6 is terminal. After critique, structural verification, final paper freeze,
and implementation/fault-injection PASS, V7 authorizes exactly one transaction.

```text
S1C-3 operational capture installed     only after verified DEPLOYMENT_PASS
natural decision episode                not proved by deployment
grounded meaning                        not proved by deployment
S1C-4                                   only after deployment PASS
K2                                      blocked
model training                          false
phase mutation                          false
dashboard scientific claim              forbidden without ordinary evidence
```
