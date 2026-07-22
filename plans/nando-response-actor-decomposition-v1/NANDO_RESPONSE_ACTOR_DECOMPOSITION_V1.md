# Nando Response Actor Spectral Decomposition V1

Status: `STOP_R9_PASS / F5_B_UNLOCKED`

Date: 2026-07-21

Base HEAD: `3d7fbefe070d66e64870b4387870a843de697804`

Authority: `false`

## 0. Execution Status

```text
R0  ownership/API/baseline inventory       COMPLETE
R1  bounded remote runner                  COMPLETE
R2  nando-operator-kernel                  COMPLETE
R3  nando-operator-proof                   COMPLETE
R4  nando-operator-runtime                 COMPLETE
R5  nando-operator-admission               COMPLETE
R6  nando-operator-learning                COMPLETE
R7  thin facade and consumer migration     COMPLETE
R8  remaining monolith split               COMPLETE
R9  STOP-DECOMPOSITION                     COMPLETE
F5-B canonical runtime context             UNLOCKED / NOT STARTED
```

## 1. Pause Contract

The functional route stops exactly here:

```text
F4R2 canonical ProtocolMode compiler       PASS on controlled evidence
F5-A executable artifact completeness      PASS in bounded no-constant domain
F5-B canonical runtime context             UNLOCKED / NOT STARTED
F5-C through F8                            NOT STARTED
production callers                         0
production authority                       false
```

No F5-B implementation may begin until `STOP-DECOMPOSITION` passes. This pause
does not invalidate STOP-F5-A and does not reopen F4 search, thresholds,
evidence, or compiler semantics.

Allowed during the pause:

```text
module and crate moves
compatibility re-exports
dependency-direction repairs required by a move
test relocation without assertion changes
build/test runner work
documentation and machine-readable ownership inventories
```

Forbidden during the pause:

```text
new runtime context extraction
new role-binding semantics
new selector or operator family
new opcode behavior
Wave scoring or threshold changes
admission policy changes
checkpoint schema changes
registry promotion
deploy, restart, or local-accept changes
production authority
```

Any behavior change discovered as necessary becomes a separately preregistered
post-decomposition task. It is not smuggled into a move-only commit.

## 2. Why The Cut Is Required Now

Current `crates/nando-response-actor/src` snapshot:

```text
Rust lines                         103,389
root modules                            49
root public re-export statements        47
binary source files                     14
binary source lines                  7,927
standalone *_tests.rs files              9
standalone *_tests.rs lines          5,669
unit-test executable                about 88 MiB
```

Largest mixed-owner files:

```text
online_collection.rs              10,322
online.rs                          5,336
bin/nando-response-miner.rs        5,127
collection_synthesis.rs            4,596
operator_live_shadow.rs            4,539
verifier.rs                        4,459
online_state.rs                    4,089
runtime.rs                         3,839
online_admission.rs                3,170
crystallized_operator.rs           3,091
package.rs                         3,077
lib.rs                             3,066
synthesis.rs                       2,707
cegis.rs                           2,295
```

The line count alone is not the architectural failure. The failure is that one
Cargo ownership boundary contains mutable learning, immutable law IR, proof,
compilation, hot execution, independent verification, admission policy,
persistence, reports, and fourteen tools.

The measured development consequence is:

```text
one source edit
-> rebuild and link one monolithic test binary     about 19.1 s
-> run the focused 21 tests                         about 0.24 s
-> full lib baseline                                about 21.6 s
-> all-target check                                 about 10.5 s
-> Graphify                                         about 19.5 s
```

The proof consequence is more serious: mixed ownership makes it easy for an
actor, verifier, learner, or admission route to reuse another owner's result as
truth. The decomposition is therefore both a speed change and a single-truth
change.

## 3. Canonical Corrections Before Drawing Crates

The existing architecture canon remains authoritative.

1. `nando-core` remains the pure Wave and grokking owner. The dynamic operator
   field, circuit synthesis, phase coherence, and `OperatorPage32` are not
   copied into a new response crate.
