# S1C Transactional Deployment V4 Attempt 2026-08-12

Status: `INVALID ENVIRONMENT / QUIESCENCE TIMEOUT / NO PRODUCTION MUTATION / V4 TERMINAL`

## Frozen Attempt

```text
paper commit             1def4272a46641f2c72a9c0efbd5818f93caa655
implementation commit    68b5e23be0f5c6e7bfdba3f0117aed670735c351
candidate commit         03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree           06a9df51797dffc127fec41672bddae29c38bb92
transaction id           20260812T011911Z-1def4272a466-s1c3v4
remote attempts          exactly one
```

## Exact Result

V4 bound the frozen production projection in the local launcher, root-owned
remote prepare path, preparation schema, independent receipt verifier, and
fault-injection suite. The one authorized transaction then built all fresh
candidate, harness, and parity-oracle artifacts and proved oracle workspace
ownership. It could not obtain the frozen 30-second quiescence window before
the 1,800-second deadline:

```text
candidate production projection     PASS
fresh candidate build               PASS
fresh test harnesses                 PASS
fresh parity-oracle builds           PASS
oracle ownership                     PASS
quiescence                           INVALID ENVIRONMENT
blocker                              INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT
resource measurements               NOT STARTED
parity execution                    NOT STARTED
preparation receipt                 NOT ISSUED
systemctl stop                      NOT REACHED
production mutation                 no
```

The result is a terminal environment preflight failure. It is not a candidate
latency failure and not a deployment rollback because production was never
stopped or changed.

## Ownership Evidence

```text
build user                           e:e, uid/gid 1000
baseline workspace mode             0750
candidate workspace mode            0750
exclusive create/fsync/unlink        PASS for both workspaces
directory fsync                      PASS for both workspaces
retained ownership probes            0
ownership root
  ba57ec08d1005fd80b79d7b6419dc1a5850517d6ba1659852e29472642db5f37
```

## Evidence

Remote directory:

```text
/var/lib/nando-wave/deployments/
  20260812T011911Z-1def4272a466-s1c3v4/
```

Local mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260812T011911Z-1def4272a466-s1c3v4/
```

```text
evidence files                         19
evidence bytes                    202,417
complete local listing root
  7ccf02735fad5223f8273cc28ca485f341ea74967053c18d70bccd934bfc9381

preflight failure SHA-256
  d639ef9803fdb42a876b48105071165ec9c0d1d7839e185b2932898309a0532e

ownership receipt SHA-256
  feabf0aeaa4bd4c053e7536302da55f35813e6d3b885a498d0033367f0806b5f
```

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

post-attempt composite gate             PASS
structural routes                       4/4 PASS
Wave causal section                     PASS
runtime admission                       PASS
response runtime                        PASS, M3 WATCH
deployment section                      PASS
false accepts                           0
runtime parity failures                 0
required actions                        0
```

Exact post-attempt composite output SHA-256:

```text
f5db09a600b8d44f86a1d742690ca7224af33b83a0315dc2f6b9c45a31ee6d63
```

## Evidence Gap And Next Boundary

V4 retained the terminal error but did not persist the attempted quiescence
samples when the deadline expired. Therefore the durable evidence proves the
timeout, but it does not authorize a claim about which process or metric
prevented the complete window. Live inspection observed unrelated builds and
ordinary production load during the wait; those observations are diagnostic,
not receipt authority.

V4 is not retried. A further S1C-3 attempt requires a new preregistration that
first repairs timeout evidence retention. It must durably write all attempted
samples and a blocker census on both PASS and timeout while preserving the
same CPU, IO, interval, latency, hard-max, contamination, rollback, and
one-attempt boundaries. S1C-4, K2, and dashboard scientific claims remain
closed until an S1C-3 deployment receives independent `DEPLOYMENT_PASS`.
