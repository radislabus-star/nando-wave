# S1C-3B Production-Load Absolute Gate Preregistration V1

Status: `DESIGN FROZEN / STRUCTURAL PASS / EXECUTION FORBIDDEN UNTIL PAPER VERIFICATION PASS`

Date: `2026-08-12 Europe/Tallinn`

Parent authority:

- `ARCHITECTURE_CANON.md`, K2 grounded-meaning boundary;
- `S1C_PRE_ACTION_DECISION_OWNER_PREREGISTRATION_V1.md`;
- `S1C_SHADOW_PRODUCER_PREREGISTRATION_V1.md`;
- `S1C_TRANSACTIONAL_DEPLOYMENT_V7_ATTEMPT_2026-08-12.md`.

## 1. Exact Blocker And Route

S1C-2 already supplies the prepared capture evaluator, durable three-ledger
journal, restart recovery, runtime join, and parity tests. V7 did not evaluate
their resource bounds. It waited for 30 consecutive host-wide quiet intervals
and terminated after 1,800 seconds before the first latency metric.

The failed condition was an environment-selection prerequisite, not a capture,
parity, durability, or latency result:

```text
V7 candidate and offline oracles      PASS
V7 process observation               PASS
30-interval quiescence               NOT OBSERVED
resource measurements                NOT STARTED
production mutation                  no
```

S1C-3B asks one narrower operational question:

> Does the frozen S1C-2 candidate satisfy every absolute resource and parity
> bound under the mini-PC's ordinary background load, and can it then be
> installed with exact rollback while preserving the product data plane?

The route is:

```text
frozen paper and source roots
-> build every executable before measurement
-> fixed CPU 4, no quiet-window search
-> paired filesystem-floor observations around each candidate round
-> unchanged absolute candidate gates
-> independent predeployment verification
-> one rollback-armed transition-serving restart
-> S1C3B_DEPLOYMENT_PASS | S1C3B_RESOURCE_VETO | S1C3B_ROLLBACK_PASS
```

S1C-3B is not V8. V7 remains terminal and is never retried. S1C-3B is a new
measurement protocol with its own paper root, implementation, schemas,
verifier, transaction ID, and single-attempt budget.

## 2. Immutable Candidate

The runtime candidate remains the last implementation whose production
projection and config were frozen before all S1C-3 attempts:

```text
candidate commit
  03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree
  06a9df51797dffc127fec41672bddae29c38bb92
candidate Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1
production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
parity oracle source SHA-256
  bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26
oracle Cargo.lock SHA-256
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
```

The candidate config differs from the installed transition role config only
by:

```text
NANDO_GROUNDED_DECISION_SHADOW_ENABLED=1
NANDO_GROUNDED_DECISION_JOURNAL=/var/lib/nando-wave/transition/
  grounded-meaning-v1/decision-contract-precommits-v1
```

No runtime Rust source, config byte, durability chronology, threshold, fixture,
or package is changed by S1C-3B. Test-only or proof-plane code may change only
before the paper is frozen. A rebuilt or edited runtime candidate creates a new
protocol and cannot inherit this authority.

## 3. Live Baseline

The read-only baseline at preregistration was:

```text
authoritative control deployment commit   b409fa339ee5f2d07ffbc18917f474188000a743
control deployment receipt root           d02a3d7ad31e73aa005467806357b172cacac73b6a4d96122518ca642cf15245
installed transition binary SHA-256        6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58
installed transition config SHA-256        cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5

ordinary CPU accepts                      13,104
ordinary CPU input tokens                 2,194,859,494
active response profiles                  2
active transition profiles                5
false accepts / parity failures           0 / 0
K1 laws / semantics / topologies          1 / 3, 1 / 3, 1 / 2
decision episodes                         0
grounded-decision journal                 ABSENT

transition / learning / control PID       165670 / 369456 / 4034438
certification / Nginx / connector PID     164668 / 682430 / 2919
all remote NRestarts                      0
transition state bytes                    46,137,208,135
deployment evidence bytes                 16,288,304,177
root filesystem free bytes                198,215,184,384
```

