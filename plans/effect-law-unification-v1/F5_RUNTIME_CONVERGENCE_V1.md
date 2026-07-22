# F5 Runtime Convergence V1

Status: `F5_D_COMPLETE_F5_E_UNLOCKED`

F5-A implementation: `be0c4b465d271e3b3a92700cedfff09867b3f068`

F5-B implementation: `a237c3cd73ab43247d32ea03a4d8530b4bbe9e0d`

F5-C implementation: `ba0824702f8fedf93a2a2f05c88dad2c17e88a6c`

F5-D implementation: `759701564f0bd69c484617f7ea1efd246a602642`

Authority: `false`

Completed implementation boundary:

```text
F5-A executable completeness     PASS for no-constant F4R2 modes
constant-bearing V2 modes        fail closed until ordinal-bound bytes exist
F5-B runtime context             COMPLETE / STOP-F5-B
F5-C mode-to-role compilation    COMPLETE / STOP-F5-C
F5-D capability/action grounding COMPLETE / STOP-F5-D
F5-E actor/VM shadow           UNLOCKED / NOT STARTED
runtime callers                  0
authority                        false
```

Receipts:

```text
STOP_F5_A_EXECUTABLE_COMPLETENESS.md
f5b/STOP_F5_B_CANONICAL_RUNTIME_CONTEXT.md
f5c/STOP_F5_C_MODE_TO_ROLE_COMPILATION.md
```

Development resume:

```text
The 103,389-line nando-response-actor decomposition completed at STOP-R9.
F5-B then resumed as the first post-decomposition feature and closed without
runtime callers or authority. The decomposition record is:

plans/nando-response-actor-decomposition-v1/
  NANDO_RESPONSE_ACTOR_DECOMPOSITION_V1.md
```

The completed cuts did not reopen or weaken STOP-F5-A or STOP-F5-B. F5-D is
the next and only unlocked functional boundary.

This plan closes exactly one boundary:

```text
ProtocolModeSetV2
-> canonical runtime surface
-> existing RuntimeRoleBinder
-> one canonical action class
-> bound actor / Operator VM shadow execution
```

It does not perform independent verifier convergence, persistence, admission,
deployment, or local accept. Those remain F6, F7, and F8.

## 1. Objective

Make the F4R2 executable structural law cause the runtime computation on a
new pre-action surface.

The runtime must not infer a selector from an actor template, recover a
capability name from a digest, or use Wave to rescue a failed structural
binding. Every successful shadow action must be derived from:

```text
immutable ProtocolModeSetV2
+ digest-bound executable facet payloads
+ current pre-action request/capability surface
+ complete bounded RuntimeRoleBinder search
= one canonical bound action class
```

The completed F5 result is still not a natural transferable operator claim.
Real independent binding evidence is not yet available, F6 is not complete,
and production authority remains false.

## 2. Canonical Ownership Boundary

```text
F4 compiler owner
  AcceptedBindingLawEvidenceV2
  + CanonicalEffectLawV3
  + frozen relation graphs
  -> ProtocolModeSetV2

F5 runtime-grounding owner
  ProtocolModeSetV2
  + executable facet payloads
  + current pre-action surface
  -> BoundProtocolActionV3 | ABSTAIN

F6 verifier owner
  raw bounded pre-action evidence
  + immutable law/IR
  + actor result
  -> independent verifier receipt | REJECT

F7 persistence owner
  verified operator
  -> new generation + restart bundle + fresh future

F8 admission owner
  immutable package + proof roots + live gate
  -> candidate authority lease | BLOCK
```

F5 must not construct `VerifiedCrystallizedOperator`, `ResponsePackageState::Active`,
or an admission file. Its strongest output is an opaque shadow-capable bound
candidate.

The implementation is not one F5 supermodule. Each owner emits one immutable
handoff object and cannot reach through the next owner:

