# Forbidden Legacy: `.nwrb` Commercial Backend

Date: 2026-07-05

## Decision

The `.nwrb role-binding profiles -> payload builder -> verifier -> catalog`
path is forbidden as a CPU80, commercial offload, market-savings, or future
Nando Wave architecture path.

The active Rust CLI/package/SDK/test path has been removed. Historical reports
may remain as audit evidence only. They must not be extended, promoted, counted
as CPU routability progress, or used as the basis for market claims.

## Why

This path pushed the project away from the core Wave idea:

```text
many relation waves
-> phase / Fourier center of mass
-> compact operator center
-> margin-gated local operator
-> verifier
```

Instead, `.nwrb` made progress look like manual profile accretion:

```text
route-specific role-binding profile
-> route-specific payload builder
-> route-specific verifier
-> route-specific catalog row
```

That is not the North Star. It is slow, profile-heavy, and too easy to confuse
with a collection of narrow heuristics.

## New Allowed Path

The only allowed CPU80 operator backend is the phase-center / phase-action
runtime path:

```text
real traffic
-> action/state atoms
-> phase-center operator package
-> margin / coherence decision
-> deterministic verifier
-> feedback/catalog
```

Rust runtime anchor:

```text
nando_core::PhaseCenterCompiler
nando_core::PhaseCenterFlatRuntime
nando_core::PhaseCenterOffloadRuntime
```

Existing proof anchor:

```text
C32 phase-center runtime
380 flat records
5312 heldout rows
1000/1000
wrong_wins = 0
```

## Hard Rule

Forbidden for future CPU80 work:

```text
.nwrb package as commercial runtime
role-binding profile registry as CPU80 backend
role-binding profile HTTP serving as product path
role-binding real-traffic shadow as savings path
role-binding CPU operator catalog as market-claim authority
new route-specific .nwrb profile work
```

Allowed temporarily:

```text
raw real-traffic trace recording / ingest plumbing
historical reports for comparison
frozen controls that cannot produce CPU80 claims
```

## CLI Guard

`nando-cli` rejects every `role-binding-*` command. The rejection points
operators to the phase-action / phase-center runtime path.

## Claim Boundary

Current `.nwrb` current5k numbers are historical only. They are not an accepted
route toward CPU80 anymore.

Future progress must be measured through phase-center operator packages on
non-synthetic real traffic, with:

```text
verified accepts
incremental reduction vs exact cache
false_accepts = 0
latency / RSS
market_claim_allowed only after real-traffic proof
```
