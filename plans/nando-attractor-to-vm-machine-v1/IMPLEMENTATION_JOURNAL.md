# Nando Attractor-to-VM Implementation Journal

This is the resumable execution log for the canonical roadmap. Each entry
records the exact boundary, evidence, modification, checks, remaining blocker,
and measured command time. A code change is not complete until its entry has a
check result.

## 2026-07-19 15:37 EEST - Stage 8 Runtime Role Grounding, Pass 1

Starting state:

```text
HEAD ca80c15
production services untouched
graphify query wall time 2.98 s
focused source inspection commands < 0.2 s total
```

Confirmed defects:

```text
observed_multi_role_runtime_surface
  derived output role, relation planes, program atoms, and role order from the
  sealed operator TransformProgram instead of raw pre-action observation

independently_bind_verifier
  ignored RoleGraph and RelationProgram and verified a caller-bound selector
```

Implemented in the current worktree:

```text
raw observation bundle now contains only context + observed source roles
raw observation bundle contains no virtual output and no program atoms
RuntimeRoleBinder maps this partial observed graph into the sealed RoleGraph
verifier re-extracts ordinal roles from raw request/output
verifier independently reruns circuit-constrained CSP binding
```

Current check state:

```text
local stable rustfmt: ENVIRONMENT_UNAVAILABLE in 0.18 s (missing rustc driver)
Rust 1.97 fmt check: one formatting delta in 4.42 s
Rust 1.97 formatting applied: 2.95 s
remote compile: PASS in 40.54 s, linker peak RSS 1,823,016 KB
pre-action observation test: PASS 1/1 in 0.08 s
64-row rich role/admission proof: PASS 1/1 in 34.18 s
scalar crystallization rerun: FAIL in 33 s, MissingRuntimeAnchor
scalar crystallization after source-neutral signature repair: PASS 1/1 in 26.22 s
rich regression after scalar repair: PASS 1/1 in 25.86 s
rich reversed-request ordinal proof: PASS 1/1 in 33.67 s (60 s with relink)
remote cargo check --lib: PASS in 15.99 s
remote crate-wide Clippy -D warnings: ENVIRONMENT/BASELINE BLOCK in 36 s
  11 pre-existing warnings outside crystallized_operator.rs
  no warning reported in the changed module
```

Scalar failure diagnosis:

```text
historical scalar blueprint source role constraint mask = 2
runtime selector-specific role mask = 2 | selector class
single unique scalar has no observable selector-class law
repair: keep concrete selector only as ephemeral anchor and expose a
source-neutral scalar role signature to the circuit binder
```

Independent verifier audit:

```text
wrong: retain only mappings whose response already equals actor response
right: compute every independently grounded response class, require exactly
       one class, then compare that class with the actor response
multiple mappings with one action remain legal; multiple actions ABSTAIN
```

Stage 8 pass boundary:

```text
raw multi-role surface excludes output/program atoms                 PASS
raw scalar surface excludes output/program atoms                     PASS
circuit-constrained partial-role CSP                                 PASS
independent raw re-extraction and response-class verification         PASS
equal support values -> diverging future role proof                   PASS
renamed fields and reversed request ordinals                          PASS
restart + external laboratory admission                              PASS
production deployment                                                 NOT RUN
remote graphify update                                                 ENVIRONMENT_UNAVAILABLE
  wrapper shebang points to missing /home/ubu Python environment
  no replacement Python route installed or enabled
```

Next boundary: Stage 9 admission must recompute authority from sealed receipts
instead of trusting externally deserialized candidate booleans or counters.

Next action:

```text
remove the same operator-derived virtual output/program atom from scalar runtime
preserve independent scalar verification
rerun focused scalar and rich proofs
```

## 2026-07-19 - Stage 7 Minimal Operator VM

Baseline inspection:

```text
memory lookup                                      0.30 s
Graphify execution-route query                     1.81 s
scoped source and AST inspection                   1.50 s total
HEAD before change                                 2cd7e11
worktree                                           clean except graphify-out/
```

Confirmed execution gap:

```text
OperatorPage32 contained TransformOp8 bytecode
BoundCrystallizedOperator executed the side-registry ResponseProgram
therefore page bytecode was proof payload, not the cause of execution
```

Implemented MVP boundary:

```text
OperatorPage32 TransformOp8  -> opcode, source order, output, format
BoundRoleEnvironment         -> runtime selector operands
sealed actor renderer        -> response shape only
Operator VM                  -> computed response
legacy actor                 -> temporary parity oracle
independent verifier         -> final truth check
```

Fail-closed limits:

```text
only PROJECT_UNIQUE_SCALAR is currently executable
unknown opcode or transform flags                 ABSTAIN
missing or duplicate source roles                 ABSTAIN
RequestTemplate renderer                          ABSTAIN
ambiguous UniqueConsensus response                ABSTAIN
VM/reference actor mismatch                       ABSTAIN
independent verifier mismatch                     ABSTAIN
```

This stage deliberately does not add count/filter/compose opcodes. First the
existing crystallized relation circuit must become the actual execution cause.

Implementation and focused verification:

```text
Rust 1.97 fmt check                               PASS 3.06 s
remote source sync                                0.54 s
remote cargo check --lib                          PASS 5.85 s
  compiler peak RSS                               832,912 KB
initial test relink                                28.95 s
  exact-name filter mistake                       0 tests (not counted)
scalar crystallized VM proof                      PASS 1/1 0.13 s
rich multi-role VM proof                          PASS 1/1 25.53 s
VM causal tests first run                         1/2 PASS 25.06 s
  fixture used root JSON instead of tool output; ProjectionFailed
VM causal tests with production payload shape     PASS 2/2 17.05 s
rich integration after VM causal fixture          PASS 1/1 25.55 s
  test-process peak RSS                           38,300 KB
```

Stage 7 result:

