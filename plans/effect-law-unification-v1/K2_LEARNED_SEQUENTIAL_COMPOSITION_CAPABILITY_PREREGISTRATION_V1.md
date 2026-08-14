# K2 Learned Sequential Composition Capability Preregistration V1

Status: `FROZEN AFTER ADVERSARIAL REVIEW / PAPER AUTHORITY ONLY`

Date: `2026-08-15`

## 1. Finite Question

This experiment asks one bounded question after
`K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS`:

```text
Can an isolated machine infer three hidden action effects from support
transitions, then construct a previously unprovided multi-action program whose
causal order reaches a preregistered exact goal in an unseen environment?
```

The target program is not present in fixture metadata, support schedules,
learner inputs, planner code, candidate order, or an MS7 DAG. It must be found
by bounded enumeration over learned effects.

The only permitted positive claim is:

```text
K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS
```

This is a generated-fixture capability result. It is not natural K2,
`K2_MEANING_PASS`, `K2_LAW_PASS`, a `LawCertificate`, a K1 registry member,
production authority, or product economics.

## 2. Prior Evidence And New Information

Reused, unchanged evidence:

```text
bounded effect induction from before/after manifests       PASS
opaque action identity control                              PASS
external effect learner isolation                          PASS
independent one-step effect verifier                        PASS
Law Lab bwrap isolation                                     PASS
exact goal oracle separation                                PASS
all-false K2AuthorityBoundaryV1                             PASS
```

New evidence required here:

```text
three arbitrary-path effects learned from hidden mappings
one target requiring at least three actions
at least one strict causal dependency edge
no single action or strict prefix reaches the goal
bounded program enumeration without a prepared DAG
semantic quotient over equivalent schedules
durable plan precommit before hidden mapping resolution
sequential sandbox execution against current work state
independent full candidate-set and terminal-state verification
```

## 3. Frozen Roles

The following owners are separate:

| Role | Sees | Must not see or do |
|---|---|---|
| Fixture owner | private action mapping, source bytes, expected goal | infer laws, choose program, certify |
| Support dispatcher | one opaque action and one support world | target goal, planner output |
| Effect learner process | opaque IDs and redacted pre/post manifests | private mapping, target, goal, executor request |
| Goal freezer | target pre-state and exact expected terminal manifest | selected program, execution outcome |
| Composition planner process | learned laws, target pre-manifest, exact typed goal, budgets | private mapping, operation list, expected action order, fixture paths outside manifests |
| Independent planner verifier | frozen planner input and output | planner inference calls, private mapping before plan freeze |
| Mapping resolver | frozen selected opaque sequence and private mapping | alter the sequence or goal |
| Sequential sandbox worker | typed operation sequence and disposable workspace | goal, learned laws, support observations, repository, network |
| Exact oracle process | frozen goal and observed terminal manifest | planner implementation, private mapping, candidate ranking |
| Journal owner | typed event payloads | create evidence, retry an indeterminate dispatch |

The learner, planner, selector/orchestrator, sequential worker, and exact
oracle executable SHA-256 values must be pairwise distinct.

## 4. Generated Fixture Family

### 4.1 Dynamic identities

The harness receives a frozen 32-byte fixture commitment. Domain-separated
SHA-256 derivation creates:

```text
three opaque action IDs
four role-path names
support world identities
target identity
distractor names
deterministic content bytes
```

No action ID or role path is a source-code literal. A second commitment must
produce disjoint action IDs, paths, contents, support roots, and target roots
for the separately frozen topology-control family in section 4.5.

### 4.2 Hidden effects

For private role paths `p0`, `p1`, `p2`, and `p3`, the hidden mapping contains:

```text
A: copy current-work p0 -> p1
B: copy current-work p1 -> p2
C: remove current-work p3
```

The opaque assignment of `A`, `B`, and `C` is commitment-dependent. The learner
and planner never receive these labels or the mapping.

### 4.3 Support denominator

Each action receives three action-specific support worlds. Every world varies
content hashes, byte lengths, distractor topology, and world identity.

```text
A worlds: p0 present, p1 absent
B worlds: p1 present, p2 absent
C worlds: p3 present
```

Exactly nine support probes execute through the sequential sandbox with a
single hidden operation. The learner receives only:

```text
opaque action ID
pre-work manifest
post-work manifest
support execution provenance root
```

It receives no operation enum, path-role label, target, goal, or private root.

### 4.4 Unseen target and goal

