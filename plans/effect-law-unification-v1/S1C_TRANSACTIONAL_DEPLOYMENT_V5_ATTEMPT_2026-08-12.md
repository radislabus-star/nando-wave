# S1C Transactional Deployment V5 Attempt 2026-08-12

Status: `PREFLIGHT FAILURE / OFFLINE CLOSURE MISSING / NO PRODUCTION MUTATION / V5 TERMINAL`

## Frozen Attempt

```text
paper commit             bb8f3e2d7142d7acc2d91ce8e098991c018b4f35
implementation commit    c0de92f2b28c07d41c09e332bd558d7feb9a41ae
candidate commit         03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree           06a9df51797dffc127fec41672bddae29c38bb92
transaction id           20260812T030446Z-bb8f3e2d7142-s1c3v5
remote attempts          exactly one
```

## Exact Result

V5 built the fresh candidate and test harnesses, built the baseline parity
oracle, and then failed while Cargo resolved the candidate parity oracle:

```text
candidate production projection     PASS
fresh candidate build               PASS
fresh test harnesses                 PASS
baseline parity oracle               PASS
candidate parity oracle              PREFLIGHT FAILURE
error                                crates.io config DNS resolution failed
oracle ownership probes              PASS for both workspaces
quiescence                           NOT STARTED
resource measurements               NOT STARTED
independent predeployment verifier   NOT STARTED
systemctl stop                       NOT REACHED
production mutation                 no
```

The exact error was:

```text
failed to get serde_json
unable to update registry crates-io
download of config.json failed
Could not resolve host: index.crates.io
```

This is a reproducibility defect in the proof preflight. It is not a candidate
latency result, a quiescence result, a deployment rollback, or scientific
evidence.

## Evidence

Remote directory:

```text
/var/lib/nando-wave/deployments/
  20260812T030446Z-bb8f3e2d7142-s1c3v5/
```

Local mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260812T030446Z-bb8f3e2d7142-s1c3v5/
```

```text
local evidence files                 18
local evidence bytes             201,556
local listing root
  080f486028266f95ee5f0e97a4377ca1ce96ebc459e26c428f5174916650dab5

remote evidence files                13
remote evidence bytes            212,488
remote listing root
  1cbd47378a960115699b7bc92719535a1bdc365f0f772625605f896a03a56557

preflight failure SHA-256
  4bcaaf8a5141e59703dc28394e2ac50709118cca20ee805664c0d161f88f673c

local prepare error SHA-256
  33bdace83b6058a1b0ac2c6fc4e64b0920b750f7dedba87e262e703a81a13c64
```

The local and remote roots intentionally cover different mirrors: the local
directory also retains launcher output and connector snapshots. Neither root
is promoted to a deployment receipt.

## Production Preservation

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

## Next Boundary

V5 is not retried. A V6 attempt must freeze one common parity-oracle package
identity and lockfile, copy the lock into both fresh oracle workspaces, and
build both with `--offline --locked` plus `CARGO_NET_OFFLINE=true`. The
independent verifier must bind the lock hash and reject missing offline flags,
lock mutation, package-name divergence, or network-capable oracle commands.

The candidate, config, resource thresholds, multi-core quiescence contract,
production affinity, one-attempt rule, and claim boundary remain unchanged.
S1C-4, grounded meaning, K2, and dashboard scientific claims remain closed.
