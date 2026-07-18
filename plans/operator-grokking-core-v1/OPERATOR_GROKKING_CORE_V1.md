# Operator Grokking Core V1

Status: implementation plan, not a completed-proof claim.

Canonical source: `ARCHITECTURE_CANON.md`.

## 1. Objective

Implement the architectural core in which a whole transferable operator
circuit is born through cross-plane phase coherence:

```text
verified partial relation waves
-> CandidateCubeField
-> OperatorGrokkingConsolidator
-> CoherentOperatorCandidate generation g+1
-> frozen future and causal controls
-> exact episodic runtime-authority cleanup
-> ProvenGrokking
-> immutable TernaryOperatorCube + typed operator
-> external admission -> ACTIVE -> CPU
```

The circuit is what groks. Phase coherence is the birth mechanism and order
parameter. The cube stores the crystallized result. Per-cell vote thresholds,
anti-center creation, synthesis success, or lifecycle promotion are not
grokking by themselves.

## 2. Scope And Non-Goals

This plan implements:

- verified partial relation waves from independent surfaces;
- bounded competing whole-circuit hypotheses;
- global cross-plane coherence and circuit margin;
- immutable generational feedback;
- compact operator storage;
- strict proof states for formation, transfer, cleanup, and ablation;
- integration with the existing actor, verifier, future, and admission shell.

This plan does not:

- replace the existing L1/L2 surface Wave;
- make MICRO-12 part of the production hot path;
- introduce a JEPA predictor;
- grant authority from a phase score;
- mutate an ACTIVE generation in place;
- count shadow or potential coverage as CPU savings;
- add source-specific function or field-name rules;
- claim M3 before real verified token savings reach 50%.

## 3. Dependency And Ownership Map

```text
nando-core
  pure Wave domain, circuit consolidation, compact page, no IO
        |
        v
nando-response-actor
  typed receipts, teacher/student adapter, BackwardWave orchestration,
  candidate generation, actor/verifier binding
        |
        v
nando-transition-inducer
  frozen causal experiments and A4-compatible proof battery
        |
        v
nando-transition-serving
  event-driven ingestion, bounded checkpoint, shadow routing only
        |
        v
external admission
  proof validation and immutable ACTIVE registry
```

Proof fixtures cannot become runtime authority. Serving cannot contain the
coherence algorithm. `nando-core` cannot depend on response schemas, files,
clocks, sessions, HTTP, or admission state.

## 4. Canonical Rust Modules

### 4.1 Pure core

Add under `crates/nando-core/src/wave/`:

```text
operator_circuit.rs
  verified fragments, circuit graph, connectivity, canonical identity

candidate_cube_field.rs
  bounded amplitudes, phases, competing circuits, episodic references

operator_grokking.rs
  cross-plane coherence, phase locking, margin, consolidation decision

operator_page.rs
  OperatorPage32, TernaryOperatorCube, dense/sparse codec and validation

operator_grokking_proof.rs
  formation/transfer/cleanup/ablation state machine
```

Do not put all five responsibilities into `phase_center_runtime.rs`.

### 4.2 Response integration

Add under `crates/nando-response-actor/src/`:

```text
verified_delta.rs
  independent typed execution receipts and VerifiedDeltaReceipt

backward_wave.rs
  verified residual -> bounded partial relation waves

operator_generation.rs
  immutable g -> shadow g+1 lifecycle and checkpoint DTOs

transferable_operator_v2.rs
  binds crystallized circuit to actor and verifier contracts
```

Existing `program.rs`, `runtime.rs`, and `verifier.rs` remain execution and
proof components. They do not become the discovery core.

## 5. Core Data Contracts

### 5.1 Verified partial evidence

```rust
struct VerifiedPartialRelationWave {
    receipt_id: ReceiptId,
    surface_id: SurfaceId,
    session_id: SessionId,
    generation: GenerationId,
    relation_edges: BoundedEdges,
    residual_phase: BoundedPhaseVector,
    outcome: VerifiedOutcomeClass,
}
```

Allowed outcomes:

```text
VerifiedPositive
RepeatedApplicabilityNegative
HardContradiction
CensoredUnknown
```

Only independently verified, non-censored outcomes update semantic phase
state. A timeout or unavailable environment never becomes an anti-wave.

