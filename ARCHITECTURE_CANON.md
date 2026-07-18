# Nando Wave Architecture Canon

Status: canonical architectural contract.

Audience: every human or AI agent changing this repository.

This file exists to prevent a locally reasonable refactor from destroying the
project's actual mechanism. Read it before editing discovery, learning, Wave,
actor, verifier, admission, serving, storage, or economics code.

## WARNING: Do Not Replace Grokking With A Table

This distinction is critical. Incorrect language here leads directly to an
incorrect architecture.

```text
what groks:       the whole connected operator circuit
birth mechanism: cross-plane phase coherence and phase locking
order parameter: circuit coherence and its margin over competing circuits
stored result:    crystallized TernaryOperatorCube + typed operator program
```

The phase does not "grok" by itself. The cube does not "grok" by itself. The
operator circuit groks **through** phase coherence:

```text
partial relation waves from independent surfaces
-> agreement across surfaces, roles, and relation planes
-> phase locking
-> abrupt circuit-coherence transition
-> one connected typed operator
-> transfer to frozen future
-> exact episodic memory cleanup
```

Required wording:

```text
Произошёл фазовый переход к когерентному операторному контуру.

Коротко:
Оператор грокнулся через межплоскостную фазовую когерентность.

The operator underwent a phase transition into a coherent circuit.

Short form:
The operator grokked through cross-plane phase coherence.
```

Forbidden claims:

```text
"coherence grokked"
"the cube grokked"
"three votes switched a cell, therefore grokking"
"the score crossed a threshold, therefore grokking"
"an anti-center appeared, therefore grokking"
"the package became ACTIVE, therefore grokking"
```

Those events may be ordinary learning, storage, routing, or lifecycle progress.
They are not sufficient evidence of grokking.

A grokking claim is allowed only when all of these are demonstrated:

1. No individual example contains the complete operator circuit.
2. Correct fragments align across independent surfaces or repositories.
3. One circuit obtains an abrupt coherence margin over matched competitors.
4. The resulting typed operator transfers to a new frozen surface.
5. Exact episodic runtime authority is removed and transfer remains unchanged.
6. No-phase, shuffled-phase/residual, magnitude-only, and matched-random controls
   destroy circuit formation or force a tie/`ABSTAIN`.
7. An independent verifier confirms execution with `false_accepts = 0`.

Every agent must correct this terminology immediately when the user, another
agent, documentation, or the agent itself uses it incorrectly. Politeness must
not preserve a technically false grokking claim. Without this boundary, Nando
Wave becomes a rule table that imitates a thinking wave machine.

## 1. What This Project Is

Nando Wave is a cache-conscious wave memory that learns transferable actions
from completed live traces and executes only independently verified actions on
CPU.

It is not primarily:

- a collection of hand-written profiles;
- a template selector;
- a DSL enumerator;
- a response cache;
- a conventional classifier with Wave terminology added afterward.

The central mechanism is:

```text
repeated signal
-> signed interference
-> competing partial operator circuits
-> globally coherent circuit formation
-> stable center of mass / phase center
-> exact-memory cleanup
-> transferable action law stored as a compact operator page
-> verifier-safe CPU execution
```

The Situn, Fourier, interference, lens, carrier, and counter-wave language is
not decoration. It records the design intuition: many small signed signals are
accumulated into a compact field; coherent structure reinforces itself, noise
is cancelled, and the stable result is stored as a wave center rather than as
a list of examples. The implementation is more exact than this intuition, but
a replacement that removes the signed field, phase, interference, or compact
center has changed the project, not refactored it.

## 2. The L1 -> L2 -> L3 Meaning

### L1: surface coherence

L1 converts text surfaces into a compact wave:

```text
byte 4-grams
+ boundary atoms
+ identities for short tokens
+ service-word atoms
-> signed ternary lane contributions
-> SurfaceWave4096
```

L1 does not understand an operator. It makes differently written observations
comparable and lets character fragments cohere into stable lexical surfaces.
The hot representation is deliberately small and sparse.

Current primary implementation:

```text
crates/nando-core/src/wave/surface_wave.rs
```

### L2: motifs, interference, and context

L2 combines active L1 lanes into reusable motifs and contextual centers.
Interference reinforces compatible motifs. Lens/carrier context constrains the
field. Counter-wave evidence suppresses noise and false attraction.

L2 is not a bag of labels. It is the intermediate structure that allows new
wording and layouts to reach the same law without exact lookup.

Relevant design and implementation references:

```text
docs/SYMBOL_CELL8_ARCHITECTURE.md
docs/NANDA_WAVE_THEOREM.md
crates/nando-core/src/wave/semantic_wave.rs
crates/nando-core/src/wave/semantic_extract.rs
```

### L3: transferable action

L3 binds roles, state transitions, and action structure. Here repeated
completed transitions can stabilize into a transferable action operator:

```text
same law across different surfaces
-> role/action centers
-> positive phase center
-> applicability subcenters and anti-centers
-> compact transferable operator
```

In this project, grokking means that examples no longer have to be retained as
the authority: their common law has stabilized into a compact center. At L3,
the important result is a transferable action, including its applicability
boundary, not merely a recognized phrase.

Relevant implementation:

```text
crates/nando-core/src/wave/l3_semantic_grokking.rs
crates/nando-core/src/wave/phase_center_runtime.rs
crates/nando-response-actor/src/online_subcenter.rs
crates/nando-response-actor/src/cegis.rs
```

### Optional JEPA-inspired research layer, not the system core

The phrase "hidden part" in the product architecture refers to a compact
JEPA-inspired latent world model, not to hidden Wave memory and not to another
name for L3 phase centers.

This is an optional future research layer. It is not required for extracting
repeatable operators from completed LLM traffic, is not part of the production
hot path, and must not block the CPU-coverage goal. The canonical core is the
bidirectional transferable Wave operator defined below.

Its conceptual contract is:

```text
current structural state
-> encoder -> latent state z_t

z_t + candidate action
-> action-conditioned predictor
-> predicted latent consequence z_t+1
```

For this repository the latent state is expected to describe structured
software/runtime consequences, such as diagnostic graphs, AST relations,
state deltas, or tool outcomes. It does not reconstruct a full textual world.
It predicts which candidate actions are promising before an expensive probe or
execution.

This layer has no authority to execute. A latent prediction is search evidence,
not truth. A real actor and independent verifier must still evaluate the
consequence. Prediction error updates latent dynamics; it must not be silently
converted into an applicability anti-center.

The existing file:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hidden_state.rs
```

does **not** by itself implement this JEPA-like contract. It derives bounded
cross-layer and combination atoms for subcenter/quarantine refinement. Its
historical `hidden_state` name must not be used as evidence that an
action-conditioned latent consequence predictor already exists.

Primary conceptual references:

```text
https://ai.meta.com/blog/v-jepa-2-world-model-benchmarks/
https://arxiv.org/abs/2506.09985
```

## 3. Canonical System Core: Bidirectional Transferable Operator

The core of the whole system is a generational, bidirectional Wave operator:

```text
state / observation
-> ForwardWave
-> transferable law instance
-> actor execution
-> independent typed receipts
-> VerifiedDeltaReceipt
-> BackwardWave
-> CandidateCubeField
-> OperatorGrokkingConsolidator
-> candidate generation g+1
```

The symbolic representation expresses a law. `ForwardWave` transfers that law
to the current structural state. `BackwardWave` converts independently verified
consequences into a bounded change of the operator field. Neither direction may
bypass independent proof.

The frozen cube is not the mechanism that groks. It is the compact crystallized
memory produced after circuit formation. Grokking occurs in the candidate field
when several incomplete, competing residual waves become one globally coherent
operator circuit.

### TransferableOperatorV2

The canonical operator representation is:

```text
TransferableOperatorV2
+-- RoleGraph
|   +-- structural roles
|   `-- binding constraints
+-- RelationProgram
|   +-- equality / delta / cardinality
|   +-- selection
|   `-- frame preservation
+-- TypedTransform
|   +-- projection
|   +-- computation
|   +-- filtering
|   `-- aggregation
+-- CompositionDag
|   `-- ordered dependent transformations
+-- RendererContract
|   `-- response form without field-name or exact-surface authority
+-- VerifierContract
+-- ForwardWave
`-- BackwardWave
```

This representation must be rich enough for one law to combine independent
surfaces that currently fragment into separate exact programs. The miner finds
repeated evidence; the operator is responsible for expressing and transferring
the law.

### Setun balanced ternary and the operator-state tensor

Balanced ternary is a foundational representation, not an L1-only hashing
detail. The canonical local state alphabet is:

```text
-1  relation opposed or forbidden
 0  relation absent
+1  relation supported
```

This is the Setun-inspired part of the architecture: zero is a first-class
state, while supported and opposed relations are symmetric signed states. The
axes define direction: reversing a relation swaps `output_role` and
`source_role`; it does not negate the cell. Wave accumulation may maintain
bounded strength, energy, real/imaginary phase, and coherence around this
ternary relation skeleton; those statistics do not erase the underlying
`-1 / 0 / +1` semantics.

A complete transferable operator cannot be stored only as one C32 or C64
phase-center record. Those records are compact phase profiles:

```text
C32 ~= 1024 bytes per record
C64 ~= 2048 bytes per record
```

They recognize and score an operator field, but they do not encode every role
binding and state transformation. The full operator memory is layered:

```text
TransferableOperatorMemory
+-- C32/C64 phase profile
+-- mode / transition / interference state
+-- role and slot center pages
+-- operator-pair action matrix
`-- TernaryOperatorCube
    `-- relation_plane x output_role x source_role -> ternary relation state
