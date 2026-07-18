# Operator Circuit Synthesis V1

## Goal

Close the missing bridge between verified partial relation waves and whole
operator circuits without weakening the canonical grokking claim.

```text
verified support waves
-> relation fragments
-> bounded connected circuit synthesis
-> freeze topology and anchors
-> disjoint future waves
-> phase-coherent whole-circuit consolidation
```

The target circuit must not be supplied through `register_circuit`. It must be
absent before support ingestion and constructed from fragments that no single
surface contains in full.

## Non-negotiable boundary

Circuit synthesis and circuit proof use disjoint evidence partitions inside
the same immutable source operator generation `g`.

```text
support partition S at operator generation g
  infers structural cells, alternatives, and phase anchors

frozen candidate set
  immutable after the boundary

future partition F at operator generation g
  ranks frozen candidates and supplies transfer proof

proven candidate
  becomes operator generation g+1
```

Reusing S as F is circular evidence and is a FAIL. A latent predictor may rank
future synthesis probes later, but it never creates verified evidence and never
grants authority.

## Modules

### RelationFragmentGenerator

Input: bounded `VerifiedPartialRelationWave` support rows.

Output: source-neutral fragments containing relation cell, ternary state,
phase, receipt, surface, and session identity.

Only positive verified outcomes may propose topology. Applicability negatives
shape routing, hard contradictions trigger repair/split/revoke, and censored
outcomes remain unknown.

### CircuitSynthesizer

The V1 version space is finite:

```text
one structural cell
x one observed ternary alternative
x one normalized support phase anchor
x dependency-closed connected graph
```

It enumerates deterministic state assignments for a connected structural graph,
rejects non-canonical role spaces, deduplicates circuit fingerprints, and stops
at the configured circuit budget. No function names, field names, response
labels, or future actions are semantic authority.

### Frozen proof handoff

Synthesized circuits are registered into a fresh `CandidateCubeField`. Only
future waves enter that field. `OperatorGrokkingConsolidator` remains the sole
component that can produce a coherent candidate, and that candidate still has
no execution authority.

## Accounting

```text
positive samples
= emitted fragments + unresolved fragments + zero-phase fragments

synthesis attempts
= emitted circuits + duplicate circuits + blocked circuits + truncated circuits
```

Every blocker is explicit in `OperatorCircuitSynthesisReport`.

## Proof ladder

1. The target topology is absent from the initial candidate set.
2. Three support surfaces each reveal only part of the law.
3. The synthesizer constructs the connected target circuit.
4. Candidate topology and anchors are frozen.
5. Three new future surfaces produce a coherent candidate.
6. No-phase, shuffled-phase, magnitude-only, and matched-random controls fail.
7. Exact support memory is dropped and future transfer remains.
8. False accepts and parity mismatches remain zero.

## Status

```text
canonical contract                         RECORDED
RelationFragmentGenerator                  PASS
bounded CircuitSynthesizer                 PASS
target-absent construction test            PASS
support receipt reuse firewall             PASS
disjoint future causal proof               PASS (laboratory)
generation firewall integration            PASS (library path)
live stage receipt integration             BLOCK
streaming checkpoint of frozen set         BLOCK
external admission of synthesized page     BLOCK
first ordinary live CPU accept              BLOCK
```

## Implemented evidence

```text
crates/nando-core/src/wave/operator_circuit_synthesis.rs
crates/nando-core/tests/operator_circuit_synthesis_causal.rs
crates/nando-response-actor/src/backward_wave.rs
crates/nando-response-actor/src/operator_generation.rs
```

Focused proof results:

```text
fragment/synthesis unit tests                         2/2 PASS
target-absent disjoint-future causal test             1/1 PASS
autonomous generation-firewall path                   1/1 PASS
```

The causal test constructs a circuit from three partial support surfaces,
drops support memory, and evaluates a frozen candidate set on three different
future surfaces. Full and restored phase form the same circuit. No-phase,
shuffled-phase, magnitude-only, and fixed random-center controls produce no
candidate.

This is not yet an organic live-grokking or product claim. No production binary
was deployed by this phase.