### 5.2 Whole circuit

```rust
struct OperatorCircuit {
    roles: BoundedRoleGraph,
    relations: BoundedRelationPlanes,
    transform: BoundedTransformProgram,
    composition: BoundedCompositionDag,
    renderer: BoundedRendererProgram,
}
```

A circuit is valid only when all referenced roles are bound, the graph is
connected, transform dependencies are acyclic, renderer outputs are sourced,
and no relation plane contains a ternary contradiction.

### 5.3 Candidate field

```rust
struct CandidateCubeField {
    generation: GenerationId,
    unresolved_cells: BoundedTernaryField,
    signed_amplitudes: BoundedAmplitudes,
    residual_phases: BoundedPhaseBank,
    competing_circuits: BoundedCircuitBank,
    episodic_refs: BoundedEvidenceRefs,
}
```

All collections are fixed-capacity or explicitly bounded. Overflow produces a
visible blocker; it never silently discards the best or newest circuit.

### 5.4 Consolidation result

```text
Memorizing
  insufficient independent partial evidence; runtime ABSTAIN

CompetingCircuits
  no circuit has the required global coherence margin; runtime ABSTAIN

CoherentOperatorCandidate
  one connected circuit won globally; SHADOW only

Inconsistent
  hard structural contradiction; split/relearn/revoke

Censored
  no semantic update
```

`CoherentOperatorCandidate` is not `ProvenGrokking` and has no execution
authority.

## 6. Consolidation Algorithm

### 6.1 Build hypotheses

For every verified fragment:

1. Canonicalize roles by structural type, cardinality, temporal position, and
   relations, never by field or function name.
2. Insert compatible relation edges into existing circuit hypotheses.
3. Fork only when one fragment admits genuinely different connected laws.
4. Reject or split a hypothesis on a hard ternary contradiction.
5. Retain a bounded best-first version space ranked by evidence coverage and
   phase coherence, not by source identity.

### 6.2 Compute global coherence

For each complete connected circuit, compute one order parameter from all its
active planes:

```text
circuit coherence
= normalized magnitude of the phase-locked cross-plane resultant
* independent-surface coverage
* connected-role coverage
* verified-receipt coverage
```

The exact fixed-point formula is frozen before the causal experiment. It must
use the existing deterministic phase-cell conventions and have an integer hot
path. Magnitude, denominators, and circuit capacity are matched across control
arms.

### 6.3 Select a circuit

Crystallization requires all of:

```text
independent surfaces >= configured proof minimum
independent sessions >= configured proof minimum
no single surface contains the entire circuit
whole circuit is connected and executable
winner coherence >= absolute floor
winner - runner-up >= frozen margin
all evidence has independent verifier lineage
hard contradictions = 0
```

Surface count is only an evidence gate. It cannot itself switch cells or create
an operator.

### 6.4 Abrupt transition

The state machine records the sequence:

```text
surface 1 -> Memorizing
surface 2 -> CompetingCircuits
surface 3+ -> CoherentOperatorCandidate
```

The transition is accepted only when the whole-circuit order parameter crosses
the frozen margin while matched competitors remain tied or collapse. A smooth
per-cell accumulation with no circuit transition is ordinary online learning.

## 7. BackwardWave And Generation Firewall

The actor and independent verifier emit typed receipts:

```text
RoleBindingReceipt
RelationEvaluationReceipt
TransformReceipt
CompositionReceipt
RendererReceipt
VerifierReceipt
        |
        v
VerifiedDeltaReceipt
```

`BackwardWave` converts only a valid `VerifiedDeltaReceipt` into signed partial
relation waves. It cannot read an unverified teacher claim as production truth.

```text
ACTIVE generation g                   immutable
-> verified feedback accumulator
-> CandidateCubeField g+1             mutable, bounded, cold/shadow
-> CoherentOperatorCandidate g+1      immutable candidate
-> replay + frozen future + ablation
-> external admission
-> ACTIVE generation g+1              immutable
```

Hard contradiction never flips a core semantic cell directly from `+1` to
`-1`. It invalidates, splits, or relearns the candidate. Direct opposition is
allowed mainly in the applicability plane after repeated independent evidence.

## 8. Compact Storage

