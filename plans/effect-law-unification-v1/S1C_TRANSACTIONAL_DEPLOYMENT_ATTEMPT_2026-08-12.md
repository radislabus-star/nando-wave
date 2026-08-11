# S1C Transactional Deployment Attempt 2026-08-12

Status: `INVALID_ENVIRONMENT / NO PRODUCTION MUTATION / V1 TERMINAL`

## Frozen Attempt

```text
proof-plane commit       dca849cb39a3ab2918b8f1e596baf0b9a28e102f
paper commit             b3ee186d49d848b1917472f427d6afc59459c7cd
candidate commit         a3ea27a49af397ef79e5c9ec80089ecf53a41d59
transaction id           20260811T223923Z-b3ee186d49d8-s1c3
```

The one V1 attempt stopped during the absolute pre-deployment resource gate.
No preparation receipt was issued and the transaction never reached
`systemctl stop`.

## Exact Result

The hot compatibility gate passed all three frozen runs:

```text
run  matched p99 ns  no-goal p99 ns  hard max ns
1            12,834             596       15,969
2            12,346             578       32,419
3            12,630             569      178,435
```

The first single-ledger durability run failed the frozen absolute ceiling:

```text
observed p99              5,010,709 ns
frozen ceiling            5,000,000 ns
excess                       10,709 ns
relative excess                0.21418%
hard max                  5,612,231 ns
records                       1,024
segments                          5
```

V1 forbids deletion, rounding, retry, threshold repair, or substitution of
this observation. The remaining single-ledger, three-ledger, idle, RSS, and
parity gates were therefore not executed.

## Evidence

Authoritative mini-PC directory:

```text
/var/lib/nando-wave/deployments/
  20260811T223923Z-b3ee186d49d8-s1c3/
```

Local evidence mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260811T223923Z-b3ee186d49d8-s1c3/
```

```text
local SHA256SUMS root      d6e7b1cde1b9a11f1b82fa55afda18bc0694a152d0a0a1c490bd0e102bcb89cc
preflight failure SHA      f46279bf06d74ee44301c114e75b292a80ac7a96aaf4699a74d48b5a7aba7641
single-sync log SHA        8398200e3b9ccd2187e35bc145606aa5926b593cf00e90ed744f2d7943fb1a77
```

## Production Preservation

After the terminal preflight failure:

```text
transition-serving        PID 165670   restarts 0   active
response-learning         PID 369456   restarts 0   active
gateway-control          PID 1035203   restarts 0   active
certification authority   PID 164668   restarts 0   active
transport / Nginx         PID 682430   restarts 0   active
local connector           PID 2919     restarts 0   active
route receipt failures    0
grounded journal          ABSENT
```

The installed binary, role config, and unit hashes remained the frozen
baseline values. No capture authority, K2 claim, dashboard claim, or natural
census was opened.

## Next Authority Boundary

V1 is spent. A further attempt requires S1C-3 V2 with a new paper and
watermark. V2 may add a preregistered quiescence eligibility gate before the
single observation while preserving the same `5,000,000 ns` p99 ceiling. It
may not reinterpret or retry the V1 result.