```text
page transform bytecode drives scalar execution                  PASS
page transform order drives rich multi-role rendering            PASS
unknown opcode fails closed                                      PASS
role-grounded operands, not actor selectors, feed VM             PASS
legacy actor parity guard                                        PASS
independent verifier                                             PASS
new count/filter/compose opcodes                                  NOT STARTED
production deployment                                             NOT RUN
```

Graph maintenance:

```text
graphify update .                                  PASS 26.08 s
  graph                                             22,914 nodes / 51,107 edges
  one-shot indexer peak RSS                         536,476 KB
graph query OperatorPage32 -> VM -> verifier        PASS 1.84 s
```

Next architectural boundary: finish Stage 9 capture-owned admission
provenance, then expose this VM operator in generic scalar live shadow before
adding broader opcodes.

## 2026-07-19 - Stage 9 Capture-Owned Admission Provenance

Confirmed boundary:

```text
external admission already resynthesized candidate programs and proof counters
but TeacherTransition rows were not committed to the capture-owned hash chain
evidence_ref_sha256 was an observation-output digest, not a ledger commitment
```

Implemented streaming proof route:

```text
StreamingEvidenceLedger record
-> bounded per-turn ordered record commitments
-> CaptureEvidenceReceipt in RuntimeParityCase
-> bounded capture-commitment-index.cbor
-> external admission reads index independently
-> every crystallized support/future receipt must be indexed
-> candidate resynthesis and ordinary proof gates continue unchanged
```

Budgets and behavior:

```text
turn receipt maximum                               512 records
capture index maximum                              16,384 records
index persistence                                  existing 64-event / 5-second checkpoint
ordinary startup history scan                      none
missing, stale, tampered, or unindexed receipt     BLOCK
old historical rows                                support only; cannot obtain new authority
```

Focused verification:

```text
fmt application                                    2.97 s
remote source sync                                 0.47 s
combined package check                             BASELINE BLOCK 14.25 s
  unrelated nando-response-miner has 3 old non-exhaustive selector matches
actor lib + external admission check               PASS 12.72 s
serving lib check                                  PASS 0.12 s
capture receipt/index tests                        PASS 2/2 13.39 s
streaming ledger restart/index test                PASS 1/1 17.27 s
live-shaped two-turn capture receipt join          PASS 1/1 2.84 s
rich candidate/resynthesis regression              PASS 1/1 10.94 s
```

Stage 9 result:

```text
capture-owned bounded commitment index             PASS
turn evidence receipt                              PASS
external admission provenance check                PASS
tamper/missing receipt fail closed                 PASS
candidate resynthesis retained                     PASS
production deployment                              NOT RUN
```

Graph maintenance:

```text
graphify update .                                  PASS 24.74 s
  graph                                             22,938 nodes / 51,194 edges
graphify capture-to-admission path                 PASS 1.74 s
```

### Production install after Stage 9

```text
commit                                               1b8cf21
remote release admission build                      68.57 s
remote release serving build                        26.70 s
composite gate before/after install                  PASS / PASS
hot serving RSS                                      88-90 MiB
cold learner RSS                                     about 381 MiB
capture commitment index                             about 357 KiB
streaming checkpoint                                 about 4.1 MiB
hot Wave state                                       about 5.3 MiB
```

## 2026-07-19 - Stage 7 Completion: Executable Renderer Bytecode

The first Stage 7 cut executed transform bytecode from `OperatorPage32`, but
still supplied `ResponseProgram` from the side registry to render the final
response. That made the page an incomplete execution cause.

Implemented the bounded renderer VM in the 128-byte page renderer section:

```text
STATIC(u8 length, bytes)
VALUE(u8 transform-result index)
EMIT
```

The decoder verifies the version, exact instruction count, operand indexes,
single final `EMIT`, UTF-8, zero padding, complete value consumption, and the
16 KiB output budget. Unknown or truncated instructions fail closed.

An old shortcut was exposed and removed: `crystallize_with_actor_template()`
always wrote a `Direct` renderer into the page even for a rich actor. It now
extracts one identical renderer contract from the actor variants; divergent
renderer variants are rejected.

Focused verification:

```text
initial remote actor lib check                       PASS 7.27 s
initial VM tests                                     PASS 2/2 13.68 s
ordinal-selector miner compile blocker repair        3 explicit match arms
VM renderer tests                                    PASS 3/3 13.44 s
scalar crystallization regression                    PASS 1/1 0.40 s
rich multi-role crystallization                      PASS 1/1 24.45 s
package-wide cargo check                             PASS 7.35 s
focused Clippy                                       1 local issue fixed 10.40 s
  remaining warnings                                 11 pre-existing online/runtime findings
```

Stage 7 completed boundary:

```text
page transform bytecode is execution cause           PASS
page renderer bytecode is execution cause            PASS
rich transform order and renderer composition        PASS
legacy actor retained only as parity oracle          PASS
independent verifier retained                        PASS
unknown transform/renderer opcode                    ABSTAIN
count/filter/compose opcodes                          NOT STARTED
```

Separate pre-existing causal-proof blocker, not hidden by this stage:

```text
crystallized_operator_causal full-phase fixture
-> 64 blueprints
-> transform_mismatches=3 for every blueprint
-> NoEligibleBlueprint
```

The test now compiles after adding the missing ordinal-selector accounting,
but the blueprint/transform fixture contract still needs repair before the
full causal proof can be called PASS.

Graph maintenance:

```text
graphify update .                                    PASS 24.18 s
  graph                                               22,945 nodes / 51,211 edges
  one-shot indexer peak RSS                           492,072 KB
graphify page-renderer-verifier query                 PASS 1.79 s
```

## 2026-07-19 - Stage 9 Closure: Raw Runtime Re-Extraction

The causal proof exposed a remaining self-validation shortcut:

```text
sealed future evidence bundle
+ caller parity payload/anchors
-> bind the evidence bundle itself
```