### 8.1 Hot page

```text
OperatorPage32                  4032 B
+-- Header64                      64 B
+-- PhaseProfile1024            1024 B
+-- StructuralRoles512           512 B
+-- TernaryCube2048             2048 B
+-- TransformProgram128          128 B
+-- CompositionDag128            128 B
`-- RendererProgram128           128 B
```

The page contains only runtime structural state. Full evidence, exact episodes,
verifier receipts, and provenance remain cold and content-addressed.

### 8.2 Ternary semantics

```text
+1  relation supported
 0  unresolved or absent
-1  relation opposed or forbidden
11  reserved encoding -> package rejected
```

Direction is encoded by role order, never by sign. Multi-role computation is
stored in typed transform instructions, not approximated by pairwise cells.

### 8.3 C64

C64 uses a plane bitmap plus packed 16-byte 8x8 ternary tiles. Dense and sparse
forms must decode to byte-identical canonical circuits and decisions.

### 8.4 Checkpoint

The mutable candidate checkpoint stores bounded accumulators and receipt IDs,
not raw traces. Restart must produce the same field, winner, and decision.
Normal startup never scans unbounded history.

## 9. Proof State

Use a type-state progression that prevents premature claims:

```text
Unformed
-> CoherentCandidate
-> FutureVerified
-> CausallyVerified
-> ExactAuthorityCleaned
-> ProvenGrokking
```

`ProvenGrokking` requires:

1. no individual example contained the complete circuit;
2. fragments came from independent surfaces;
3. a whole connected circuit obtained an abrupt coherence margin;
4. frozen-future transfer passed;
5. exact episodic runtime authority was removed and transfer stayed identical;
6. full phase passed;
7. no phase, shuffled residual, magnitude only, and matched random center tied
   or abstained;
8. restoring phase restored the same circuit and transfer;
9. exact lookup overlap was zero;
10. false accepts and runtime parity mismatches were zero.

The proof object has no method that grants runtime authority. Admission remains
external.

## 10. Implementation Phases

### Phase 0: Freeze baseline and vocabulary

Changes:

- record current A4 result and live runtime baseline;
- freeze canonical terminology and outcome classes;
- identify the existing phase math reused by the new core;
- prohibit unrelated refactoring during implementation.

Gate:

```text
baseline artifact exists
current false accepts = 0
current parity mismatches = 0
no runtime behavior changed
```

### Phase 1: Pure circuit and field types

Implement `OperatorCircuit`, canonical role ordering, connectivity validation,
bounded `CandidateCubeField`, and deterministic circuit hashing.

Gate:

```text
renamed surfaces -> identical circuit hash
permuted roles -> identical canonical circuit
disconnected graph -> rejected
capacity overflow -> explicit blocker
```

### Phase 2: OperatorGrokkingConsolidator

Implement compatible-circuit construction, global phase resultant, coherence,
runner-up margin, and explicit decisions.

Gate:

```text
surface A -> Memorizing
surface B -> CompetingCircuits
surface C+ -> one CoherentOperatorCandidate
per-cell threshold cannot produce candidate
```

### Phase 3: OperatorPage32 codec

Implement balanced-ternary packing, compact roles, transform instructions,
composition, renderer, header hashes, and strict decoder.

Gate:

```text
encoded C32 page <= 4096 B
canonical target = 4032 B
roundtrip byte-identical
reserved 11 rejected
dense/sparse parity exact
```

### Phase 4: Typed receipts and BackwardWave

Add the independent receipt chain and translate verified residuals into core
partial waves. Classify positive, applicability negative, contradiction, and
censored outcomes separately.

Gate:

```text
unverified outcome -> no update
censored outcome -> no semantic update
repeatable applicability negative -> bounded counter-wave
hard contradiction -> split/relearn/revoke, never silent anti-center
```

### Phase 5: TransferableOperatorV2

Bind a coherent circuit to existing actor and independent verifier contracts.
Do not replace actor/verifier semantics with cube lookup.

Gate:

```text
project/status/count/filter/compose represented without source-name authority
actor output equals independent verifier reconstruction
ambiguous role binding -> ABSTAIN
```

### Phase 6: Generation firewall and checkpoint

Integrate immutable generations into the streaming miner. Existing ACTIVE
packages continue serving while g+1 learns in shadow.

Gate:

```text
ACTIVE bytes never mutate
restart gives identical candidate field and decision
checkpoint is bounded
ordinary startup performs no history rescan
```

### Phase 7: Matched causal proof

Port the A4 ladder to the new circuit core and run five frozen contours:

```text
A old operator baseline
B rich oracle without BackwardWave
C rich induced without BackwardWave
D rich induced with per-cell BackwardWave control
E rich induced with BackwardWave + OperatorGrokkingConsolidator
```

Within E run full/no/shuffled/magnitude/random/restore controls with equal
version-space and compute budgets.

Gate:

```text
only full phase forms and transfers the correct circuit
restore phase reproduces its hash
exact episodic authority cleanup preserves transfer
wrong accepts = 0
```

### Phase 8: Live shadow integration

Feed completed live traces through the existing teacher/student path. Candidate
circuits remain shadow-only until independent future and causal proof pass.

Gate:

```text
live CandidateCubeField grows event by event
at least one ordinary CoherentOperatorCandidate forms
every candidate has one emitted or explicit-blocked outcome
no silent evidence loss
```

### Phase 9: External admission and CPU execution

Package a proven g+1 with circuit, typed actor, verifier binding, proof hashes,
and economics metadata. Run the mandatory composite gate before authority.

Gate:

```text
nando-live-transition-gate = PASS
support/future/receipts complete
false accepts = 0
runtime parity mismatches = 0
first ordinary non-wait ACTIVE operator executes on CPU
```

### Phase 10: Coverage expansion to M3

Rank unresolved live traffic by verified executable token opportunity. Extend
relation/transform capacity only when a real high-value class cannot be
represented by the existing operator core.

Final product gate:

```text
verified input-token saving share >= 50%
in three consecutive independent windows
economics hard gate = YES
Product M3 = YES
false accepts = 0
runtime parity mismatches = 0
```

If 50% is impossible for the observed traffic, publish a complete token-weighted
decidability partition and a verified upper bound. Potential coverage is never
reported as actual savings.

## 11. Focused Verification Order

During implementation, use the smallest check that can invalidate the current
phase:

```text
1. nando-core operator-circuit unit tests
2. nando-core consolidator causal tests
3. nando-response-actor receipt and generation tests
4. nando-transition-inducer matched A4 proof
5. one workspace release build
6. mandatory nando-live-transition-gate
7. live runtime and economics verification
```

Do not run a workspace-wide heavy suite after every edit. Record wall time for
every command and announce commands expected to exceed 60 seconds.

## 12. Budgets

```text
OperatorPage32                         <= 4096 B (target 4032 B)
hot-path heap allocations             0 after worker initialization
candidate circuits per field          fixed configured maximum
partial waves and evidence refs        fixed configured maximum
runtime decision complexity           bounded by active page count and top-k
normal startup history scan            0 rows
unbounded raw payloads in checkpoint   0
false accepts                          0
runtime parity mismatches              0
```

Every overflow, dropped fragment, rejected circuit, failed receipt, and blocked
candidate gets an explicit counter and reason.

## 13. Commit Sequence

Keep each algorithmic change isolated:

```text
1. core circuit types and canonicalization
2. candidate field and bounded persistence DTO
3. whole-circuit consolidator
4. OperatorPage32 codec
5. typed receipt chain
6. BackwardWave adapter
7. TransferableOperatorV2 binding
8. generation firewall
9. causal proof harness
10. live shadow integration
11. external admission integration
12. economics and dashboard truth fields
```

Do not combine a scoring change with a storage refactor or serving deployment.
Each commit must preserve the previous safety shell and name its proof scope.

## 14. Completion Definition

The architectural core is complete only when one automatically induced whole
operator circuit:

```text
forms from partial independent surfaces
-> wins by cross-plane phase coherence
-> transfers to frozen future
-> survives exact episodic runtime-authority cleanup
-> fails under matched phase ablations
-> roundtrips through OperatorPage32
-> becomes immutable g+1
-> passes independent admission
-> executes an ordinary live request on CPU
```

The product is complete only at the separate M3 gate. Neither a compiled cube,
a passing laboratory test, nor an ACTIVE wait operator is the final result.