The target contains `p0` and `p3`, but neither `p1` nor `p2`. Its content
hashes, lengths, distractor topology, and tree root are absent from support.

The frozen exact terminal goal requires:

```text
p0 preserved
p1 present with p0 bytes
p2 present with p0 bytes
p3 absent
all distractors preserved byte-identically
```

The goal envelope and target manifest are frozen before the planner request.
No action sequence has yet been selected.

### 4.5 Distinct topology control

A second complete fixture is learned and planned by the same frozen learner,
planner, worker, verifier, and oracle binaries. Its private effects are:

```text
D: copy current-work q0 -> q1
E: copy current-work q1 -> q2
F: copy current-work q2 -> q3
```

Its target contains only `q0`; its goal requires `q0`, `q1`, `q2`, and `q3`.
The only valid satisfying order is `D -> E -> F`. This family has no remove
effect and no commutative schedule class. Learned topology is normalized from
effect read/write dependencies, never from the private labels shown here.

## 5. Learned Effect Contract

`K2CompositionEffectLearningRequestV1` contains:

```text
public experiment context root
dynamic opaque catalog
nine redacted support observations
bounded effect-language root
learner executable identity
budgets
authority = all false
```

The bounded V1 language is:

```text
CopyCurrentWorkFile { source_path, target_path }
RemoveCurrentWorkPath { path }
```

The learner enumerates all manifest-delta-consistent candidates for each
opaque action and intersects them across its three support worlds. Each action
must finish with exactly one law. The complete candidate roots and rejection
counts are part of the learned law.

The law set is durable before the target manifest or goal enters any planner
process. Before event 0, the fixture owner also publishes a crash-atomic
private mapping artifact. The experiment freeze binds its artifact receipt
root and SHA-256, but neither learner nor planner receives those values.

`K2CompositionTargetIndependenceReceiptV1` proves that target tree roots,
content hashes, byte lengths, distractor topology, and target bytes are absent
from support and learner protocol bytes. Planning is forbidden without this
typed receipt.

## 6. Composition Planner Contract

### 6.1 Planner input

`K2CompositionPlanningRequestV1` contains only:

```text
learned law-set root and complete learned laws
target pre-manifest
typed exact goal root and expected terminal manifest
maximum depth = 3
repetition policy = each opaque action at most once
program grammar root
semantic quotient schema root
planner executable identity
authority = all false
```

It excludes the private mapping, operation plans, support source bytes,
expected sequence, selected representative, and execution outcome.

The external planner emits canonical stdout and a
`K2CompositionPlannerProcessReceiptV1` binding executable SHA-256, request and
outcome roots, limits, cleared environment, no-network isolation, and byte
budgets. The orchestrator cannot substitute an in-process plan.

### 6.2 Complete bounded enumeration

With three actions and depth one through three, the planner must account for
exactly:

```text
length 1: 3 programs
length 2: 6 programs
length 3: 6 programs
total:   15 programs
```

Every program receives exactly one disposition:

```text
VALID_PREDICTION
INAPPLICABLE_AT_STEP
BUDGET_REJECTED
```

No beam search, heuristic truncation, model ranking, stable-hash pruning, or
fixture-specific shortcut is allowed.

The main fixture denominator is frozen exactly:

```text
valid predictions                    8
inapplicable programs                7
budget-rejected programs             0
semantic classes                     5
satisfying semantic classes          1
satisfying class members              3
```

The topology-control denominator is frozen exactly:

```text
valid predictions                    3
inapplicable programs               12
budget-rejected programs             0
semantic classes                     3
satisfying semantic classes          1
satisfying class members              1
```

### 6.3 Sequential semantics

Each next learned effect is applied to the predicted current-work manifest
produced by the preceding effect. Therefore `B` is invalid before `A` in the
target because `p1` does not yet exist.

The planner must prove:

```text
minimum satisfying depth                  3
single-action satisfying programs         0
length-two satisfying programs            0
strict satisfying prefixes                0
causal dependency edges                   at least A -> B
```

### 6.4 Semantic quotient

Programs are equivalent only when they have equal depth, equal action
multiplicity, and produce the exact same terminal tree manifest from the same
frozen target. Lower-depth goal matches are counted separately and must be
zero. Equivalent schedules differing only in the placement of independent
`C` collapse into one semantic class:

```text
[A, B, C]
[A, C, B]
[C, A, B]
```

All three preserve `A -> B`. The selected object is the unique satisfying
semantic class. A canonical representative may then be chosen by program root
as a serialization tie-break. The tie-break does not create semantic
authority.

