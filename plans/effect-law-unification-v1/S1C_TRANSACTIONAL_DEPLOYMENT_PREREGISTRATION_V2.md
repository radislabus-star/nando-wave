# S1C Transactional Deployment Preregistration V2

Status: `PAPER FROZEN / EXECUTION FORBIDDEN UNTIL V2 PAPER VERIFICATION PASS`

Date: `2026-08-12 Europe/Tallinn`

Parent authority:

- `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md`
- `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md`
- `S1C_TRANSACTIONAL_DEPLOYMENT_ATTEMPT_2026-08-12.md`

## 1. Exact Question And Delta From V1

S1C-3 V2 asks the same deployment question as V1. It keeps the exact source,
config, baseline, resource ceilings, deployment owner, rollback route, and
claim boundary. It changes only the validity conditions of the predeployment
measurement environment:

```text
build candidate and all measurement executables
-> prove that no compiler or build process remains
-> observe one preregistered 30-second quiescence window
-> freeze QuiescenceReceiptV2
-> start a continuous contamination monitor
-> execute already-built binaries directly
-> same absolute 3 + 3 + 3 resource gates
-> parity, idle, and RSS gates
-> prepare or terminate before production mutation
```

V1 observed `5,010,709 ns` against an unchanged `5,000,000 ns` p99 limit
while its measured route still invoked Cargo and another build was active on
the host. V2 does not round, reinterpret, delete, or retry that result. V1 is
terminal. V2 removes compilation from the measured route and makes one new,
independently watermarked observation under the same limit.

## 2. Frozen Identity

The candidate remains exactly:

```text
source commit
  a3ea27a49af397ef79e5c9ec80089ecf53a41d59

source tree
  670d9c4ed170a76f107db13262abcd7cc035578e

Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1

candidate role-config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

The production baseline remains the exact V1 baseline unless atomic
revalidation reports drift:

```text
source commit
  663959064a37caf7eb917fc99dfedb6386355fa6

source tree
  05460ccbc9c44ac8b7174318903c0211de709e2e

deployment receipt root
  785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b

installed transition-serving SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58

installed role config SHA-256
  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5