This could prove a circuit whose relations were not independently observable
in the supplied raw request and payload.

Implemented one shared authority path:

```text
raw request + provider payload
-> recompute raw_input_sha256
-> independently enumerate scalar/ordinal roles
-> construct a fresh ObservedRuntimeSurface
-> bounded circuit binding
-> actor
-> independent verifier
```

`CrystallizationParityReceipt::anchors` no longer provide binding authority.
Teacher/future bundles still teach and phase-select the law, but execution
authority now comes only from re-extracted pre-action structure. Empty request
text cannot manufacture a scalar context relation.

Focused verification:

```text
remote all-target cargo check                         PASS 7.37 s
scalar raw-grounding/restart                          PASS 1/1 13.56 s
strong phase causal proof                             PASS 1/1 0.67 s
  full phase                                          winner
  no/shuffled/magnitude/random controls               ABSTAIN
  incompatible raw runtime circuit                    MissingRuntimeAnchor
rich 64-row raw re-extraction                         PASS 1/1 10.88 s
raw request tamper with unchanged sealed receipt      FutureEvidenceMismatch 13.68 s
```

```text
graphify update .                                     PASS 24.16 s
  graph                                                22,950 nodes / 51,224 edges
  one-shot indexer peak RSS                            492,744 KB
```

## 2026-07-19 - Deployment Checkpoint e8aa612

The raw runtime re-extraction generation was built remotely, passed the
composite gate, and was installed as versioned hot/cold binaries.

```text
remote release admission build                       PASS 64.03 s
remote release serving build                         PASS 53.10 s
serving sha256                                        96d3ae356e8f6f02...
admission sha256                                      29fcb0eeea133d17...
post-deploy composite gate                            PASS
hot serving RSS                                       about 83 MiB
cold learner RSS                                      about 391 MiB
cold checkpoint                                       byte-identical
checkpoint rows / buckets                             14,945 / 34
warm miner state                                      5.34 MiB
false accepts / parity failures                       0 / 0
verified token saving                                 1.0% (M3 WATCH)
```

This checkpoint is the rollback boundary before the first generalized
collection opcode. It is not the 50% product result.

## 2026-07-19 - Stage 8 First Generalized VM Law: COUNT

Implemented the first non-projection law in the crystallized Operator VM:

```text
completed live trace
-> exact source-neutral collection count
-> collection structural role
-> TypedProgramAtom(COUNT_COLLECTION)
-> phase-coherent blueprint
-> OperatorPage32 transform + renderer bytecode
-> raw runtime re-grounding
-> independent collection verifier
-> sealed crystallization
-> external admission
-> registry restart
-> CPU VM response
```

The operation is structurally bounded to a top-level JSON collection with one
array field. Field names are not part of the law. The learned program is
canonicalized to:

```text
SelectOnlyArrayField -> Count -> PlainText -> typed renderer
```

Important defects found and removed while closing the route:

```text
root array was canonicalized as {items:[...]}          count the sole array
unknown opcode rejected at wrong decoder stage         explicit UnsupportedOpcode
accidental scalar template outranked cardinality law   exact count law first
single-role support/runtime role signatures differed   source-neutral shared signature
single-role support/runtime phase atoms differed        type + unique cardinality
collection candidate polluted scalar enumeration        expected-type-only candidate
collection proof mislabeled value projection            collection verifier schema
```

Focused verification and measured wall time:

```text
initial actor cargo check                               PASS 5.79 s
VM tests after first cut                                3/5 13.53 s
VM tests after root-collection correction              PASS 5/5 13.36 s
crystallizer regressions                               PASS 2/2 0.08 s
live count extraction after law ranking                PASS 1/1 13.67 s
scalar runtime regression diagnosis                    15.31 s
scalar full lifecycle after role/phase alignment       PASS 1/1 25.98 s
count full lifecycle before admission schema           BLOCK 22.78 s
count full lifecycle final                             PASS 1/1 42.56 s
  support / frozen future                              32 / 32
  verified shadow executions                          32 / 32
  admission candidate                                 1
  registry restart                                    PASS
  new-surface CPU response                            PASS
  wrong accepts / parity failures                     0 / 0
```

Current product boundary:

```text
PROJECT scalar                                        PASS
COUNT collection                                      PASS in focused lifecycle
FILTER / STATUS / COMPOSE                              NOT IMPLEMENTED IN VM
live organic count ACTIVE                              NOT YET OBSERVED
verified production saving >= 50%                     NOT ACHIEVED
```

Graph maintenance for this generation:

```text
graphify update .                                      PASS 24.20 s
graph                                                   22,968 nodes / 51,302 edges
one-shot indexer peak RSS                               492,696 KB
remote response-actor all-targets check                 PASS 7.06 s
remote check peak RSS                                   943,720 KB
```

## 2026-07-19 - Stage 8b Generalized VM Law: STATUS

Added a third executable transform opcode for bounded integer status mapping.
The winner-owned `TransformOp8` stores the mapping in two flag bits:

```text
0 -> zero is success   success / failure
1 -> zero is pass      PASS / FAIL
2 -> zero is ok        OK / ERROR
3 -> zero is true      true / false
```

The learned route is:

```text
completed verified trace
-> exact ProjectStatus hypothesis outranks accidental text composition
-> source-neutral Integer role
-> PROJECT_STATUS(mapping) atom
-> frozen phase-coherent blueprint
-> OperatorPage32 VM
-> independent status verifier
-> status external evidence schema
-> admission + restart + CPU execution
```

Focused verification:

```text
status foundation all-targets check                    PASS 7.02 s
Operator VM tests                                      PASS 6/6 13.40 s
renamed status trace extraction                        PASS 1/1 13.52 s
status full lifecycle                                  PASS 1/1 7.01 s
scalar full lifecycle regression                       PASS 1/1 12.83 s
count full lifecycle regression                        PASS 1/1 28.56 s
status support / frozen future                         32 / 32
status verified executions                             32 / 32
status admission / restart / new-surface CPU           PASS
wrong accepts / parity failures                        0 / 0
```

