# Operator Identification And Active Inquiry V1

Status: `IMPLEMENTED_LIVE_CPU_PASS`

Date: `2026-07-23`

Implementation authority: `true`

Production authority: `true for one narrow admitted package`

Structural code-conformance gate: `PASS`

The design below is now implemented. It replaced the fixed natural-readiness
threshold with bounded semantic version-space collapse, immutable candidate
freeze, independent transfer evidence, and an adaptive proof basis. Legacy
fixed-row packages retain their old control policy; they cannot enter the
adaptive route by relabeling.

The first admitted result is one-shot operator identification, not a circuit
grokking claim: one narrow scalar continuation program is ACTIVE and has
executed verified live requests on CPU. Scientific circuit grokking still
requires fragment-only support, phase controls, cleanup, and future transfer.

## 1. Decision

Operator readiness must be determined by semantic uncertainty, not by a fixed
number of observations.

```text
support continues while several executable explanations survive
-> semantic version space collapses to one executable class
-> candidate freeze

first independent post-freeze case
-> CPU shadow execution
-> actor / independent verifier parity
-> narrow authority candidate
```

A simple operator may require one support observation and one independent
future observation. A complex operator may require more evidence. Additional
evidence is requested only while a concrete ambiguity remains.

There is no universal `32 support + 32 future` gate. Repeated evidence that
does not reduce uncertainty cannot advance identification.

## 2. Objective

Build one cold learning module that answers:

```text
Which executable operator laws remain possible?
What evidence eliminated each rejected law?
Are the survivors semantically different?
What next observation would best distinguish them?
When is exactly one executable semantic class identified?
What narrow applicability scope has actually been proven?
```

The module produces an immutable candidate and a missing-evidence contract. It
does not execute hot traffic, verify its own output, grant authority, modify an
ACTIVE operator, or decide production admission.

## 3. The Core Distinction

Three states must not be collapsed into one word:

```text
INDUCED
  at least one typed program explains observed evidence

IDENTIFIED
  all complete surviving programs belong to one semantic equivalence class

AUTHORIZED
  an independent post-freeze execution transferred and external admission
  granted a narrow immutable lease
```

One observation can induce and sometimes identify a program. It cannot by
itself prove transfer to an independent case.

## 4. Identification Versus Grokking

Version-space collapse is the practical readiness condition, but it is not
automatically a scientific grokking claim.

```text
Identification convergence
  several candidate programs -> one semantic class

Circuit grokking
  partial cross-plane relation waves
  -> phase-coherent connected circuit
  -> causal phase controls fail without the phase mechanism
  -> future transfer after exact episodic authority cleanup
```

Both routes may produce the same `IdentifiedOperatorCandidate` interface.
Only the second route may claim circuit-level grokking, and only after phase
ablation, cleanup, transfer, and zero false accepts.

For a simple trace that already contains a complete typed law, use the claim
`one-shot operator identification`, not `natural circuit grokking`.

## 5. End-To-End Architecture

```text
completed trace
-> immutable capture and verifier outcome
-> OperatorObservation adapter
-> typed candidate generation
-> VersionSpaceArena
-> exact evidence elimination
-> SemanticProgramQuotient
-> IdentificationDecision
   |
   +-- Empty
   |     -> MODEL_REVISION
   |
   +-- Ambiguous
   |     -> DistinguishingProbePlanner
   |     -> MissingEvidenceContract
   |     -> next verified observation
   |
   +-- Identified
         -> CandidateFreezeReceipt
         -> immutable support watermark
         -> independent future observation
         -> CPU shadow actor
         -> independent verifier
         -> TransferProofReceipt
         -> crystallizer / VM artifact
         -> external admission
         -> ACTIVE_NARROW | BLOCK
```

Every subsequent ACTIVE execution remains:

```text
Phase Router
-> unique Role Grounder result
-> Operator VM
-> independent Verifier
-> EMIT | FALLBACK
```

