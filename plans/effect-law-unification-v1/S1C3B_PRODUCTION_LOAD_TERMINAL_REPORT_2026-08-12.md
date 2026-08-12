# S1C-3B Production-Load Terminal Report 2026-08-12

Status: `TERMINAL PREFLIGHT_FAILURE / NO PRODUCTION MUTATION / ATTEMPT CONSUMED`

## Result

The sole preregistered S1C-3B transaction terminated before production
mutation while evaluating the collected idle-CPU metric:

```text
transaction             20260812T093629Z-36ffc0cbf56b-s1c3b-v1
terminal state          PREFLIGHT_FAILURE
production mutation     false
capture installed       false
resource verdict        none
deployment verdict      none
attempt count           1 / 1, consumed
S1C-4                   CLOSED
```

This result is not `S1C3B_RESOURCE_VETO`. The executor did not finish the
measurement monitor, resource receipt, or independent resource verification,
so the collected measurement logs have diagnostic authority only.

## Implementation Defect

The Rust harness emitted the frozen public metric:

```text
S1C_IDLE_CPU elapsed_ticks=0 ticks_per_second=100 percent_of_one_core=0.000000
```

The executor regex parsed that value correctly, but the call site stored it as
`percent_micros`. The evaluator then accessed `percent_of_one_core` and raised:

```text
KeyError: 'percent_of_one_core'
```

The failure occurred at
`ops/remote-backend/s1c3b_remote_transaction_v1.py:778` in the committed
attempt implementation. Postmortem hardening now uses one frozen
`IDLE_METRIC_FIELDS` tuple and includes a regression test against the exact
observed log shape. This repair does not change the 0.25 idle-CPU threshold,
any latency limit, capture behavior, or production authority.

## Evidence Preservation

```text
local evidence
  plans/effect-law-unification-v1/evidence/
    S1C3B_PRODUCTION_LOAD_ATTEMPT_V1/
    20260812T093629Z-36ffc0cbf56b-s1c3b-v1
  files 40
  bytes 230792
  normalized manifest SHA-256
    45150667dcd94fd2db8b2f6d9c3d77db3c07c8e9b5cb3fe40ec1fbbfe38b4c26

remote evidence
  /var/lib/nando-wave/deployments/
    20260812T093629Z-36ffc0cbf56b-s1c3b-v1
  files 35
  bytes 229242
  normalized manifest SHA-256
    908a454f843a38e19de8076a7b011aaa7e8d0176fe074cdc47ef05f6f165bc42

shared files            35
shared byte mismatches  0
```

The five local-only files are the source-bundle verification, connector
before/after snapshots, and local orchestrator stdout/stderr. They are not
missing remote measurement evidence. The exact comparison metadata is in
`evidence/S1C3B_PRODUCTION_LOAD_POSTMORTEM_V1/EVIDENCE_MANIFEST_V1.json`.

## Production Preservation

Immediately after the terminal attempt and again during postmortem:

```text
transition-serving binary SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58
transition-serving PID       165670, restarts 0
response-learning PID        369456, restarts 0
gateway-control PID          4034438, restarts 0
certification PID            164668, restarts 0
Nginx / gateway PID          682430, restarts 0
connector PID                2919, restarts 0
route receipt failures       0
```

No S1C capture file, journal authority, model training, phase mutation, package
admission, or K2 evidence was created by this attempt.

## Verification

```text
S1C-3B fault and regression tests     30 / 30 PASS
nando-gateway-control tests           57 / 57 PASS
strict scoped Clippy                  PASS
rustfmt                               PASS
Python compile and shell syntax       PASS
terminal outcome structural route     PASS, authority false
attempt authority structural route    PASS, authority false
runtime preservation structural route PASS, authority false
science boundary structural route     PASS, authority false
```

An initial combined structural worksheet returned `VETO` because it mixed four
decision owners. That rejected packet is preserved. The final four owner-scoped
worksheets each pass with no weak triads, conflicts, evidence gaps, or repair
queue. Their `authority_ready=false` status is intentional: structural
coherence does not create runtime or scientific authority.

## Terminal Boundary

The frozen paper authorizes exactly one transaction. That transaction has been
consumed. The parser regression repair is not authorization to rerun S1C-3B or
to create S1C-3C automatically.

```text
S1C-3B capture              NOT INSTALLED
S1C-3B attempt              TERMINAL PREFLIGHT_FAILURE
production                  UNCHANGED
S1C-4 natural census        CLOSED
grounded meaning            NOT PROVED
K2                          CLOSED
```
