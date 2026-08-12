# S1C-3B Terminal Dashboard Deployment 2026-08-12

Status: `CONTROL PLANE DEPLOYED / TERMINAL PREFLIGHT BOUNDARY VISIBLE / DATA PLANE PRESERVED`

## Live Result

The control page now projects the sole S1C-3B attempt without promoting raw
measurement logs into a resource or deployment verdict:

```text
dashboard build          2026.08.12-control-v14
capture                  NOT INSTALLED
attempt                  TERMINAL PREFLIGHT_FAILURE
production               UNCHANGED
resource verdict         null
deployment verdict       null
attempt consumed         true
S1C-4                    CLOSED
authority ready          false
model training           false
phase mutation           false
```

K1 remains `1 / 3` and closed. Live safety remains `false accepts 0 / parity
failures 0`.

## Deployment

```text
source commit
  66edd011f310fcdc953c78995feb73b06537efc0

rollback commit
  b409fa339ee5f2d07ffbc18917f474188000a743

receipt
  /var/lib/nando-wave/deployments/
    20260812T101314Z-66edd011f310/deployment-receipt.json

receipt root
  ebba93ad892fb2a0e6b83561e8cb5dc98a24b071ab7ff3cf2b9a9b7f26fffd0a

installed control SHA-256
  5d4f2f7e4f5cd04c653cc1dbca61b4e0741cd80e55d24a15bbdfa7e7f0b1e860

installed sidecar SHA-256
  79e695cc1f0f339f8c68820b893e95bb89466986c3e20d132b0b512b358c9c2e
```

The receipt is root-owned mode `0400`. Its stored canonical payload root equals
the independently recomputed root. The transaction contains rollback copies of
the prior gateway-control binary and sidecar.

## Runtime Preservation

```text
transition-serving          PID 165670   unchanged   restarts 0
response-learning           PID 369456   unchanged   restarts 0
certification authority     PID 164668   unchanged   restarts 0
transport / Nginx           PID 682430   unchanged   restarts 0
local connector             PID 2919     unchanged   restarts 0
gateway-control             4034438 -> 235056       intentional restart
route receipt failures      0
```

All identities survived a further 15-second check. The receipt records
`hot_pid_unchanged=true`, `nginx_pid_unchanged=true`, and
`hot_restart_allowed=false`.

## Verification

```text
S1C-3B fault and regression tests     30 / 30 PASS
nando-gateway-control tests           57 / 57 PASS
strict scoped Clippy                  PASS
rustfmt                               PASS
structural owner routes               4 / 4 PASS, authority false
release / installed binary SHA        MATCH
sidecar / installed SHA               MATCH
receipt canonical root                MATCH
desktop 1280 x 800 overflow           0
mobile 390 x 844 overflow             0
console messages                      0
JavaScript errors                     0
```

The desktop evidence helper returned `PASS`. Desktop and mobile checks both
rendered `NOT INSTALLED / TERMINAL PREFLIGHT FAILURE / UNCHANGED / CLOSED`.
The isolated QA session contained one tab; it was closed after verification,
and no browser process remained.

## Boundary

This deployment changes only the control-plane truth projection. It does not
retry S1C-3B, install capture, start S1C-4, create grounded meaning, train a
model, mutate phase state, activate a package, or grant K2 authority.