## 6. Existing Components To Reuse

This is not a second learner.

```text
crates/nando-operator-learning/src/version_space.rs
  existing bounded AST arena, survivor set, phase ranking, exact-check budget

crates/nando-operator-learning/src/cegis.rs
  existing counterexample classification and repair actions

crates/nando-operator-learning/src/generation_evidence_v3/
  existing support/future hash-chain ledger and immutable freeze

crates/nando-core/src/wave/operator_blueprint.rs
  existing role alignment, circuit synthesis, future phase evaluation

crates/nando-response-actor/src/crystallized_operator.rs
  existing actor/verifier-bound crystallization and restart bundle

crates/nando-operator-proof/
  existing independent verifier and proof ownership

crates/nando-operator-admission/
  sole authority owner
```

The current `LiveScalarShadowState` must become an adapter to this core. It
must not own a second private interpretation of support readiness.

## 7. Target Module Tree

The new orchestration owner lives only in the cold learning crate:

```text
crates/nando-operator-learning/src/operator_identification/
├─ mod.rs
├─ observation.rs
├─ state.rs
├─ semantic_quotient.rs
├─ inquiry.rs
├─ freeze.rs
├─ checkpoint.rs
└─ report.rs
```

Responsibilities:

```text
observation.rs
  converts verified domain evidence into source-neutral OperatorObservation

state.rs
  owns the identification state machine and calls the existing VersionSpaceArena

semantic_quotient.rs
  groups only programs proven behaviorally equivalent under canonical contracts

inquiry.rs
  selects or describes the cheapest next distinguishing observation

freeze.rs
  creates an immutable CandidateFreezeReceipt after identification

checkpoint.rs
  bounded deterministic cold-state persistence and restart parity

report.rs
  diagnostics, economics, blockers, survivor explanations; no authority
```

Pure semantic fingerprinting belongs in `nando-operator-kernel`, not in the
learner:

```text
CanonicalEffectLawV3
+ role relation schema
+ typed transform / composition program
+ renderer behavior
+ verifier contract
-> ProgramSemanticClassId
```

Unknown equivalence remains separate. Two programs are never merged merely
because they produced the same text on observed examples.

## Live Result

Snapshot: `2026-07-23`, after release deployment.

```text
request-event identity                    PASS
bounded candidate search                  COMPLETE
semantic version-space collapse           PASS
candidate freeze                          PASS
independent transfer proof                PASS
4032-byte crystallized page               PASS
external composite gate                   PASS
ACTIVE response packages                  1
active package                            crystallized-scalar-2069a1d9b37eca4f
real Nginx -> CPU executions              PASS
actor output                              wait(cell_id)
false accepts                             0
runtime parity mismatches                 0
M3                                       WATCH
```

The verified live route is:

```text
Nginx /v2 request
-> RequestEventId
-> Phase Router
-> crystallized scalar package
-> runtime role binding
-> typed actor
-> independent verifier
-> local_accept=true
-> verified economics receipt
```

The hot runtime may refresh an expiring cached lease only by re-reading the
immutable external admission receipt. It cannot mint, widen, or self-renew
authority without that receipt.

## 8. Core Data Contracts

### 8.1 OperatorObservation

```rust
struct OperatorObservation {
    observation_id: ObservationId,
    lineage_id: LineageId,
    event_id: EventId,
    generation_id: GenerationId,
    pre_action_relation_root: Sha256,
    observed_action_root: Sha256,
    observed_delta_root: Sha256,
    outcome: LearningOutcome,
    role_surface: BoundedRoleSurface,
    proof_receipt_root: Sha256,
}
```

Rules:

- raw names, request text, and payload values are not semantic identity;
- raw material may remain in a bounded capture archive for replay;
- censored outcomes are accounted but do not eliminate or reinforce programs;
- support and future lineages are disjoint;
- duplicate event, request, or receipt roots are rejected.

### 8.2 LearningOutcome

