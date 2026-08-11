# S1C Transactional Deployment Preregistration V1

Status: `PAPER FROZEN / EXECUTION FORBIDDEN UNTIL PAPER VERIFICATION PASS`

Date: `2026-08-11 Europe/Tallinn`

Parent authority:

- `S1C_PRE_ACTION_DECISION_OWNER_PREREGISTRATION_V1.md`
- `S1C_SHADOW_PRODUCER_PREREGISTRATION_V1.md`
- `S1C_SHADOW_PRODUCER_SOURCE_VERIFICATION_2026-08-11.md`

## 1. Exact Question And Claim Boundary

S1C-3 asks one operational question:

> Can the exact S1C-2 source candidate be installed with capture enabled in
> the sole owning production runtime, survive one intentional restart, satisfy
> the frozen absolute resource and parity gates, and retain an exact rollback
> route without changing serving, learning, admission, certification, or
> scientific authority?

The route is:

```text
frozen source commit
-> one candidate release binary
-> fresh absolute pre-deployment gates
-> immutable preparation receipt and rollback copy
-> stop only transition-serving
-> install exact binary plus exact role config
-> start only transition-serving
-> health, parity, resource, journal, and 15-second survival gates
-> S1C3_DEPLOYMENT_PASS | S1C3_ROLLBACK_PASS | S1C3_VETO
```

S1C-3 may prove installation, rollback, restart attribution, capture
availability, serving parity, and bounded resource behavior. It cannot prove
that ordinary traffic contains an exact pre-action goal, a decision episode,
K2, a new K1 law, model learning, or phase causality.

```text
capture authority                   false
model training                      false
phase mutation                      false
serving authority                   unchanged
admission authority                 unchanged
certification authority             unchanged
K2 claim                            forbidden
dashboard scientific claim          forbidden
S1C-4 natural census                not started by this slice
```

## 2. Frozen Source And Parent Identity

The deployment candidate source is the already accepted S1C-2 commit, not the
later paper commit:

```text
candidate source commit
  a3ea27a49af397ef79e5c9ec80089ecf53a41d59

candidate source tree
  670d9c4ed170a76f107db13262abcd7cc035578e

candidate parent
  af776d44dbd1d1d212b3f1b67e74a440b09014c1

Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1

S1C-2 source verification SHA-256
  d75a4880cf8efe21bb524b898c1e4b5d2630c0bde700e4a886329f35d94ce660
```

The six source-file hashes remain those frozen in the S1C-2 source
verification. Any source, lockfile, generated Rust, feature, profile, or build
script change creates S1C-3 V2. Paper files are not candidate source.

The candidate must be built on `e@192.168.3.94` from a clean detached checkout
of the exact source commit. The preparation receipt binds:

```text
source commit and tree
Cargo.lock hash
rustc -Vv
cargo -V
target triple
release binary size and SHA-256
build command and exit code
tracked worktree cleanliness
```

The candidate binary hash is assigned once by the successful clean build and
written to the immutable preparation receipt before any production file or
service changes. A rebuild after observing any resource or deployment result
is forbidden under V1.

## 3. Frozen Production Baseline

The read-only paper snapshot found:

```text
deployed source commit
  663959064a37caf7eb917fc99dfedb6386355fa6

deployed source tree
  05460ccbc9c44ac8b7174318903c0211de709e2e

authoritative deployment receipt root
  785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b

installed transition-serving binary SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58

transition-serving systemd unit SHA-256
  6e9d2fe41b1db95f94768d1ab41dffce1f15be92e2f774832c7fe392bb77b135

current role config SHA-256
  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5

phase-center config SHA-256
  5c019cebbde083f963c03619ff1d938786f5b4ec58730dddd5b34adeb33cce31

authority config SHA-256
  d40b7262ff6d744a393b0fc03a5d06610d01728aa2f4603199ca8567189ec88f

grounded-decision journal
  ABSENT
```

These values are an orientation snapshot, not permission to ignore drift. The
preparation step must re-read all values. Any old binary, source receipt,
systemd unit, phase config, authority config, or role-config mismatch is
`STALE_BEFORE_DEPLOYMENT`; no production mutation occurs.

## 4. Exact Runtime Owner And Allowed Mutation

One process owns this slice:

```text
unit       nando-transition-serving.service
binary     /opt/nando-wave/bin/nando-transition-serving
config     /etc/nando-wave/roles/transition-serving.env
journal    /var/lib/nando-wave/transition/grounded-meaning-v1/
             decision-contract-precommits-v1/
```

The exact candidate config is stored at:

```text
plans/effect-law-unification-v1/evidence/
  S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1/
  transition-serving.env.candidate
```

Its SHA-256 is:

```text
1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

It differs from the current config by exactly:

```text
NANDO_GROUNDED_DECISION_SHADOW_ENABLED=1
NANDO_GROUNDED_DECISION_JOURNAL=/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1
```

No other environment value, unit byte, binary, registry, admission artifact,
certificate, package, ledger, dashboard file, timer, path unit, or socket may
change.

The following remain running with identical `MainPID`, `NRestarts`, unit hash,
and relevant config hashes across the transaction:

```text
nando-transport-gateway.service
nando-gateway-control.service
nando-response-learning.service
nando-operator-certification-authority.service
local nando-connector process
```

`nando-response-admission` path/timer activity and the live-transition path/
timer may continue normally, but S1C-3 may not start, stop, restart, enable,
disable, reload, or rewrite them.

## 5. Absolute Pre-Deployment Gates

All gates use the one frozen candidate binary and exact candidate config. They
run before production mutation and preserve every raw metrics line and exit
code. Synthetic fixtures are allowed only for resource and parity testing;
they are permanently excluded from natural evidence.

Each release latency run uses one process, pinned CPU `4`, fixed fixture order,
4,096 matched and 4,096 no-goal cases, and no warmup replacement, outlier
deletion, or retry. Exactly three runs are allowed:

```text
matched p99                         <= 1,000,000 ns, PASS 3/3
no-goal p99                         <=   250,000 ns, PASS 3/3
hard max                            <= 2,000,000 ns, PASS 3/3
```

The exact durability gates run three times each in release mode:

```text
single-ledger records               1,024 per run
three-ledger episodes                 256 per run
sync p99                            <= 5,000,000 ns, PASS 3/3
sync hard max                      <= 20,000,000 ns, PASS 3/3
```

The prepared candidate must also pass:

```text
ordinary output parity             byte-identical
status/reason/package parity        exact
false accepts                       0
runtime parity failures             0
canonical precommit payload        <= 32 KiB
typed goal predicate               <= 4 KiB
available K1 actions               <= 256
framed segment                      64 MiB
combined journal quota             2 GiB
persisted raw payload              0 bytes
idle CPU, 60-second average        <= 0.25% of one core
isolated steady-state RSS delta    <= 16 MiB
```

The idle observation is valid only when its isolated input files and request
counters are unchanged. The RSS comparison uses the same fixture, allocator,
authority artifacts, warmup count, sample schedule, and process ownership for
baseline and candidate. Missing or incomparable evidence is
`INVALID_ENVIRONMENT`, never PASS.

Any absolute, parity, identity, durability, or raw-payload failure is VETO.
There is no relative-gate escape and no threshold repair after observation.

## 6. Immutable Preparation Receipt

Before stopping the service, create a mode `0400` preparation directory below:

```text
/var/lib/nando-wave/deployments/<UTC>-<paper-commit>-s1c3/
```

The preparation payload uses schema
`nando.s1c3-transaction-preparation.v1` and binds:

```text
paper contract and paper manifest roots
candidate source commit, tree, lockfile, binary hash, and binary size
old deployed commit, receipt root, binary hash, and binary size
old and candidate role-config hashes
unchanged unit, phase-config, and authority-config hashes
rollback binary, config, unit, metadata, and rollback-manifest root
all owning and untouched MainPID / NRestarts / ActiveState tuples
gateway, hot, control, and CPU health roots before
false accepts and runtime parity failures before
local connector PID, command hash, and route-receipt failures before
journal absent/present state, canonical tree manifest, bytes, and root before
candidate resource and parity receipt roots
deployment intent sequence and creation timestamp
```

The rollback directory is fully populated and verified before the preparation
receipt becomes immutable. It contains the exact old binary and old role
config even if a generic deployment helper would omit either file.

The generic `nando.deployment-receipt.v1` is supporting evidence only. It does
not by itself satisfy S1C-3 because V1 additionally requires config-pair
identity, `NRestarts`, journal preservation, resource roots, restart
attribution, and 15-second survival.

## 7. Exact Transaction Chronology

Only one prepared transaction may run:

```text
1. revalidate every preparation identity and health root
2. arm rollback with the immutable old binary and old config
3. systemctl stop nando-transition-serving.service
4. prove old PID exited and all untouched PIDs survived
5. install candidate binary and candidate config to temporary sibling files
6. fsync both files and both parent directories
7. verify temporary hashes against the preparation receipt
8. rename candidate config and candidate binary into place
9. fsync both parent directories and verify installed hashes
10. systemctl start nando-transition-serving.service
11. wait for one nonzero new PID and health PASS
12. execute post-start gates and the exact 15-second survival interval
13. finalize one immutable deployment or rollback receipt
```

The service is stopped before the two-file swap. Therefore no running process
can observe a half-installed pair. A host/process failure after step 3 leaves
an armed transaction: recovery restores the old pair first, preserves the
journal, starts the old service, and emits `S1C3_ROLLBACK_PASS` or
`S1C3_VETO`. Recovery never resumes midway by guessing which candidate byte
was installed.

The intended service PID must change exactly once. Manual `stop` plus `start`
must not increase `NRestarts`. An extra transition-serving PID change or any
untouched PID/restart change is VETO and triggers rollback.

## 8. Post-Start Gates

Before PASS, require all of:

```text
installed binary hash              == prepared candidate binary hash
installed role config hash         == 1e6e6726...24708d6
unit/phase/authority hashes         unchanged
new transition PID                 nonzero and != old PID
transition NRestarts               unchanged
untouched service PIDs/restarts     unchanged
all required services              active/running
hot health                         PASS
gateway/control health             PASS
CPU mode/admission                 unchanged and PASS
active product packages            unchanged
false accepts                      0
runtime parity failures            0
HTTP response/fallback route       byte/decision equivalent
process environment                exact two S1C values present
startup journal recovery           PASS
startup log                        no grounded-decision unavailable/error
raw payload scan                   0 bytes
combined journal bytes             <= 2 GiB
hot RSS delta                      <= 16 MiB
15-second PID survival             PASS
connector route receipt failures   unchanged
```

The post-start journal receipt records exact relative paths, sizes, per-file
hashes, manifest root, and total bytes. A moving active-segment snapshot is an
operational deployment observation only; it is not a scientific episode root.

After the first health PASS, wait exactly 15 seconds without restart, reload,
or config change and repeat PID, `NRestarts`, health, environment, binary,
config, false-accept, parity, and untouched-service checks.

No dashboard wording changes in S1C-3. The existing dashboard remains a view
of S1A/S1B facts. Capture availability belongs in the deployment receipt until
S1C-4 separately freezes and implements a truthful census projection.

## 9. Rollback Contract

Rollback is mandatory on every identity, install, start, health, parity,
resource, journal, survival, or receipt-finalization failure after the service
has been stopped.

Rollback order:

```text
stop candidate transition-serving if running
capture forward journal preservation manifest
restore exact old role config
restore exact old binary
fsync restored files and parent directories
verify old hashes
start transition-serving once
verify old-source health and 15-second survival
prove every pre-rollback journal file remains present
prove each preserved prefix is byte-identical and no file shrank
emit immutable rollback receipt
```

Rollback disables capture by restoring the old config. It never removes,
truncates, rewrites, rotates, or reclassifies forward journal evidence. A
rollback that restores service but loses journal bytes is `S1C3_VETO`, not
PASS.

The rollback source identity is the current deployed source commit
`663959064a37caf7eb917fc99dfedb6386355fa6`, bound to the exact old binary and
the authoritative current deployment receipt. The older rollback commit inside
that deployment receipt is not substituted for the current installed source.

## 10. Terminal Verdicts

```text
STALE_BEFORE_DEPLOYMENT
  frozen old production identity changed before mutation; no deployment.

