# Effect Law Unification Refactor V1

Status: canonical architecture and migration plan. No runtime or admission
authority is granted by this document.

Date: 2026-07-21 Europe/Tallinn

```text
R1 evidence decomposition       COMPLETE / STOP-A
F0 evidence completion          COMPLETE / STOP-F0
F1 diagnostic ownership        COMPLETE / STOP-F1
F2 canonical effect law         COMPLETE / STOP-F2 V3
F3 dual classification          COMPLETE / STOP-F3R
B1A binding version space       COMPLETE / INSUFFICIENT at STOP-B1A
B1B0 scientific preregistration COMPLETE / STOP-B1B0R
B1B-S support acquisition       COMPLETE / SUPPORT FROZEN
B1B-F0 future protocol          COMPLETE / PROTOCOL FROZEN
B1B-F future acquisition        COMPLETE / FUTURE FROZEN
B1B adjudication                CONTROLLED_FIXTURE_PASS / REVIEWED
B1B proof-owner split           REQUIRED / NOT STARTED
AcceptedBindingLawEvidence      BLOCK
F4 protocol mode compiler       BLOCKED
production authority            false
service deployment              out of scope
```

## 1. Objective

Replace competing semantic identities and surface-bound continuation selectors
with one canonical, versioned path:

```text
verified transition
-> CanonicalEffectLawV3
-> EffectLawId
-> bounded ProtocolMode set
-> runtime role grounding
-> CrystallizedOperator
-> independent verifier
-> external admission
```

The refactor must preserve the Nando boundary:

```text
Wave discovers, consolidates, and ranks applicability.
Structural binding determines runtime operands.
The typed actor executes.
The independent verifier establishes truth.
External admission alone grants authority.
```

The refactor is successful only if the final hot path is simpler than the
current one. It must not create a third actor, a third semantic signature, or
another generic correction/consensus core.

## 2. STOP-A Evidence Baseline

Canonical human report:

```text
plans/runtime-role-grounding-v1/R1_EVIDENCE_DECOMPOSITION_STOP_A.md
```

Canonical machine artifact:

```text
/home/ubu/tmp/nando-r1/continuation-evidence-stop-a.json
sha256 e9e43513bca355a0ec77588d995c1a77c11188d59d8b1b5fc7dea8b9b1f9e9d0
schema nando.semantic-law-evidence-audit.v1
```

Verified denominator:

```text
wait receipts                  96
write_stdin receipts           33
runtime-parity receipts       129
actor candidates                6
unexplained denominator rows    0
```

The target physical actor replayed across the 33 protocol-scoped
`write_stdin` receipts:

```text
EXACT          24
WRONG           3
ABSTAIN         6
VERIFY_FAILED   0
```

Those 33 rows are not one physical-signature ownership set. They contain 32
rows from the actor's declared member signature and one execution-budget-
equivalent row from a second physical signature. The actor's full 129-cell
matrix is `24 EXACT / 3 WRONG / 102 ABSTAIN / 0 VERIFY_FAILED`; the scoped
`24/3/6/0` result is obtained by filtering rows by protocol class, not by
claiming all 33 are owned by one signature.

Observed modes:

```text
PrefixPresentUniqueAligned      24 -> EXACT
PrefixPresentUniqueConflicting   3 -> WRONG
PrefixAbsent                     6 -> ABSTAIN
```

The three WRONG rows prove a binding defect:

```text
selector candidate count = 1
action shape              = correct
renderer                  = not causal
constant contract         = not causal
selected identity digest  != independently expected identity digest
```

The six `write_stdin` ABSTAIN rows and one current `wait` ABSTAIN row prove an
unobserved or unsupported runtime surface. They do not prove a negative
operator law and must not update an anti-center.

### 2.1 STOP-A count reconciliation

The canonical STOP-A machine artifact contains 728 unique
`missing_parity_frame_ids`; there are no duplicates. The 725 count in the human
report came from an earlier live bounded-pool snapshot and was transcribed
before the final STOP-A replay. Later F0 replay contained 714 such IDs as the
live pool continued to move. These are snapshot ages of censored evidence, not
changes to the fixed 129-row scored denominator.