Current VM law boundary:

```text
PROJECT scalar                                         PASS
COUNT collection                                       PASS
STATUS integer mapping                                 PASS
FILTER / COMPOSE                                       NOT IMPLEMENTED
organic live ACTIVE coverage                           NOT YET MEASURED
verified production saving >= 50%                      NOT ACHIEVED
```

```text
graphify update .                                      PASS 24.32 s
graph                                                   22,978 nodes / 51,342 edges
one-shot indexer peak RSS                               493,248 KB
```

## 2026-07-19 - Stage 8c Rich VM Law: FILTER

Added the first two-operand Rich Operator transform. The circuit now owns both
the collection role and the request-bound predicate role:

```text
TransformOp8 FILTER_REQUEST_VALUE
  source_a = structurally bound collection
  source_b = structurally bound request predicate
  output   = filtered canonical collection
```

The completed path is:

```text
64 completed verified traces
-> exact request-conditioned filter hypothesis
-> two-role relation circuit
-> 32 support / 32 frozen future
-> full-phase winner; all causal controls abstain
-> runtime role grounding from raw request/payload
-> OperatorPage32 FILTER bytecode
-> independently re-grounded verifier
-> external admission resynthesis
-> restart
-> CPU execution on renamed fields
```

Important defects removed during this stage:

```text
SelectedValue omitted from source-neutral policy        FIXED
unbound Rich template executed against future           REMOVED
request predicate collapsed to generic payload scalar   FIXED
runtime binding failures hidden as MissingRuntimeAnchor FIXED
circuit phase-fit compared with legacy atom threshold   FIXED
```

The runtime route is now controlled by the crystallized circuit itself. Raw
`phase_fit_fixed` is normalized onto a positive relation-count-independent
scale; support fixes the minimum route threshold and all frozen future rows
must meet it. The exact RoleGraph binder remains the applicability authority.

Focused verification and measured wall time:

```text
renamed two-role FILTER extraction                      PASS 1/1 14.30 s
FILTER full lifecycle                                   PASS 1/1 54.74 s
support / frozen future                                 32 / 32
verified future executions                              32 / 32
admission / restart / renamed-surface CPU               PASS
wrong accepts / parity failures                         0 / 0
```

Post-FILTER regression and graph receipts:

```text
PROJECT scalar lifecycle regression                    PASS 18.20 s
STATUS lifecycle regression                            PASS 10.62 s
COUNT lifecycle regression                             PASS 30.04 s
response-actor all-targets check                       PASS 9.25 s
rustfmt --check                                        PASS 2.81 s
remote Graphify update                                 PASS 17.95 s
graph                                                  24,297 nodes / 57,345 edges
```

Strict workspace Clippy was attempted in 12.26 s. The two warnings introduced
by this stage were fixed. The command remains non-PASS because eleven existing
warnings remain in older online/semantic modules outside this functional cut;
they were not mixed into the FILTER change.

Current VM law boundary:

```text
PROJECT scalar                                          PASS
COUNT collection                                        PASS
STATUS integer mapping                                  PASS
FILTER collection by request predicate                  PASS
COMPOSE                                                  NEXT
organic live ACTIVE coverage                            NOT YET MEASURED
verified production saving >= 50%                       NOT ACHIEVED
```

## 2026-07-19 - Stage 8d Plan: Native COMPOSE

The bounded collection version space already emits `FILTER -> COUNT`. No new
semantic primitive or composite shortcut is required. The remaining cut is to
make existing `CompositionDag` executable:

```text
external canonical roles
-> runtime selectors
-> typed VM value arena keyed by role
-> FILTER writes a virtual collection role
-> COUNT reads that virtual role
-> renderer reads the unique sink transform
```

Required invariants:

```text
each transform output is unique
external sources bind exactly once
internal sources must have exactly one producer
topological order is deterministic
cycles / missing producers / multiple sinks -> ABSTAIN
actor and independent verifier reproduce the same composition
no FILTER_COUNT composite opcode
```

## 2026-07-19 - Stage 8d Result: Native FILTER -> COUNT COMPOSE

The existing two primitive laws now execute as one typed dataflow program. No
composite opcode was added:

```text
collection role + request predicate role
-> FILTER writes virtual collection role
-> COUNT reads virtual collection role
-> renderer reads the unique final sink
```

The VM binds only external roles, stores typed intermediate values by canonical
role ID, rejects duplicate producers, forward references, cycles, missing
operands, and multiple output sinks, and executes the transforms in the step
order encoded in the high byte of `parameter`.

Two restart defects were exposed and fixed by the end-to-end proof:

```text
SurfaceFragmentBundle sorted TypedProgramAtom by opcode
-> COUNT preceded FILTER despite its later topological step
-> canonicalization now sorts by explicit step before opcode

renderer compiled against raw [COUNT, FILTER] indexes
while VM decoded topological [FILTER, COUNT]
-> renderer emitted the intermediate array after restart
-> page, composition edges, renderer, and VM now share one ordered program
```

Measured focused verification:

```text
FILTER -> COUNT structural extraction                     PASS 1/1 15.99 s
first lifecycle blocker localization                      10.39 s
restart parity blocker localization                       39.12 s
exact actor/VM mismatch localization                      38.99 s
final FILTER -> COUNT lifecycle                           PASS 1/1 43.97 s
observations / executable                                 64 / 64
support / frozen future                                   32 / 32
full-phase winners / causal-control passes                1 / 1
external admission / restart / CPU                        PASS
wrong accepts / runtime parity failures                   0 / 0
```

The next verification cut is regression of the four existing primitive
families, formatting, all-target compilation, and an updated exact-commit
Graphify receipt. Organic live coverage remains unmeasured and must not be
inferred from this laboratory lifecycle.

