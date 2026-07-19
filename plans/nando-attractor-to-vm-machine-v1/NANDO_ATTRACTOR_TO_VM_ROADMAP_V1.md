# Nando Attractor-to-VM Machine Roadmap V1

Status: canonical implementation roadmap.

Architecture authority: [`../../ARCHITECTURE_CANON.md`](../../ARCHITECTURE_CANON.md).
Product goal: [`../../docs/GOAL.md`](../../docs/GOAL.md).
Implementation journal: [`IMPLEMENTATION_JOURNAL.md`](IMPLEMENTATION_JOURNAL.md).

This roadmap replaces the old Cell32/Chat-0 and streaming-overlay roadmaps as
the current implementation order. Those files remain historical evidence.

## 0. Completion Definition

The project is complete only when the full path is live:

```text
completed live trace
-> partial relation evidence
-> newly synthesized circuit-attractor
-> causal grokking proof
-> exact-memory cleanup
-> crystallized versioned bytecode
-> dynamic registry
-> phase route + structural grounding
-> VM execution
-> independent verification
-> real avoided LLM call
-> verified economics
```

Product completion additionally requires:

```text
verified input-token saving share >= 50%
three consecutive independent windows
false_accepts = 0
runtime_parity_mismatches = 0
verification coverage = 100% for CPU accepts
economics hard gate = YES
Product M3 = YES
```

No architecture score, laboratory proof, optimistic upper bound, shadow accept,
candidate, or ACTIVE package substitutes for these numbers.

## 1. Global Safety Envelope

These invariants apply to every stage:

```text
production is Rust only
normal learning is streaming, one-pass, event-driven, and bounded
normal startup does not replay unbounded history
teacher action/response is available only after trace completion
runtime reads only state_before + current observation
field/function/operator names are not semantic authority
ACTIVE generations are immutable
unknown, ambiguous, exhausted, or malformed means ABSTAIN
actor cannot verify or admit itself
future evidence is event-time disjoint from support
no fabricated support, future, receipts, or savings
false_accepts = 0
runtime_parity_mismatches = 0
```

Every bounded search returns one of:

```text
Complete(result)
Exhausted { stage, explored, frontier_remaining }
InvalidEvidence(reason)
Censored(reason)
```

Only `Complete` can advance. Exhaustion is not a weak success. Censored outcomes
do not train positive centers, anti-centers, or semantic residuals.

## 2. Current Baseline And Honest Boundary

The repository already contains substantial pieces:

```text
relation fragment and blueprint types                 IMPLEMENTED
bounded competing-circuit synthesis                   IMPLEMENTED
phase-dependent blueprint selection                   IMPLEMENTED
OperatorPage32 fixed 4032-byte format                  IMPLEMENTED
typed transform fragments and crystallization         PARTIAL
sealed future/provenance chain                         PARTIAL
runtime role grounding from raw surface               BLOCK
whole winner-owned rich actor                          BLOCK
general versioned Operator VM                          BLOCK
live attractor-to-VM package                           BLOCK
50% verified token saving                              BLOCK
```

This table is a navigation boundary, not a PASS claim. Each stage below must
produce its own current artifact and receipt.

## Stage 0: Canon And Baseline Freeze

Objective: prevent architecture drift before new core work.

Deliverables:

```text
canonical goal and roadmap
current signal tree with exact blocker
live baseline: CPU share, verified savings, future, candidates, ACTIVE
runtime baseline: p99, RSS, idle CPU, checkpoint and disk growth
source and behavior oracle commitments
```

Gate:

```text
one current goal
one current roadmap
old plans explicitly historical
all current product metrics labelled by evidence level and timestamp
```

Forbidden shortcut: using an old shadow percentage as current savings.

## Stage 1: Total Structural Evidence

Objective: turn every completed trace into one canonical structural outcome.

Route:

```text
raw completed trace
-> canonical event or explicit rejection
-> local role namespace
-> relation fragments
-> SurfaceFragmentBundle
-> evidence receipt
```

Required evidence includes roles, type/cardinality, equality, membership,
ordering, temporal relations, changed region, and unchanged complement. Names
may be retained for audit but cannot drive structural identity or runtime route.