Canonical STOP-A count: 728. Later live counts must carry their artifact SHA and
must never replace or be added to the canonical count.

STOP-F0 receipt:

```text
plans/effect-law-unification-v1/STOP_F0_EVIDENCE_COMPLETION.md
```

## 3. Root Architectural Defects

### 3.1 Competing semantic authority

The current code can answer "are these the same law?" through two different
representations:

```text
EffectGraph canonical digest
teacher_semantic_law_signature
```

Both mechanisms are locally useful, but they must not independently own
semantic equality. A graph can currently preserve topology while omitting an
effect-significant constant or state mutation that the semantic signature
retains. The two groupings can therefore disagree downstream.

### 3.2 Surface adapter hidden inside a semantic selector

`ContinuationHandle` currently recognizes two physical text prefixes in the
generic runtime and verifier. It is therefore not a source-neutral role by
itself. The physical prefix may be valid adapter data, but it cannot define the
canonical effect law.

### 3.3 Unique candidate is not a valid binding proof

STOP-A proves that one observed candidate can still be the wrong continuation
identity. A new selector must bind a role through structural and temporal
relations, not through candidate count, prefix presence, field name, raw value,
or deterministic ordering.

### 3.4 Parallel execution authority

The semantic path executes `UniqueConsensus`, while the crystallized-operator
path already owns bounded role search and canonical action equivalence. Adding
another executor would preserve the defect. New V3 packages must converge on
the existing crystallized path.

### 3.5 Diagnostic ownership drift

R1 necessarily added a large read-only audit to `online_state.rs` and a
selector-count helper to `runtime.rs`. This is acceptable only as temporary
diagnostic plumbing. Evidence reporting must not become a production semantic
owner.

## 4. Single-Authority Identity Model

Four identities must remain distinct:

```text
PhysicalProgramId
  Exact wire action identity. Names, transport, argument schema, and semantic
  constants remain visible. Used for exact parity and adapter ownership.

TransferFamilyId
  Source-neutral mining hint. It may share search pressure across physical
  programs. It never groups production authority.

EffectLawId
  The only authority for semantic-law equality. Derived only from canonical
  CanonicalEffectLawV3 bytes.

OperatorPackageId
  Identity of the complete executable artifact: law, modes, role program,
  actor, verifier, proof roots, resource budget, and generation.
```

The current `teacher_semantic_law_signature` becomes either:

1. a compatibility view derived from `EffectLawId`; or
2. a diagnostic-only legacy value during dual-run migration.

It must not remain an independent grouping authority.

## 5. Canonical Effect Law Contract

Proposed source-neutral contract:

```rust
struct CanonicalEffectLawV3 {
    schema: EffectLawSchema,
    topology: CanonicalEffectGraph,
    role_schema: Box<[EffectRole]>,
    semantic_facets: Box<[SemanticFacet]>,
    preconditions: Box<[EffectPredicate]>,
    postconditions: Box<[EffectPredicate]>,
    preserved_frame: Box<[FramePredicate]>,
}
```

This is a contract shape, not permission to introduce an unbounded DSL. Every
collection is bounded and deterministically ordered. Unknown, ambiguous,
incomplete, or over-budget canonicalization returns no `EffectLawId`.

`EffectLawId` is computed as:

```text
sha256(domain_separator || canonical_versioned_bytes(CanonicalEffectLawV3))
```

Canonical bytes must not contain:

- raw request or response text;
- concrete continuation values;
- field, function, or argument names used only by a physical adapter;
- source paths, timestamps, frame IDs, or evidence IDs;
- hash-map iteration order;
- Wave score, threshold, or winner identity.

Canonical bytes must retain every effect-significant distinction:

- role type and cardinality;
- relation topology and data flow;
- semantic constants;
- input mutation versus no-op polling;
- continuation versus termination;
- state and preserved-frame postconditions.

Required identity examples:

```text
wait(handle)                         same effect as write_stdin(handle, chars="")
write_stdin(handle, chars="")       different from non-empty chars
write_stdin(handle, chars="x")      input mutation remains semantic
terminate=true                       different from continuation polling
direct versus wrapped transport      same effect when relations are equal
role/field rename                     same effect
changed preserved frame              different effect
```

These are effect-level identities. `wait` and `write_stdin` remain different
physical protocol modes and different physical action classes.

## 6. Protocol Mode Contract

An effect law can be implemented by several physical protocols:

```rust
struct ProtocolMode {
    effect_law_id: EffectLawId,
    source_role_schema: Box<[SourceRole]>,
    selector_program: SelectorProgram,
    observed_value_type: AtomValueType,
    emitted_value_type: AtomValueType,
    capability_protocol: CapabilityProtocol,
    argument_role_schema: Box<[ArgumentRole]>,
    constant_contract: ConstantContract,
    structural_guard: StructuralGuard,
    temporal_contract: TemporalContract,
    cardinality_contract: CardinalityContract,
}
```

Physical function names, argument names, prefixes, and JSON paths may appear in
a capability/surface binding receipt. They must not become unconditional
fillers in the effect law.

Constants require explicit classes:

```text
semantic constant       non-empty chars, terminate=true, transformation value
protocol no-op constant empty chars for polling
execution budget        max_tokens, max_output_tokens, timeout/yield values
transport default       adapter-defined default with a versioned capability
```

Only semantic constants participate in `EffectLawId`. All classes participate
in exact physical parity and the complete `OperatorPackageId`.

## 7. Binding Provenance Required Before Selector Design

STOP-A identifies the symptom but not the correct structural binding law. F0
must produce a privacy-safe candidate provenance for all exceptional rows.

For each of the three WRONG rows, enumerate every bounded continuation-like
candidate visible to the runtime extractor and record only structural data:

```text
message/turn distance
content-part ordinal
tool-call relation class
source event class
protocol/capability class
value type
candidate digest
expected-digest match boolean
temporal and cardinality relations
```

For the six `write_stdin` and one `wait` ABSTAIN rows, determine:

```text
expected identity structurally present elsewhere
expected identity absent from captured surface
surface unsupported by the current extractor
environment/capture unavailable
```

Raw values remain forbidden. If the available receipts cannot distinguish the
correct binding, the result is `INSUFFICIENT_BINDING_EVIDENCE`; no ordinal,
latest-item, or prefix rule may be invented.

## 8. Runtime Outcome Matrix

Do not collapse all failures into one ABSTAIN counter. Each mode by receipt
must contain two independent dimensions:

```rust
enum GuardOutcome {
    NotApplicable,
    UniqueBinding,
    AmbiguousBinding,
    Invalid,
    Censored,
}

enum ExecutionOutcome {
    Exact(ActionClassId),
    Wrong(ActionClassId),
    VerifyFailed,
    NotExecuted,
}
```

An admissible mode cover requires:

```text
every positive receipt has at least one Exact
every applicable mode has Wrong = 0
every applicable mode has VerifyFailed = 0
all simultaneous modes produce one normalized action class, or capability
binding makes exactly one physical mode applicable
all applicability negatives remain non-executable
search completion is Complete, not merely within a lucky prefix of the search
```

A deterministic hash or lexical tie-break can order candidates. It cannot
grant semantic or execution authority.

## 9. Target Dependency Direction

```text
contracts
  -> effect_graph
      -> effect_law
          -> protocol_mode compiler
              -> crystallized_operator

contracts + effect-law IR
  -> independent verifier

verified receipts
  -> external admission

online_state/evidence adapters
  -> compiler inputs only

proof fixtures
  -X-> production runtime authority
diagnostic reports
  -X-> admission authority
Wave score
  -X-> invalid structural binding
```

Module ownership after migration:

| Module | Sole responsibility |
|---|---|
| `effect_graph.rs` | bounded canonical relation topology |
| `effect_law.rs` | canonical semantic effect and `EffectLawId` |
| `protocol_mode.rs` | mode IR, selector IR, capability and constant contracts |
| `teacher_join.rs` | physical identity and legacy compatibility |
| `semantic_alias.rs` | evidence over `EffectLawId`, not independent semantics |
| `online_state.rs` | bounded evidence lifecycle and generation ownership |
| `crystallized_operator.rs` | complete role binding and action equivalence |
| `runtime.rs` | execution of an already bound operator |
| `verifier.rs` | independent reconstruction and verification |
| `online_admission.rs` | final external authority |

## 10. Migration Sequence

This track is named F0-F8 to avoid collision with the executor's completed R1
and the older runtime-role-grounding R1-R8 plan.

### F0: Reconcile And Complete Binding Evidence

Inputs:

- STOP-A report;
- 129-row machine matrix;
- 728 machine-listed missing parity frame IDs;
- three WRONG, six `write_stdin` ABSTAIN, one `wait` ABSTAIN.

Work:

1. Reconcile 725 versus 728 censored rows.
2. Add privacy-safe structural candidate provenance.
3. Classify each exceptional row without selecting a repair rule.
4. Preserve all censored outcomes as unknown, never as anti-centers.

STOP-F0 receipt:

```text
all count discrepancies explained
all exceptional rows structurally classified
correct binding law identifiable, or explicit INSUFFICIENT_BINDING_EVIDENCE
authority false
```

No EffectLaw or selector implementation is allowed before STOP-F0.

### F1: Extract Diagnostic Ownership

Work:

1. Move the R1 audit implementation out of `online_state.rs` into a dedicated
   read-only diagnostic module.
2. Keep production state access bounded and immutable.
3. Move diagnostic selector introspection out of the generic runtime owner, or
   clearly mark an internal adapter-only API with no authority call path.
4. Freeze golden STOP-A JSON before movement and require byte-identical output.

STOP-F1 receipt:

```text
diagnostic JSON byte-identical
runtime behavior unchanged
no diagnostic -> authority path
module ownership gate PASS
```

Completed receipt:

```text
plans/effect-law-unification-v1/STOP_F1_DIAGNOSTIC_OWNERSHIP.md
```

### F2: Implement CanonicalEffectLawV3

Completed receipt:

```text
plans/effect-law-unification-v1/STOP_F2_CANONICAL_EFFECT_LAW_V3.md
```

Work:

1. Add the bounded canonical IR and deterministic serializer.
2. Refactor `EffectGraphBuilder` to expose topology without deciding semantic
   equality by itself.
3. Build semantic facets from independently verified transition relations.
4. Derive `EffectLawId` from the complete IR.
5. Keep all legacy signatures available for shadow comparison only.

Required tests:

- alpha/wire rename invariance;
- atom and map order invariance;
- poll versus input mutation separation;
- continuation versus termination separation;
- preserved-frame separation;
- ambiguous/incomplete/over-budget fail-closed behavior;
- deterministic serialization and restart digest.

STOP-F2 receipt:

```text
identity matrix PASS
canonical bytes deterministic
legacy runtime unchanged
authority false
```

### F3: Dual-Run Semantic Classification

Work:

1. Compute legacy semantic signature and V3 `EffectLawId` for the same verified
   rows.
2. Produce explicit legacy-to-V3 merge/split maps.
3. Explain every disagreement using retained structural facets.
4. Keep `SemanticAliasGraph` authority on V1 during this phase.

Required report:

```text
legacy cohort -> V3 cohorts
V3 cohort -> physical programs
false legacy merges
unexpected V3 splits
unknown/censored rows
```

STOP-F3 receipt: no unexplained merge or split. If disagreement cannot be
resolved from verified evidence, V3 remains shadow.

Completed receipts:

```text
plans/effect-law-unification-v1/STOP_F3_DUAL_CLASSIFICATION_V1_V3.md
plans/effect-law-unification-v1/STOP_F3R_PAIRWISE_DISCREPANCY_REPAIR.md
```

### B1A: Freeze Binding Version Space

Work:

1. Build `PreActionBindingSurfaceV1` without expected response, teacher action,
   post-state, or expected value digest.