```text
VerifiedPass
  exact positive consistency evidence

ApplicabilityNegative
  the law must not apply in this pre-action state

HardContradiction
  the current representation or program family is wrong

Censored(reason)
  unknown; no semantic update
```

### 8.3 VersionSpaceState

```rust
enum VersionSpaceState {
    Collecting,
    Empty(ModelRevisionReason),
    Ambiguous(AmbiguityReport),
    Identified(IdentifiedSemanticClass),
    Exhausted(SearchExhaustion),
    Contradicted(ContradictionReport),
}
```

`Exhausted` is not `Identified`. One remaining program is not a winner if the
candidate generator or exact search stopped before completeness.

### 8.4 SemanticProgramQuotient

```rust
struct SemanticProgramClass {
    class_id: ProgramSemanticClassId,
    members: BoundedProgramIds,
    canonical_effect_law: CanonicalEffectLawV3,
    role_schema_root: Sha256,
    executable_artifact_root: Sha256,
    verifier_contract_root: Sha256,
}
```

Programs may share a semantic class only when their physical differences are
carried as explicit protocol facets or when canonical equivalence is proven.

Example:

```text
semantic effect:
  continue one pending execution

protocol modes:
  wait(cell_id, budgets)
  write_stdin(session_id, chars="", budgets)

different effect:
  write_stdin(session_id, chars!="", budgets)
```

The shared effect law does not erase protocol-specific runtime binding.

### 8.5 AmbiguityReport

```rust
struct AmbiguityReport {
    surviving_programs: usize,
    surviving_semantic_classes: usize,
    unresolved_relations: BoundedRelationDifferences,
    unresolved_roles: BoundedRoleDifferences,
    search_complete: bool,
    cheapest_probe: Option<MissingEvidenceContract>,
}
```

This report replaces `support_below_N`.

### 8.6 MissingEvidenceContract

```rust
struct MissingEvidenceContract {
    competing_class_roots: BoundedClassRoots,
    required_observable_difference: RelationPredicate,
    accepted_source: EvidenceSourceContract,
    expected_partition_gain: u32,
    estimated_cost: ProbeCost,
    stable_tie_break_sha256: Sha256,
}
```

It describes evidence; it does not fabricate authority evidence.

### 8.7 CandidateFreezeReceipt

```rust
struct CandidateFreezeReceipt {
    generation_id: GenerationId,
    semantic_class_id: ProgramSemanticClassId,
    canonical_program_root: Sha256,
    support_partition_root: Sha256,
    support_watermark: CaptureSequence,
    search_completion_root: Sha256,
    eliminated_class_root: Sha256,
    applicability_scope_root: Sha256,
    freeze_root: Sha256,
}
```

Only `operator_identification::freeze` constructs this receipt. It remains a
candidate capability and carries no execution authority.

### 8.8 TransferProofReceipt

Owned by `nando-operator-proof`:

```rust
struct TransferProofReceipt {
    freeze_root: Sha256,
    independent_lineage_root: Sha256,
    bound_role_environment_root: Sha256,
    actor_action_root: Sha256,
    observed_delta_root: Sha256,
    verifier_receipt_root: Sha256,
    parity: ExactParity,
    applicability_scope_root: Sha256,
}
```

This is the input to crystallization/admission. Admission does not infer
readiness from `Vec::len()`.

## 9. Identification Algorithm

### 9.1 First Observation

```text
verified observation
-> enumerate typed programs allowed by the VM grammar
-> intern programs into VersionSpaceArena
-> evaluate exact consistency
-> eliminate impossible programs with explicit reasons
-> compute semantic quotient
```

Decision:

```text
0 classes
  -> Empty / model revision

1 class and complete search
  -> Identified

more than 1 class
  -> Ambiguous

incomplete bounded search
  -> Collecting or Exhausted, never Identified
```

### 9.2 Additional Support

Each non-censored observation updates all surviving classes:

```text
positive mismatch
  -> eliminate member or entire class

applicability false accept
  -> strengthen guard, add anti-center candidate, or split class

hard contradiction
  -> invalidate representation, repair grammar, or create generation g+1

verified agreement
  -> preserve class; do not increase truth merely because it repeated
```

A duplicate observation may improve operational confidence metrics but has
zero identification gain and cannot trigger freeze.

### 9.3 Readiness

Freeze is allowed only when all conditions hold:

```text
candidate generation complete within declared budgets
semantic class count == 1
at least one verified support observation exists
class has compilable typed VM behavior
runtime role binding is decidable inside a bounded applicability scope
all known applicability negatives reject
no unresolved hard contradiction
support ledger is valid and bounded
```

There is no minimum support row count beyond the existence of real evidence.

### 9.4 Future Transfer

After freeze, support is immutable.

The first independent eligible observation is executed on CPU in shadow:

```text
new lineage
-> Phase Router candidate
-> unique Role Grounder result
-> VM execution
-> independent Verifier
```

Outcomes:

```text
VerifiedPass
  -> TransferProofReceipt
  -> narrow authority candidate

Runtime or verifier ABSTAIN
  -> retain candidate and emit MissingEvidenceContract

ApplicabilityNegative
  -> narrow the next-generation guard; no false accept

HardContradiction
  -> reject freeze, split/repair in generation g+1

Censored
  -> no semantic conclusion
```

If the verifier can establish truth without teacher output, this first future
case may support authority immediately. If truth is available only after the
teacher/LLM result, the case remains shadow proof and CPU replacement starts on
the following matching event.

## 10. Distinguishing Probe Planner

The planner operates only when multiple semantic classes survive.

For a proposed probe `p`:

```text
guaranteed_gain(p) =
  minimum number of semantic classes eliminated across possible outcomes

score(p) =
  guaranteed_gain(p) / estimated_cost(p)
```

Stable SHA-256 ordering breaks exact ties.

Sources:

```text
passive live traffic
  wait for a naturally occurring surface satisfying the contract

existing sealed corpus
  discovery only; cannot become post-freeze authority

development harness
  controlled probe; permanently excluded from natural claims

external causal inquiry
  requests a missing observable relation, not a desired answer
```

Wave may rank probes and hypotheses. It cannot turn an exact inconsistency into
truth or grant authority.

## 11. Narrow Authority

Authority begins narrow and expands generationally.

Example:

```text
effect law:
  continue pending execution

initial applicability:
  exactly one pending continuation handle
  advertised wait capability exists
  role binding is unique
  no mutable input payload
```

The first ACTIVE generation handles only this scope. More verified evidence may
produce generation `g+1` with a larger basin. Generation `g` remains immutable.

Every hot execution still requires:

```text
route margin
+ unique role binding
+ guard
+ VM result
+ independent verifier
```

Failure returns `FALLBACK`; it never teaches the current ACTIVE generation in
place.

### 11.1 Scope Subsumption And Operator Lifecycle

A broader operator never silently deletes or overrides a narrower ACTIVE
operator.

```text
wait_v1
  scope: one uniquely bound pending handle
  status: ACTIVE_NARROW

continue_execution_v2
  scope: several independently proven continuation protocol modes
  status: SHADOW until transfer and overlap proof pass
```

The broader law may use the narrower operator as a verified opcode:

```text
continue_execution_v2
-> choose protocol mode
-> CALL_OPERATOR(wait_v1)
```

Alternatively, it may subsume the narrow scope only after both generations
produce one action-equivalence class on the complete overlap contract. The old
generation then becomes `SHADOWED_BY(v2)`, not deleted, and remains available
for rollback and bounded fallback.

Dispatch across overlapping ACTIVE scopes is deterministic:

```text
collect applicable ACTIVE generations
-> retain uniquely grounded and independently verifiable candidates
-> choose the most specific proven applicability scope
-> if tied candidates are action-equivalent, choose the newest admitted generation
-> otherwise ABSTAIN / provider fallback
```