INVALID_ENVIRONMENT
  required pre-deployment evidence is missing or incomparable; no deployment.

S1C3_DEPLOYMENT_PASS
  one exact candidate pair installed, one intended PID change observed, every
  post-start and survival gate passed, and the immutable receipt finalized.

S1C3_ROLLBACK_PASS
  candidate did not pass, exact old service was restored, 15-second survival
  passed, and all forward journal bytes were preserved. S1C-3 remains failed.

S1C3_VETO
  serving, authority, resource, durability, identity, rollback, or evidence
  preservation failed or cannot be proven.
```

Only `S1C3_DEPLOYMENT_PASS` permits the separately frozen S1C-4 finite natural
census. It still leaves:

```text
natural exact-goal surface          UNKNOWN
decision episodes                  0 until observed
K2                                 BLOCKED
model training                     false
phase mutation                     false
```

An empty journal, only `MISSING_EXACT_GOAL` censors, or zero decision episodes
after installation is not an S1C-3 failure. Those are S1C-4 scientific facts,
and waiting cannot be used to reinterpret the deployment verdict.

## 11. Frozen Stop Rules

Stop before production mutation if:

```text
paper verification is not PASS
candidate source is not exact a3ea27a
candidate binary was rebuilt after an observed gate
old production identity drifted
rollback pair or preparation receipt is incomplete
absolute resource or parity gate is not PASS
any untouched owner would require a restart or config edit
```

Stop and roll back after mutation if:

```text
installed pair differs from preparation
capture startup cannot be proven
service or authority health regresses
false accepts or parity failures increase
resource or disk budget is breached
an extra PID/restart occurs
an untouched service or connector changes identity
raw payload bytes appear in the journal
the final receipt cannot be made immutable
```

No gate, denominator, hash, service set, resource ceiling, survival interval,
or rollback condition may be weakened after execution starts. A required
repair creates S1C-3 V2 and a new paper and deployment watermark.

## 12. Paper Acceptance Gate

Before any S1C-3 runner, build, install, config write, or service command:

- the adversarial critique is complete and every accepted P0/P1 repair is in
  this contract;
- split structural packets for identity, transaction chronology, owner
  isolation, rollback/evidence, resources, and claim authority all pass;
- every structural packet has `authority_ready=false`;
- the final preregistration, critique, exact candidate config, packets, and
  packet results are content-addressed by one immutable paper manifest;
- a paper verification receipt grants one S1C-3 execution attempt only;
- only paper artifacts are committed and pushed;
- production, services, connector, and `graphify-out/` remain untouched.