```text
F5-A artifact owner
  -> ExecutableProtocolModeArtifactV3

F5-B context owner
  -> CanonicalRuntimeRequestV3 + ExtractionReceiptV3

F5-C binder owner
  -> CompleteRuntimeRoleBindingReportV3

F5-D action owner
  -> BoundProtocolActionSetV3 | ABSTAIN

F5-E VM shadow owner
  -> OperatorShadowExecutionReceiptV3
```

The artifact owner cannot inspect live payload values. The context owner
cannot select an operator. The binder cannot compile or execute an actor. The
action owner cannot verify itself or grant authority. The shadow owner cannot
persist an ACTIVE package.

## 3. Current Proven Input

F4R2 currently proves on controlled evidence:

```text
sealed graph payload set                 PASS
structural selector generation           PASS
selector execution over every graph      PASS
labels used only after execution         PASS
bounded exact cover                      PASS
one action-equivalence class             PASS
typed ProtocolModeProgramV2              PASS
wrong / verify failed / negatives        0 / 0 / 0
execution_authority                      false
production callers                       0
```

The F4 search must remain frozen during F5. A runtime failure is evidence
about the F5 bridge unless a separate receipt proves the F4 artifact is
semantically incomplete.

## 4. Live Process And Traffic Boundary

Planning snapshot on 2026-07-21:

```text
127.0.0.1:8787  nando-nginx-gateway
       |
       v
127.0.0.1:18789 nando-transition-serving (hot)
  local accept enabled by config
  response authority VETO
  response ACTIVE packages 0
  cgroup memory about 162 MiB, peak about 248 MiB
  instantaneous CPU about 0.6% of one core

127.0.0.1:18790 nando-response-learning (cold)
  local accept disabled
  session watcher + miner enabled
  queue capacity 4096, observed backlog 0
  cgroup memory about 663 MiB, peak about 986 MiB
  instantaneous CPU about 13.6% of one core
```

These numbers are a planning snapshot, not acceptance claims. They must be
remeasured at every STOP that touches runtime cost.

The process contract is immutable:

```text
hot serving
  admitted immutable registry
  -> route -> bind -> actor -> verifier -> project -> accept/fallback

cold learning
  completed traces
  -> evidence -> Wave/CEGIS -> F4/F5 candidate -> checkpoint/bundle

forward boundary
  immutable candidate bundle -> independent admission

feedback boundary
  verified bounded receipts only
```

The hot process must never open the miner checkpoint, run F4 compilation,
drain historical replay, or wait for the cold process.

## 5. Incoming Request Contract

The existing ingress route remains authoritative:

```text
/v1|v2/responses or /v1|v2/chat/completions
-> body hash + client intent id
-> bounded JSON parse
-> request text + projection normalization
-> current advertised capability surface
-> clone immutable Arc<ResponseExecutor> under a short read lock
-> execute outside the lock
-> projector + runtime receipt
-> local response or Nginx-managed fallback
```

F5 adds no synchronous compiler, filesystem IO, network IO, checkpoint IO,
mutex-owned learner state, or queue wait to this path.

The runtime must construct one canonical request context per request. It must
not re-serialize or recursively rescan the full JSON once per package or once
per mode.

The existing Axum body limit is 67,108,864 bytes. Initial JSON parsing is an
ingress baseline outside the F5 algorithm, but F5 must not add a second full
walk of a potentially 64-MiB body. Its structural extractor stops at its own
smaller deterministic node budget. If initial parse cost itself violates the
service budget, that becomes a separate ingress-hardening change with its own
baseline; it must not be hidden inside F5.

Chat normalization may already materialize one owned adapter payload. F5 adds
no further full-payload clone and must use the same normalized snapshot for
context extraction, binding, actor, and receipt generation.

Proposed ephemeral contract:

```text
CanonicalRuntimeRequestV3
  request_sha256
  projection
  bounded request relation atoms
  bounded structural role candidates
  bounded relation edges
  current advertised capability descriptors
  completion/temporal/cardinality state
  extraction completion receipt
  borrowed provider payload handle
```

The object is request-local and is never serialized with raw values. Durable
receipts contain hashes, counts, verdicts, generation roots, and timings only.

## 6. P0 Gaps Found Before Implementation

### 6.1 Commitment Is Not Executable Payload