2. `nando-operator-learning` may orchestrate phase synthesis through
   `nando-core`; it does not become a second phase engine.
3. The kernel stores the crystallized language and immutable contracts. It
   does not discover laws and does not grant authority.
4. The runtime binds and executes. It never learns and never verifies itself.
5. The verifier recomputes expected consequences from raw bounded evidence and
   immutable law data. It never trusts actor-selected operands or values.
6. Admission grants authority. Learning, runtime, proof, and the compatibility
   facade cannot grant authority.
7. A proof fixture remains proof-only after it moves to another crate.
8. F5-A canonical bytes and roots remain byte-identical throughout the cut.

## 4. Target Architecture

The product-level tree remains simple:

```text
nando-operator-kernel
|-- EffectLaw IR
|-- ProtocolMode IR
|-- ExecutableOperatorArtifact
|-- versioned VM bytecode and typed contracts
`-- canonical bytes, roots, and immutable receipts

nando-operator-learning
|-- evidence and capture joins
|-- quotient and competing hypotheses
|-- nando-core Wave/grokking orchestration
|-- protocol and artifact compilers
|-- crystallizer
|-- BackwardWave candidate generation
`-- bounded checkpoint state

nando-operator-runtime
|-- canonical pre-action context
|-- structural dispatch
|-- runtime role grounding
|-- Operator VM
|-- renderer
`-- actor execution trace

nando-operator-admission
|-- independent verifier orchestration
|-- proof receipt validation
|-- package and generation policy
|-- admission decision
`-- authority lease

nando-response-actor
|-- compatibility re-exports
|-- application-level orchestration
|-- stable binary names
`-- cross-owner integration tests
```

One internal support crate is required to prevent a second truth:

```text
nando-operator-proof
|-- deterministic proof reconstruction
|-- independent expected-delta derivation
|-- causal and parity receipt verification
`-- no mutable learner state and no authority
```

Without this boundary, admission would either depend on the mutable learner or
duplicate proof semantics. Both outcomes are forbidden. The proof crate is an
implementation boundary supporting the four product organs, not a fifth source
of product authority.

## 5. Dependency DAG

Arrows mean "depends on":

```text
nando-operator-kernel    -> nando-core
nando-operator-proof     -> nando-operator-kernel, nando-core
nando-operator-learning  -> nando-operator-kernel,
                            nando-operator-proof,
                            nando-core
nando-operator-runtime   -> nando-operator-kernel, nando-core
nando-operator-admission -> nando-operator-kernel,
                            nando-operator-proof
nando-response-actor     -> nando-operator-learning,
                            nando-operator-runtime,
                            nando-operator-admission,
                            nando-operator-kernel
nando-transition-serving -> compatibility facade during migration
                            -> direct owner crates after cutover
```

Hard dependency vetoes:

```text
nando-core                 -> any response/operator crate
nando-operator-kernel      -> learning/runtime/admission/proof implementation
nando-operator-runtime     -> learning/proof/admission
nando-operator-learning    -> runtime/admission
nando-operator-admission   -> runtime/learning
nando-operator-proof       -> runtime/learning/admission
owner crate                -> nando-response-actor facade
```

The facade is the top of the dependency graph, never a shared lower layer.

## 6. Owner Contracts

### 6.1 nando-operator-kernel

Owns:

```text
source-neutral value and relation types
canonical EffectLaw representation
ProtocolMode and executable-artifact representation
VM instruction and operand representation
generation, action, and receipt identity types
canonical serialization and domain-separated digests
bounded validation that has no IO or mutable state
```

Does not own:

```text
TeacherTransition or completed-trace joins
support/future acquisition
quotient search or phase ranking
live request extraction
actor execution
independent verification
admission policy
filesystem, network, process, environment, or wall clock
```

Candidate current sources, split by symbol rather than moved blindly:

```text
contracts.rs
effect_law_v3.rs and effect_law_v3/canonical.rs
protocol_mode.rs and protocol_mode/selector.rs
executable_protocol_mode/mod.rs and validation.rs
program.rs typed IR only
verified_delta.rs immutable receipt types
operator_generation.rs immutable generation types only
shared canonical JSON/CBOR and SHA-256 helpers
```

`effect_law_v3/evidence.rs`, trust orchestration, compilers, and runtime
execution do not enter the kernel.

### 6.2 nando-operator-proof

Owns:

```text
pure proof reconstruction from immutable evidence
independent role and expected-delta derivation
actor mutation and selector mutation checks
sufficiency, minimality, parity, and causal controls
trusted receipt validation after external roots are supplied
proof report construction without authority
```

Does not own:

```text
evidence capture
mutable learner state
candidate generation policy
actor execution
hot routing
ACTIVE state or authority lease
```

Candidate current sources:

```text
verifier.rs independent semantics
causal.rs
decidability.rs
version_space.rs pure search/proof portions
binding_evidence_adjudication trusted resolver and receipt validation
online_admission resynthesis logic after removal of policy and IO
crystallized_operator parity validation portions
```

The current admission replay through `LiveScalarShadowState` must become a pure
proof reconstruction service before admission moves. It may preserve the exact
algorithm, but it cannot retain a dependency on mutable online learner state.

### 6.3 nando-operator-learning

Owns:

```text
completed-trace evidence
capture provenance and teacher join
effect-law observation and quotient construction
binding evidence acquisition and adjudication inputs
protocol compiler and F5-A artifact compiler
Wave/CEGIS orchestration through nando-core
crystallization compiler
positive/anti-center and BackwardWave candidate updates
online learner checkpoint and reports
```

Does not own:

```text
hot request handling
runtime actor execution
independent verifier truth
admission authority
ACTIVE registry mutation
```

Candidate current sources:

```text
evidence.rs and evidence_graph.rs
capture_provenance.rs and teacher_join.rs
binding_evidence*.rs
effect_graph.rs and effect_law_v3/evidence.rs
protocol compiler implementation
executable_protocol_mode/compiler.rs
synthesis.rs and collection_synthesis.rs
online.rs, online_state.rs, online_checkpoint.rs
online_collection.rs, semantic_alias.rs, online_subcenter.rs
cegis.rs, family_discovery.rs, opportunity.rs
operator_live_shadow.rs learning/proof-fixture portions
crystallized_operator compiler portions
backward_wave.rs and mutable operator_generation portions
```

`nando-core` keeps the actual generic Wave algorithms. This crate supplies
evidence and consumes their typed results.

### 6.4 nando-operator-runtime

Owns:

```text
one bounded canonical request context
structural dispatch keys
complete bounded role grounding
action-equivalence collapse
Operator VM execution
renderer execution
actor execution trace without authority
restart of immutable runtime artifacts
```

Does not own:

```text
completed-trace labels
support/future state
quotient or circuit synthesis
proof reconstruction
admission policy
checkpoint migration
```

Candidate current sources:

```text
runtime.rs after verifier calls are removed from the owner
grounding.rs
operator_vm.rs
output_graph.rs runtime projection portions
program.rs execution implementation after IR moves to kernel
crystallized_operator bind/restore portions
future F5-B runtime_context_v3, only after decomposition completes
```

The old combined `runtime -> verifier` call becomes facade orchestration:

```text
runtime.execute()
-> ActorExecutionTrace
-> admission/proof independently verifies
```

The result is behavior-preserving; the actor no longer imports the verifier.

### 6.5 nando-operator-admission

Owns:

```text
independent verifier invocation
proof-root and package validation
support/future and negative-evidence policy
generation and lifecycle policy
admission report
authority lease construction
```

Does not own:

```text
candidate discovery
mutable online learner state
actor-selected expected values
hot actor implementation
Wave thresholds
```

Candidate current sources:

```text
authority.rs
admission_bundle.rs
online_admission.rs policy portions
package.rs
lifecycle.rs
rollover.rs policy portions
bin/nando-response-admission.rs application wrapper
```