```

Conceptually, `TernaryOperatorCube` is a three-dimensional relational field of
operator state. Relation kind is a real axis; sign is the value stored in the
cell, not another semantic axis. Its canonical two-bit encoding is:

```text
00 =  0
01 = +1
10 = -1
11 = reserved and invalid in an admitted package
```

The cube is nonlinear memory and applicability structure. It does not replace
the typed transform program. Pairwise role relations alone cannot express a
multi-role computation such as filtering a collection by a predicate and then
aggregating the selected values.

The historical operator runtime represented this tensor as:

```rust
HashMap<(action_center, output_slot_id, source_slot_id, sign_key), i16>
```

where the sparse key selected action, output role, source role, and polarity,
and the `i16` value accumulated learned binding strength. It was compiled into
a flat role-binding table for runtime. This historical form is evidence of the
required relation shape, not a mandate to restore the same `HashMap` storage.

The current HEAD no longer contains `state_delta_role_binding_edges` in
`WavePredictorHebbianField`. Therefore the operator-state tensor is an explicit
restoration obligation for `TransferableOperatorV2`, not a component that may
be assumed present because C32/C64 phase scoring still exists.

### Compact roles and typed transforms

The hot role table uses structural identity only:

```rust
struct StructuralRole16 {
    type_class: u8,
    cardinality_class: u8,
    temporal_class: u8,
    relation_flags: u8,
    phase_center: u16,
    selector_center: u16,
    constraint_mask: u32,
    role_signature_hash: u32,
}
```

`role_signature_hash` is derived only from type, cardinality, relations,
temporal position, and structural constraints. Repository identity, source
surface, field names, and provenance are forbidden inputs. Provenance remains
in cold proof receipts.

Exact multi-role semantics is executed by a bounded typed block:

```rust
struct TransformOp8 {
    opcode: u8,
    output: u8,
    source_a: u8,
    source_b: u8,
    parameter: u16,
    flags: u16,
}
```

Sixteen operations require 128 bytes. The cube says which structural relations
are supported or opposed; `TransformProgram` says exactly what deterministic
computation to execute. Neither field names nor pre-named traffic families are
semantic authority.

### OperatorPage32 budget

The canonical hot C32 package fits in one 4 KiB page:

```text
OperatorPage32
+-- Header64                 64 B
+-- PhaseProfile1024       1024 B
+-- StructuralRoles512     512 B
+-- TernaryCube2048       2048 B
+-- TransformProgram128     128 B
+-- CompositionDag128       128 B
`-- RendererProgram128      128 B
                            ------
                              4032 B
```

The verifier contract and proof lineage are bound by hashes in the header and
stored outside the hot page. They remain mandatory authority; moving them cold
does not let the actor authorize itself.

For C64, each relation plane uses bitmap-addressed sparse tiles:

```text
u64 present_tiles
+ packed sequence of 16-byte tiles

one tile = 8 x 8 ternary cells = 128 bits = 16 bytes
```

The tile coordinate is its bit position, so individual tile headers are not
stored. This preserves exact dense/sparse parity while keeping typical C64
operators in the expected few-KiB range.

### Generational tensor updates

`ForwardWave` reads the cube and typed program to bind and transform roles.
`BackwardWave` never rewrites the ACTIVE cube:

```text
immutable Cube generation g
-> typed signed residual
-> CubeEvidenceAccumulator
-> CandidateCubeField
-> OperatorGrokkingConsolidator
-> crystallized candidate Cube generation g+1
-> replay / frozen future / ablation
-> admission
```

For core relation planes, a contradiction to `+1` becomes unresolved and
triggers split or relearning before any polarity change. Direct learned
opposition is primarily allowed in the applicability plane after repeated
independent evidence. This prevents one failure from flipping the semantic law.

### Operator grokking: field to circuit to cube

The implementation core is named and ordered exactly as follows. These names
are lifecycle boundaries, not interchangeable labels:

```text
VerifiedPartialRelationWaves
    from independent surfaces, roles, and planes
              |
              v
CandidateCubeField
    +-- partial operator circuits
    +-- competing circuits
    +-- signed amplitudes
    +-- verified residual phases
    `-- cross-plane relations
              |
              v
OperatorGrokkingConsolidator
    +-- global interference
    +-- cross-plane phase locking
    +-- whole-circuit connectivity
    +-- circuit coherence
    `-- margin over competing circuits
              |
              v
CoherentOperatorCandidate generation g+1
    SHADOW only; no execution authority
              |
              v
TransferableOperatorV2
    +-- RoleGraph
    +-- RelationProgram
    +-- TransformProgram
    +-- CompositionDag
    +-- RendererContract
    `-- VerifierContract
              |
              v
Frozen future + causal controls
              |
              v
Exact episodic runtime-authority cleanup
              |
              v
ProvenGrokking
              |
              v
immutable TernaryOperatorCube + typed operator
              |
              v