```

Any source, tree, lockfile, candidate config, deployed baseline, unit,
phase-config, authority-config, or authoritative-receipt drift is terminal
`STALE_BEFORE_DEPLOYMENT`. No production mutation follows.

## 3. Frozen Executable Set

All compilation must finish before quiescence eligibility can begin. One clean
detached candidate checkout produces:

```text
1. release nando-transition-serving candidate binary
2. release nando-response-actor lib test harness
3. release nando-transition-serving lib test harness
4. release baseline parity oracle
5. release candidate parity oracle
```

The two harness paths are selected from Cargo JSON compiler-artifact records,
not by newest-file or wildcard order. Each record must identify the expected
package, `lib` test target, release profile, executable path, and exact source
checkout. The two oracle paths are deterministic release outputs of their
separate manifests.

Before eligibility, the executor binds every executable with:

```text
absolute path
SHA-256
size
mode
source commit or baseline commit
Cargo target identity
build command exit status
```

After the quiescence receipt is frozen, the executor may invoke only these
already-built measurement executables plus `taskset`. It may not invoke Cargo,
rustc, a linker, a build system, or a source generator.

## 4. Builder Detection

Process detection reads both `/proc/<pid>/comm` and the basename of
`/proc/<pid>/exe`. It never searches command-line text. This avoids treating a
shell waiter that merely contains words such as `cargo build` as a compiler.

The forbidden build executable set is frozen as:

```text
cargo rustc sccache
cc cc1 cc1plus gcc g++ clang clang++
ld ld.lld lld mold
ninja make cmake meson
```

A process matches when either normalized `comm` or executable basename equals
one of these names. Unreadable or vanished `/proc` entries are recorded as
races and rescanned; they are not silently converted into a match. The
executor's own completed build processes receive no exemption because none may
remain when eligibility starts.

## 5. Preregistered Quiescence Gate

The gate waits at most `1,800 seconds`. Eligibility requires 30 consecutive
one-second intervals. A failed interval resets the consecutive count; it does
not consume a candidate measurement because no candidate metric has started.

Every eligible interval must satisfy all of these conditions:

```text
foreign build processes at interval start     0
foreign build processes at interval end       0
CPU 4 non-idle fraction per interval           <= 20.00%
CPU 4 mean non-idle fraction over 30 intervals <=  5.00%
/proc/pressure/io some avg10                   <=  0.20
/proc/pressure/io full avg10                   <=  0.05
sample interval                                0.90 .. 1.50 seconds
```

`loadavg`, all I/O PSI fields, CPU 4 counters, process-race counts, and the
complete forbidden-process matches are retained for every attempted sample.
`loadavg` is observational and is not a hidden eligibility threshold.

Eligibility timeout yields `INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT`. It is a
terminal V2 result with no preparation receipt, no `systemctl stop`, and no
production mutation.

## 6. Immutable Quiescence Receipt

Before any candidate metric begins, the executor atomically writes, fsyncs,
renames, directory-fsyncs, and changes to mode `0400`:

```text
nando.s1c3-quiescence-receipt.v2
├─ transaction_id
├─ candidate source and tree
├─ executable identities
├─ detector schema and forbidden names
├─ maximum wait and exact thresholds
├─ eligibility_started_at
├─ eligibility_reached_at
├─ all attempted samples
├─ exact final 30-sample eligible window
├─ eligible_window_root_sha256
└─ quiescence_root_sha256
```

The receipt is immutable. A changed executable, missing sample, nonconsecutive
window, process match, CPU violation, PSI violation, root mismatch, or mode
mismatch is VETO before measurement.

## 7. Measurement Contamination Monitor

Immediately after the immutable quiescence receipt is frozen, one monitor runs
continuously until the last direct metric and both direct parity oracles have
finished. It samples at least once every two seconds and at every executable
boundary.

The monitor records forbidden process matches, process-race counts, CPU 4,
I/O PSI, loadavg, current metric label, and monotonic timestamps. I/O PSI and
CPU values during durability tests are evidence, not a second threshold,
because the candidate itself performs the measured writes. The unchanged p99
and hard-max ceilings remain the outcome threshold.

Any forbidden build process observed after the first metric begins yields
`INVALID_ENVIRONMENT_MEASUREMENT_CONTAMINATED`. No metric is deleted and no
retry is allowed. Missing monitor coverage, a sample gap over two seconds, or
monitor failure has the same terminal result.

The executor freezes `nando.s1c3-measurement-contamination-receipt.v2` and the
resource receipt binds its root. The contamination receipt must prove zero
forbidden process matches.

## 8. Direct Measurement Route

Each harness is executed directly:

```text
taskset -c 4 <response-actor-test-binary> <exact-test-name>
taskset -c 4 <transition-serving-test-binary> <exact-test-name>
```

The fixed test names, `--ignored`, `--exact`, `--nocapture`,
`--test-threads=1`, and `RUST_TEST_THREADS=1` remain unchanged. Each stdout
record is retained and parsed once. The order is fixed:

```text
hot compatibility              3 runs
single-ledger durability       3 runs
three-ledger durability        3 runs
isolated idle CPU              1 run
isolated RSS off/on            1 paired run
baseline parity oracle         1 run
candidate parity oracle        1 run
```

The absolute limits remain exactly:

```text
hot matched p99                 <=  1,000,000 ns, PASS 3/3
hot no-goal p99                 <=    250,000 ns, PASS 3/3
hot hard max                    <=  2,000,000 ns, PASS 3/3
single-ledger p99               <=  5,000,000 ns, PASS 3/3
single-ledger hard max          <= 20,000,000 ns, PASS 3/3
three-ledger p99                <=  5,000,000 ns, PASS 3/3
three-ledger hard max           <= 20,000,000 ns, PASS 3/3
idle CPU                        <= 0.25% of one core
isolated RSS delta              <= 16 MiB
ordinary parity                byte-identical
false accepts                  0
runtime parity failures        0
```

The frozen record counts remain 4,096 matched, 4,096 no-goal, 1,024
single-ledger, and 256 three-ledger samples per run. There is no warmup
replacement, outlier deletion, rounding, relative escape, threshold repair,
or rerun after the first metric begins.

## 9. Resource Receipt V2

`nando.s1c3-resource-receipt.v2` binds:

```text
quiescence_root_sha256
measurement_contamination_root_sha256
all five executable hashes
direct_exec_only = true
compiler_invocations_after_quiescence = 0
the exact 3 + 3 + 3 measurements
idle and RSS evidence
the unchanged frozen bounds
all_pass = true
```

The verifier independently recomputes both roots, executable identities,
sample continuity, eligibility math, direct execution declaration, run counts,
denominators, and every absolute threshold.

## 10. Transaction And Rollback

After resource and parity PASS, the V1 transaction chronology remains exact:

```text
revalidate baseline
-> bind rollback bytes and preparation receipt
-> arm rollback
-> stop only nando-transition-serving.service
-> prove old PID exited
-> fsync and install candidate binary/config pair
-> start only nando-transition-serving.service
-> health, parity, journal, economics, and 15-second survival
-> S1C3_DEPLOYMENT_PASS | S1C3_ROLLBACK_PASS | S1C3_VETO
```

Only transition-serving may change PID. `NRestarts` stays unchanged. Transport,
learning, gateway-control, certification authority, Nginx, and the local
connector must preserve identity and route-receipt failures. Rollback restores
the exact old binary/config pair and preserves every forward journal prefix.

V2 allows exactly one remote attempt. A quiescence timeout, contamination,
resource failure, parity failure, stale baseline, deployment failure, or
verified rollback spends that attempt.

## 11. Claim Boundary

S1C-3 V2 may prove only installation and operational capture availability.
It cannot prove a natural decision episode or meaning:

```text
capture authority                   false
model training                      false
phase mutation                      false
serving and admission authority     unchanged
K1                                  unchanged
K2                                  blocked
S1C-4 natural census                not started
dashboard scientific claim          forbidden
```

An empty grounded-decision journal remains a valid deployment result. No
synthetic or targeted ordinary traffic may be introduced to make it nonempty.

## 12. Terminal Decision Tree

```text
paper or identity failure
  -> VETO, no remote attempt

quiescence timeout
  -> INVALID_ENVIRONMENT, no production mutation, V2 spent

measurement contamination or any absolute gate failure
  -> terminal preflight failure, no production mutation, V2 spent

preparation PASS then transaction PASS
  -> S1C3_DEPLOYMENT_PASS

post-stop failure with exact recovery
  -> S1C3_ROLLBACK_PASS

recovery or receipt failure
  -> S1C3_VETO
```

No branch authorizes a second V2 attempt.

