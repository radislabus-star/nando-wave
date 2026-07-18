# Local-Role Operator Blueprints V1

## Objective

Replace global-role fragment union with bounded synthesis over independently
local surface graphs.

```text
SurfaceFragmentBundle
-> structural role colors
-> competing RoleAlignmentHypothesis values
-> bounded CircuitBeam
-> CandidateOperatorBlueprint values
-> FrozenOperatorBlueprintSet
-> independent future coherence
```

The operator circuit groks through cross-plane phase coherence. Role alignment
and beam synthesis only construct the competing version space.

## Input contract

```rust
struct SurfaceFragmentBundle {
    lineage_sha256: [u8; 32],
    surface_sha256: [u8; 32],
    roles: Box<[StructuralRoleSignature]>,
    relations: Box<[LocalRelationFragment]>,
    program_atoms: Box<[TypedProgramAtom]>,
}
```

Role IDs are indexes local to one bundle. Field names, function names, manual
family labels, and future runtime actions are not part of structural identity.
The full lineage is produced by trusted canonical evidence code.

`StructuralRoleSignature` includes type, cardinality class, temporal position,
structural constraints, and sorted neighboring relation planes. Two or three
deterministic graph-color refinement rounds produce alignment candidates.
Symmetric colors preserve all bounded partial bijections; they are not resolved
by arbitrary ordering.

## Bounded circuit beam

```text
seed             one verified relation
expand           only dependency-connected relation/program atom
branch           local-role alignment, ternary state, typed transform
deduplicate      canonical blueprint fingerprint
retain           at most 64 incomplete blueprints
depth            at most 12
expansions       at most 4096
budget outcome   ABSTAIN/BUDGET_EXHAUSTED
```

Caller configuration may lower these limits but cannot raise them.

## Blueprint contract

```rust
struct CandidateOperatorBlueprint {
    role_graph: RoleGraph,
    relation_program: RelationProgram,
    transform_program: Box<[TransformOp8]>,
    composition_dag: CompositionDag,
    renderer_hypothesis: RendererContract,
    verifier_contract: VerifierContract,
    phase_anchors: Box<[PhaseCenterCell]>,
}
```

Transform and renderer candidates are imported from the existing bounded
version space. The verifier contract is only a proposed obligation set; an
independent verifier remains the authority boundary.

## Freeze contract

Support builds and freezes X/Y/Z. It does not rank them to authority.

`FrozenOperatorBlueprintSet` commits:

```text
support lineage SHA-256 values
candidate-set SHA-256
canonicalizer version
synthesis configuration
source operator generation
```

Future evidence is rejected when any full lineage commitment is already in the
support set. Receipt identity alone is insufficient. Frozen candidates and
anchors cannot mutate during future scoring.

## Coherence contract

Each lineage contributes at most one phase sample to each edge.

```text
edge coherence
-> minimum mandatory-edge coverage
-> geometric mean over mandatory edges
-> cross-plane closure
-> whole-circuit coherence and runner-up margin
```

Crystallization requires all mandatory planes. No-phase, shuffled-phase,
magnitude-only, and matched-random controls must tie or abstain.

## Implementation sequence

1. Add bounded local bundle, role signature, relation, and program-atom types.
2. Add deterministic graph-color refinement.
3. Enumerate bounded partial-bijection role alignments.
4. Build dependency-closed competing blueprint beam.
5. Reuse current typed transform/renderer version-space representations.
6. Freeze full SHA-256 provenance and candidate-set commitment.
7. Bind independent future bundles to every frozen blueprint.
8. Add X/Y/Z causal proof with target absent from initial candidates.
9. Connect verified live receipts only after the pure proof passes.

## Current boundary

```text
canonical design                         RECORDED
local bundle and role signatures         PASS
three-round structural role colors       PASS
bounded role alignment                   PASS
competing blueprint beam                 PASS
silent alignment/beam truncation         FIXED: incomplete -> freeze denied
full-lineage frozen future               PASS (laboratory)
X/Y/Z phase causal proof                 PASS (laboratory)
no/shuffled/magnitude/random controls     ABSTAIN
focused nando-core Clippy                 PASS
live integration                         BLOCK
external admission                       BLOCK
ordinary CPU economics                   BLOCK
```

Exact capability boundary:

```text
REAL
  local SurfaceFragmentBundle
  -> competing relation/phase blueprints
  -> frozen set
  -> future coherence
  -> winner fingerprint

PARTIAL
  TransformProgram
  CompositionDag

PLACEHOLDER COMMITMENTS
  RendererContract
  VerifierContract

ABSENT
  executable CrystallizedOperator
  OperatorPage32 compilation
  actor/verifier binding
  external admission
  CPU execution
```

Implemented files:

```text
crates/nando-core/src/wave/operator_blueprint.rs
crates/nando-core/tests/operator_blueprint_causal.rs
```

The causal proof uses six support surfaces. Every support surface contains one
local relation only, local role numbering is permuted, and the bounded beam
constructs multiple complete relation/phase blueprints. `program_atoms` is
empty in this proof, so it proves relation-circuit formation rather than a full
action program. Support memory is dropped after the candidate-set freeze.
Three new full-lineage future surfaces select one circuit under full phase;
no-phase, shuffled-phase, magnitude-only, and matched-random controls produce
no winner.

Role ambiguity and future phase selection are currently separate proofs. The
causal fixture uses structurally easy role signatures; a single combined proof
with symmetric role ambiguity remains open. This proves the bounded pure-core
relation mechanism, not organic live discovery, complete action synthesis, or
runtime authority.