Post-COMPOSE regression receipts:

```text
Operator VM unit tests                                  PASS 8/8 13.93 s
PROJECT scalar lifecycle                               PASS 18.49 s
STATUS lifecycle                                       PASS 11.03 s
COUNT + FILTER->COUNT lifecycle                        PASS 2/2 30.99 s
FILTER lifecycle                                       PASS 38.35 s
nando-core operator blueprint                          PASS 9/9 2.86 s
response-actor all-targets check                       PASS 7.47 s
rustfmt --check                                        PASS 2.80 s
```

The VM regression exposed an invalid legacy fixture in which two independent
transforms wrote the same canonical output role. The production invariant was
kept fail-closed; the fixture now assigns one output role per topological step.

Exact source graph and pre-deployment runtime baseline:

```text
code commit                                             44f515b
remote Graphify update                                  PASS 22.83 s
graph                                                   24,310 nodes / 57,384 edges
communities                                             1,070
graph built_at_commit                                   44f515b786...
hot serving                                             RUNNING
hot serving RSS                                         79.7 MiB
cold learner                                            OFF
previous deployed serving SHA-256                       96d3ae356e8f...
```

Production wiring inspection confirmed that `OnlineResponseMiner` already
feeds completed transitions into `LiveScalarShadowState`, checkpoints the
state, exposes crystallized admission candidates, and that transition serving
consumes those candidates. The next cut is therefore release/deployment and
organic live evidence, not another bridge abstraction.

## 2026-07-19 - Stage 8e Plan: Shared Evidence Arena

Post-deployment cold learner measurements:

```text
checkpoint restore                                       3 s
rows / buckets                                           15,002 / 34 hot pools
reported miner warm state                                5.1 MiB
process RSS after restore                                396.7 MiB
process RSS after allocator purge                        257.0 MiB
idle CPU                                                 0.26 CPU-s / 10 s
checkpoint write                                         2.54 s
checkpoint bytes                                         80 MiB
```

The large report-level `negative_rows` value is a cumulative counter, not a
retained-row count. The actual ownership defect is repeated full
`RelationFrame` allocation across bounded bucket reservoirs. With 262 restored
buckets, each bucket may retain 32 support positives, 8 support negatives, 32
future positives, and 8 future negatives. Checkpoint restore currently creates
independent allocations for equal frames and performs no interning.

The bounded repair keeps the existing CBOR shape and admission semantics:

```text
one live RelationFrame
-> SharedRelationFrame(Arc<RelationFrame>)
-> cheap clones across bucket reservoirs

checkpoint decode
-> validate frame_id + learning digest
-> deterministic interning across all bucket reservoirs
-> equal evidence shares one allocation

synthesis/admission
-> materialize owned RelationFrame only for the active cohort
```

The first gate is behavioral parity plus a measured cold restart RSS reduction.
Checkpoint arena encoding is a later step; transparent serde deliberately keeps
the current schema readable during this cut.

Stage 8e code receipts before deployment:

```text
response-actor library check                            PASS 6.25 s
response-actor all-targets check                        PASS 7.43 s
bounded reservoir behavior                              PASS 15.98 s
checkpoint CBOR roundtrip                               PASS 0.45 s
future reservoir restore compaction                     PASS 0.41 s
restore interning of equal learning variants            PASS 13.84 s
final fmt + all-targets                                 PASS 10.96 s
```

The interning digest intentionally excludes economics-only metadata such as
estimated token count. A test first exposed this distinction. The arena key is
the pair `(frame_id, learning_digest)`: equal learning variants share storage,
while structurally different variants remain separate.

The first production restart correctly failed before claiming readiness:

```text
warmup phase                                            failed
blocker                                                 online_checkpoint_frame_id_content_conflict
RSS while failed                                        91.5 MiB
```

The checkpoint legitimately contains both the original positive frame and a
synthetic cross-bucket negative with the same frame ID but a different verifier
label. Treating the ID alone as the arena key was wrong. The corrected pair key
preserves these two semantic variants while still deduplicating repeated copies
of each. Corrected variant interning plus checkpoint roundtrip passed in
13.98 s; final fmt and all-target check passed in 11.10 s.

Corrected production restart and A/B result:

```text
corrected commit                                         a1ff1b6
release build                                            PASS 55.35 s
bootstrap rollback -> READY                              3.80 s
bootstrap composite gate                                PASS 0.39 s
corrected deploy + post-deploy gate                      PASS 1.89 s
hot response ACTIVE / local accept                       1 / enabled
cold restore                                             READY 4 s
old post-purge RSS                                       267,780 KiB
new post-purge RSS                                       258,556 KiB
measured reduction                                       9,224 KiB / 3.4%
```

This is a bounded-growth improvement, not the complete cold-memory solution.
Most retained frames are unique support/teacher evidence, so exact interning
cannot collapse them. The next memory architecture step is a compact cold frame
arena/checkpoint, but product work returns first to executable coverage.

Fresh live extraction boundary:

```text
observations / executable                                585 / 88
unsupported scalar                                      471
support / frozen future                                  33 / 22
live crystallized candidates                             0
```

`UnsupportedScalarProgram` currently merges request-shape, provider-payload,
opcode, and transform-flag failures. New observations must record these as
separate blockers before choosing the next operator extension.

### 2026-07-19 - Stage 8f: exact blockers and direct-payload extraction

The legacy `UnsupportedScalarProgram` counter hid six independent failure
boundaries. New observations now retain explicit outcomes for payload
serialization, invalid request text, missing provider input, unsupported
transform opcode, unsupported transform flags, and unsupported program kind.
The legacy variant remains readable so existing checkpoints are not rejected.

Live capture can also retain a verified tool value directly, without the outer
Responses API `input[]` envelope. Training now builds a source-neutral,
ephemeral synthesis view:

```text
direct observed payload
-> user request + function_call_output envelope
-> bounded program enumeration and exact replay

original direct payload
-> structural role grounding
-> runtime actor and independent verifier
```

The teacher response is never inserted into the synthesis view. Exact
derivation and version-space enumeration use the same view, while canonical
roles and runtime applicability continue to be derived from the original
pre-action payload.

Focused receipts:

```text
initial exact test filter (0 tests; corrected immediately)  13.97 s
direct-payload test exposed renderer shape mismatch           0.49 s
ownership compile correction                                  7.70 s
direct JSON -> COUNT circuit evidence                 PASS   13.98 s
project/status/count/filter/compose extraction        6/6     0.57 s
response-actor all-target compile                    PASS     7.51 s
changed Rust module rustfmt (edition 2024)            PASS     0.44 s
```

This closes one confirmed input-shape loss. It does not retroactively reclassify
the existing 471 legacy failures; the production value is measured from new
completed events after deployment.

Workspace-wide `cargo fmt --check` still reports pre-existing formatting drift
in committed modules outside this change. It is not silently reformatted here;
the changed module itself was formatted with the workspace's Rust 2024 edition.

### 2026-07-19 - Stage 8g: bounded support-only reclassification

The deployed direct-payload adapter cannot retroactively split the 496 opaque
legacy blocker counters because rejected transitions were never retained in
`LiveScalarShadowState`. The main self-training checkpoint does retain bounded
teacher reservoirs with parity cases. Strategy version `69` therefore rebuilds
the live scalar shadow from those reservoirs exactly once:

```text
bounded teacher pools + retained parity cases
-> source-neutral extraction v69
-> historical support only
-> no reconstructed frozen future
-> preserved teacher/student state
```

This is not a history scan and does not read the multi-gigabyte ledgers. The
existing migration path caps one parity signature at 32 cases and preserves the
already learned V2 state after rebuilding bounded Wave buckets.

Focused receipts:

```text
old migration fixture exposed stale expected bound 40       FAIL 14.06 s
fixture aligned with current bounded contract 32/32
support-only migration and parity preservation              PASS 14.24 s
response-actor all-target compile                           PASS  7.44 s
changed Rust module rustfmt (edition 2024)                   PASS  0.42 s
```

The first v69 production migration completed in 9 seconds and converted the
opaque historical counter into an actionable bounded sample:

```text
bounded observations                                      45
executable                                                 2
request_text_invalid                                      37
unsupported_renderer_program                               5
no_exact_source_neutral_program                            1
historical future rows                                     0
```

### 2026-07-19 - Stage 8h: request-independent evidence without text

The dominant v69 blocker was not missing structural evidence. Thirty-seven of
45 retained parity cases had a verified provider payload but no retained user
request text. `COUNT`, scalar projection, and status mapping do not require a
request value. The synthesis view now omits the user message when request text
is empty instead of rejecting the whole trace. Request-dependent filters still
fail closed because their selector cannot be derived without request evidence.

Since production had already persisted checkpoint strategy v69, strategy v70
performs one more bounded support-only migration under this rule.

Focused receipts:

```text
direct payload with and without request text              2/2  14.05 s
support-only migration after extractor change             PASS  2.81 s
response-actor all-target compile                         PASS  7.54 s
changed Rust modules rustfmt (edition 2024)                PASS  0.43 s
```

The v70 production migration completed in 12 seconds and improved bounded
historical extraction from 2 to 15 executable traces. A read-only copy of the
77 MiB checkpoint was then evaluated on the remote machine to avoid repeated
production deployments. The temporary env-gated diagnostic test was removed
immediately after use.

### 2026-07-19 - Stage 8i: raw evidence seal plus normalized execution view

The checkpoint diagnostic split the remaining renderer aggregate and found the
real blocker: synthesized `JsonField` selectors could not be transferred from
the temporary provider envelope back to direct raw payloads. The repair keeps
the two representations explicitly separate:

```text
raw request + raw payload
-> raw_input_sha256 / surface commitment / sealed receipt

raw payload
-> deterministic provider_payload_view (no teacher response)
-> role grounding / actor / independent verifier
```

Complete provider envelopes remain borrowed. Only direct values or envelopes
missing their observed user message allocate a bounded normalized view. A
`JsonField` becomes a request ordinal when the request proves that relation;
otherwise it degrades to `UniqueScalar(type)`, for which the runtime binder must
still find exactly one action-equivalence class or ABSTAIN.

The single-role binder also had a search-control defect: one invalid selector
returned early with `MissingRuntimeAnchor` instead of continuing the bounded
candidate search. It now skips that candidate and still grants authority only
when all successful bindings collapse to one response class.

Observed progression:

```text
v69 bounded extraction                              2 / 45 executable
v70 bounded extraction                             15 / 44 executable
offline exact blocker split                        12 / 41 executable
selected-template canonicalization blockers        19
offline after structural remap                     25 / 41 executable
remaining canonicalization / no-exact / law-shape   5 / 10 / 1
```

Focused receipts:

```text
alternating direct/envelope lifecycle exposed raw/view seal mismatch FAIL 15.94 s
second lifecycle exposed candidate-loop early return              FAIL 15.81 s
final support32/future32/crystallize/admit/restart/CPU lifecycle   PASS 17.83 s
project/status/count/filter/compose extraction                     6/6  0.58 s
direct collection with/without request                             2/2  0.46 s
support-only migration parity                                      PASS 2.81 s
response-actor all-target compile                                  PASS 7.52 s
```

Strategy version 71 performs the final bounded support-only
reclassification. It never reconstructs historical future evidence.

Temporary release and bundle artifacts had reached 304 MiB in `/tmp`,
including one failed full-history bundle. Obsolete copies were removed in
0.02 seconds; only the current rollback binary was retained during deployment.