2. Enumerate bounded label-blind candidates and structural relations.
3. Freeze every candidate graph before joining expected binding receipts.
4. Evaluate the exact bounded hypothesis space without Wave or thresholds.
5. Emit an identifiable action-equivalence class or explicit
   `INSUFFICIENT_BINDING_EVIDENCE` with all ties and distinguishing probes.

Completed receipt:

```text
plans/effect-law-unification-v1/STOP_B1A_BINDING_EVIDENCE.md
plans/effect-law-unification-v1/STOP_B1A_BINDING_EVIDENCE.json
```

STOP-B1A result:

```text
frozen denominator                    129 / 129
exceptional rows                       10 / 10
complete hypotheses                     0
unresolved ties                        86 / 86
distinguishing probes                  86 / 86
verdict                                INSUFFICIENT_BINDING_EVIDENCE
missing discriminator                  PROVEN
resolving causal relation              UNKNOWN
candidate relation H1                  parent_action_to_capability_instance / UNPROVEN
selector / ProtocolMode / authority    NOT CREATED
```

### B1B: Acquire Causal Binding Evidence

B1B is split by a physical support freeze. STOP-B1B0R preregisters H0
`relation_not_observable`, H1 `parent_action_to_capability_instance`, and six
causal interventions without choosing either hypothesis. New traces must
determine which, if any, pre-action relation separates the expected action
class from its competitors while names, values, order, prefixes, and layout
are held constant or adversarially varied.

Teacher or expected action data remains evaluation-only and must not enter
candidate extraction. B1B may produce a new trusted evidence package; it may
not compile a selector or `ProtocolMode`.

The mandatory acquisition order is:

```text
B1B-S controlled label-blind support capture
-> independent capture owner seals exact index prefix
-> STOP-B1B-S checkpoint
-> B1B-F0 freezes source, slots, sessions, challenges, and budgets
-> B1B-F new post-freeze future capture
-> trusted label resolver and H0/H1 adjudication
-> STOP-B1B
```

STOP-B1B-S froze 12 controlled pre-action rows across four session lineages,
with two rows for each I1-I6 intervention. The owner independently validated
the canonical evidence-ledger record, capture receipt, capture-index membership,
frozen candidate graph, and exact watermark. Expected labels are deliberately
not joined at this stage, so positive/negative denominators remain pending the
trusted resolver; H0 and H1 remain unproven. Future is not open.

STOP-B1B-F0 binds the future-only owner to the exact committed B1B-S freeze,
watermark, receipt, capture-index root, and sequence 12 boundary. It freezes 12
label-blind slots over four new session partitions, two rows for each I1-I6,
at least three unseen wire shapes, field-name disjointness from support, and an
ordinal/layout trap with the same candidate action set. No future row was
captured while defining or testing this protocol; test fixtures are proof-only.
Restart reconstruction requires the exact pinned support-freeze and watermark
artifacts plus a future receipt persisted outside the bundle, then repeats the
full prefix and structural-challenge checks.

STOP-B1B-F executed that protocol through two separate processes. The producer
published an exact capture-index extension before sending the bounded raw batch
through a pipe; the capture owner consumed the pipe, independently rebuilt all
candidate graphs, and persisted only commitments and privacy-safe structural
state. The result contains 12/12 post-watermark rows, four future-only session
lineages, two rows for each I1-I6 intervention, 12 distinct unseen shape roots,
and six ordinal/layout trap pairs. No expected label was joined, so H0 and H1
remain unproven and B1B adjudication is still a separate closed stage.

Preregistration receipt:

```text
plans/effect-law-unification-v1/STOP_B1B0_PREREGISTRATION.md
plans/effect-law-unification-v1/STOP_B1B0_PREREGISTRATION.json
plans/effect-law-unification-v1/STOP_B1B0R_TRUSTED_ACQUISITION_BOUNDARY.md
plans/effect-law-unification-v1/STOP_B1B0R_PREREGISTRATION.json
plans/effect-law-unification-v1/STOP_B1B_S_SUPPORT_FREEZE.md
plans/effect-law-unification-v1/STOP_B1B_S_SUPPORT_FREEZE.json
plans/effect-law-unification-v1/STOP_B1B_S_FREEZE.json
plans/effect-law-unification-v1/STOP_B1B_S_WATERMARK.json
plans/effect-law-unification-v1/STOP_B1B_F0_FUTURE_ACQUISITION_PROTOCOL.json
plans/effect-law-unification-v1/STOP_B1B_F0_FUTURE_ACQUISITION_FREEZE.md
plans/effect-law-unification-v1/STOP_B1B_F_ACQUISITION_REPORT.json
plans/effect-law-unification-v1/STOP_B1B_F_CAPTURE_REPORT.json
plans/effect-law-unification-v1/STOP_B1B_F_EXTERNAL_RECEIPT.json
plans/effect-law-unification-v1/STOP_B1B_F_FREEZE.json
plans/effect-law-unification-v1/STOP_B1B_F_FUTURE_FREEZE.md
```

`STOP-B1B0R` preserves the original no-run freeze and repairs its acquisition
boundary before any evidence is opened. A separately pinned capture watermark
must be an exact prefix of the later capture index. Support records must precede
that watermark and future records must follow it. Every label joins a concrete
capture receipt and indexed record. Support and future remain session-disjoint,
each partition requires at least three sessions, and one session cannot repeat
the same `(label, intervention)` vote.

`STOP-B1B-S` is support-only. Its checked-in watermark is the physical boundary
that B1B-F must extend. Reclassifying these support rows as future, replacing the
watermark, joining labels before all future graphs freeze, or opening F4 from
this receipt is forbidden.

`STOP-B1B-F0` is protocol-only. Its Rust causal fixtures exercise chronology,
lineage, topology, and leakage attacks but are not admissible rows. B1B-F must
run the frozen schedule without changing F0 after observing a future outcome.

`STOP-B1B-F` is future-only. Its checked-in external receipt and frozen bundle
bind the exact 12-row extension; they contain no expected action or adjudicated
hypothesis. Reopening acquisition, regenerating support, joining labels inside
the capture owner, or starting F4 from this receipt is forbidden.

STOP-B1B receipt:

```text
all B1A ties evaluated against newly captured causal evidence
H0 and H1 adjudicated from pre-action wire evidence
one action-equivalence class with wrong bindings = 0, or renewed INSUFFICIENT
applicability-negative denominator present in support and future
no selector / ProtocolMode / execution authority
```

STOP-B1B executed the registered controlled adjudication route over the
immutable B1B-S and B1B-F artifacts. Inside that fixture, a bounded synthetic
physical observer reproduced all 24 capture records and candidate graphs
byte-for-byte, executed every candidate against its reconstructed pre-action
scene, and emitted verifier receipts. A trust-owner path pinned the exact label
manifest before the adjudicator resolved it.

```text
support / future                 12 / 12
positive per partition            6 / 6
applicability-negative             6 / 6
I1-I6 predictions                  6 / 6 PASS
wrong bindings                     0
negative accepts                   0
parity failures                    0
H0 relation_not_observable         REJECTED
H1 parent_action_to_capability_instance CONTROLLED_SUPPORTED
independent physical truth         BLOCK
AcceptedBindingLawEvidence         BLOCK
F4                                 BLOCKED
authority                          false
```

Controlled receipt:

```text
plans/effect-law-unification-v1/STOP_B1B_CAUSAL_ADJUDICATION.md
plans/effect-law-unification-v1/STOP_B1B_PHYSICAL_LABEL_RECEIPTS.json
plans/effect-law-unification-v1/STOP_B1B_LABEL_MANIFEST.json
plans/effect-law-unification-v1/STOP_B1B_EXTERNAL_LABEL_TRUST.json
plans/effect-law-unification-v1/STOP_B1B_ADJUDICATION.json
```

This controlled result identifies a plausible missing relation but compiles no
runtime mode. The checked-in machine receipt retains its historical
`f4_status=UNLOCKED_NOT_STARTED` field byte-for-byte. Post-review, that field is
not compiler authority.