`ProtocolCapabilityContractV2` currently carries a protocol-facet root and
physical-program digests. A SHA-256 commitment cannot tell runtime which
advertised capability satisfies the role or how to construct its arguments.

F5 therefore requires digest-bound payload bytes:

```text
ProtocolFacetPayloadV3
  schema/version
  capability kind role
  argument topology and typed argument roles
  effect contract
  admissible no-op/default semantics
  source-neutral structural constraints
  explicit canonical payload root
```

F5-A must first prove whether the existing `protocol_facet_root_sha256` was
derived from those exact canonical bytes. Today it enters through the frozen
row as an opaque identity. If it is not a byte commitment, it must not be
overloaded. Add a distinct `capability_payload_root_sha256`, bump the mode
schema, and regenerate the controlled artifact from the same frozen evidence.
The F4 selector matrix, exact cover, and verdict must remain byte-for-byte or
semantically identical under an explicit migration receipt. This is a schema
repair, not permission to reopen search or thresholds.

Physical names are not part of the semantic law. The actual symbol is bound
only from the capability declarations in the current request. Missing payload
bytes, root mismatch, unknown schema, or no matching advertised capability
must produce `ABSTAIN`.

The same rule applies to semantic constants. A constant digest is provenance,
not executable data. Any required constant must arrive in a bounded,
privacy-validated payload whose root is already committed by the mode.

### 6.2 V1 Actor Heuristics Are A Second Truth

The current `VerifiedCrystallizedOperator::bind_pre_action()` enters
`bind_raw_pre_action_components()`, which generates runtime selectors from
transform shape and actor-template hints.

No V3 mode may use that route. V3 selectors must be compiled into
`RoleGraph`/`OperatorCircuit` constraints and grounded by
`RuntimeRoleBinder`. The old route remains a V1 compatibility decoder only.

### 6.3 Proof Graph And Runtime Surface Can Drift

F4 uses `FrozenCandidateRelationGraphV1`; the crystallized runtime consumes
`SurfaceFragmentBundle`. Two independent extractors would create two laws.

F5 must add one pure, versioned translation/extraction owner shared by:

```text
frozen evidence graph -> canonical runtime structural view
live request          -> canonical runtime structural view
```

Renaming, wrapper changes, and field reordering must preserve the canonical
view when the relation law is unchanged.

### 6.4 Search Completion At The Exact Cap

`RuntimeRoleBinder` is bounded at 64 alignments. Before F5 trusts
`SearchCompletion::Complete`, a boundary test must prove that producing
exactly the cap does not hide an unexplored frontier. Reaching a cap with any
unvisited branch is `Exhausted`, never `Complete`.

### 6.5 Action Equivalence Must Precede Rendering

Comparing rendered response strings would split one action across Responses,
Chat Completions, JSON, and SSE projections. F5 action identity must be over:

```text
effect law
+ bound capability role
+ typed source-role values
+ semantic constants
+ canonical argument roles
```

Projection and wire rendering happen after canonical action collapse.

### 6.6 Binder Must Not Hide The Structural Version Space

The current `RuntimeRoleBinder::bind()` retains only mappings with the best
phase fit. F5 needs an auditable boundary between exact structure and phase.
Extend the existing binder/report rather than creating a second binder:

```text
complete exact structural mappings + phase scores
-> explicit fixed-point phase winner set
-> canonical action for every mapping in that winner set
```

The receipt must record total exact mappings, phase-winning mappings, runner-up
margin, and action classes. A no-phase control evaluates the same complete
structural set. Hidden pre-report phase pruning is forbidden.

## 7. Runtime Budgets

These deterministic limits are normative runtime caps, not current performance
claims.

```text
request contexts per request                 1
full-payload clones added by F5              0
runtime JSON nodes visited                   <= 4096
runtime role candidates                      <= 64
canonical roles per operator                 <= 32
relations per operator                       <= 256
advertised capabilities                      <= 64
stored modes per mode set                    <= 32
structurally dispatched runtime modes        <= 32 total
mappings per mode                            <= 64
total mapping evaluations per request        <= 2048
request text                                 <= existing 16 KiB bound
direct wrapped provider payload              <= existing 64 KiB bound
```