Admission consumes `nando-operator-proof`; it does not import learning or
runtime. The outer application may invoke runtime and admission in sequence.

### 6.6 nando-response-actor facade

Owns only:

```text
stable `nando_response_actor::*` re-exports during migration
application wiring between independent owners
stable command names and thin bin entrypoints
cross-owner integration and compatibility tests
```

Targets:

```text
root lib.rs                         <= 500 lines
each bin wrapper                    <= 200 lines
facade total                        <= 15,000 lines including tests/bins
new domain algorithm lines          0
```

No owner crate may import the facade.

## 7. Required Baseline Artifacts

Before the first Rust move, create:

```text
BASELINE.json
MODULE_OWNERSHIP.json
PUBLIC_API_SURFACE.txt
SCHEMA_AND_GOLDEN_ROOTS.json
KNOWN_TEST_FAILURES.txt
KNOWN_CLIPPY_DIAGNOSTICS.txt
DEPENDENCY_DAG.md
BUILD_TIMINGS.json
```

`BASELINE.json` pins:

```text
Git HEAD and dirty-file exclusions
103,389-line denominator and per-file counts
49 root modules and 47 public re-export statements
Cargo metadata dependency graph
F4R2 mode-set canonical bytes and root
F5-A artifact canonical bytes and root
all public schema string values
all checked-in golden JSON hashes
focused and full test denominators
exact 26-test historical failure set
exact 12 library and 8 test-only Clippy debt sets
Graphify source receipt and route map
test binary size and warm/cold timing
service invocation IDs
authority, ACTIVE, false-accept, and parity state
```

The baseline checker fails when an expected failure disappears for an unknown
reason as well as when a new failure appears. Expected debt is not silently
treated as PASS.

## 8. Execution Plan And STOP Points

### R0: Freeze And Ownership Inventory

Work:

1. Generate the baseline artifacts.
2. Classify every current module and public symbol by one owner.
3. Mark mixed files by symbol ranges rather than assigning the whole file.
4. Record side effects: filesystem, network, process, environment, clock,
   checkpoint, and authority.
5. Record every external caller from `nando-transition-serving` and all bins.
6. Run Graphify and NANDA owner-route review.

STOP-R0:

```text
source files accounted                         100%
public symbols accounted                       100%
mixed-owner files explicitly split in map      100%
unowned side effects                              0
unknown external callers                          0
baseline artifacts canonical                  PASS
authority                                      false
```

Stop and hand off the ownership map. Do not create new crates until reviewed.

### R1: Remote Development Runner

This is build tooling, not operator behavior.

Work:

1. Use one stable remote checkout on `e@192.168.3.94`.
2. Use a bounded incremental `target-dev` for edit loops.
3. Use a clean non-incremental `target-proof` once per STOP.
4. Add `fast`, `stop`, and `release` profiles.
5. Compile each test binary once and run owner filters from that binary.
6. Compare exact known-failure and Clippy fingerprints automatically.
7. Run Graphify once per STOP, from the exact commit.
8. Leave no persistent build process.

STOP-R1:

```text
stable remote source path                      PASS
fast profile leaves background processes          0
proof profile uses clean target                PASS
baseline/failure fingerprint comparison        PASS
local live services touched                       0
warm owner edit loop target                    <= 8 s or WATCH
```

R1 may proceed in parallel with documentation, but not with a second code
movement touching the same crate facade.

### R2: Extract nando-operator-kernel

Final status `STOP-R2`:

```text
canonical JSON/SHA utilities        moved
relation/program contracts          moved
binding predicate vocabulary        moved
ProtocolMode IR/validation           moved
canonical EffectLaw IR/validation    moved
kernel tests / Clippy                PASS
full failure fingerprint             PASS
executable artifact IR/validation    moved
VM immutable contracts               moved or canonical owner retained
compatibility public paths           preserved
STOP-R2                              PASS at 4dd22e0
```