external admission -> ACTIVE -> CPU
```

`CoherentOperatorCandidate` means that a whole circuit has formed. It is not a
grokking proof and has no authority. `ProvenGrokking` is a proof state granted
only after frozen-future transfer, exact-memory cleanup, causal controls, and
independent verification all pass. Runtime admission remains a separate later
decision.

Per-cell threshold updates are not grokking:

```text
three votes for one cell
-> set the cell to +1
```

is ordinary online table learning. The canonical grokking path is circuit-level:

```text
partial evidence from independent surface A
-> competing circuit hypotheses X / Y / Z

partial evidence from independent surface B
-> competing circuits X / Y

partial evidence from independent surface C+
-> cross-plane phase coherence selects X globally
-> linked role, relation, transform, composition, and renderer states resolve
   as one circuit
-> immutable TernaryOperatorCube and typed program crystallize together
```

No individual surface is required to contain the whole law. Surface count alone
must never trigger crystallization. Independent surfaces are an evidence gate;
global phase coherence and circuit consistency perform the selection.

Before crystallization, candidate state lives outside the hot operator page:

```text
CandidateCubeField
+-- unresolved ternary cells
+-- signed amplitudes
+-- verified residual phases
+-- competing cube circuits
+-- transform/program hypotheses
+-- cross-plane coherence
`-- bounded exact episodic support
```

`OperatorGrokkingConsolidator` evaluates whole connected circuits, not isolated
cells. A circuit includes compatible `RoleGraph`, relation planes,
`TransformProgram`, `CompositionDag`, and `RendererProgram`. It may emit only
one of:

```text
CRYSTALLIZED candidate generation g+1
COMPETING_CIRCUITS -> keep accumulating evidence and ABSTAIN
INCONSISTENT -> split or reject the hypothesis
CENSORED -> no semantic update
```

The learning stages are:

```text
memorization
-> exact episodic support, competing circuits, future ABSTAIN

circuit formation
-> abrupt coherent resolution of one transferable operator
-> frozen-future transfer appears

cleanup
-> exact episodic authority is removed
-> transfer and decisions remain identical
```

Cleanup is mandatory proof that the operator is carried by the circuit rather
than lookup. Proof receipts and immutable support manifests remain available as
cold audit evidence; they are not runtime exact-memory authority.

The causal grokking battery is:

```text
full phase             -> circuit forms and transfers
no phase               -> tie or ABSTAIN
shuffled residual      -> tie or ABSTAIN
magnitude only         -> tie or ABSTAIN
matched random center  -> tie or ABSTAIN
phase restored         -> same circuit and transfer return
exact memory removed   -> transfer remains
false accepts          -> 0
```

A grokking claim additionally requires an observable delayed transition from
memorization to circuit formation, independent heldout surfaces, no exact
lookup overlap, and an immutable artifact containing the crystallized circuit.

The A4 laboratory result is supporting evidence for this shape: it exhibited
`memorization -> circuit formation -> cleanup`, transfer `0 -> 1440/1440`, and
exact memory `13 -> 0`. It remains a grokking candidate rather than proof of
organic live grokking because its cross-surface threshold and cleanup policy
were architecture-defined. The production claim remains open until the live
`OperatorGrokkingConsolidator` forms and cleans up a circuit under the stronger
contract above.

Supporting A4 artifact:

```text
/home/ubu/projects/rsmod/results/raw-phase-grokking-a4-2026-07-10/PROOF_REPORT.md
```

### Mandatory compact-operator tests

```text
renamed surface       -> identical cube hash
permuted roles        -> canonical cube
dense vs sparse       -> exact parity
reserved 11           -> package rejected
symmetric bindings    -> ABSTAIN
restart               -> identical decision
shuffled residual     -> causal degradation
false accepts         -> 0
```

### ForwardWave

```text
state_before + current observation
-> structural role binding
-> relation evaluation
-> transform and composition
-> renderer
-> predicted relation frame
-> actor result
```

`ForwardWave` does not mean unconstrained prediction. It instantiates a known
operator law on the current state. Runtime receives no target response,
`state_after`, or future action.

### Independent typed execution trace

The actor cannot describe its own success. Each stage emits a bounded receipt:

```text
RoleBindingReceipt
-> RelationEvaluationReceipt
-> TransformReceipt
-> CompositionReceipt
-> RendererReceipt
-> VerifierReceipt
-> VerifiedDeltaReceipt
```

Every receipt binds at least the generation, operator, input relation hashes,
output relation hash, stage result, event time, evidence source, and previous
receipt hash.

In production, the residual is derived without teacher authority:

```text
observed_relation_frame - predicted_relation_frame
-> typed residual wave
```

The observed frame must come from independent tool output, state transition, or
verifier evidence. A teacher response may be used for support and development
experiments after a trace is complete, but it is not production runtime
authority. With no independent observation, the outcome is censored `UNKNOWN`.

