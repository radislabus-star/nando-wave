# Signal Path API V1

## Purpose

`nando-gateway-control` exposes one authenticated, read-only projection of the
complete live route:

```text
Codex window
-> Nginx gateway
-> transition serving
-> externally admitted ACTIVE OperatorPackage
-> verified CPU local accept
```

The endpoint is:

```text
GET /control/<control-key>/api/v1/signal-path
```

It is intentionally under the existing opaque control-key route. Unknown keys
receive the same `404` response as every other control endpoint.

## Contract

The response schema is `nando.signal-path-status.v1`. It contains:

- five ordered stages with owner, `PASS | WATCH | BLOCK`, and reason;
- `first_non_pass`, which points to the earliest broken edge;
- the current Codex-window connection snapshot;
- compact summaries of ACTIVE packages, without actor pages or proof payloads;
- exact Nando/miner/CPU token counters and integer parts-per-million shares;
- false-accept, runtime-parity, and bridge-failure counters.

`complete=true` requires all five stages to be `PASS`.

## Authority Boundary

The endpoint only reads existing reports and health snapshots. It must never:

- invoke an operator or synthetic probe;
- change gateway mode;
- create an authority lease;
- promote a package;
- infer CPU success from `ADMISSION OPEN` alone.

The CPU stage is `PASS` only when both the control plane and serving runtime
report local accept enabled, the serving executor has the same ACTIVE package
count as the registry, exact token accounting is fresh, at least one verified
local accept is observed, and all safety counters are zero.

The read-only controller snapshot has a separate freshness budget,
`NANDO_RESPONSE_CONTROLLER_REPORT_MAX_AGE_SECONDS` (default `90` seconds),
because the controller normally reconciles every `60` seconds. This diagnostic
budget cannot extend the shorter execution-authority lease.

## Failure Semantics

Missing, stale, contradictory, or malformed evidence closes the affected stage.
A valid authenticated request still returns HTTP `200`; route failure is part
of the JSON state, not an HTTP transport error. Responses carry
`Cache-Control: no-store`.