The `2048` limit is a global per-request operation budget:

```text
32 structurally dispatched runtime modes
* 64 complete mappings per evaluated mode
= 2048 mapping evaluations maximum
```

It is not the product of all registry storage maxima. `modes per mode set` is
a package/storage bound; the immutable dispatch index must return at most 32
mode references in total across all matched mode sets. If exact observable
dispatch leaves more than 32 runtime modes, the result is
`ABSTAIN_DISPATCH_EXHAUSTED`. Runtime must not truncate by package order,
fingerprint, phase score, or arrival time.

Any other exceeded deterministic operation budget yields
`ABSTAIN_BUDGET_EXHAUSTED`. Budget exhaustion is censored evidence and must not
create an anti-center.

Performance gates are measured on the local T480 because it is the production
host. Builds and broad test suites run on the remote 20-core machine.

```text
no-match incremental p99 target              <= 250 us
matched bind+actor shadow p99 target          <= 1 ms
live-shadow p99 veto ceiling                  <= 2 ms
hot RSS delta for 2048 operators + index      <= 16 MiB
filesystem/network IO on request              0
blocking learner queue operations             0
```

The latency values are measured acceptance gates, not semantic timeouts and
not an alternative source of truth. Deterministic node/mode/mapping counters
control fail-closed runtime behavior. Scheduler delay or an elapsed-time
overrun is recorded as `CENSORED_TIMEOUT`; it must not train a positive center,
anti-center, or residual wave.

The page budget is also explicit: `2048 * 4032 = 8,257,536` bytes, or
`7.875 MiB`, leaving `8.125 MiB` inside the 16-MiB target for immutable
dispatch indices, generation metadata, and alignment scratch. If an
object-heavy hot representation cannot fit, compact it or keep it cold; do not
silently raise the target.

If a measured target is missed, STOP with measured profiles. Do not weaken
binding or safety to recover latency.

## 8. Structural Dispatch Before Binding

A hot registry cannot run a full role CSP for every operator. Build an
immutable dispatch index when the executor is loaded:

```text
observable RuntimeDispatchKey
  role-count band
  value-type multiset
  capability kind/arity shape
  completion state
  cardinality class
  observable relation-plane signature
-> bounded runtime mode references across all mode sets
```

The key must contain only pre-action observables. It must not contain teacher
actions, expected values, package labels, field names, or target patches.

The load-time index may contain many mode sets, each with at most 32 stored
modes. A request may receive at most 32 exact-observable runtime mode
references in total across all matched sets. If a dispatch bucket exceeds that
global request budget, runtime must report `ABSTAIN_DISPATCH_EXHAUSTED` and
abstain. Truncating by package order, fingerprint, phase score, or arrival time
is forbidden because it can discard the correct law without evidence.

## 9. Binding And Action Collapse

For every structurally dispatched mode:

```text
compile selector predicates into RoleGraph/OperatorCircuit
-> RuntimeRoleBinder complete structural report (..., max_mappings=64)
-> require SearchCompletion::Complete
-> retain every exact relation-satisfying mapping with its phase score
-> derive an explicit full/no-phase winner set
-> bind capability only from current advertised surface
-> derive canonical BoundProtocolActionV3 for each winning mapping
-> group by canonical action-equivalence digest
-> exactly one action class: continue
-> zero classes: ABSTAIN
-> multiple classes: ABSTAIN_AMBIGUOUS_ACTION
```

Several raw mappings may survive only when they produce byte-identical
canonical actions. The selected representative is deterministic and is not
authority.

Current-request capability declarations are the F5 authority source. The
session capability cache may help evidence capture, but it cannot supply a
missing runtime capability. Anonymous requests and stale session state must
remain safe.

## 10. Wave Position

Wave is not removed, but it moves after structural validity:

```text
structural dispatch
-> complete exact binding
-> canonical action classes
-> phase applicability/ranking among valid candidates
-> unique margin or ABSTAIN
```