### BackwardWave

`BackwardWave` is a typed transformation from verified residual to a bounded
phase update, not a metaphor and not a discrete rejection table:

```text
zero verified residual
-> phase-aligned reinforcement

repeatable applicability residual
-> phase-inverted counter-wave
-> distributed anti-center / narrower applicability field

hard structural residual
-> localize RoleGraph / RelationProgram / Transform / Composition / Renderer
-> center bifurcation, repair, split, or revoke

censored outcome
-> no semantic phase update
-> optional bounded availability/uncertainty accounting only
```

### Generation firewall

An ACTIVE operator is immutable. Feedback never mutates production authority in
place:

```text
ACTIVE generation g                         immutable
-> verified feedback
-> bounded BackwardWave accumulator
-> candidate generation g+1                 shadow
-> replay + frozen future + causal ablation
-> external admission
-> ACTIVE generation g+1                    immutable
```

This firewall preserves the proof for generation `g`, prevents feedback
oscillation, and gives every behavioral change a new evidence lineage.

### Canonical matched-capacity experiment

The system core is accepted only through five frozen contours:

```text
A. old operator
   current baseline

B. rich oracle operator without BackwardWave
   tests whether TransferableOperatorV2 is expressive enough
   claim_authority = false

C. rich automatically induced operator without BackwardWave
   tests automatic operator induction

D. rich automatically induced operator with BackwardWave
   but only independent per-cell threshold consolidation
   tests feedback without circuit-level grokking

E. rich automatically induced operator with BackwardWave
   and OperatorGrokkingConsolidator
   tests circuit formation and cleanup
```

Interpretation:

```text
B - A = expressive-capacity gain
C / B = automatic-induction quality relative to the oracle ceiling
D - C = contribution of ordinary backward feedback
E - D = contribution of circuit-level grokking consolidation
```

Freeze the stream, teacher groups, support/future partition, verifier, top-k,
hypothesis budget, runtime budget, and package budget. Required BackwardWave
controls are shuffled residual phase, magnitude-only residual, discrete
anti-center only, and no backward feedback. Product authority belongs only to
the automatically induced contour.

Primary measurements are independent surfaces per law, frozen-future
executions, eliminated exact checks, package bytes, p99, and wrong accepts.
Potential coverage is secondary and is never counted as actual CPU savings.

### Implementation status boundary

The pure architectural core is now implemented:

```text
crates/nando-core/src/wave/operator_circuit.rs
crates/nando-core/src/wave/operator_grokking.rs
crates/nando-core/src/wave/operator_grokking_proof.rs
crates/nando-core/src/wave/operator_page.rs
crates/nando-response-actor/src/verified_delta.rs
crates/nando-response-actor/src/backward_wave.rs
crates/nando-response-actor/src/operator_generation.rs
crates/nando-response-actor/src/transferable_operator_v2.rs
crates/nando-core/tests/operator_grokking_causal.rs
```

It includes bounded competing circuits, whole-circuit phase consolidation,
strict proof states, the 4032-byte operator page, typed verified residuals,
immutable `g -> g+1` feedback, actor/verifier binding, and matched causal tests.

This is not yet a production grokking claim. Automatic emission of all typed
stage receipts from live actor/verifier execution, bounded streaming checkpoint
integration, live frozen-future proof, external admission of a generated page,
exact episodic runtime-authority cleanup, and real CPU economics remain open.
Simple traces that already contain a complete law may train an operator, but
they cannot satisfy the grokking claim.

### Missing bridge: fragments must create new circuits

The consolidator must not be limited to choosing among externally registered
complete circuits. The canonical discovery path is:

```text
independently verified support residuals
-> RelationFragmentGenerator
-> bounded CircuitSynthesizer
-> newly constructed connected circuit topologies
-> frozen candidate topology set
-> disjoint future relation waves
-> OperatorGrokkingConsolidator
```

`CircuitSynthesizer` proposes connected law shapes. It does not decide truth,
grant authority, or constitute grokking. The whole operator circuit groks only
when disjoint future waves phase-lock across surfaces and planes and one circuit
obtains a causal coherence margin.

WARNING: support events used to infer topology or phase anchors must never be
reused as future proof. Fitting candidate anchors and scoring coherence on the
same waves is circular and can make shuffled phase look successful. Support and
future are disjoint evidence partitions under the same immutable source
operator generation `g`; only a proven circuit becomes operator generation
`g+1`. Confusing evidence partitions with operator generations incorrectly
skips to `g+2` and is a contract violation.

The first implementation is deliberately bounded. Every positive verified
sample becomes an accounted structural fragment; censored, applicability
negative, and hard-contradiction outcomes cannot create topology. Every missing
candidate has an explicit blocker such as no positive evidence, non-canonical
roles, disconnected graph, zero phase magnitude, or capacity exhaustion.

### Canonical correction: roles are local to each surface