Authority therefore belongs to:

```text
operator generation + immutable applicability scope + authority lease
```

It never belongs to a semantic family name or to the widest known operator.

## 12. Example: wait

First trace:

```text
before:
  one pending execution with handle H1

action:
  wait(cell_id=H1)

after:
  continuation output observed
```

Candidate space may contain:

```text
literal H1
first identifier in text
first numeric token
unique pending-handle role
```

Exact structural constraints eliminate literal and unrelated selectors.

If one semantic class remains:

```text
FIND unique role pending_handle
-> CALL advertised wait capability with pending_handle
```

the candidate freezes after this single support observation.

Second independent session:

```text
different handle H2
different event and request roots
same structural role
-> CPU shadow emits wait(H2)
-> verifier PASS
-> ACTIVE_NARROW candidate
```

If several selectors still survive, the engine asks for the exact distinction
instead of waiting for an arbitrary number of repeated waits.

## 13. Example: ababab

Observation:

```text
ababab -> a
```

Surviving classes may include:

```text
period two
copy offset two
toggle finite state
repeat literal block "ab"
```

Repeating the same example has zero identification gain. The planner requests
an observation where predictions diverge. Freeze occurs only after the
semantic quotient contains one class.

## 14. Example: Rich Multi-Role Operator

For:

```text
read two values
-> bind each to the role referenced by the request
-> render them in request order
```

one trace may leave competing role permutations. Support continues until
relation evidence resolves:

```text
source role
request-reference ordinal
output role
renderer slot
```

The number of observations is an output of this process. The module reports the
remaining role permutations and the next required distinction.

## 15. Budgets

Budgets bound resources; they do not certify knowledge.

Existing starting budgets may remain:

```text
AST nodes                    <= 100,000
program depth                <= 4
complete candidates          <= 4,096
exact checks per work slice  <= 32
evidence rows per partition  <= 2,048
```

Important distinctions:

```text
exact_checks_per_slice = 32
  scheduling batch size; not evidence readiness

[u8; 32]
  SHA-256 representation; not evidence readiness

OperatorPage32
  page format name; not evidence readiness

evidence capacity
  memory ceiling; reaching it produces Exhausted/ABSTAIN
```

No code may interpret a storage capacity or work-slice size as proof.

## 16. Metrics

Primary identification metrics:

```text
observations_to_first_candidate
observations_to_identification
semantic_classes_remaining
programs_per_semantic_class
information_gain_per_observation
duplicate_zero-gain_observations
distinguishing_probes_requested
probe cost and latency
search completion state
```

Transfer and product metrics:

```text
identified candidates
independent transfer passes
ACTIVE_NARROW packages
verified CPU accepts
verified input tokens saved
fallback rate
false accepts
runtime parity mismatches
verifier cost
p50 / p95 / p99 hot latency
```

Economics may prioritize which ambiguous family receives learner CPU:

```text
expected verified token savings
--------------------------------
remaining inquiry and proof cost
```

Economics changes scheduling only. It cannot change semantic truth.

## 17. Failure Modes

```text
zero surviving classes
  MODEL_REVISION

multiple classes
  AMBIGUOUS + MissingEvidenceContract

one survivor but incomplete search
  SEARCH_INCOMPLETE

one syntactic program with unknown semantic equivalence
  AMBIGUOUS

role-binding tie
  ABSTAIN + role distinction request

support/future lineage overlap
  REJECT

future contradiction
  SPLIT / REPAIR / REVOKE candidate

censored future
  UNKNOWN; no anti-center

budget exhausted
  EXHAUSTED; no freeze

verifier unavailable
  FALLBACK; no authority
```

## 18. Ownership Boundaries

