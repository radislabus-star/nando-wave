# K1 Exact Phase C Factual Dashboard

Date: 2026-08-14

## Purpose

Phase B2 enabled the exact writer without restarting services. Phase C exposes
that operational state without turning it into a scientific claim.

```text
authority policy and cold health
-> writer state + policy root + minimum V8 schema

authority-owned scheduler report
-> exact wake decision + exact counters + active immutable generation

durable certification ledger
-> LawCertificate count

three separate projections
-> one factual control page
```

## Ownership

```text
certification authority   owns policy bytes and exact wake status
certification ledger      owns active freeze and LawCertificate count
cold summary API          projects bounded runtime facts
gateway control API       joins facts without granting authority
control HTML              renders API values without deriving proof
```

Writer `ON`, transport `PASS`, an active generation, or a V8-compatible reader
does not prove Law #2. Only the durable certification route may change Law #2
from `NOT PROVED` to `PASS`.

## Summary Contract

The cold summary advances to `nando.k1-natural-scheduler-summary.v2` and adds:

```text
exact_wake_status
|- decision
|- blocker
|- readiness_pass_rows
|- exact_unseen_opportunities
|- exact_attempted_deterministic_roots
|- legacy_unbound_terminals
|- trailing_24h_freezes
`- next_eligible_at_unix

active_generation
|- schema
|- generation_sequence
|- consequence_type
|- future_min_sequence
|- selected_at_unix
|- authority_ready
`- phase_mutation_allowed
```

The control API reads current policy state from cold `/health`, not from an old
scheduler report. It accepts summary V1 during deployment compatibility but
marks the exact projection unavailable until V2 and a validated exact wake
status are both present.

Unknown exact counters remain `null` and render as an em dash. They must never
be converted to zero.

## Page Contract

The research section remains four rows:

```text
Exact writer   policy, wake state, immutable generation, V8 floor, exact counts
K1 laws        durable basis, Law #2 status, active discovery blocker
S1C-4          closed natural census result
K2 meaning     next separately preregistered route
```

The page does not show coarse families as laws and does not infer Law #2 from
writer activity. The backend K1 projection supplies `law_2_status` from the
validated durable gate.

## Deployment Boundary

Only these binaries may change:

```text
nando-transition-serving-k1-v8   cold learner only
nando-gateway-control            control API and HTML only
```

Hot serving, Nginx, connector, and the certification authority are protected.
Builds and Rust tests run on the mini-PC with 20 jobs. Deployment must preserve
the Phase B2 policy, ledger prefix, active legacy generation or its linked
terminal outcome, false accepts `0`, and parity failures `0`.

## Structural Check

The first combined worksheet was correctly vetoed because it mixed runtime,
projection, rendering, and proof owners. After splitting by owner route, all
seven changed routes passed independently:

```text
summary writer       PASS
summary generation   PASS
control API          PASS
runtime rendering    PASS
proof rendering      PASS
proof ledger         PASS
claim boundary       PASS
```

These are structural coherence checks only. Tests, live parity, deployment
receipts, and natural certification remain separate authorities.

## Phase D Observation

After deployment, observe ordinary traffic only. No synthetic request may be
created. A bounded observation may end with the active generation still waiting
for independent future; that is a valid operational result, not Law #2.