Production v71 receipt:

```text
commit / binary SHA-256                           14e7b0a / bbadc8b8...63b8
release build                                     PASS 54.64 s
bounded checkpoint migration                      READY 10 s
observations / executable                         52 / 40
executable share of bounded retained evidence     76.9%
laws / total support                              2 / 33
frozen laws                                       1
historical future                                 0
selected-template canonicalization blockers       0
legacy opaque blockers                            0
no-exact / law-shape blockers                     10 / 2
post-deploy composite gate                        PASS 0.39 s
false accepts / runtime parity mismatches          0 / 0
verified economics share                          1.0% (M3 WATCH)
```

The next product boundary is no longer extraction for the first law. It is
event-time evidence accumulation:

```text
frozen support 32
-> new independent completed live traces only
-> future 32
-> sealed candidate
-> external admission
-> ACTIVE
-> verified CPU accepts
```

## 2026-07-19 - R1 outcome-guided role grounding

The typed call path now compiles completed teacher actions into name-free
runtime selectors. The teacher value is used only after trace completion to
align an observed structural position; neither the value nor the field label
is retained in the actor, route, or package.

```text
completed teacher action value
-> all structurally observed turn outputs
-> output/scalar ordinal hypotheses
-> exact teacher parity
-> name-free runtime selector
```

Focused remote receipts:

```text
teacher-value ordinal tests                         2/2 PASS 13.64 s
custom-tool crystallize/admit/restart/CPU lifecycle 1/1 PASS  6.44 s
response-actor cargo check                              PASS  5.93 s
```

Read-only migration of the retained 79 MiB v71 checkpoint takes 62.3-63.3
seconds and peaks at approximately 394 MiB RSS on the remote builder. No
production service or future partition is modified by these diagnostics.

The diagnostic sequence narrowed eight rejected traces without exposing their
payload values:

```text
wire shape mismatch                    8
-> dynamic numeric role mismatch       6
-> dynamic string role mismatch        2
-> teacher role value extracted        8/8
-> value observed in turn outputs      0/8
```

Current exact boundary:

```text
teacher role value
-> request text / non-output payload / absent capture   IN PROGRESS
-> structural role selector                             BLOCKED
```

The next diagnostic classifies only the structural source of the value. It
must not log or persist the value itself.

The source classifier then proved the capture boundary:

```text
teacher role value absent from captured pre-action payload  6
teacher role value not representable as current scalar      2
teacher role value present in retained turn outputs          0
```

`session_stream.rs` was overwriting `runtime_provider_payload` on every tool
output. A later action could therefore use a value from an earlier output while
the CPU parity case retained only the most recent output. This was replaced by
bounded single-pass accumulation for the active turn:

```text
tool call/output pairs                         append in event order
maximum retained input items                   128
maximum serialized provider payload            128 KiB
overflow                                       parity disabled until next turn
partial payload authority                      forbidden
```

Remote receipt:

```text
two-output active-turn retention test          1/1 PASS 0.17 s
nando-transition-serving cargo check               PASS 0.14 s
```

The old v71 checkpoint cannot recover values that capture discarded. Only new
completed live traces may supply support/future evidence for this fixed path.

Final focused remote gate for the v72 source set:

```text
nando-response-actor all-targets check                 PASS 10.15 s
nando-transition-serving all-targets check             PASS  7.48 s
typed Operator VM actor                                1/1  14.41 s
custom-tool crystallize/admit/restart/CPU lifecycle    1/1   6.63 s
templated crystallize/admit/restart/CPU lifecycle      1/1  40.01 s
multi-output active-turn capture                       1/1  29.36 s
```

The template and serving test binaries are the expensive focused checks. Do
not repeat them after documentation-only or diagnostic-only edits.

## 2026-07-19 - Action-equivalent teacher laws (v73)

The next live blocker was not a missing operator primitive. Equivalent polling
actions were partitioned into separate laws because their teacher responses
contained execution-budget arguments:

```text
same semantic action
+ different yield_time_ms / max_tokens / max_output_tokens
+ empty write_stdin chars
-> different law hashes
-> support and future fragmentation
```

Strategy v73 establishes one shared, fail-closed action-equivalence contract
for teacher grouping, crystallization parity, and external admission replay.
It removes only bounded execution-cost arguments and an empty poll payload.
The tool symbol, dynamic role value, source/projection suffix, and every other
semantic argument remain exact. Unknown source shapes are never normalized.

The full custom-tool proof now crosses the complete boundary:

```text
64 budget-varying completed traces
-> 1 teacher law
-> support 32 / frozen future 32
-> 4 competing blueprints
-> 1 full-phase winner
-> crystallized operator
-> external admission candidate
-> registry restart
-> CPU execution
```

Focused remote receipts:

```text
shared normalizer adversarial test                         PASS
custom-tool crystallize/admit/restart/CPU lifecycle  1/1  PASS 14.16 s
wrong dynamic session role                                REJECT
false accepts                                                  0
nando-response-actor all-targets check                     PASS  7.86 s
```

The v72 production checkpoint must be migrated once to v73 so retained
historical support is regrouped under this contract. Historical observations
remain support-only; frozen future and verifier receipts are never fabricated.

## 2026-07-19 - Streaming factorized actor version space (v74)

The v73 live migration merged budget-equivalent polling actions successfully:

```text
live scalar laws                         37 -> 7
false accepts                                 0
next blocker        hypothesisbudgetexhausted = 1
```

The blocker exposed a representation error. One semantic role repeated across
many retained turn outputs produced more than 64 physical selectors. The law
was already source-neutral, but every `(law, selector adapter)` pair was still
stored as a complete actor program. Report generation then repeated that broad
selector search over all support rows, keeping the cold learner near one CPU.

Strategy v74 factorizes the version space:

```text
completed support surface
-> bounded physical adapter set (maximum 512)
-> one streaming intersection per new support row
-> compact law-level actor consensus
-> final authority budget (maximum 64)
```