WARNING: a transferable operator must not be synthesized by unioning relation
fragments that already share caller-assigned global role IDs. Every completed
surface owns a private local role namespace. Transferability is born while the
system maintains competing structural role-alignment hypotheses across those
namespaces.

```text
SurfaceFragmentBundle A: local roles a0, a1, a2
SurfaceFragmentBundle B: local roles b0, b1
SurfaceFragmentBundle C: local roles c0, c1, c2
        |
        v
StructuralRoleSignature + graph-color refinement
        |
        v
RoleAlignmentHypothesis X / Y / Z
        |
        v
bounded dependency-closed circuit beam
        |
        v
CandidateOperatorBlueprint X / Y / Z
        |
        v
FrozenOperatorBlueprintSet
        |
        v
independent future binding and cross-plane phase coherence
```

Support creates a version space; it never chooses the winner. A blueprint
contains a role graph, relation program, existing typed transform program,
composition DAG, renderer hypothesis, verifier obligations, and phase anchors.
The actor and verifier engines are reused; circuit synthesis must not introduce
a second execution language.

The frozen set commits full 32-byte support lineages, its canonical candidate
set, canonicalizer version, bounded synthesis configuration, and source operator
generation. Receipt IDs are indexing aids and are never sufficient provenance.
Future evidence must have lineages absent from support. Structural equivalence
to support is allowed and required for transfer; byte-identical evidence lineage
reuse is forbidden.

Circuit ranking uses one contribution per lineage for each edge, then edge
coherence, plane coherence, and whole-circuit closure. Raw sample frequency must
not let one common relation drown a weak mandatory edge. Crystallization requires
minimum edge coverage, all mandatory planes, a whole-circuit coherence floor,
and margin over the runner-up.

Absolute core limits are not caller-expandable:

```text
blueprints <= 64
beam depth <= 12
beam expansions <= 4096
roles <= 32
relations <= 256
```

Budget exhaustion preserves the incomplete training state but yields ABSTAIN,
never the first candidate. A JEPA-like predictor may later rank expansions and
probes, but it cannot create evidence, crystallize a circuit, or grant authority.

Canonical implementation plan:

```text
plans/local-role-operator-blueprints-v1/LOCAL_ROLE_OPERATOR_BLUEPRINTS_V1.md
```

Pure-core implementation:

```text
crates/nando-core/src/wave/operator_blueprint.rs
crates/nando-core/tests/operator_blueprint_causal.rs
```

The pure bounded proof now constructs competing blueprints from local partial
graphs and resolves one only on full-lineage future phase evidence. This is a
laboratory mechanism PASS. Live bundle extraction, streaming persistence,
external admission, CPU execution, and economics remain BLOCK.

Canonical implementation plan:

```text
plans/operator-grokking-core-v1/OPERATOR_GROKKING_CORE_V1.md
plans/operator-circuit-synthesis-v1/OPERATOR_CIRCUIT_SYNTHESIS_V1.md
```

## 4. Canonical Learning Path

Training may inspect a completed trace, including the action and answer that
actually occurred. That is the teacher signal. This is legitimate
self-training, not runtime leakage.

```text
completed live trace
-> post-action teacher signal
-> structural alignment and grouping
-> L1/L2 relation-wave representation
-> positive phase center
-> counterexamples
-> anti-center, applicability subcenter, or repaired law
-> transferable action operator
-> compact deterministic actor/program
-> independent frozen-future receipts
-> external admission
-> ACTIVE registry
```

Current ownership:

```text
completed trace and teacher join
  crates/nando-transition-serving/src/session_stream.rs

streaming teacher/student state
  crates/nando-response-actor/src/online.rs
  crates/nando-response-actor/src/online_state.rs
  crates/nando-response-actor/src/online_checkpoint.rs

structural grouping and semantic equivalence
  crates/nando-response-actor/src/semantic_alias.rs
  crates/nando-response-actor/src/online_subcenter.rs

counterexamples, repair, anti-centers, winning laws
  crates/nando-response-actor/src/cegis.rs

ordinary structured-result induction
  crates/nando-response-actor/src/online_collection.rs
  crates/nando-response-actor/src/collection_synthesis.rs

program and execution
  crates/nando-response-actor/src/program.rs
  crates/nando-response-actor/src/runtime.rs

independent verification and admission
  crates/nando-response-actor/src/verifier.rs
  crates/nando-response-actor/src/online_admission.rs
  crates/nando-response-actor/src/bin/nando-response-admission.rs
```

## 5. The Required Feedback Loop

Every evaluated outcome must first be classified. It is incorrect to collapse
all non-PASS outcomes into one negative class:

```text
verified positive
-> reinforce center / law

repeated applicability negative
-> accumulate evidence from independent sessions
-> derive an action-neutral distinguishing relation
-> form an anti-center
-> narrow the route without changing the actor law

hard actor / verifier / teacher contradiction
-> invalidate the unsafe winner
-> split into applicability subcenters, repair, or revoke
-> begin a new frozen generation

timeout / unavailable environment / missing evaluator / not evaluated
-> censored UNKNOWN
-> do not reinforce a center
-> do not create an anti-center
-> do not count as evidence against the law
```