Accounting gate:

```text
completed_traces = accepted_bundles + rejected + censored
accepted_bundles = unique + replay_duplicates + conflicts
conflicting IDs = 0 before authority
byte-identical replay produces byte-identical bundle and receipt
```

Code ownership:

```text
capture/normalization adapters       nando-transition-serving
structural contracts and bundles    nando-core
streaming evidence ledger           cold learner boundary
```

## Stage 2: Dynamic Operator Field

Objective: represent unresolved operator knowledge before any complete operator
exists.

Required field:

```text
DynamicCandidateCubeField
+-- local role-alignment hypotheses
+-- unresolved ternary relation planes
+-- transform hypotheses
+-- composition hypotheses
+-- renderer hypotheses
+-- positive fixed-point phase waves
+-- counter-waves and anti-centers
+-- competing connected circuits
`-- bounded exact episodic support
```

The field must be compacted incrementally. It may retain bounded exemplars and
commitments, never unbounded raw payload history.

Gate:

```text
one-pass update is deterministic
checkpoint roundtrip is byte-identical
memory and disk budgets are enforced in code
no caller can inject a complete authoritative circuit
```

Forbidden shortcut: storing a prebuilt operator under the name "field".

## Stage 3: New Circuit Synthesis

Objective: construct circuit topologies that were absent from every individual
surface and absent from a caller-supplied candidate list.

Mechanism:

```text
surface-local role graphs
-> structural color refinement
-> symmetric role orbits
-> bounded role-alignment hypotheses
-> connected fragment expansion
-> competing blueprints X / Y / Z
-> canonical fingerprints
```

Initial hard budgets:

```text
beam width          64
maximum depth       12
maximum expansions  4096
```

These are implementation starting points, not scientific constants. Raising
them requires measured live opportunity and bounded CPU/RSS evidence.

Gate:

```text
support contains no full target circuit
at least two complete competing circuits survive support
permuting surfaces or local roles preserves candidate-set SHA
search exhaustion prevents freeze
renamed fields preserve canonical circuit identity
```

## Stage 4: Circuit-Attractor Dynamics

Objective: make candidates actual basins of a coupled field rather than a
ranked table.

Required dynamics:

```text
partial noisy surface
-> excites a bounded local circuit field
-> compatible cross-plane components reinforce
-> incompatible components interfere destructively
-> anti-centers repel inapplicable regions
-> state converges or ABSTAINS
```

Measurements:

```text
basin recovery radius
convergence ticks and CPU cost
winner coherence and runner-up margin
cross-plane closure
spurious attractor count
oscillation/non-convergence count
noise and missing-fragment tolerance
```

Gate:

```text
partial evidence recovers the same circuit
bounded perturbations return to the same basin
nearby negative surfaces converge to ABSTAIN or another valid basin
spurious attractors = 0 on the registered proof corpus
```

Forbidden shortcut: independent per-cell vote thresholds.

## Stage 5: Causal Grokking Proof

Objective: prove an abrupt circuit-level transition from episodic memory to a
transferable law.

Required sequence:

```text
one surface  -> exact memory, future ABSTAIN
two surfaces -> competing laws, future ABSTAIN
three or more independent surfaces
             -> cross-plane phase locking
             -> whole circuit crystallization
             -> abrupt future transfer
cleanup      -> episodic runtime authority removed
             -> identical transfer remains