This snapshot is orientation, not stale authority. Immediately before
preparation and again immediately before installation, the executor must bind
the current deployment receipt, binary, config, unit, phase config, authority
config, service snapshot, health semantics, route probe, economics safety, and
connector counters. Drift produces `STALE_BEFORE_MUTATION` without stopping a
service.

## 4. Production-Load Measurement Contract

### 4.1 No Environment Search

S1C-3B has no quiescence wait, no CPU pool, no lowest-load selector, no quiet
hour, and no retry. Measurement CPU is frozen to logical CPU `4`; its sibling
is CPU `5`. Ordinary production work continues unmodified on every CPU.

No workload, service, timer, user process, compiler, or unrelated agent is
stopped, reniced, re-affined, paused, or signalled. Production affinity and CPU
frequency policy remain unchanged.

All transaction-owned compilation must finish before `measurement_started_at`.
After that timestamp, the executor may launch only the frozen test harnesses,
parity executables, filesystem-floor probe, isolated RSS candidates, and its
read-only monitor. Every executable identity is hashed before and after the
measurement sequence.

### 4.2 Frozen Order And Denominators

There are exactly three rounds and no warmup. Each round has this order:

```text
filesystem floor before
-> capture-disabled hot latency
-> single-ledger durability
-> three-ledger stage durability
-> filesystem floor after
```

After all three rounds:

```text
capture-disabled idle CPU
-> isolated RSS capture off
-> isolated RSS capture on
-> baseline parity oracle
-> candidate parity oracle
```

Candidate denominators remain exact:

```text
hot matched cases per run             4,096
hot no-goal cases per run             4,096
single-ledger records per run         1,024
three-ledger episodes per run           256
rounds                                    3
```

The filesystem-floor probe is diagnostic only. It performs `256` exclusive
create, 4 KiB write, `fdatasync`, close, and unlink operations in a fresh
directory before and after each round. It persists all `512` raw duration
samples per round, p50, p99, hard maximum, filesystem identity, and bytes.
Its values may explain environmental drift but cannot make a failing candidate
pass and cannot veto a candidate that satisfies every absolute bound.

Quantiles, maxima, and samples are never subtracted, normalized, corrected,
trimmed, winsorized, rescaled, or replaced. Baseline/candidate ratios are
diagnostic only.

### 4.3 Absolute Candidate Gates

Every candidate run must independently pass:

```text
hot matched p99                       <= 1,000,000 ns
hot no-goal p99                       <=   250,000 ns
hot hard max                          <= 2,000,000 ns

single-ledger sync p99                <= 5,000,000 ns
single-ledger sync hard max           <= 20,000,000 ns

precommit sync p99                    <= 5,000,000 ns
precommit sync hard max               <= 20,000,000 ns
settlement sync p99                   <= 5,000,000 ns
settlement sync hard max              <= 20,000,000 ns
aggregate episode hard max            <= 20,000,000 ns
aggregate episode p99                 diagnostic, retained

capture-disabled idle CPU             <= 0.25% of one core
capture-on minus capture-off RSS      <= 16 MiB
ordinary output parity                byte-identical, 16 / 16 rows
false accepts                         0
runtime parity failures               0
```

An observation such as `5,010,709 ns` receives the frozen `FAIL` result for
that run. It does not trigger a question, threshold change, fourth run, or
selected rerun.

### 4.4 Background Evidence

A monitor samples at interval `<= 0.5 s` and at every metric boundary:

```text
monotonic and wall time
per-CPU busy deltas for CPUs 4 and 5
/proc/pressure/cpu and /proc/pressure/io
load averages
memory available
block-device counters
complete typed process snapshot
active metric label
```

Ordinary background processes, including unrelated build processes, are
recorded but do not censor an absolute PASS. Extra load can create a false
negative, not a false absolute PASS. Monitor error, maximum sample gap over
`2 s`, executable drift, wrong CPU affinity, missing metric boundary, malformed
sample, or transaction-owned compiler after measurement start is an instrument
failure and terminal `S1C3B_RESOURCE_VETO`.