Required repair before F4:

```text
frozen graphs
-> independently observed PhysicalTrialReceipts
-> separate TrustedLabelResolver
-> CausalAdjudicator
-> private AcceptedBindingLawEvidence constructor
-> F4
```

The physical-trial owner cannot infer labels from intervention metadata or the
expected law. The resolver cannot rebuild frozen graphs. The adjudicator cannot
construct a selector or `ProtocolMode`. Until these ownership gates pass, F4 is
blocked.

### F4: Compile Bounded Protocol Modes

Work:

1. Compile physical programs under one `EffectLawId` into competing structural
   `ProtocolMode` candidates.
2. Derive selector and capability requirements from evidence; do not inject a
   manual answer rule.
3. Run complete bounded search and retain the completion receipt.
4. Build the two-dimensional guard/execution matrix.
5. Apply exact cover only to already safe structural modes.
6. Compare all minimal covers by normalized action equivalence.

STOP-F4 receipt:

```text
complete search
every positive covered
WRONG = 0
VERIFY_FAILED = 0
negative applicability accepted = 0
all admissible covers canonically action-equivalent, otherwise BLOCK
```

### F5: Converge On Crystallized Runtime

Work:

1. Compile V3 mode sets into the existing crystallized-operator route.
2. Use `RuntimeRoleBinder` for all runtime operand selection.
3. Require complete search and one action-equivalence class.
4. Bind physical capability symbols only from the advertised capability
   surface.
5. Keep Wave after structural validity as a bounded applicability/ranking
   mechanism.
6. Make `execute_unique_consensus` a V1 compatibility decoder; no new V3
   package may be emitted through it.

STOP-F5 receipt:

```text
renamed surface PASS
multiple mappings, same action PASS
multiple mappings, different actions ABSTAIN
missing capability ABSTAIN
search exhaustion ABSTAIN
Wave cannot override failed binding
```

### F6: Independent Verifier Convergence

The verifier receives raw bounded request/output evidence, immutable IR, and
the actor result. It must not trust actor-selected selectors, values, mappings,
or expected output.

Work:

1. Independently interpret the selector and effect contracts.
2. Reconstruct role candidates and temporal/cardinality relations.
3. Recompute the physical action class and effect postconditions.
4. Verify preserved frame and exact protocol parity.

STOP-F6 adversarial gate:

```text
actor selector mutation       REJECT
role swap                     REJECT
semantic constant mutation    REJECT
capability mutation           REJECT
duplicate candidate paths     ABSTAIN
missing expected role         ABSTAIN
false accepts                 0
parity mismatches             0
```

### F7: New Generation And Persistence

V3 uses a new schema and checkpoint namespace. Existing ACTIVE or frozen
packages are never reinterpreted in place.

Any change to these values creates a new generation:

```text
EffectLaw schema or bytes
ProtocolMode set
SelectorProgram
RoleGraph or RelationProgram
actor or renderer
verifier contract
capability version
```

Old receipts may contribute support only when their provenance remains valid.
They cannot be relabeled as post-freeze future evidence.

STOP-F7 receipt:

```text
new generation digest
fresh frozen future
support/future lineage disjointness
restart byte identity
no ambient episodic memory needed for execution
old generation unchanged
```

### F8: External Admission And Live Shadow

Admission independently recomputes or validates:

```text
EffectLawId
mode-set root
selector/capability roots
binding root
action-equivalence root
actor/verifier roots
support and future roots
phase-control roots
resource budget
```

Required causal controls:

```text
full phase
no phase
shuffled phase
magnitude only
matched random center
no Wave routing
```

Required product gate:

```text
false accepts             0
parity mismatches         0
wrong actions             0
restart mismatches        0
censored semantic updates 0
full phase provides measured search/applicability gain
```

The first deployment is SHADOW only. `nando-live-transition-gate` must return
PASS before a candidate is even eligible for authority. WATCH, VETO, or ERROR
blocks promotion. A final authority change remains a separate explicit action.