No candidates are silently truncated. More than 512 per-surface adapters or
more than 64 final consensus programs remains fail-closed. Reports and future
evaluation re-ground the compact consensus against each committed raw surface;
they no longer rerun broad selector induction. Raw request/provider payloads
remain stored only in `TeacherTransition` and are not duplicated in the cache.

Checkpoint v74 rebuilds this compact intersection once from retained support.
Historical rows remain in support and cannot create frozen future.

Focused remote receipts:

```text
>64 repeated physical-role adapters -> <=64 consensus  1/1 PASS  0.17 s
custom crystallize/admit/restart/CPU lifecycle          1/1 PASS  3.47 s
historical rebuild never creates future                 1/1 PASS  0.46 s
nando-response-actor all-targets check                      PASS  7.26 s
```

The ready custom lifecycle evaluation improved from 14.16 seconds under v73
to 3.47 seconds under v74 (approximately 4.1x) while preserving the same full
admission and CPU-execution proof.

## 2026-07-19 - Lossless historical support migration (v75)

The first v74 production migration exposed support loss across strategy
versions:

```text
v73 live scalar executable support    58
v74 live scalar executable support    32
```

Every strategy bump created a default `LiveScalarShadowState` and rebuilt it
only from the general teacher-pool migration reservoir. The shadow miner's own
already verified support was discarded even though it was bounded and carried
runtime parity evidence.

Strategy v75 rebuilds historical support from the union:

```text
checkpoint live-scalar support
union teacher-pool migration reservoir
-> dedupe by frame_id
-> reclassify through the current extractor
-> bounded support
-> future = 0
```

This preserves learned support while still forbidding old-generation future
authority. Law keys and actor consensus are recomputed under the new strategy;
no stale derived route is trusted.

Focused remote receipts:

```text
v74 checkpoint support 32 / future 8
-> v75 migrated support 32 / future 0       1/1 PASS  1.25 s
nando-response-actor all-targets check          PASS  7.23 s
```

## 2026-07-19 - Per-law support accounting

The v75 report showed 40 preserved support rows across eight laws, but exposed
only aggregate counts. `LiveScalarShadowReport` now publishes one bounded row
per law:

```text
law commitment
teacher action symbol (diagnostic only)
operation kind
support / future rows
distinct support sessions
compact actor consensus size
```

No request text, payload value, field name, or teacher argument is emitted.
These diagnostics never participate in routing or authority; they distinguish
insufficient live evidence from residual structural over-partitioning.

Focused remote receipt:

```text
custom-tool law report + full admission/CPU lifecycle  1/1 PASS  3.45 s
```

## 2026-07-19 - Physical adapter quotient and phase-selected role topology (v76)

The per-law report exposed a `write_stdin` law with six support rows but zero
actor hypotheses. The miner was intersecting exact physical selector programs
across renamed surfaces. That recreated a lookup boundary inside the new
operator path: the semantic action was stable, but its JSON/content adapter was
not byte-identical.

Strategy v76 separates three levels:

```text
bounded physical adapters (maximum 512)
-> source-neutral unary topology or competing multi-role topologies (maximum 64)
-> frozen blueprint set
-> independent future phase coherence
-> exactly one winner-owned actor or ABSTAIN
```

Unary function/custom-tool adapters now form a bounded union and quotient to
one action law. Multi-role order is deliberately not collapsed: equal-valued
support can retain several actor topologies, and independent future must resolve
their role order. Request ordinal is preserved only as a structural relation in
multi-role role signatures. Field names and JSON paths remain physical runtime
anchors and never enter the transferable circuit.

Future circuit selection uses structural `BlueprintFutureEvidence` without an
actor commitment. Binding an actor before winner selection would reveal the
answer. After the sealed structural winner, the second executable seal rebinds
the committed raw surface, runs the winner-owned actor, independently rebuilds
the verifier, and commits binding/execution receipts.

Rich scalar transforms now write separate virtual result slots:

```text
source_0 -> value_slot_0
source_1 -> value_slot_1
[value_slot_0, value_slot_1] -> winner-owned renderer
```

This makes the learned relation circuit causal input to the VM instead of
attaching multiple projections to one invalid output slot.

Focused remote receipts:

```text
ambiguous multi-role support -> 3 competing circuits
-> full phase winner / four controls ABSTAIN
-> 32/32 future actor+verifier receipts              1/1 PASS  5.65 s
custom-tool physical adapters -> one law
-> crystallize/admit/restart/CPU                     1/1 PASS  6.59 s
pre-action leakage guard + page/binder roundtrip     2/2 PASS  0.04 s
nando-response-actor all-targets check                   PASS  7.73 s
```

The operator circuit groks through cross-plane phase coherence. The compact
operator page stores the crystallized result; it does not itself perform
grokking.

## 2026-07-20 - Align active-turn evidence budgets (v77)

The first v76 live deployment proved that session ingestion was healthy but
reported 31 `payload_too_large` rejections. Session capture already bounded an
active-turn provider envelope to 128 KiB, while the Rich Operator learner
applied a second 64 KiB limit before synthesis. Valid bounded multi-output
turns were therefore discarded after capture.

Strategy v77 uses one 128 KiB contract across capture and operator evidence.
The limit remains hard; no unbounded payload enters support, future, checkpoint,
or runtime parity receipts. Historical rows are reconsidered as support only,
and strategy migration still cannot manufacture frozen future.

Live diagnosis before the change:

```text
session watcher events    37,386
worker enqueued/processed 482/482
worker failed/backlog     0/0
live observations         131
live executable/support   76/71
payload_too_large         31
```

Focused remote receipts:

```text
64-128 KiB bounded active-turn payload remains executable evidence  1/1 PASS  0.02 s
nando-response-actor all-targets check                                  PASS  7.26 s
```
