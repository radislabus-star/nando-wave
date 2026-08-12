# S1C-3C Terminal Dashboard Deployment V16 2026-08-12

Status: `CONTROL PLANE DEPLOYED / TERMINAL RESOURCE VETO VISIBLE / DATA PLANE PRESERVED`

## Live Projection

Dashboard build `2026.08.12-control-v16` exposes the terminal S1C-3C boundary
without granting scientific authority:

```text
capture                  NOT INSTALLED
attempt                  TERMINAL / RESOURCE VETO
production               UNCHANGED
authority envelope       UNSEALED
S1C-4                    CLOSED
rerun                    FORBIDDEN
```

The embedded `data:,` favicon removes the browser's unrelated
`/favicon.ico` request. No runtime, learning, admission, or phase behavior was
changed.

## Deployment Receipt

```text
source commit
  47f54ba85da2ec48ad9e57a5113a8402bb576bbd

rollback commit
  051005735070593446718b9def77a611dfaa09e2

receipt
  /var/lib/nando-wave/deployments/
    20260812T122742Z-47f54ba85da2/deployment-receipt.json

receipt root
  9e468f4322d4b5ec7bf95af7a736233045dc5daa8ca804b57dff37aaa0184ccd

installed and release control SHA-256
  89e3a4e37a9d3892bb25c085357073acad837cd363adf46264174920ce020d06

terminal status sidecar SHA-256
  b4af2ea30749e8240f9d6b439a8cd8f8310cf3e7a916e85823b2664666133d09
```

The receipt is root-owned mode `0400`. Independently deleting its stored root,
canonicalizing the remaining JSON, and hashing it reproduced the stored root.
The installed gateway-control binary equals the release artifact byte for
byte.

## Runtime Preservation

```text
gateway-control             PID 433679   active   restarts 0
transition-serving          PID 165670   active   restarts 0
response-learning           PID 369456   active   restarts 0
certification authority     PID 164668   active   restarts 0
transport / Nginx           PID 682430   active   restarts 0
local connector             PID 2919     active   restarts 0
route receipt failures      0
```

All five remote service identities survived a further 15-second check. Control,
hot, and edge health passed. The receipt records unchanged hot-serving and
Nginx PIDs with no restart exception.

## Browser Verification

The existing isolated QA tab was reloaded and reused; no duplicate tab was
opened. It was closed after verification while the two user-owned tabs were
preserved.

```text
desktop effective viewport       1280 x 800
mobile viewport                   390 x 844
horizontal overflow               0 / 0
out-of-viewport elements          0 / 0
console warnings or errors        0 / 0
favicon.ico requests              0
expected terminal labels          5 / 5 present
```

Both layouts were visually inspected. The page rendered `NOT INSTALLED`,
`TERMINAL / RESOURCE VETO`, `UNCHANGED`, `UNSEALED`, and `CLOSED` without text
overlap.

## Boundary

This deployment completes only the truthful control-plane projection of the
already consumed S1C-3C attempt. It does not rerun the transaction, install
capture, seal the authority envelope, start S1C-4 or S2, train a model, mutate
phase state, activate a package, or prove grounded meaning.