## 11. Adversarial Corpus

The refactor is not complete on copied production receipts alone. At minimum,
the frozen proof must include:

```text
renamed physical function and argument names
direct and wrapped transports
one valid handle plus unrelated scalars
two continuation-like handles
same value in distinct temporal positions
different values under one coarse layout
missing recognized prefix
new content-part layout
nested or repeated candidate paths
wait and empty write_stdin as effect-equivalent protocol modes
non-empty write_stdin as a different law
terminate=true as a different law
missing advertised capability
multiple advertised capabilities
actor/verifier mutation tests
restart and checkpoint tests
```

At least one future case must break any support-only ordinal or layout shortcut.

## 12. Resource And Hot-Path Budget

The hot runtime must not:

- build or canonicalize an `EffectGraph`;
- scan historical receipts;
- run exact-cover synthesis;
- load raw corpus text;
- invoke diagnostic replay;
- perform unbounded selector or role search.

Hot execution uses only the compact compiled package, bounded runtime surface,
role binder, actor, and verifier. Every search bound and package-size bound is
part of the package and admission receipt.

Per-phase verification policy:

```text
local host       read-only inspection, focused formatting/diff checks
remote host      focused Rust tests and bounded diagnostics
checkpoint       full tests, Clippy, restart parity
final F8         exact-commit Graphify, release gate, live structural gate
```

Remote build root:

```text
e@192.168.3.94:/home/e/projects/nando-wave-build
```

## 13. Rollback And Compatibility

Migration is dual-run, not big-bang:

```text
V1 runtime remains available
V3 starts with effect_law_v3_shadow = false/disabled authority
V1 and V3 state use separate schema namespaces
no ACTIVE generation is mutated
disable V3 to roll back
fallback path remains intact
```

Legacy semantic signatures and `UniqueConsensus` may be deleted only after:

1. F8 passes;
2. active/frozen V1 package inventory is zero or explicitly migrated through a
   new proof generation;
3. restart and rollback drills pass;
4. no code path uses them for grouping or execution authority.

Deletion is a separate cleanup commit, not part of the behavioral switch.

## 14. Commit And STOP Discipline

Each phase is one isolated behavioral or ownership slice. Do not combine:

- diagnostic extraction with semantic behavior;
- identity migration with runtime switching;
- runtime switching with verifier changes;
- generation migration with admission authority;
- cleanup/deletion with a new proof claim.

At every STOP, the executor must provide:

```text
exact source commit or dirty diff base
files and owners changed
machine-readable receipt paths and hashes
denominator and outcome matrix
focused test results
resource measurements
structural gate verdict
unresolved cases
authority state
background process state
```

The executor must stop on any unresolved WATCH, count mismatch, non-equivalent
cover, incomplete search, wrong action, verifier failure, or generation/proof
lineage ambiguity.

## 15. Comment Policy

Comments are required only at authority boundaries and non-obvious invariants:

- why one identity may or may not affect authority;
- why censored outcomes do not update semantic memory;
- why Wave cannot override structural validity;
- why old future evidence cannot be reused after a generation change;
- why actor and verifier implementations intentionally diverge.

Do not narrate ordinary assignments, iteration, or serialization.

## 16. Definition Of Done

The refactor is complete only when:

```text
one CanonicalEffectLawV3 owns semantic equality
EffectGraph is topology, not a competing semantic authority
legacy signatures no longer group V3 authority
structural protocol modes cover verified surfaces without WRONG
RuntimeRoleBinder owns runtime operands
all mappings collapse to one action class or ABSTAIN
the independent verifier reconstructs the decision
new generation survives restart byte-identically
fresh frozen future passes
phase controls establish causal Wave contribution
false accepts = 0
parity mismatches = 0
external admission is the only authority
```

The desired final runtime remains small:

```text
Phase Router
-> Role Grounder
-> Operator VM / typed actor
-> Independent Verifier
-> EMIT or ABSTAIN
```

The complexity belongs in bounded learning, compilation, and proof. It must not
remain as multiple competing authorities in the hot path.