## 5. Preparation And Independent Verification

Only a complete resource PASS may create rollback artifacts or preparation.
The preparation binds:

```text
paper commit, tree, manifest root, critique and paper-verification roots
candidate source, tree, projection, lock, config and release binary
all test, probe and oracle executable identities
measurement receipt and raw-sample roots
monitor receipt root
parity receipt root
current production receipt and artifact roots
service, health, economics, route, connector and journal snapshots
rollback manifest root
```

The independent verifier receives the immutable remote directory after
measurement and recomputes all roots, field sets, denominators, percentiles,
maxima, ordering, affinity, monitor coverage, executable identities, parity,
resource bounds, and claim restrictions. Executor summaries are never
authority.

```text
verification verdict S1C3B_PREPARATION_PASS
authority true
exact preparation root binding
```

is required before the deployment subcommand can run. Structural coherence
alone has `authority_ready=false` and cannot authorize installation.

## 6. One Rollback-Armed Mutation

The only mutable production owner is:

```text
unit       nando-transition-serving.service
binary     /opt/nando-wave/bin/nando-transition-serving
config     /etc/nando-wave/roles/transition-serving.env
journal    /var/lib/nando-wave/transition/grounded-meaning-v1/
             decision-contract-precommits-v1/
```

The exact transaction is:

```text
revalidate current baseline
-> arm immutable rollback
-> stop only transition-serving
-> prove old PID exited
-> atomically install candidate binary and exact config
-> start only transition-serving
-> verify new PID and zero NRestarts delta
-> verify capture env and journal open
-> verify semantic health, route and package parity
-> verify false accepts / runtime parity remain 0 / 0
-> verify RSS delta <= 16 MiB
-> survive 15 seconds
-> finalize receipt
```

Learning, gateway-control, certification authority, Nginx, and local connector
must preserve PID, restart count, identity, and health. One intentional
transition-serving PID change is required and attributed in the receipt.

Any post-stop error triggers exact rollback of the binary and config, restart
of the prior transition-serving runtime, journal cleanup only if it was created
by this failed attempt and contains no ordinary records, independent rollback
verification, and `S1C3B_ROLLBACK_PASS`. A rollback failure is `S1C3B_VETO`.

## 7. Terminal Outcomes And Attempt Budget

Exactly one S1C-3B remote transaction is allowed after paper freeze and
implementation verification:

```text
S1C3B_DEPLOYMENT_PASS
  resource, parity, verification, installation and survival passed

S1C3B_RESOURCE_VETO
  predeployment evidence failed; production mutation did not occur

S1C3B_ROLLBACK_PASS
  installation gate failed after stop; old runtime was restored exactly

S1C3B_VETO
  receipt, verifier, rollback, identity or safety invariant failed
```

There is no environment timeout and no automatic S1C-3C. A terminal veto is a
real answer under the frozen ordinary-load protocol.

## 8. Scientific And Product Boundary

Deployment PASS proves only that the prepared capture is installed, available,
durable, parity-preserving, bounded under observed production load, and
rollback-covered.

```text
natural decision episode              not proved by deployment
grounded meaning                      not proved by deployment
K2 law                                not proved by deployment
model training                        false
phase mutation                        false
package admission or activation       unchanged
K1 product execution                  unchanged
```

After `S1C3B_DEPLOYMENT_PASS`, S1C-4 may start as a separate read-only ordinary
census. Its first truthful dashboard state is `CAPTURE INSTALLED / ORDINARY
DECISION EVIDENCE COLLECTING`, never `GROUNDED MEANING PASS`.

## 9. Required Pre-Freeze Work

Before this paper can become executable:

1. publish a separate adversarial critique;
2. repair every P0/P1 finding in this contract;
3. run the NANDA structural claim-boundary gate and retain authority false;
4. freeze exact paper and evidence roots in a paper-verification report;
5. implement new S1C-3B executor, independent verifier, tests and runner;
6. pass fault injection, focused Rust, strict Clippy, fmt and installer tests;
7. commit and push every implementation byte before the sole transaction.

Until all seven steps pass, production execution is forbidden.
