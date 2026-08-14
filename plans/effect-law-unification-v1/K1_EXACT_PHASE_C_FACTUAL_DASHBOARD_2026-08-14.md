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

## Live Deployment Result

Source commit `c223f6f162dc73cc19bf643c3de66f3fe52fe48e` is deployed for the
cold learner and control plane. The authoritative deployment receipt is:

```text
/var/lib/nando-wave/deployments/20260814T090529Z-c223f6f-phase-c/deployment-receipt.json
receipt root  5fa8fc009be2bf1b17598e64a71cbfc363b6c26e08eb076ad204e07715ce39c0
mode          0400
```

The root was independently recomputed after installation. Installed and
release hashes match:

```text
cold     4bffd556aa0833e4e2434f05de022646533dc58c841b6f6ee6b54ba3ae82c99c
control  74ed9ecbecabeb1e282aaf1bc2e9cf46f5b8b117704e5d059c7185aabcc2625a
```

Live acceptance:

```text
policy writer                           ON
policy root                             e1c87b4a...13c53
summary                                 V2
dashboard                               2026.08.14-control-v23
exact projection                        available
legacy generation                       606 preserved active
legacy freeze root                      27d823aa...1fc0
ledger revision                         1216 -> 1216
V8 freezes                              0 -> 0
Law certificates                        1
Law #2                                  NOT PROVED
false accepts / parity failures         0 / 0
composite gate                          PASS
```

Protected PIDs remained `hot 1816591`, `authority 150005`, `Nginx 682430`,
and local connector `2919`. Changed PIDs are `cold 298492` and `control
298415`; all service restart counters remain zero.

## First Attempt Failure Analysis

The first transaction at
`/var/lib/nando-wave/deployments/20260814T084104Z-c223f6f-phase-c` rolled back
both changed binaries and preserved the policy and state. Its abort receipt root
is `f0abcefe40e2b245e69c8a7b95e81b358fc20c4f409d98954332c760e267af3a`.

The rollback preserved generation `606`, freeze root `27d823aa...1fc0`, ledger
revision `1216`, and V8 count `0`. The process had sufficient memory, no crash,
no journal error, and no restart. The same release bytes, policy, state, and
acceptance predicates passed on the traced transaction.

The exact failed shell predicate cannot be recovered because abort receipt V1
recorded only the generic transaction reason and deleted the temporary response
files. Therefore the first failure is classified as a non-reproduced transient
deployment check, not as a proven code or state defect. Future transactional
harnesses must persist `failure_stage`, the bounded failed response, and its
hash before rollback; a generic abort reason is insufficient for root-cause
authority.

## P13 Natural Observation

The post-deployment natural-only observation receipt is:

```text
/var/lib/nando-wave/deployments/20260814T090529Z-c223f6f-phase-c/evidence/p13-natural-observation-v1.json
receipt root  9a476d3c5d53215c00caa8a738130dd416f5322a4b1533c57710bba6f010cef3
mode          0400
```

The independently recomputed root matches. During the bounded 120-second
window, the observer issued GET requests only and no synthetic model request.
No ordinary LLM request arrived in that window. Generation `606` and its root
remained active and immutable, V8 delta stayed zero, LawCertificate count stayed
`1`, false accepts and parity failures stayed `0 / 0`, and all PIDs survived.

This is an operational PASS with an empty natural traffic window. It neither
proves nor refutes Law #2.

## Browser Acceptance

The existing managed Chrome tab was refreshed after deployment. Desktop and
mobile checks passed with no horizontal overflow, no out-of-viewport elements,
and no JavaScript errors. The original `1536 x 769` viewport was restored, no
new tab was created, and the refreshed Nando control tab remains active.