Required controls:

```text
full phase
no phase
shuffled phase
magnitude-only phase
matched random center
```

No control may turn a failed binding into an execution. Full phase must buy a
measured reduction in candidate checks or ambiguity on heldout surfaces; if it
does not, F5 still may prove structural runtime convergence, but no Wave gain
is claimed.

## 11. Actor And VM Boundary

F5 compiles the canonical bound action into the existing typed actor and
Operator VM vocabulary. It must not add a parallel protocol interpreter.

```text
BoundProtocolActionV3
-> existing ResponseProgram/TransformOp8 representation
-> existing OperatorPage/VM execution
-> actor/VM shadow parity
```

The bound physical symbol comes from the current capability surface. The mode
owns the relation law, role schema, arguments, constants, and effect contract.

In F5 the actor may remain a shadow parity oracle. The admitted hot path must
eventually execute one VM program plus the independent F6 verifier, not actor
and VM as two permanent computations.

## 12. Incoming Traffic And Backpressure

F5 candidate learning and proof run in the cold process. The hot process does
not load an unverified F5 candidate.

Real traffic shadowing uses the existing out-of-band trace/session route. If a
new handoff queue is unavoidable, it must satisfy all of these constraints:

```text
try_send only
bounded capacity, no wait
compact structural envelope, no raw-body persistence
drop counter exposed
dropped/timeout/unavailable = CENSORED
no semantic Wave update from a dropped event
hot fallback remains available
```

Do not create a third learner queue while the existing 4096-entry miner bridge
can carry the required envelope.

Traffic classes remain distinct:

```text
ordinary traffic       may become future evidence after freeze
development/control    diagnostics only, never future authority
replay/backfill         support only when provenance allows
synthetic/adversarial   proof controls, never organic coverage
```

Deduplicate by event ID and structural digest before any Wave or evidence
update. One physical request cannot vote twice through Responses and Chat
projection aliases.

Each hot request keeps exactly one terminal delivery/economics outcome. F5
shadow execution uses separate counters and must not increment local accepts,
avoided upstream calls, verified token savings, or delivered CPU responses.
An actor shadow result followed by provider fallback is still one provider
delivery, not two completed actions.

## 13. Concurrency And Generation Safety

Each request must pin one immutable executor generation and one mode-set root
before binding. A registry refresh cannot mix old role graphs with new modes or
new verifier roots.

Required concurrency properties:

```text
short read lock only to clone Arc executor
no cache lock held during bind/actor/verifier
atomic whole-generation swap
old in-flight generation remains internally consistent
new requests see either old or new generation, never a mixture
kill switch / authority revocation checked again before local emit in F8
```

F5 must carry generation and mode-set roots into every shadow receipt so F6/F7
cannot accidentally combine evidence from two generations.

## 14. Failure Taxonomy

Every attempt receives exactly one bounded verdict:

```text
BOUND_ONE_ACTION_CLASS
ABSTAIN_UNSUPPORTED_PROJECTION
ABSTAIN_CONTEXT_EXTRACTION_EXHAUSTED
ABSTAIN_DISPATCH_EXHAUSTED
ABSTAIN_BINDING_EXHAUSTED
ABSTAIN_BUDGET_EXHAUSTED
ABSTAIN_NO_STRUCTURAL_MAPPING
ABSTAIN_AMBIGUOUS_ACTION
ABSTAIN_MISSING_CAPABILITY
ABSTAIN_AMBIGUOUS_CAPABILITY
ABSTAIN_PHASE_MARGIN
ABSTAIN_ACTOR
VERIFY_FAILED                 reserved for F6 result
CENSORED_QUEUE_FULL
CENSORED_TIMEOUT
CENSORED_ENVIRONMENT
```

Only verified semantic applicability evidence may update an anti-center.
Budget exhaustion, queue pressure, stale authority, timeout, and unavailable
environment are operational uncertainty.

## 15. Implementation Sequence And STOP Points

### F5-A: Executable Completeness Audit

Work:

1. Inventory every `ProtocolModeProgramV2` field as executable bytes,
   commitment-only metadata, or proof-only provenance.
2. Prove the derivation of every existing facet/constant root.
3. Define digest-bound facet/constant payload schemas for missing executable
   bytes; introduce a versioned payload root when an old root is identity-only.
4. Recompile from the same frozen evidence and prove F4 matrix/cover parity.
5. Prove canonical encode/decode and root verification.
6. Add no runtime caller.

STOP-F5-A:

```text
every required runtime value has committed bytes or a live binding source
hash-only execution routes                          0
opaque identity treated as payload root             0
F4 matrix/cover drift after schema repair            0
physical names in semantic law                     0
authority                                           false
```

If this STOP fails, revise the F5 bridge contract. Do not reopen F4 thresholds
or inject a name dictionary.

F5-A completed on 2026-07-21 with a distinct
`ExecutableProtocolModeArtifactV3`. The legacy protocol-facet root is
revalidated against its canonical physical-evidence bytes but remains
proof/provenance at runtime. A separate payload root commits the source-neutral
capability class and typed argument topology. The physical symbol is explicitly
owned by the future current-request capability binder. V2 constant roots do not
carry executable ordinal bindings, so any non-empty constant contract or raw
physical constant rejects artifact compilation instead of becoming a hash
lookup. F4R2 mode-set bytes remain unchanged, production callers remain zero,
and authority remains false.

### F5-B: Canonical Runtime Context

Work:

1. Extract one bounded pre-action context per request.
2. Share the pure structural semantics with the frozen evidence graph.
3. Preserve borrowed payload access; do not clone the full request.
4. Add completion/budget receipts.

STOP-F5-B:

```text
direct vs wrapped surface canonical parity          PASS
renamed/reordered surface canonical parity          PASS
teacher/action leakage scan                         PASS
one extraction per request                          PASS
budget exhaustion                                   ABSTAIN
raw durable payloads                                0
```

F5-B completed on 2026-07-21 at
`a237c3cd73ab43247d32ea03a4d8530b4bbe9e0d`. Frozen evidence and live requests
now use one source-neutral structural walker owned by
`nando-operator-kernel`. Learning adapts the frozen graph; runtime performs one
bounded request-local extraction and retains only a borrowed payload handle.
Event shape/class computation shares the same JSON node counter, and the
runtime synopsis is restricted to a bounded recent-event and capability
window. Wide events, overfull capability sets, and oversized request text
ABSTAIN. Production callers and authority remain zero.

### F5-C: Mode-To-Role Compilation

Completed on 2026-07-22 at
`ba0824702f8fedf93a2a2f05c88dad2c17e88a6c`. Selector predicates compile into
the existing `RoleGraph` and `OperatorCircuit` vocabulary. An immutable
capability-aware bitset index performs exact observable preselection without
package-order truncation, while `RuntimeRoleBinder` remains the independent
owner of complete structural mappings. The full mapping set, phase-winner
view, and runner-up margin are separate report fields. Production callers and
authority remain zero.

Work:

1. Compile selector predicates into the existing `RoleGraph` and
   `OperatorCircuit` vocabulary.
2. Do not add a second runtime predicate evaluator.
3. Build the immutable structural dispatch index.
4. Make the complete structural mapping set observable before phase pruning.
5. Prove exact-cap search completion semantics.

STOP-F5-C:

```text
new graph language                                  0
V3 calls bind_raw_pre_action_components             0
hidden pre-report phase pruning                     0
exactly-at-cap hidden frontier                       0
overfull dispatch bucket                            ABSTAIN
runtime modes after dispatch                        <= 32 or ABSTAIN
total mapping evaluations                           <= 2048 or ABSTAIN
package/fingerprint/phase-order truncation           0
missing/tampered mode payload                       REJECT
```

### F5-D: Runtime Binding And Capability Grounding

Work:

1. Run `RuntimeRoleBinder` only on structurally indexed candidates.
2. Bind protocol symbols from the current advertised capability surface.
3. Derive one canonical action per mapping.
4. Collapse mappings by canonical action identity before rendering.

