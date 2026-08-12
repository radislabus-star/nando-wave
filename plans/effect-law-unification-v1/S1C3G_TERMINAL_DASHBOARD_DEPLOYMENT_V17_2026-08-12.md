# S1C-3G Terminal Dashboard Deployment V17 2026-08-12

Status: `CONTROL PLANE DEPLOYED / ROLLBACK PASS VISIBLE / BASELINE RESTORED / S1C-4 CLOSED`

## Live Projection

Dashboard build `2026.08.12-control-v17` exposes the consumed S1C-3G
transaction without upgrading its scientific authority:

```text
S1C-3G                  ROLLBACK PASS
capture                 NOT INSTALLED
production              BASELINE RESTORED
authority envelope      ROLLBACK SEALED
S1C-4                   CLOSED
rerun                   FORBIDDEN
scientific authority    FALSE
```

The startup message `response_authority_runtime_build_mismatch` is displayed
only as a proximate diagnostic. The failed candidate projection was not
persisted, so the dashboard does not claim that message as the complete cause.

## Deployment Identity

```text
terminal transaction
  20260812T180143Z-1369da0a49ef-s1c3g-v1

dashboard source commit
  54a2b2a69d9d0232150047b75f0e84e774160906

remote deployment directory
  /var/lib/nando-wave/deployments/
    20260812T182750Z-54a2b2a69d9d-control-v17

installed gateway-control SHA-256
  862c9587dab08c542c1a5effb5591fd8ef15a0b34e17ce253dfd73dd8923ae1e

installed terminal sidecar SHA-256
  375073f761d5b2eb4a5ef8609e96566075840f5be227d137214fae8120fe9753

embedded terminal status root
  180ccb0e04748c9246a2d2316c85aa8b6aa6426ae8350576526dc8fc5c385745
```

The sidecar was installed with candidate write, file sync, atomic rename, and
directory sync. The previous S1C-3C sidecar is preserved byte for byte in the
deployment directory.

## Runtime Preservation

```text
gateway-control       PID 433679 -> 986562   active   restarts 0
transition-serving    PID 950239              active   restarts 0
transport / Nginx     PID 682430              active   restarts 0
local connector       PID 2919                active
route receipt failures                         0
false accepts                                  0
runtime parity failures                        0
```

Control, hot, edge, and connector-routed health passed. The three remote PIDs
survived a further 15-second check; only gateway-control was intentionally
restarted.

## Verification

```text
gateway-control tests               58 / 58 PASS
strict scoped Clippy                PASS
cargo fmt                           PASS
git diff --check                    PASS
Structural Gate terminal routes      4 / 4 PASS
API build                           control-v17
API terminal projection             exact rooted S1C-3G status
desktop viewport                    1440 x 1000
mobile viewport                      390 x 844
horizontal overflow                    0 / 0
console or page errors                 0 / 0
required terminal labels               4 / 4
```

Both layouts were visually inspected. The isolated QA tab was closed after the
check.

## Boundary

This deployment completes only the truthful terminal projection of the
already consumed S1C-3G attempt. It does not rerun S1C-3G, install capture,
open S1C-4 or S2, train a model, mutate phase state, activate a package, prove
grounded meaning, prove K2, or create Law #2.