The source-neutral dependency topology is derived only from learned effect
read/write sets. The main topology contains one strict edge and one independent
node; the control topology is a strict three-node chain. Private fixture labels
are unavailable to topology normalization.

## 7. Independent Verification

The independent verifier must not call the planner evaluator or its manifest
application helper. It separately:

1. validates all roots and authority fields;
2. re-enumerates all 15 action sequences;
3. applies each learned law with an independent transition implementation;
4. reconstructs every rejection step and terminal manifest;
5. rebuilds semantic classes;
6. proves one satisfying class and minimum depth three;
7. proves every strict prefix fails the goal;
8. compares the complete reconstructed set with planner output.

Any missing, extra, reordered, tampered, or incorrectly classified candidate
is terminal `PLANNER_PARITY_FAILURE`.

Planner transitions and verifier transitions live in separate source modules.
A code-route gate rejects any call from verifier code into planner evaluation
or planner manifest-application functions, and rejects any shared function
pointer used for both decisions.

## 8. Durable Temporal Route

`K2LearnedCompositionJournalV1` is append-only and crash-atomic. Its legal
event order is:

```text
0   EXPERIMENT_FROZEN
1-18 SUPPORT_DISPATCHED / SUPPORT_OBSERVED pairs
19  LEARNED_LAWS_FROZEN
20  TARGET_AND_GOAL_FROZEN
21  PLANNING_REQUEST_FROZEN
22  PLAN_FROZEN
23  INDEPENDENT_PLAN_VERIFICATION_FROZEN
24  EXECUTION_DISPATCHED
25  EXECUTION_OBSERVED
26  EXACT_GOAL_VERIFIED
27  ABLATIONS_FROZEN
28  TERMINAL
```

Publishing uses temp file, file `fsync`, no-replace publication, and directory
`fsync`. Restart replays every prefix. A published execution dispatch without
an observation is `INDETERMINATE` and forbids same-identity rerun.

The private mapping is reopened only after event 23. Resolution may translate
the frozen representative opaque IDs into operations but cannot alter their
order. Reopen requires byte equality with the artifact published before event
0 and exact equality with the mapping root and artifact receipt frozen in the
experiment.

## 9. Sequential Sandbox Execution

The target representative executes once in a new generated-capability-only
sequential sandbox protocol. It does not change Law Lab V1 schemas.

The worker:

```text
mounts one immutable source snapshot
creates one disposable current-work tree
applies the frozen typed operations in order to current work
emits pre/post manifests and one result per operation
has no network, repository, private mapping, goal, secrets, or production mount
```

The adapter independently scans the actual current-work filesystem after the
worker exits and independently rescans the immutable source. It checks
operation/result parity, source immutability, unaffected-entry preservation,
isolation, budgets, and cleanup. The adapter-observed post-manifest must equal
both the worker outcome and the planner prediction for the selected semantic
class.

A separate exact oracle process compares the observed terminal manifest with
the preregistered goal and cannot call planner or worker code.

## 10. Required Negative Controls

All controls and expected verdicts freeze before implementation:

```text
1  remove one support world                  INSUFFICIENT_SUPPORT
2  erase post-action deltas                  NO_IDENTIFIABLE_EFFECT
3  duplicate matching copy source            AMBIGUOUS_EFFECT
4  distinct three-copy topology              DISTINCT_TOPOLOGY_PASS
5  leak private mapping to learner            PRIVATE_INPUT_REJECTED
6  leak expected sequence to planner          PRIVATE_INPUT_REJECTED
7  omit learned law B                         NO_SATISFYING_PROGRAM
8  force B before A                           INAPPLICABLE_AT_STEP
9  truncate depth to two                      NO_SATISFYING_PROGRAM
10 omit C from selected program               EXACT_GOAL_UNSATISFIED
11 tamper one candidate terminal root         PLANNER_PARITY_FAILURE
12 drop one enumerated program                PLANNER_PARITY_FAILURE
13 split equivalent schedules into classes   QUOTIENT_MISMATCH
14 merge different terminal states            QUOTIENT_MISMATCH
15 execute a nonrepresentative wrong order    SANDBOX_EXECUTION_REJECTED
16 cross-experiment plan replay               CROSS_EXPERIMENT_REPLAY
17 authority bit set true                     AUTHORITY_BOUNDARY_VIOLATED
18 force an artificial budget disposition     PROGRAM_DENOMINATOR_MISMATCH
```