This is the current decisive research/product boundary. A system that counts a
repeatable applicability negative but does not feed it back into the field is
not the intended learning loop. A clean law needs both a positive center and a
learned boundary against negative states. Conversely, poisoning that boundary
with infrastructure failures or unknown outcomes is also a learning failure.

An anti-center means repeatable *non-applicability*, not arbitrary failure. It
requires independent-session evidence and an observable pre-decision relation
that distinguishes the negative surface from positive support. A hard semantic
contradiction means that the current operator law or its partition is unsafe;
it must cause repair, split, or revocation rather than being hidden behind a
broader anti-center.

The next meaningful live progression is:

```text
live counterexample
-> live anti-center or clean subcenter
-> growing clean frozen future
-> independent receipts
-> first ACTIVE ordinary project/status/count/filter/compose law
```

If no action-neutral pre-decision distinction exists, the correct result is
`ABSTAIN`. Never invent a discriminator from the future action.

### Three complementary intelligence levels

These are distinct responsibilities, not competing implementations:

```text
JEPA-inspired latent consequence model
  encodes state z_t and predicts z_t+1 under a candidate action

self-correcting Wave operator
  binds a known law to an actor and learns when it is applicable

external causal law discovery (for example, MICRO-12 research)
  investigates unresolved contradictions and proposes genuinely new actions
```

Their loop is:

```text
latent predictor ranks candidate consequences
-> external discovery probes and proposes a new verified law
-> Wave compresses repeated experience into centers
-> actor executes the law
-> verifier classifies the consequence
-> prediction error updates latent dynamics
-> applicability evidence updates Wave boundaries
-> unexplained hard contradiction returns to causal discovery
```

The external researcher is not part of the hot runtime and has no execution
authority. The latent predictor does not authorize actions. Wave phase centers
remain a separate compact memory for recognition and applicability; they are
not the JEPA latent state.

## 6. Runtime Boundary

Runtime is intentionally narrower than training:

```text
state_before + current observation
-> Wave route
-> exact applicability boundary
-> deterministic actor
-> independent verifier
-> ACCEPT

any uncertainty or disagreement
-> ABSTAIN
-> upstream model
```

Runtime must not read the future action, final teacher response, `state_after`,
or proof-only training atoms. The actor cannot authorize itself. The miner
cannot grant execution authority. Admission is external.

Typed programs and renderers are an execution and proof language around a law
discovered by the Wave. They are useful, but they are not the intelligence
core. Never turn discovery into selection among a few pre-named programs.

## 7. Non-Negotiable Invariants

1. Completed action/response is allowed and required as a training label.
2. Future action/response is forbidden in runtime routing and guards.
3. Field names, function names, and manual family IDs are not semantic
   authority; transfer must survive renamed surfaces.
4. Repeatable applicability negatives must update anti-centers. Hard semantic
   contradictions must trigger applicability subcenters, CEGIS repair, or
   revocation. Merely recording either outcome is insufficient.
5. Censored outcomes such as timeout, unavailable environment, missing
   evaluator, or `NOT_EVALUATED` are unknown evidence. They must never train a
   positive center or anti-center.
6. `false_accepts = 0` is a hard requirement.
7. `runtime_parity_mismatches = 0` is a hard requirement.
8. Every local accept has an independent verifier receipt.
9. The miner emits evidence-bearing candidates; external admission grants
   authority.
10. Frozen future is event-time independent from support. Never fabricate or
   backfill it from support.
11. Potential, shadow, ACTIVE, and real CPU coverage are different numbers.
12. State is bounded and compact. Normal startup must not rescan unbounded
    history, and the hot path must not append unbounded payloads.
13. Serving and learning remain streaming, event-driven Rust with low idle CPU.
14. One algorithmic mechanism changes at a time; refactoring and scoring changes
    are separate commits.
15. JEPA-like latent predictions, Wave applicability evidence, and verifier
    truth are three different signals. They must have separate state and update
    rules; none may masquerade as another.
16. ACTIVE generations are immutable. Verified feedback can only construct a
    separately proven candidate generation.
17. BackwardWave updates require a typed `VerifiedDeltaReceipt` whose observed
    side is independent from the actor.
18. Independent per-cell thresholds are ordinary online learning and must never
    be reported as grokking. Grokking requires coherent whole-circuit formation.
19. A grokked operator must preserve heldout transfer after exact episodic
    runtime authority is removed.

Accounting identities must have no silent loss:

```text
admission_ready_cohorts
= emitted_candidates + explicitly_blocked_candidates

collection_observations
= executable + ambiguous + irreducible

local_accepts
= independently_verified_accepts
```