```

Causal controls:

```text
full phase             circuit forms and transfers
no phase               tie or ABSTAIN
shuffled phase         tie or ABSTAIN
magnitude only         tie or ABSTAIN
matched random center  tie or ABSTAIN
no anti-centers        applicability boundary worsens
phase restored         same circuit and transfer return
```

Gate:

```text
future transfer PASS
exact episodic authority removed PASS
perturbation recovery PASS
spurious attractors 0
lookup overlap 0
false accepts 0
immutable causal receipt emitted
```

This gate proves only the scoped operator class in its corpus. It does not prove
universal intelligence or product M3.

## Stage 6: Operator IR And Crystallizer

Objective: compile a proven attractor into a complete executable operator whose
circuit causes its computation.

Required IR:

```text
RoleGraph
RelationProgram
ApplicabilityBoundary
BindingProgram
TransformProgram
CompositionDag
RendererProgram
VerifierContract
phase routing profile
proof commitments
generation
```

Crystallization route:

```text
sealed circuit winner
-> complete IR validation
-> deterministic lowering
-> bytecode static validation
-> OperatorPage32 + bounded extension pages
-> immutable OperatorPackage
```

Gate:

```text
non-empty transforms owned by winner
role IDs affect actual operands and outputs
renderer executes instructions, not a remembered teacher string
composition graph is acyclic before CALL_OPERATOR lowering
same sealed input produces byte-identical package
manual diagnostic reports cannot create authority
```

Forbidden shortcut: attaching a caller-provided actor template after Wave has
selected only a relation circuit.

## Stage 7: Versioned Operator VM

Objective: execute new admitted skills as bytecode without adding Rust branches.

Initial instruction families:

```text
roles:      FIND_ROLE, BIND_ROLE, LOAD_ROLE
proof:      ASSERT_RELATION, ASSERT_GUARD
data:       COMPARE, FILTER, MAP, COUNT, TRANSFORM
control:    BRANCH, CALL_OPERATOR
rendering:  FORMAT, EMIT
safety:     ABSTAIN
```

VM contracts:

```text
explicit bytecode version
deterministic fixed-width decoding
static verifier before registry insertion
instruction/stack/call/page/allocation/time gas
no filesystem, network, clock, process, or mutable global access
unknown opcode or version -> package reject
runtime ambiguity or exhausted gas -> ABSTAIN
```

Gate:

```text
project/status/count/filter/compose expressible without new Rust route branches
package restart roundtrip preserves execution
interpreter and independent verifier parity on all accepted cases
mutation corpus is rejected or ABSTAINS
p99 and hot-byte budgets are recorded
```

## Stage 8: Runtime Role Grounding And Independent Verification

Objective: bind canonical roles to a new raw surface without caller-provided
selectors or semantic names.

Route:

```text
raw pre-action surface
-> independent structural extraction
-> ObservedRuntimeSurface
-> bounded role CSP constrained by circuit
-> action-equivalence classes
-> unique effective binding or ABSTAIN
-> VM execution
-> separate verifier extraction and grounding
-> exact outcome comparison
```

Gate:

```text
field renaming PASS
layout change PASS
role order change PASS
same values then diverging values exposes role swap
multiple mappings with same action PASS
multiple mappings with different actions ABSTAIN
tampered surface/raw input/selector REJECT
actor mutation verifier REJECT
```

The verifier cannot receive actor-selected bindings. It independently rebuilds
the observed structure from committed raw input.

## Stage 9: Seals, Registry, Router, And Admission

Objective: create the only legal authority path from proof to CPU execution.

Two sequential seals avoid circular authority:

```text
future evidence
-> SealedBlueprintWinnerReceipt
-> binding + bytecode compilation
-> independent execution receipts
-> ExecutableParitySeal
-> external admission
-> immutable ACTIVE generation
```

The dynamic registry stores admitted package generations. The Phase Router
activates only a small bounded local candidate field and cannot bypass guards,
grounding, VM validation, or the independent verifier.

Accounting gate:

```text
admission_ready_cohorts
= emitted_candidates + explicitly_blocked_candidates

every emitted candidate
-> admitted | explicit blocker

local_accepts
= independently_verified_accepts
```

Required deployment gate: `nando-live-transition-gate`. Any `WATCH`, `VETO`, or
`ERROR` blocks local accept.

## Stage 10: BackwardWave And Generation g+1

Objective: convert verified execution consequences into bounded field updates
without mutating the ACTIVE operator.

Typed outcomes:

```text
VerifiedPass
  -> reinforce the basin in candidate generation g+1

ApplicabilityNegative
  -> counter-wave and narrower applicability basin

HardContradiction
  -> typed residual localization
  -> split, repair, or revoke candidate lineage

Censored
  -> measurement-channel availability only
  -> no semantic or phase update