STOP-F5-D:

```text
renamed capability surface                          PASS
multiple mappings, same action                      PASS
multiple mappings, different actions                ABSTAIN
missing capability                                  ABSTAIN
duplicate compatible capabilities, same action      PASS
duplicate capabilities, different actions           ABSTAIN
search exhaustion                                   ABSTAIN
wrong bindings                                      0
negative accepts                                    0
```

Stop and hand over the complete mapping/action matrix. Aggregate counts are
not sufficient.

### F5-E: Winner-Owned Actor And VM Shadow

Work:

1. Compile `BoundProtocolActionV3` into existing actor/VM types.
2. Bind selectors and physical symbols from the F5 environment only.
3. Compare actor and VM results in shadow.
4. Keep all output out of live response authority.

STOP-F5-E:

```text
actor program root owned by mode+binding             PASS
manual actor template                                0
actor/VM parity mismatches                           0
unknown opcode                                       ABSTAIN
output budget violation                              ABSTAIN
authority                                            false
```

### F5-F: Phase Integration

Work:

1. Rank only structurally valid candidates.
2. Record exact checks with and without phase.
3. Run all phase controls on frozen heldout surfaces.

STOP-F5-F:

```text
Wave rescues failed structural binding               0
full phase wrong actions                             0
all controls wrong actions                          0
full phase search/applicability gain                 measured or WATCH
tie                                                  ABSTAIN
```

### F5-G: Incoming-Traffic Shadow And Performance

Work:

1. Replay a frozen ordinary-traffic window in the cold process.
2. Run Responses/Chat and streaming/non-streaming projection controls.
3. Run concurrent generation-swap and overload tests.
4. Measure T480 latency/RSS and cold-worker queue behavior.
5. Do not deploy or restart hot serving.

STOP-F5-G:

```text
ordinary denominator accounted                       100%
shadow attempts assigned one verdict                 100%
queue drops                                          accounted/censored
hot request waits on learner                         0
mixed-generation receipts                            0
raw payload persistence                              0
false accepts                                        0
local accepts from F5                                0
latency/RSS budgets                                  PASS or explicit WATCH
```

### STOP-F5: Runtime Convergence Receipt

F5 is complete only when all prior STOP receipts are immutable and this matrix
passes:

```text
renamed surface                                      PASS
multiple mappings, same canonical action             PASS
multiple mappings, different canonical actions       ABSTAIN
missing/ambiguous capability                         ABSTAIN
search/dispatch/context exhaustion                   ABSTAIN
Wave cannot override failed binding                  PASS
actor/VM shadow parity mismatches                    0
wrong bindings / negative accepts                    0 / 0
production callers                                   0
execution authority                                  false
```

Then F6, and only F6, may begin.

## 16. Adversarial Traffic Matrix

At minimum test:

```text
Responses JSON, non-streaming
Responses SSE
Chat Completions JSON, non-streaming
Chat Completions SSE
function capability
custom-tool capability where currently supported
renamed capability with identical schema
same name with incompatible schema
two compatible capabilities with one action class
two compatible capabilities with different action classes
missing tools declaration
stale session capability but missing current declaration
anonymous request without stable session
field order permutation
wrapper insertion/removal
irrelevant large arrays before and after target roles
target beyond extraction budget
duplicate JSON role candidates
mixed scalar types
empty and non-empty protocol constants
malformed/oversized request
registry swap during 100 concurrent requests
queue saturation and cold-worker pause
```

Target beyond extraction budget must abstain. It must not accept an earlier
plausible role merely because the correct role was not visited.

## 17. Observability Contract

Add bounded counters/histograms, never raw names or high-cardinality labels:

```text
f5_context_total
f5_context_exhausted_total
f5_dispatch_candidates_total
f5_dispatch_exhausted_total
f5_runtime_modes_total
f5_binding_complete_total
f5_binding_exhausted_total
f5_mapping_count
f5_mapping_evaluations_total
f5_budget_exhausted_total
f5_action_class_count
f5_missing_capability_total
f5_ambiguous_capability_total
f5_phase_candidate_count
f5_shadow_executed_total
f5_shadow_abstain_by_class
f5_actor_vm_parity_mismatch_total
f5_queue_dropped_total
f5_timeout_censored_total
f5_context_ns / bind_ns / actor_vm_ns / total_ns
f5_hot_registry_rss_bytes
f5_generation_root / mode_set_root
```