```text
nando-transition-serving
  capture only; no law decision

nando-operator-learning
  candidate generation, version space, quotient, inquiry, candidate freeze

nando-core Wave
  phase representation, circuit synthesis, coherence and ranking

nando-operator-kernel
  canonical semantic contracts and VM IR

nando-operator-runtime
  role grounding and deterministic VM execution

nando-operator-proof
  independent transfer and execution proof

nando-operator-admission
  sole authority owner

nando-response-actor
  thin integration facade; no private second learner
```

Forbidden:

- `LiveScalarShadowState` deciding readiness from row counts;
- CEGIS selecting the first passing program while other semantic classes live;
- phase score overriding exact inconsistency;
- diagnostics or reports constructing freeze/proof capabilities;
- admission deriving truth from raw vector lengths;
- support evidence reused as future;
- a new Rust branch per learned operator.

## 19. Required API Changes

Extend the existing arena rather than replacing it:

```rust
impl VersionSpaceArena {
    fn apply_observation(&mut self, observation: &OperatorObservation)
        -> EvidenceUpdateReport;

    fn search_completion(&self) -> SearchCompletion;

    fn semantic_classes(&self) -> &[SemanticProgramClass];

    fn identification_state(&self) -> VersionSpaceState;
}
```

The existing CEGIS coordinator should consume `VersionSpaceState`:

```text
Ambiguous
  -> counterexample or probe planning

Identified
  -> freeze candidate

Contradicted
  -> repair/split
```

The live adapter submits observations and reads decisions. It does not inspect
raw survivor counts to invent its own readiness rule.

## 20. Verification Matrix

Focused causal tests:

```text
one unique support observation
  -> Identified

one support + one independent future for wait
  -> TransferProofReceipt

same support repeated many times
  -> no semantic progress

ababab repeated
  -> remains Ambiguous

one distinguishing witness
  -> semantic class count decreases

two syntax-different equivalent programs
  -> one proven semantic class

unknown equivalence
  -> remains two classes

incomplete candidate generation with one current survivor
  -> no freeze

censored observation
  -> byte-identical semantic state

applicability negative
  -> guard/anti-center update, no false accept

support/future lineage reuse
  -> reject

tampered freeze or verifier receipt
  -> reject

checkpoint restart
  -> byte-identical identification decision

no-phase / shuffled-phase controls
  -> no grokking claim when phase is causal
```

Product gate:

```text
real wait trace
-> identified candidate
-> independent live future
-> CPU shadow
-> independent verifier PASS
-> external admission
-> ACTIVE_NARROW
-> verified token counter increases
-> false accepts = 0
-> parity mismatches = 0
```

## 21. Implementation Plan

### R0. Owner approval and baseline

- accept this document or amend it before code;
- preserve the current dirty experiment without committing it;
- record the last clean focused baseline;
- classify every literal `32` as evidence gate, work-slice budget, storage cap,
  digest width, or page-format name.

STOP-R0:

```text
algorithm accepted
fixed evidence-count readiness forbidden
code unchanged
```

### R1. Semantic quotient

- add canonical `ProgramSemanticClassId` in the kernel;
- group only proven equivalent programs;
- preserve unknown equivalence as separate classes;
- test effect law versus protocol-mode separation.

STOP-R1:

```text
quotient unit proofs PASS
no runtime or authority callers
```

### R2. Evidence-driven VersionSpaceArena

- add observation-level elimination;
- expose complete/incomplete search state;
- expose semantic classes and elimination reasons;
- remove first-passing-program winner semantics.

STOP-R2:

```text
duplicate evidence has zero gain
distinguishing evidence reduces classes
incomplete search cannot identify
```

### R3. OperatorIdentificationMachine

- implement the explicit state machine;
- reuse `GenerationEvidenceLedgerV3`;
- create opaque `IdentifiedOperatorCandidate`;
- create deterministic reports and checkpoint.

STOP-R3:

```text
one-shot simple identification PASS
ambiguous sequence remains AMBIGUOUS
restart parity PASS
authority false
```