Work units:

```text
R2-A canonical serialization and digest utilities
R2-B immutable EffectLaw and relation IR
R2-C ProtocolMode and executable artifact IR
R2-D VM instruction, operand, action, and receipt contracts
R2-E compatibility re-exports from nando-response-actor
```

For every moved type:

1. Preview all syntax matches with `ast-grep` before rewriting imports.
2. Move the single definition; do not duplicate it.
3. Preserve serde field order, tags, defaults, visibility, and error mapping.
4. Keep the old public path through a facade re-export.
5. Move unit tests with their owner.
6. Compare canonical JSON/CBOR bytes and SHA-256 roots.

STOP-R2:

```text
kernel imports learning/runtime/admission/proof       0
kernel side effects                                   0
duplicate type definitions                            0
public path removals                                   0
schema string drift                                    0
F4R2 byte/root drift                                   0
F5-A byte/root drift                                   0
focused kernel tests                                PASS
full failure-set delta                                 0
authority                                           false
```

### R3: Extract nando-operator-proof

Final status `STOP-R3`:

```text
independent verifier implementation              moved
proof-owned raw surface reconstruction           moved
source-neutral verifier compiler                 moved
decidability and verified-delta contracts         moved
trusted V2 binding proof route                    moved
compatibility public paths                        preserved
proof and facade remote fingerprints              PASS
exact-HEAD Graphify                               PASS
owner-local structural route                      PASS
live/service parity                               PASS
STOP-R3                                           PASS at 4138a15
```

Work:

1. Move pure verifier and proof reconstruction.
2. Separate trusted-root validation from evidence acquisition.
3. Move deterministic resynthesis out of mutable online state.
4. Make expected action/delta derivation accept raw bounded evidence and
   immutable kernel IR.
5. Preserve actor, selector, value, capability, and renderer mutation kills.
6. Leave policy and authority in the old admission owner until R5.

STOP-R3:

```text
proof imports runtime/learning/admission                 0
proof reads actor-selected expected truth                0
proof mutable global/checkpoint state                    0
proof fixture authority                                  0
mutation kills preserved                              PASS
resynthesis output/root parity                        PASS
full failure-set delta                                   0
authority                                             false
```

### R4: Extract nando-operator-runtime

This stage moves only existing runtime behavior. F5-B remains paused.

Work:

1. Move current program execution, grounding, VM, and rendering.
2. Split `crystallized_operator.rs` into immutable artifact, compiler,
   bind/restore, and parity-proof owners.
3. Replace the runtime's direct verifier import with an application handoff
   carrying an immutable actor execution trace.
4. Keep every current ABSTAIN and budget outcome identical.
5. Keep hot dependencies free of `zstd`, checkpoint IO, learner state, and
   proof fixtures.

STOP-R4:

```text
runtime imports learning/proof/admission                  0
runtime filesystem/network/checkpoint IO                  0
runtime calls verifier directly                           0
actor output and ABSTAIN parity                         PASS
runtime mutation tests                                  PASS
F5-B symbols added                                         0
production caller behavior delta                           0
authority                                              false
```

### R5: Extract nando-operator-admission

Work:

1. Move authority, package, lifecycle, and admission policy.
2. Invoke proof reconstruction through `nando-operator-proof`.
3. Keep admission independent from runtime and learning crates.
4. Keep policy inputs immutable and digest-bound.
5. Preserve all external admission reports and binary names.

STOP-R5:

```text
admission imports runtime/learning                        0
actor as verifier oracle                                  0
caller-provided proof counters as authority               0
admission report/schema drift                             0
ACTIVE/authority state change                             0
tamper and restart controls                            PASS
full failure-set delta                                    0
```

### R6: Extract nando-operator-learning

This is the largest cut and is split into three reviewed substages. R6-C uses
bounded owner-local commits because its mixed facade files cannot move as one
unit without crossing runtime, proof, and admission boundaries:

```text
R6-A evidence, capture, quotient, binding evidence
R6-B compilers, crystallizer, causal controls, BackwardWave
R6-C online state, CEGIS, collection learner, checkpoint, reports
```

Current substage status:

```text
R6-A evidence, capture, quotient, binding evidence     COMPLETE at dae7ec3
R6-B compilers, generation, causal controls, Wave      COMPLETE at e5a15a4
R6-C online state, CEGIS, checkpoint, reports          COMPLETE at 716ae73
```

Work:

1. Move cold learning state and compilers.
2. Keep generic Wave algorithms in `nando-core`.
3. Keep authority and hot runtime imports forbidden.
4. Preserve checkpoint bytes and restart behavior.
5. Preserve support/future watermarks and all censored outcomes.
6. Move proof fixtures out of production modules when their owner is proof/eval.

STOP-R6:

```text
learning imports runtime/admission                         0
duplicate Wave/coherence implementation                    0
checkpoint byte/root drift                                 0
support/future denominator drift                           0
censored-to-semantic update regressions                    0
false accepts                                              0
parity mismatches                                          0
authority                                              false
```

Final status `STOP-R6`:

```text
learning core, CEGIS, phase fitting, contracts          moved
learning imports runtime/admission                         0
historical failure fingerprint                          PASS
new Clippy diagnostics                                     0
response facade tracked Rust lines                    56,255
learning tracked Rust lines                           30,734
cross-owner online orchestration                 R7/R8 debt
authority                                              false
STOP-R6                                                 PASS at 716ae73
```

### R7: Thin Facade, Binaries, And Consumers

Final status `STOP-R7`:

```text
facade root definitions                                  0
facade root lib.rs                                     399 lines
Cargo binary names                                  13/13
largest root binary wrapper                            166 lines
public compatibility surface                        342/342 lines
transition-serving owner imports                        PASS
learning imports runtime/admission                         0
STOP-R7                                                 PASS at 54c3350
```

Work:

1. Reduce `nando-response-actor/src/lib.rs` to compatibility exports and
   orchestration.
2. Move 5,127 lines of miner implementation behind a learning application API;
   retain the existing bin as a thin wrapper.
3. Keep every binary name, CLI argument, exit code, and report schema stable.
4. Migrate hot serving imports to kernel/runtime/admission owners.
5. Migrate cold learning imports to learning/proof owners.
6. Keep the facade temporarily for external compatibility and integration
   tests.

STOP-R7:

```text
facade domain algorithms                                  0
facade root lib.rs                                    <= 500 lines
each bin wrapper                                      <= 200 lines
binary names/arguments/exit codes parity                PASS
transition-serving hot dependency on learning             0
cold learner dependency on runtime authority               0
public compatibility manifest drift                        0
```

### R8: Split Remaining Oversized Owner Files

Final status `STOP-R8`:

```text
tracked response-actor Rust lines                      56,411
largest production file                                 2,476
production hard VETO files                                  0
test soft WATCH files                                        0
new generic junk drawers                                    0
historical test identities preserved                    26/26
STOP-R8                                                 PASS at 54c3350
```

After crate ownership is correct, split internal files by route. Do not create
`utils`, `helpers`, `common`, or another generic junk drawer.

Priority cuts:

```text
online_collection.rs
  -> state
  -> ingest
  -> support/future generation
  -> reconciliation/resynthesis
  -> checkpoint
  -> report

online.rs
  -> transition intake
  -> learner update
  -> candidate emission
  -> report

collection_synthesis.rs
  -> candidate enumeration
  -> transform compilation
  -> verifier-contract construction
  -> causal controls

verifier.rs
  -> structural derivation
  -> execution comparison
  -> receipt construction
  -> mutation controls

runtime.rs
  -> context/view
  -> selector execution
  -> VM execution
  -> projection
```

File budgets:

```text
production file soft WATCH                         > 1,200 lines
production file hard VETO                          > 2,500 lines
test file soft WATCH                               > 2,500 lines
new crate non-test hard VETO                       > 40,000 lines
new helper without one explicit owner                    VETO
```

