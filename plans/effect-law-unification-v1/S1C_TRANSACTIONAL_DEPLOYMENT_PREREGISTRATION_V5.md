# S1C Transactional Deployment Preregistration V5

Status: `DESIGN FROZEN / PAPER VERIFICATION PASS / ONE V5 ATTEMPT AUTHORIZED`

Date: `2026-08-12 Europe/Tallinn`

## 1. Exact Blocker

V4 reached a fresh candidate build and oracle-ownership PASS, then exhausted
the 1,800-second quiescence deadline before any resource metric or production
mutation:

```text
transaction                         20260812T011911Z-1def4272a466-s1c3v4
terminal error                      INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT
measurement CPU                    fixed CPU 4
attempted samples in receipt        absent
production mutation                 no
```

V4 exposed two proof-plane defects:

1. one hard-coded logical CPU could make environment eligibility depend on
   incidental host scheduling even when an equivalent core was quiet;
2. timeout discarded the attempted interval series, so the terminal reason
   could not be independently decomposed into build, CPU, IO, interval, or
   mean-window blockers.

V5 repairs only this pre-measurement environment gate.

## 2. Immutable Candidate Boundary

V5 reuses the exact V4 runtime candidate and config. There is no Rust candidate
change.

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

Runtime append order, three fsyncs, ledger format, failure censors, hot path,
service unit, phase memory, authority config, and production process affinity
remain immutable.

## 3. Frozen CPU Pool

The mini-PC sysfs topology exposes two equivalent 5.4 GHz P-core pairs:

```text
core_id 8 logical CPUs       4,5
core_id 12 logical CPUs      6,7
```

To avoid sibling interference, V5 freezes one representative thread from each
physical core:

```text
measurement_cpu_pool      [4,6]
selection_order           ascending logical CPU
selection_tie_break       lowest CPU number
physical-core rows        4 -> [4,5], 6 -> [6,7]
```

The pool is part of the paper identity. It cannot be enlarged or reordered at
runtime. The executor reads `/sys/devices/system/cpu/cpuN/topology/core_id` and
`cpufreq/cpuinfo_max_freq`; it must prove every representative and sibling is
online, that the two representatives have different physical `core_id` values,
that each declared pair shares one core, and that both cores have the same
maximum-frequency class.

## 4. Pre-Measurement Selection

All four logical CPUs in the two physical-core rows are observed simultaneously
from one pair of `/proc/stat` snapshots per interval. Global process and IO
observations are captured once per interval and bound to every row.

For each logical CPU independently, the unchanged gate is:

```text
continuous intervals                         30
interval duration                  0.90 .. 1.50 s
per-interval non-idle                       <= 20%
mean non-idle over the 30 intervals          <= 5%
IO some avg10                                <= 0.20
IO full avg10                                <= 0.05
forbidden build process at either boundary       none
process observation races                        none
```

No hot, durability, RSS, parity, or other result is available during CPU
selection. Each physical-core candidate keeps a trailing 30-interval window.
Both logical siblings must pass every per-interval condition and each sibling's
30-interval mean must be at most 5%. A failed per-interval condition clears
that core's window; a mean failure advances the trailing window without
discarding otherwise eligible intervals. The first timestamp where one or more
physical cores have a complete passing window freezes the lowest-numbered
representative CPU. Selection is therefore based only on preregistered
environment readiness, never measured candidate speed.

After freeze:

```text
hot latency tests              selected CPU only
single-ledger tests            selected CPU only
three-ledger tests             selected CPU only
idle CPU test                  selected CPU only
RSS capture off/on             selected CPU only
parity                         functional evidence, no latency authority
```

The executor process is excluded from all pool CPUs after the fresh builds and
before the first selection observation. No production process affinity is
changed.

## 5. Durable Timeout Census

V5 must write `quiescence-receipt.json` atomically with mode `0400` on both
PASS and timeout. The receipt contains:

```text
schema                         nando.s1c3-quiescence-receipt.v5
verdict                        PASS | TIMEOUT
measurement_cpu_pool           [4,6]
selected_cpu                   integer | null
topology rows                  representative, siblings, core_id, max frequency
attempted interval count
all attempted interval rows
per-core longest eligible streak
per-logical-CPU minimum completed-window mean
blocker census
attempted_samples_root_sha256
eligible_window_root_sha256    hash | null
executable_set binding
ownership receipt binding
receipt root
```

Each attempted interval row records both CPUs and classifies every failed
condition explicitly. The blocker census counts:

```text
forbidden_build_process
process_observation_race
interval_duration
cpu_busy_per_interval, per logical CPU
cpu_busy_window_mean, per logical CPU
io_some
io_full
```

The independent verifier recomputes every count, streak, mean, root, selection,
and timeout verdict from the retained series. A timeout receipt is terminal and
cannot authorize resource measurement or production mutation.

## 6. Unchanged Resource And Deployment Gates

V5 inherits V4 byte-for-byte except for schema version, selected-CPU binding,
and durable timeout evidence:

```text
hot matched p99                         <= 1 ms, PASS 3/3
hot no-goal p99                      <= 0.25 ms, PASS 3/3
hot hard max                            <= 2 ms, PASS 3/3
single-ledger p99                       <= 5 ms, PASS 3/3
single-ledger hard max                 <= 20 ms, PASS 3/3
precommit p99                           <= 5 ms, PASS 3/3
settlement p99                          <= 5 ms, PASS 3/3
each stage hard max                    <= 20 ms, PASS 3/3
aggregate episode hard max            <= 20 ms, PASS 3/3
idle CPU                           <= 0.25% of one core
RSS delta                             <= 16 MiB
parity rows                                  16 exact
false accepts                                  0
runtime parity failures                        0
```

Fresh checkout, target, harnesses, parity oracles, ownership receipt,
executable identities, contamination monitor, rollback chronology, service
survival, connector identity, and exact production hashes remain mandatory.

## 7. Attempt And Claim Boundary

After final paper verification, V5 authorizes exactly one transaction. V4 is
terminal and is never retried.

```text
S1C-3 operational capture installed     only after verified DEPLOYMENT_PASS
natural decision episode                not proved by V5
grounded meaning                        not proved by V5
S1C-4                                   blocked until deployment PASS
K2                                      blocked
model training                          false
phase mutation                          false
dashboard scientific claim              forbidden
```

V5 changes where a benchmark may run, not what passes the benchmark.
