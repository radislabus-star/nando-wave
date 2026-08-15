# K2 Generated Capability Dashboard V24 Preregistration

Status: `FROZEN / READ-ONLY OBSERVATION SLICE`

Date: `2026-08-15`

## 1. Question

Can the live control page show the three completed generated causal-capability
results without promoting any of them into natural K2, K1, product execution,
or production authority?

The only allowed result is a read-only display change. This slice cannot create
scientific evidence or mutate any runtime state.

## 2. Evidence Owners

```text
hidden-effect induction
  K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_EXECUTION_EVIDENCE_2026-08-14.md
  SHA-256 aef3dd0025ecdf5ca6b5df0873da842321b03a9240eab2978d2ce8c4521eb9cb

explicit learned composition
  K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_V1/capability-receipt.json
  SHA-256 95baf02f6a20a5b6bf884f8a47a0c00b5830ce0f775770273285e266ecb4ebb0

hidden representation transfer
  K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_V1/capability-receipt.json
  SHA-256 c5c07cd2990d5f71f935977a932416c7daf6c6ff3b747d9e75243631ddf95a35
```

The compiled control binary reads only these checked-in bytes. Missing,
malformed, non-PASS, or authority-bearing evidence renders `UNVERIFIED`, never
`PASS`.

## 3. Required Display

The generated block is separate from live natural K1/K2 rows and states:

```text
hidden effects learned          PASS
explicit composition            PASS
hidden representation           PASS
confirm exact goals             2 / 2
search evaluations              61 / 67 vs 8,659 each
negative controls               18 / 18
production authority            FALSE
Natural K2                      NOT PROVED
```

The existing natural row is renamed `Natural K2` and remains `NOT PROVED`.
Generated evidence must not alter `/api/v1/dashboard`, K1 counters, discovery,
S1C-4, CPU admission, economics, phase memory, certificates, or packages.

## 4. Change Budget

```text
live_dashboard_v21.html          one flat status section
live_dashboard.rs                receipt projection plus tests
main.rs                          unchanged
API schema                       unchanged
JavaScript fetch route           unchanged
service restarts                 gateway control exactly once if deployed
hot serving restart              forbidden
Nginx restart                    forbidden
local connector restart          forbidden
```

## 5. Deployment Transaction

```text
remote 20-job build
-> verify tests, fmt and strict Clippy
-> stage new gateway-control binary
-> record old and staged SHA-256
-> preserve rollback copy
-> atomic install
-> restart nando-gateway-control only
-> verify dashboard build, API, service survival and browser rendering
-> rollback old binary if any required check fails
```

Frozen baseline:

```text
dashboard build                  2026.08.14-control-v23
gateway-control PID              298415
hot-serving PID                  1816591
Nginx PID                        682430
local connector PID              2919
false accepts                    0
parity failures                  0
services                         3 / 3
K1 laws                          1 / 3
Natural K2                       NOT PROVED
```

## 6. PASS Contract

All source and receipt tests pass, the new block contains the exact eight facts,
natural and generated rows remain separate, production authority renders
`FALSE`, Natural K2 renders `NOT PROVED`, desktop and mobile have no overflow or
JavaScript errors, and only the gateway-control PID changes during deployment.