The topology control executes a second complete support, learner, planner,
verification, target sandbox, and oracle route with disjoint IDs and paths.
Controls 1-3 and 5-14 and 16-18 are deterministic typed verifier controls and
spawn no external process. Control 15 performs the one permitted negative
target execution and exact-oracle invocation.

## 11. Frozen Budgets

```text
complete fixture routes                2
opaque actions per route               3
support worlds per action              3
support sandbox probes                18
positive target sandbox executions     2
negative target sandbox executions     1
effect learner processes               exactly 2
composition planner processes          exactly 2
exact oracle processes                 exactly 3
maximum sequence depth                 3
enumerated programs                    exactly 15 per fixture
maximum manifest entries               48
maximum manifest bytes                 96 KiB
maximum protocol message               1 MiB
learner/planner wall time               3 seconds each
learner/planner CPU                     2 seconds each
address space                           256 MiB each
journal events                          exactly 29
journal bytes                           at most 4 MiB
retained generated experiments          0 after test cleanup
```

Source budgets are frozen independently of behavior:

```text
new model/types file                   < 1,800 lines
new learner file                       < 1,200 lines
new planner file                       < 1,200 lines
new independent verifier file          < 1,200 lines
new sandbox file                       < 1,500 lines
new journal file                       < 1,200 lines
new production module total            < 7,500 lines
new integration test file              < 2,000 lines
```

Existing Law Lab V1, K2 V1, and learned-capability implementation files named
in the preflight are frozen byte-for-byte. The only permitted shared-file edit
is an additive sibling-module registration with no existing declaration or
export changed. The new route is a separate generated-only protocol, not a V1
schema extension.

All Rust builds and tests run on the mini-PC with `CARGO_BUILD_JOBS=20` and a
dedicated frozen target directory. No model API, LLM call, training job,
background timer, or production traffic is used.

## 12. Exact Success Verdict

`K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS` requires all of:

```text
support executions exact                        18 / 18
unique learned laws                              6 / 6 across two routes
law set frozen before target/goal                yes
targets and goals independent and preregistered  2 / 2
program denominators accounted                   30 / 30
main valid / inapplicable                        8 / 7
control valid / inapplicable                     3 / 12
unique satisfying semantic class                 1 per route
main / control satisfying members                3 / 1
minimum satisfying depth                         3 in both routes
strict satisfying prefixes                       0 in both routes
independent candidate verification               30 / 30
plan durable before private mapping resolution   yes
target sequential sandboxes                      2 / 2 PASS
predicted terminal == observed terminal          2 / 2
separate exact oracles                           2 / 2 PASS
negative controls                                18 / 18 exact
journal events                                   29 / 29 per route
restart parity                                   every prefix
false authority bits                             0
production mutations                             0
temporary fixture paths                          0
```

## 13. Stop Rules

Implementation is forbidden while any condition is true:

```text
unresolved P0 or P1 critique finding
NANDA packet WATCH or VETO
implementation preflight not READY_TO_IMPLEMENT
baseline source bytes drift
Law Lab V1 schema change required
learner or planner sees private mapping or expected sequence
planner does not enumerate the complete denominator
independent verifier shares planner transition code
target can be solved at depth below three
production, dashboard, K1 registry, phase memory, or deployment path touched
any preflight-frozen Law Lab V1, K2 V1, or learned-capability file changes bytes
any new source file exceeds its frozen size budget
```

An implementation defect may be repaired without changing the frozen fixture,
budgets, success thresholds, or controls. Any scientific contract change
requires a new preregistration revision.

## 14. Required Tests And Claim Boundary

Required source and real-process tests include:

```text
canonical encoding and root tamper rejection
dynamic identity and path derivation
private/public protocol byte exclusion
generic arbitrary-path effect induction
complete 15-program accounting
minimum-depth and strict-prefix proof
semantic quotient reconstruction
independent verifier source separation
sequential current-work executor behavior
planner, worker, and oracle executable identity separation
crash atomicity and replay at every journal prefix
dispatch-without-observation no-rerun
all 18 negative controls
main and distinct-topology complete real-process routes
unchanged prior learned-capability real-process test
unchanged K2 V1 and Law Lab tests
strict Clippy, fmt, diff, and structural gates
```

Even a complete PASS changes no production state and grants no natural or
runtime authority. The next scientific question after PASS is a separately
preregistered comparison between this complete explicit planner and a bounded
hidden representation over a generated train/confirmatory split.
