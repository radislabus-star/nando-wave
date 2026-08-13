# S1C-4 Natural Census Live Opening

Status: `DEPLOYED / COLLECTING`

## Exact Deployment

```text
implementation commit     e0ccff9e38411890e8158307eb7ec3adccaf5fb9
implementation tree       2b16af3ee0c8ad73c968082edc06203bc53e2208
deployment receipt root   cae629b40f5c02df5856d60ddee3ee84725b12c12f256cccbf3534ef997e28f7
rollback commit           6f83abf21c2450cf6d89337c83e48f6d721aa8b5
transition binary         b55baae07551297f8f28b7e3b002f5ef92075bb20ebef8fc20c54353845d4f3e
control binary            1cf29e0cad125ebcaac155476764e05f162b9ea450f370d0cfa8562ef9415aef
```

Authoritative receipt:

```text
/var/lib/nando-wave/deployments/20260813T035900Z-e0ccff9e3841/deployment-receipt.json
```

The receipt is mode `0400`. Its recorded root was independently recomputed
from the canonical payload and matched. The rollback snapshot contains the
previous transition/control binaries and the exact three legacy V1 journal
prefixes.

## Frozen Natural Window

```text
cursor root               9a1d2a49cd862ba8dde518bde148bd8234dfb741a7c48fa48b3079b4f21a9831
start sequence            182329
start request ordinal      91165
start input tokens     19071218082
opened at unix         1786593704
deadline unix          1786680104
request limit                 1024
quiescence seconds               60
```

The cursor is bound to the deployed transition binary and the deployment
receipt root. It was opened by one durable `nando.s1c4-open-request.v1` and
contains no retrospective traffic.

Observed snapshot at `2026-08-13T04:04:47Z`:

```text
state / verdict                 COLLECTING / COLLECTING
denominator / classified       20 / 20
denominator input tokens       5,130,887
goal bound                     0
alternative bearing            0
decision episodes              0
distinct lineages              0
terminal censor                MISSING_EXACT_GOAL 20
source / exact join            true / true
queue / writer / duplicate     0 / 0 / 0
false accepts / parity         0 / 0
```

Counts after this snapshot remain append-only and will advance with ordinary
traffic. The terminal verdict is produced after exactly 1024 denominator
requests or 24 hours, followed by the frozen 60-second quiescence interval.

## Runtime Verification

```text
production composite gate      PASS
local accept eligible          true
transition PID                 1816591, NRestarts 0
control PID                    1816583, NRestarts 0
response learning PID          369456, NRestarts 0
certification authority PID    164668, NRestarts 0
Nginx PID                      682430, unchanged, NRestarts 0
local connector PID            2919, unchanged
dashboard build                2026.08.13-control-v19
```

Responsive and page checks passed through the unchanged local connector at
desktop, laptop, tablet, mobile, and narrow mobile viewports. The page had no
browser errors, and every named browser session was closed after inspection.

## Claim Boundary

This deployment proves that the finite S1C-4 instrument is installed and is
classifying its exact ordinary-traffic denominator. It does not prove grounded
decision meaning and does not authorize K2.

```text
S1C-4                    COLLECTING
K2                       CLOSED
S2                       NOT STARTED
model training           FORBIDDEN
phase mutation           FORBIDDEN
package activation       FORBIDDEN
Law #2 promotion         FORBIDDEN
synthetic traffic        NOT USED
```

The next action is no action: leave the finite natural window running. Stop at
its terminal report and review that result before any S2 or K2 work.
