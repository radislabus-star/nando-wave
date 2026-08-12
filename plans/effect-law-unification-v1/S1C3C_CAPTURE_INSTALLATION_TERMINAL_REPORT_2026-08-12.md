# S1C-3C Capture Installation Terminal Report 2026-08-12

Status: `TERMINAL RESOURCE_VETO / NO PRODUCTION MUTATION / AUTHORITY ENVELOPE UNSEALED`

## Result

The only preregistered S1C-3C remote transaction was executed once and stopped
before production mutation:

```text
transaction             20260812T113705Z-2a1505055ce9-s1c3c-v1
operational state       RESOURCE_VETO
production mutation     false
capture installed       false
attempt count           1 / 1, consumed
authority envelope      UNSEALED
authority               false
S1C-4                   CLOSED
S2                      BLOCKED
```

This attempt must not be rerun. There is no automatic S1C-3D.

## Resource Veto

The frozen resource receipt records exactly three resource failures:

```text
parity byte identity               FAIL
three-sync round 2 settlement p99  5.097076 ms > 5 ms
three-sync round 3 settlement p99  6.104611 ms > 5 ms
```

Round 1 settlement p99 was `4.431531 ms` and passed. The measurement monitor
itself passed, reported no monitor errors, and observed a maximum sample gap of
`0.585548462 s` against the `2 s` limit.

Both parity oracle processes returned `101` while reading the registry:

```text
registry: Os { code: 13, kind: PermissionDenied, message: "Permission denied" }
```

The baseline and candidate output hashes consequently differed. The resource
mechanism correctly treated this as a parity failure and vetoed installation.

## Unsealed Authority Envelope

After the operational VETO, the frozen S1C-3C verifier called the pinned S1C-3B
resource verifier. That verifier unconditionally requires:

```text
baseline_output_sha256 == candidate_output_sha256
```

This is valid for a PASS but prevents independent sealing of an observed
parity-mismatch VETO. Its exact result was:

```json
{"authority":false,"error":"parity_output_mismatch","valid":false}
```

The verifier is frozen and was not changed retroactively. The two facts remain
separate: the durable resource mechanism reached `RESOURCE_VETO`, while the
S1C-3C authority envelope is `UNSEALED`. The authority-free postmortem does not
repair or replace preregistered authority.

## Evidence

Raw evidence remains on disk and outside Git:

```text
local
  plans/effect-law-unification-v1/evidence/
    S1C3C_CAPTURE_INSTALLATION_ATTEMPT_V1/
    20260812T113705Z-2a1505055ce9-s1c3c-v1
  46 files / 74,701,441 bytes
  normalized root
    54c223887103d3f781e23df124f158c794a86533823559557377b75c1ea54bee

remote
  /var/lib/nando-wave/deployments/
    20260812T113705Z-2a1505055ce9-s1c3c-v1
  39 files / 74,696,658 bytes
  normalized root
    eaaf3977b4d5545d87a017e4c79b6b2e3eaba4aac45082583281e5ffa03588dd

shared remote files      39
shared byte mismatches   0
```

The committed normalized manifest binds every remote relative path, size, and
SHA-256. The bounded postmortem report records only terminal fields, roots, and
the three failure rows.

## Verification

```text
postmortem schema                 nando.s1c3c-postmortem-verification.v1
postmortem valid                  true
postmortem authority              false
postmortem root
  5daeb142e7b5782d330a6aeca1166afcfae0f96ba00cd163a283bcc1990e60fd
terminal status root
  28a0ed19d511072b4e4155fa2f7649a327e42fdf7c328cc0ccf7da78840a4384
resource root
  174bc9ac3f7e7a6d53561bca3059c721bb59e2e6886baf6af29bb201016d23d0
parity root
  6390d4ac7b5acebbbf7b9d56b692662a64c7ddfa7fc39a8482a1797f9f62af6e
```

Production services remained active with zero restarts. Transition serving,
Nginx, and the local connector retained PIDs `165670`, `682430`, and `2919`;
route receipt failures remained zero.

## Boundary

```text
S1C production capture     NOT INSTALLED
S1C-3C attempt             TERMINAL RESOURCE_VETO
authority envelope         UNSEALED
production                 UNCHANGED
S1C-4 natural census       CLOSED
grounded meaning           NOT PROVED
K2                         CLOSED
```
