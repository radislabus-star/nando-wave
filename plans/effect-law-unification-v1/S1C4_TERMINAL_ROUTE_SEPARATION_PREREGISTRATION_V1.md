# S1C-4 Terminal Route Separation Preregistration V1

Status: `FROZEN BEFORE IMPLEMENTATION / DISPLAY AND HEALTH SEMANTICS ONLY`

Date: `2026-08-13`

## Plain-Language Problem

The cold learner currently reports
`NO_ROUTE_BOUND_TOPOLOGY_FRAME_TERMINAL_LINK` even though the route that owns
that condition, legacy MS3, is intentionally disabled with
`NANDO_MULTI_SOURCE_RESEARCH_ENABLED=0`.

This is not the active K1 scheduler blocker and it is not the S1C-4 result:

```text
authenticated remote evidence       11,822 route-bound frames and growing
legacy MS3 freezer                  disabled by configuration
active K1 scheduler                 waiting_for_evidence / ready_now 0
S1C-4 natural census                TERMINAL / EMPTY_GOAL_SURFACE
S1C-4 exact denominator             1,024 / 1,024 classified
S1C-4 exact pre-action goals        0
```

The existing health label collapses these three routes and makes an inactive
legacy route look like a live evidence failure.

## Frozen Change

The implementation may change only:

1. the cold `/health` compatibility projection for the disabled legacy MS3
   route;
2. the compact control dashboard projection and wording;
3. focused tests and the paper/live receipt for this slice.

When legacy MS3 is disabled, its blocker becomes
`LEGACY_MS3_RESEARCH_DISABLED`. The health projection must expose the route as
legacy and disabled while preserving the existing compatibility fields.

When legacy MS3 is enabled, the existing exact blocker precedence remains:

```text
ready
-> null
no source
-> NO_LIVE_POST_ACTION_EVIDENCE_SOURCE
no route-bound remote frame
-> NO_ROUTE_BOUND_REMOTE_EVIDENCE
no route-bound frozen generation
-> NO_ROUTE_BOUND_TOPOLOGY_FRAME_TERMINAL_LINK
otherwise
-> LIVE_POST_ACTION_EVIDENCE_NOT_YET_VERIFIED
```

## Dashboard Contract

The dashboard must keep three independent rows:

```text
K1 discovery
-> LawCertificate progress plus active scheduler state and blocker

S1C-4 evidence-surface test
-> terminal verdict, exact denominator, exact pre-action goals

K2 grounded meaning
-> CLOSED after EMPTY_GOAL_SURFACE
-> next requirement is a separately preregistered goal-bearing environment
```

`EMPTY_GOAL_SURFACE` must be explained in plain language: all 1,024 ordinary
requests were captured and joined, but none carried an exact machine-readable
goal before action. Waiting longer cannot change that immutable window.

## Forbidden Effects

```text
legacy MS3 activation                  forbidden
K1 scheduler state mutation           forbidden
S1C-4 report rewrite or reopen         forbidden
retrospective goal injection           forbidden
generated or targeted LLM traffic      forbidden
Law #2 promotion                       forbidden
K2 authority                           forbidden
model training                         forbidden
phase mutation                         forbidden
package activation                     forbidden
hot serving or Nginx restart           forbidden
connector restart                      forbidden
```

## Preservation And Tests

The immutable S1C-4 terminal report, K1 scheduler journal, remote evidence
spool, topology archive, terminal archive, certification ledger, economics,
and connector receipts are read-only inputs. Deployment rollback must preserve
their exact existing bytes and every naturally arriving append-only suffix.

Required tests:

```text
legacy disabled -> LEGACY_MS3_RESEARCH_DISABLED
legacy enabled -> existing blocker matrix unchanged
route-bound evidence counts remain factual and separate
dashboard renders K1, S1C-4, and K2 as three rows
EMPTY_GOAL_SURFACE explanation includes 1,024 denominator and zero goals
dashboard invalid report remains unavailable
transition-serving and gateway-control tests
strict Clippy and fmt
live cold-health projection parity
S1C-4 report SHA-256 unchanged
K1 scheduler summary root unchanged across scoped restart
Nginx and connector PIDs unchanged
false accepts and parity failures remain zero
```

## Claim Boundary

This slice can prove only that health and dashboard semantics match the already
durable route state. It cannot prove a new law, a grounded meaning, answer
quality, learning authority, or phase causality.