```

Gate:

```text
ACTIVE page g remains byte-identical
verified residual changes only candidate g+1
censored receipt causes zero semantic delta
shuffled residual destroys the claimed correction
checkpoint restart preserves pending generation exactly
new generation repeats frozen future and admission
```

## Stage 11: Recursive Operator Growth

Objective: let admitted programs compose into larger verified skills.

Ladder:

```text
K0 relation primitives
-> K1 transferable actions
-> K2 action compositions
-> K3 verified strategies
-> K4 methods for discovering new strategies
```

`CALL_OPERATOR` initially permits only a bounded acyclic dependency DAG. Every
higher operator commits to exact callee generations. A compressed higher opcode
must retain an unfoldable verifier path to admitted lower-level behavior.

Gate per level:

```text
novel heldout transfer beyond components
no authority inheritance shortcut
bounded depth and gas
callee revoke propagates to dependants
full unfold-and-verify parity
false accepts 0
```

K4 is research scope until K0-K3 are live and independently verified.

## Stage 12: Live Product Expansion To M3

Objective: grow actual verified CPU savings by synthesizing the most valuable
provable operators from ordinary live traffic.

Opportunity order:

```text
all ordinary input tokens
-> executable / ambiguous / irreducible / censored accounting
-> potential verified token volume
-> structural repeatability
-> verifier availability
-> VM/IR complexity and runtime cost
-> next primitive or operator family
```

No new primitive is added merely because it is elegant. It must unlock a large
measured volume of currently uncovered live tokens.

Milestones:

```text
M1  >= 100 real avoided calls, >= 1% verified tokens, wrong accepts 0
M2  reproducible 10-20% verified tokens over independent days/sessions
M3  >= 50% verified tokens in three windows, all hard gates PASS
```

If the measured executable upper bound is below 50%, publish a complete
denominator proof and continue by descending potential token value. Do not
rename the upper bound as savings and do not weaken M3.

## 3. Required Implementation Order

The stages are dependency ordered:

```text
0 Canon/baseline
-> 1 Total evidence
-> 2 Dynamic field
-> 3 Circuit synthesis
-> 4 Attractor dynamics
-> 5 Causal grokking
-> 6 IR/crystallizer
-> 7 VM
-> 8 Runtime grounding/verifier
-> 9 Seals/registry/admission
-> 10 BackwardWave g+1
-> 11 Recursive growth
-> 12 Live M3 expansion
```

Parallel work is allowed only across non-authority boundaries, for example VM
fuzzing while field proofs run. It is forbidden to skip from a laboratory
circuit winner directly to ACTIVE.

## 4. Immediate Next Work

The next code milestone is not another opcode and not a dashboard change:

```text
raw pre-action payload
-> ObservedRuntimeSurface
-> unique circuit-constrained role grounding
-> winner-owned transform operands
-> independent verifier re-grounding
-> ABSTAIN on semantic role ambiguity
```

This closes the current gap where a learned circuit can accompany an actor
without actually determining that actor's runtime operands.

After that, implement the minimal versioned VM around the already proven scalar
path, then widen bytecode only in live token-opportunity order.

## 5. Reporting Contract

Every implementation report uses this tree and marks `PASS`, `WATCH`, or
`BLOCK` from current evidence:

```text
Live trace capture
-> Structural evidence
-> Dynamic operator field
-> Competing circuit synthesis
-> Circuit-attractor convergence
-> Causal grokking + cleanup
-> Crystallizer + bytecode
-> Runtime role grounding
-> Operator VM
-> Independent verifier
-> Admission + ACTIVE registry
-> Real CPU accepts
-> Verified token savings
```

For every command longer than 60 seconds, announce expected duration first and
report actual wall time afterward. Do not hide a product `BLOCK` behind a lower
layer's `PASS`.

## 6. Scientific Claim Ladder

Allowed claims are staged:

```text
S0  bounded field implemented
S1  stable circuit-attractor recovered from partial evidence
S2  causal phase-dependent grokking and cleanup demonstrated
S3  attractor crystallized into executable deterministic bytecode
S4  independently verified live operator admitted and executed
S5  recursive operator composition transfers
S6  Product M3 achieved
```

Each claim requires every previous claim. The scientific literature motivates
the mechanism but grants none of these statuses automatically.