An existing file may temporarily exceed a budget only with a recorded next
cut and no F5-B resume.

STOP-R8:

```text
unexplained hard file-budget violations                 0
new generic junk-drawer modules                          0
duplicate algorithms                                     0
owner-local test commands                              PASS
Graphify dependency cycles                               0
```

### R9: STOP-DECOMPOSITION

Final status `STOP-R9`:

```text
tracked source ownership                           198/198
operator dependency cycles                              0
owner crate tests                                  223 PASS
response baseline                         287 PASS / 26 known FAIL
serving baseline                            47 PASS / 3 known FAIL
workspace all-target check                             PASS
owner and serving Clippy                               PASS
public API SHA parity                                  PASS
F4R2 / F5-A focused parity                       3/3 + 3/3 PASS
Graphify exact-HEAD                                    PASS
NANDA owner routes                                6/6 PASS
live composite gate                                    PASS
response M3                                           WATCH
response ACTIVE packages                                  0
authority                                             false
STOP-R9                                                 PASS at 54c3350
```

Run the clean remote proof profile from the exact final commit.

Required matrix:

```text
all current source files assigned one owner           100%
public compatibility manifest parity                  PASS
schema and canonical-byte parity                      PASS
F4R2 mode-set parity                                  PASS
F5-A executable artifact parity                       PASS
checkpoint and restart parity                         PASS
focused owner suites                                  PASS
full historical failure-set delta                        0
new Clippy diagnostics                                   0
false accepts                                            0
runtime parity mismatches                                0
service restarts                                         0
deployment                                               no
ACTIVE packages                                          0
authority                                             false
```

Performance acceptance:

```text
kernel warm focused loop target                       <= 5 s or WATCH
runtime warm focused loop target                      <= 8 s or WATCH
admission warm focused loop target                    <= 8 s or WATCH
learning warm focused loop target                    <= 12 s or WATCH
facade unit-test binary target                       <= 50 MiB or WATCH
clean STOP proof machine time                         recorded
```

Performance WATCH does not invalidate behavior-preserving decomposition, but
it blocks claims that the refactor accelerated development.

## 9. Test Strategy

### Per edit

```text
fmt for touched crate
cargo check for touched crate
owner-focused tests only
no Graphify rebuild
no full workspace baseline
```

### Per owner STOP

```text
owner crate tests
direct dependent compile checks
canonical byte/root parity
exact historical failure fingerprint
new-diagnostic Clippy comparison
owner-local NANDA route
Graphify update from exact commit
```

### Final STOP only

```text
clean workspace all-target check
all owner suites
full historical baseline comparison
all schema/golden/restart checks
all owner-local structural routes
read-only live transition gate
service invocation and authority check
```

The four expensive phase-ablation proofs remain mandatory at the appropriate
STOP. They are not rerun after every import move.

## 10. Move Discipline

Each movement commit follows this sequence:

```text
Graphify scoped query
-> ast-grep inventory without rewrite
-> move one owner cluster
-> compatibility re-export
-> owner tests
-> dependent compile check
-> byte/root parity
-> commit
-> STOP receipt
```

Rules:

1. Never edit a moved algorithm and its call sites semantically in one commit.
2. Never copy then leave two implementations active.
3. Never make an item public only to satisfy a test. Move the test or add a
   test-only support boundary.
4. Never use `include!`, path escapes, or symlinks as a fake crate split.
5. Preserve comments only where they explain an authority or causality
   boundary. Do not narrate obvious moves.
6. One owner STOP equals one scoped commit or a small preregistered commit set.
7. Do not commit unrelated dirty files.

## 11. Structural Gates

The whole architecture intentionally cannot be represented as one owner-local
route. A combined learner/runtime/verifier/admission worksheet should return
`VETO` because it mixes authority.

Run separate owner routes:

```text
kernel IR -> immutable artifact                       authority_ready=false
evidence -> learning -> candidate                     authority_ready=false
raw evidence -> proof -> verifier receipt             authority_ready=false
immutable artifact -> runtime -> actor trace          authority_ready=false
package + proof receipt -> admission -> lease         authority_ready=false
```

Structural PASS proves route coherence only. It never changes authority.

Plan review on 2026-07-21:

```text
combined multi-owner worksheet          VETO as expected
reason                                  mixed owners and 46 entities
split owner invariants                  15
split owner invariant PASS              15 / 15
authority_ready                         false
```

The split routes cover kernel/Wave ownership, kernel exclusions, learning
input and authority boundaries, runtime execution and verifier separation,
proof derivation and dependency exclusions, admission authority and trust,
facade dependency direction, and all three pause/parity relations.

## 12. Risk Register

### Risk: public facade hides cycles

Control: owner crates may not depend on `nando-response-actor`; verify with
Cargo metadata and Graphify after every STOP.

### Risk: kernel becomes a new monolith

Control: kernel contains data, canonicalization, and deterministic validation
only. Search, IO, mutable state, actor execution, and policy are VETOes.

### Risk: proof and runtime share one interpreter

Control: both consume the same immutable IR, but runtime executes while proof
independently derives expected consequences. Runtime implementation imports are
forbidden in proof.

### Risk: admission still depends on learner replay

Control: move deterministic reconstruction into the proof crate before moving
admission. Admission cannot import mutable learner state.

### Risk: compatibility facade remains forever

Control: direct consumers migrate in R7. The facade keeps public compatibility
but owns no production algorithm.

### Risk: line-count refactor creates many tiny crates

Control: crates correspond to authority owners, not file aesthetics. Internal
modules handle subroutes; new crates require a dependency or process boundary.

### Risk: known failing tests hide regressions

Control: compare the exact failing-test identity set, not only aggregate counts.
The debt is separately repaired after decomposition.

### Risk: refactor silently resumes F5-B

Control: reject new runtime-context symbols, production callers, schema
versions, or behavior changes until STOP-DECOMPOSITION.

## 13. Resume Gate For F5-B

F5-B may resume only after all are true:

```text
STOP-R0 through STOP-R9                         accepted
final decomposition commit                     pinned
ARCHITECTURE_CANON ownership map                updated to implemented paths
F5-A artifact bytes/root                        unchanged
runtime owner                                   isolated
proof owner                                     isolated
admission owner                                 isolated
hot/cold dependency split                       proven
background build processes                      0
services restarted                              0
authority                                       false
```

The first post-decomposition feature commit is still F5-B canonical runtime
context. It must not be bundled with the final refactor commit.

## 14. Executor Handoff Template

Every STOP handoff must report:

```text
STOP id
exact HEAD
changed files
moved symbols and old/new owners
dependency edges added/removed
public API additions/removals
canonical byte/root deltas
focused test counts and wall time
historical failure-set delta
Clippy diagnostic delta
Graphify nodes/edges and cycle result
NANDA owner-route verdict
authority and ACTIVE state
service invocation IDs
background processes remaining
unrelated dirty files preserved
next allowed STOP
```

The executor stops after every owner boundary. It does not silently continue
into F5-B.

## 15. Final Tree After Decomposition

```text
real traces
-> nando-operator-learning
   -> evidence / quotient / Wave orchestration / crystallizer
   -> immutable candidate artifact
-> nando-operator-admission
   -> nando-operator-proof independent reconstruction
   -> candidate authority lease or BLOCK
-> immutable registry
-> nando-operator-runtime
   -> context / grounding / VM / renderer
   -> actor execution trace
-> nando-operator-admission
   -> independent verifier
   -> ACCEPT or ABSTAIN

nando-operator-kernel
   shared immutable language only

nando-response-actor
   compatibility and orchestration only
```

This decomposition does not create intelligence. It gives the existing and
future intelligence one canonical language, one owner per truth, a smaller hot
path, and a development loop that does not relink 103,389 lines for every local
operator change.