The report must distinguish global response-runtime status from the current F5
generation. Existing transition profiles must not be reported as response
ACTIVE packages.

## 18. File Ownership Map

Expected code ownership, subject to STOP-F5-A:

```text
protocol_mode.rs / protocol_mode/*
  immutable F4 artifact validation only

executable_protocol_mode/*
  F5-A cold artifact compiler, executable payload, and restart validation only

new runtime_context_v3.rs
  bounded pre-action extraction and dispatch keys

operator_blueprint.rs
  existing RuntimeRoleBinder and completion repair only

crystallized_operator.rs or a narrow submodule
  V3 mode-to-existing-runtime bridge and bound action

operator_vm.rs
  only if an existing typed opcode cannot encode the proven action

package.rs
  no F5 production route; benchmark/test adapter only

session_stream.rs / miner_worker.rs
  reuse existing cold evidence handoff; no new hot authority

nando-transition-serving/src/lib.rs
  no F5 behavior change before F8; benchmark instrumentation only if needed
```

Do not put proof fixtures, trusted labels, or F4 candidate generation into
`runtime_context_v3.rs`, `package.rs`, or serving.

## 19. Commit And Handoff Discipline

Use one scoped commit per STOP:

```text
F5-A executable payload contract
F5-B canonical runtime context
F5-C mode-to-role compilation
F5-D bounded binding and action collapse
F5-E actor/VM shadow
F5-F phase integration
F5-G traffic/performance receipts
STOP-F5 documentation
```

At every STOP the executor must report:

```text
exact commit
changed files
commands and wall time
focused tests
baseline delta
full denominator
wrong/negative/parity counts
latency and RSS where applicable
authority state
service invocation IDs
background processes left running
```

No deploy, restart, threshold change, registry promotion, or live authority
change is part of F5.

## 20. Prohibited Shortcuts

```text
field-name dictionary as runtime selector
physical capability name as semantic law identity
hash lookup presented as executable grounding
actor template choosing its own operands
caller-supplied coverage matrix
Wave ranking before structural validity
package-order truncation of overfull candidate sets
partial search reported as Complete
rendered text used as cross-projection action identity
current support rows relabeled as frozen future
live traffic timeout turned into anti-center
unverified F5 candidate loaded by hot serving
ResponsePackage ACTIVE or admission bytes emitted by F5
```

## 21. Path After F5

```text
STOP-F5
-> F6 independent verifier convergence
   re-extract and re-bind from raw bounded evidence
   reject actor selector/value/capability mutations
-> F7 new schema/generation/restart
   fresh post-freeze future, byte-identical restore
-> F8 external admission and live shadow
   composite gate PASS required
-> separate explicit authority decision
```

The North Star remains unchanged. F5 proves that the law can cause one bounded
runtime action on incoming structure. It does not prove natural discovery,
independent verification, production safety, 50% CPU coverage, or M3.

## 22. Structural Plan Review

The first four cross-owner worksheets correctly returned `VETO` for mixing
artifact/context/binder/action, hot/cold traffic, F5-F8 authority, and runtime
budget owners.

After splitting at the immutable handoff objects, seven owner-local routes
returned structural `PASS`:

```text
artifact completeness owner     PASS / authority_ready=false
runtime context owner           PASS / authority_ready=false
structural binder owner         PASS / authority_ready=false
canonical action owner          PASS / authority_ready=false
actor/VM shadow owner           PASS / authority_ready=false
hot serving owner               PASS / authority_ready=false
cold learning owner             PASS / authority_ready=false
```

These are coherence checks, not proof authority. Any implementation that
collapses these owners must rerun the broad gate and is expected to stop on
`VETO` until the ownership conflict is repaired.