### R4. Active inquiry

- emit `MissingEvidenceContract`;
- implement guaranteed partition-gain scoring;
- stable hash tie-break;
- support passive live opportunities and development-only probes.

STOP-R4:

```text
probe separates a fixture version space
repeated non-distinguishing evidence is not selected
```

### R5. Live scalar adapter

- replace private count-based readiness in `operator_live_shadow`;
- preserve extraction and circuit synthesis;
- route observations into `OperatorIdentificationMachine`;
- retain all existing capture and privacy boundaries.

STOP-R5:

```text
no `support_below_32` or replacement magic count in the natural route
wait identifies from the evidence actually needed
no admission candidate yet
```

### R6. Independent future transfer

- freeze support at identification;
- send the first independent matching case through CPU shadow;
- independently reconstruct actor, role binding, and verifier;
- emit opaque `TransferProofReceipt`.

STOP-R6:

```text
1 support + 1 future wait control PASS
lineage reuse REJECT
tamper REJECT
authority false
```

### R7. Crystallization and admission

- compile the identified class into versioned VM data;
- admission consumes proof capabilities, not row-count thresholds;
- activate only the proven applicability scope;
- preserve provider fallback.

STOP-R7:

```text
ACTIVE_NARROW wait package
real verified CPU accepts increase
false accepts 0
parity mismatches 0
```

### R8. Rich operators

- apply the same state machine to status, count, filter, multi-role projection,
  protocol continuation modes, and composition;
- collect extra evidence only for unresolved role/relation/program ambiguity;
- prioritize by verified token economics.

STOP-R8:

```text
no operator-specific Rust branch
all new behavior is VM data
CPU share increases from actual accepts
```

## 22. Documentation And Claim Updates

After owner approval and implementation:

- link this plan from `ARCHITECTURE_CANON.md`;
- update `docs/CORE.md` with the live identification state;
- mark fixed-count documents as historical, not canonical;
- show `semantic classes remaining` and `next missing evidence` on the control
  page instead of `support N/32`;
- never label shadow, candidate, or potential coverage as CPU execution.

## 23. Final Formula

```text
Observation proposes programs.
Exact evidence eliminates programs.
Semantic quotient identifies one law.
Active inquiry asks only for missing distinctions.
Wave ranks and, for distributed laws, forms the coherent circuit.
Freeze separates discovery from transfer.
The independent verifier proves the new case.
Admission grants narrow authority.
The VM executes on CPU.
```

The number of examples is a measured consequence of ambiguity, not an
architectural constant.

## 24. Implementation Snapshot

Status: controlled implementation complete through adaptive admission
candidate; production deployment and authority are not claimed.

```text
semantic quotient                         PASS
complete bounded version space            PASS
adaptive inquiry and stable tie-break      PASS
sealed candidate freeze                    PASS
singleton role anchor after identification PASS
1 support + 1 independent future control   PASS
mixed direct/enveloped provider surfaces   PASS
status/count/filter/composition/multi-role PASS
adaptive package proof basis               PASS
legacy 32+32 isolated as control           PASS
live HTML policy observability             DEPLOYED
running learner publishes adaptive policy  NO, legacy source visible
production ACTIVE_NARROW                   NOT DEPLOYED
```

The singleton role anchor cannot identify a law and cannot grant authority. It
only canonicalizes the already identified one-class support surface. The
post-freeze future still has to rebind, execute, verify, and seal the transfer
proof.

## 25. Restart Capture Invariant

An observer restart never replays an arbitrary session tail through the
capture owner:

```text
last committed source offset
-> skip the already committed row
-> censor the unfinished turn
-> next authoritative turn_context
-> fresh capture and learning resume
```

Reconstructing an old turn can help diagnostics, but it cannot create support,
future, or a new frame binding. The bounded loss of one partial turn is
preferable to rebinding an immutable frame or presenting restart replay as
independent transfer evidence.