## 8. Truthful Proof and Economics

Always report these levels separately:

```text
discovered optimistic upper bound
shadow executions
independently verified frozen future
admission-ready candidates
ACTIVE authority
actual local CPU accepts
independently verified input-token savings
```

Do not call potential coverage savings. Do not call shadow traffic CPU accepts.
Do not call a laboratory proof product completion.

Product M3 means all of the following, not an architecture score:

```text
verified input-token saving share >= 50%
for three consecutive independent windows
false_accepts = 0
runtime parity mismatches = 0
economics hard gate = YES
```

## 9. Behavioral Oracle

For changes to discovery, grouping, Wave feedback, or transferable actions,
compare behavior with the preserved pre-refactor tree:

```text
/home/ubu/projects/rsmod/worktrees/nando-wave-pre-refactor-2026-07-10
HEAD 6071708bbdd15f5df0be31f68379986d796e24b1
```

This is a behavioral oracle, not code to copy blindly. Preserve the current
independent verifier, admission, fallback, parity, storage, and runtime safety
shell. Restore useful learning behavior inside that shell, one mechanism at a
time, and keep an improvement only when live coverage grows without a safety
regression.

## 10. Known Destructive Failure Modes

These mistakes have already damaged coverage and must not be repeated:

- Calling the completed-trace teacher label "leakage" and removing it from
  training. Result: structural fragmentation and support split across many
  weak groups.
- Making exact template or DSL selection the center of discovery. Result:
  surface-bound programs, support such as 12/32 or 13/32, and loss of transfer.
- Counting counterexamples without feeding them into Wave repair. Result: no
  applicability boundary and no live anti-center.
- Treating timeout, unavailable environment, or unevaluated work as negative
  knowledge. Result: a poisoned anti-center that learns infrastructure noise.
- Hiding a hard actor/verifier/teacher contradiction inside a broad anti-center.
  Result: the unsafe operator survives instead of being split or revoked.
- Merging all actions globally without preserving structural role alignment.
  Result: inconsistent roles and unsynthesizable families.
- Deduplicating packages only by actor text while ignoring phase centers,
  anti-centers, predicates, margin, or verifier authority.
- Improving dashboards, gates, or infrastructure while ordinary CPU coverage
  remains unchanged, then reporting the technical PASS as product progress.
- Performing a broad academically motivated rewrite before understanding and
  reproducing the original L1/L2/L3 behavior.
- Mixing a move-only refactor with a learning/scoring change, making regressions
  impossible to attribute.
- Mutating an ACTIVE center in place from live feedback. Result: destroyed proof
  lineage, oscillation, and behavior with no frozen generation boundary.
- Calling a counter or discrete reject list `BackwardWave` without proving a
  typed residual-to-phase update and its causal phase ablation.
- Treating a C32/C64 phase record as the complete operator. Result: phase
  recognition survives while role binding, transformation state, and transfer
  capacity disappear.
- Replacing balanced ternary operator state with unsigned presence bits. Result:
  loss of neutral state, signed relation opposition, and the original
  Setun-inspired interference semantics.
- Calling gradual per-cell threshold updates grokking. Result: a rule table is
  mistaken for emergent circuit formation, and the central Wave claim is lost.

## 11. Required Protocol Before Core Changes

Every agent must do this before editing core behavior:

1. Read this canon and the directly relevant implementation files.
2. Draw the current signal tree in plain language and mark the exact blocker.
3. Name the touched boundary: capture, discovery, Wave, CEGIS, actor, verifier,
   admission, runtime, storage, or economics.
4. Record a short live baseline: actual CPU share, verified token savings,
   teacher pools, winners, future, candidates, ACTIVE packages, false accepts,
   parity failures, latency, memory, and disk growth.
5. Compare with the behavioral oracle when recovery or core learning is
   involved.
6. Make one scoped change. Do not add a manual operator class as a shortcut.
7. Run focused checks, then one release build, the mandatory live transition
   gate, and a real runtime check.
8. Report actual wall time for long commands and do not disappear silently.
9. Commit the change with a narrow message.
10. Update this canon only when the architecture itself changes.

## 12. Supporting Documents

The canon is short by design. Deeper evidence and implementation detail live
here:

```text
docs/NORTH_STAR.md
docs/NANDA_WAVE_THEOREM.md
docs/NANDO_WAVE_SIGNAL_PATH_L1_TO_OPERATOR.md
docs/L3_SEMANTIC_GROKKING.md
docs/SYMBOL_CELL8_ARCHITECTURE.md
docs/LEXICON_FOUNDATION_V1.md
docs/OPERATOR_PRODUCT_LINES_AND_CAPACITY.md
docs/architecture_lineage/03_role_filler_binding.md
docs/RISKS.md
```

If a supporting document contradicts this canon, stop and resolve the
contradiction explicitly. Do not silently choose the interpretation that makes
the planned refactor easier.
