# S1C Transactional Deployment V2 Attempt 2026-08-12

Status: `PREFLIGHT IMPLEMENTATION FAILURE / NO QUIESCENCE / NO PRODUCTION MUTATION / V2 TERMINAL`

## Frozen Attempt

```text
paper commit             33380e3110e021a6c2d959ba7e04492e79e5093a
implementation commit    19ec792
candidate commit         a3ea27a49af397ef79e5c9ec80089ecf53a41d59
transaction id           20260811T235553Z-33380e3110e0-s1c3v2
```

## Exact Result

The candidate release binary and both direct lib-test harnesses built
successfully. The first parity-oracle prebuild stopped before quiescence:

```text
error
  failed to write oracle-baseline/Cargo.lock

cause
  Permission denied (os error 13)

ownership route
  root created oracle-baseline directory
  -> Cargo ran as user e
  -> directory was not writable by e
```

No `QuiescenceReceiptV2`, metric, preparation receipt, or service command was
issued. This is an implementation ownership defect, not a latency result.

## Evidence

Remote directory:

```text
/var/lib/nando-wave/deployments/
  20260811T235553Z-33380e3110e0-s1c3v2/
```

Local mirror:

```text
/home/ubu/.local/state/nando-wave/s1c3/
  20260811T235553Z-33380e3110e0-s1c3v2/
```

```text
complete local evidence listing root
  9922f5eb4e3ca474e2868a6abdd62e148491e204a90c9cd75debd2a9891a0720

preflight failure SHA-256
  8072cf91412ecdbc6290d3c685466cb6dc729cd89db94405e0b1d1aacdd84c9b

parity baseline build log SHA-256
  84f075d56a3287e2e472c0fc089eab8a769a8d2051387110a6ee1faec5dfd5c6
```

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

The installed binary and config hashes remain the frozen baseline. V2 is not
retried. A further attempt requires a new V3 paper and watermark.

