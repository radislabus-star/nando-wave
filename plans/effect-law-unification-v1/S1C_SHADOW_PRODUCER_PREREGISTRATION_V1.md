# S1C Shadow Producer Preregistration V1

Status: `PAPER FREEZE CANDIDATE / S1C-1 PASS / NO RUNTIME CHANGE / AUTHORITY FALSE`

Date: 2026-08-11

Parent commit: `d43fc8cd4fcb73e6fb15bcde143a68960272425e`

Parent receipt:
`S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_V3_FINAL_RECEIPT.json`

Parent receipt SHA-256:
`fa29cc86e5610a844080b97a35d3816c73597c45e443a0a80651bbf54050c455`

Parent plan: `GROUNDED_MEANING_ARCHITECTURE_V1.md`

This document freezes S1C-2 before implementation. It authorizes no runtime
change by itself. Its accepted paper claim is only that one source-only shadow
producer slice can be implemented and falsified without changing serving,
execution authority, natural evidence status, training, or phase memory.

## 1. Exact Question And Claim

S1C-1 supplied typed contracts, an exact goal binder primitive, a prepared
response evaluation, a K1 action index, temporal receipts, and a crash-tested
precommit journal. None is wired into the request path.

S1C-2 asks one bounded engineering question:

> Can transition-serving produce a complete authority-false decision shadow
> from an eligible pre-action typed goal, using exactly one prepared serving
> evaluation and durable evidence before and after execution, while preserving
> the existing HTTP decision and bytes?

The permitted result is:

```text
exact eligible pre-action goal
+ one immutable executor and K1 authority snapshot
-> evaluate_pre_action exactly once
-> sync DecisionContractPrecommitV1
-> consume that same PreparedResponseEvaluation
-> existing independently verified execution
-> sync SelectedActionBindingReceiptV1
-> exact predicate verification at the same-request terminal horizon
-> sync GoalSatisfactionReceiptV1
```

S1C-2 does not claim that ordinary traffic contains an eligible goal. An honest
source implementation may emit only `MISSING_EXACT_GOAL` in later natural use.
That outcome is evidence about the goal surface, not an implementation failure.

## 2. Parent State And Authority

The frozen parent receipt establishes:

```text
S1C-1 source acceptance                PASS
tests                                 1137 PASS / 15 ignored
strict Clippy and fmt                  PASS
serving oracle                         BYTE_IDENTICAL
structural routes                      3 / 3 PASS
installed binary changed               false
service restarted                      false
natural evidence changed               false
authority_ready                        false
capture activation allowed             false
S1C-2 allowed                          false
deployment allowed                     false
```

This paper supersedes only the parent field `s1c2_allowed=false` after its own
critique, six structural routes, manifest, and verification receipt pass. It
does not supersede any other parent authority field.

The accepted S1C-2 paper receipt may set only:

```text
s1c2_source_implementation_allowed      true
capture_activation_allowed              false
deployment_allowed                      false
model_training_allowed                  false
phase_mutation_allowed                  false
authority_ready                         false
```

## 3. Current Runtime Map And Exact Blockers

Current code points at the parent commit are:

```text
handle_openai
  crates/nando-transition-serving/src/lib.rs:4821
pre-action topology extraction
  crates/nando-transition-serving/src/lib.rs:4858
try_response_actor
  crates/nando-transition-serving/src/lib.rs:5173
admitted executor cache read
  crates/nando-transition-serving/src/lib.rs:5208
existing exact-Wave precommit
  crates/nando-transition-serving/src/lib.rs:5257
current compatibility execution
  crates/nando-transition-serving/src/lib.rs:5282
PreparedResponseEvaluation
  crates/nando-response-actor/src/package.rs:710
evaluate_pre_action
  crates/nando-response-actor/src/package.rs:1085
execute_prepared
  crates/nando-response-actor/src/package.rs:1521
precommit journal owner
  crates/nando-transition-serving/src/grounded_decision_capture.rs:17
response cache
  crates/nando-transition-serving/src/lib.rs:501
off-path response refresh
  crates/nando-transition-serving/src/lib.rs:6718
```

There are four exact blockers:

1. `ResponseExecutorCache` publishes an executor but no matching
   `K1ActionIndexV1`.
2. `try_response_actor` still calls the combined `execute`, so no durable
   precommit can precede selection while sharing the same evaluation.
3. the journal persists only `DecisionContractPrecommitV1`; selected-action and
   satisfaction receipts have no runtime producer or durable owner;
4. no natural exact-goal ingress is proven present in ordinary traffic.

S1C-2 repairs the first three as source capability. It must expose the fourth
as `MISSING_EXACT_GOAL`, never conceal it with inference.

## 4. Feature And Serving Contract

The only activation flag is:

```text
NANDO_GROUNDED_DECISION_SHADOW_ENABLED=false
```

Requirements:

- absent, empty, malformed, or non-true values mean false;
- false is the compiled and configured default;
- false opens no grounded-decision journal and executes the parent compatibility
  route;
- true enables evidence observation only;
- true cannot enable local accept, admission, certification, training, or phase
  mutation;
- a shadow error cannot turn an upstream fallback into a local accept or a
  local accept into fallback;
- response status, headers, body, package choice, and runtime receipt remain
  equivalent to the parent oracle.

Every persisted S1C-2 record has:

```text
authority_ready=false
model_training_allowed=false
phase_mutation_allowed=false
```

Where the existing schema stores only the first and third flags, the missing
training flag is fixed by the journal owner and slice receipt, not invented as
runtime authority.

## 5. Exact Goal Ingress

### 5.1 Eligible classes

The goal must exist and freeze before `evaluate_pre_action`. Exactly three
source classes are eligible:

1. an exact typed goal in an authenticated protocol contract;
2. an exact consequence mechanically derived from bounded, source-neutral,
   pre-action typed fields by a frozen deterministic binder;
3. an externally supplied content-addressed typed-goal envelope whose provider
   capture proves it existed before transition-serving evaluation.

All three must materialize the existing `ExactPreActionGoalInputV1`, validate
the `TypedGoalPredicateArtifactV1`, and call only
`bind_exact_pre_action_goal_v1`. The binder must bind the already supplied
typed target; it cannot choose the target or consequence type.

S1C-2 may implement only source-neutral parsing and exact validation for those
classes. It may not add a semantic classifier or infer a goal from behavior.

### 5.2 Absolute denylist

These fields cannot affect goal existence, comparator, consequence type,
target, verifier, horizon, or any binder root, even when hashed:

```text
request free text
embeddings or LLM classification
package ID, bundle ID, package origin, or package proof counters
applicable-action count, ranking, margin, tie, or selected action
actor output, verifier result, runtime receipt, or upstream outcome
teacher labels, generated fixtures, synthetic traffic, or Law Lab output
K1 scheduler candidate, identifier result, phase state, or future row
```

The provider request root and topology root may bind identity and provenance;
they cannot become a goal target by themselves.

### 5.3 Missing surface

If no eligible exact input exists, the shadow result is exactly:

```text
MISSING_EXACT_GOAL
precommit_written=false
evaluation_for_evidence_performed=false
serving_route=parent_compatibility
```

No placeholder goal, default success predicate, request-text hash, package
target, or post-action repair is allowed. A later S1C-4 census may therefore
terminate `EMPTY_GOAL_SURFACE`.

## 6. Atomic Decision Authority Snapshot

The off-path refresh owner must construct and publish one immutable object:

```text
ResponseDecisionSnapshotV1
|- Arc<ResponseExecutor>
|- Arc<K1ActionIndexV1>
`- DecisionAuthoritySnapshotV1 root
```

Its inputs are one coherent read of:

```text
response registry
external response admission
anchored operator-certification ledger
K1 vocabulary gate
runtime contract
```

The refresh owner reads all input fingerprints before construction, validates
the executor and K1 index, rereads all fingerprints, and publishes only when
the two fingerprint tuples match exactly. On mismatch it retains no partial
candidate. The cache write swaps executor, K1 index, and authority root under
one write lock.

Request threads:

- clone executor and K1 index under one read lock;
- never read registry, admission, certification, anchor, or vocabulary files;
- never rebuild a K1 index;
- treat a missing/mismatched K1 index as a named shadow censor while preserving
  normal serving;
- continue to use external admission as the only execution authority.

The current `ResponseExecutor::build_k1_action_index_v1` remains the authority
validator. A narrowly scoped accessor or builder adjustment in the
preregistered response-actor files is allowed only to construct and carry this
atomic snapshot off-path. A new schema, new authority source, or file read on
the request thread requires a new preregistration revision.

## 7. One-Evaluator Temporal Route

For a natural-evidence-eligible request with capture enabled and an exact goal,
the order is frozen:

```text
1. freeze request identity, topology, exact goal, and sequence coordinates
2. clone one ResponseDecisionSnapshotV1
3. call evaluate_pre_action exactly once with its K1 index
4. inspect PreparedK1EvidenceV1 without consuming the prepared object
5. seal DecisionContractPrecommitV1
6. append framed precommit and sync it
7. retain DecisionContractDurabilityReceiptV1
8. run the existing exact-Wave precommit unchanged
9. call execute_prepared with the same PreparedResponseEvaluation
10. preserve the parent HTTP projection and runtime receipt route
11. if independently verified K1 execution exists, seal and sync selected action
12. verify the exact goal predicate at the same-request terminal horizon
13. seal and sync GoalSatisfactionReceiptV1
```

Forbidden routes:

```text
evaluate_pre_action -> discard -> executor.execute
evaluate_pre_action -> evaluate_pre_action again
precommit after execute_prepared
selected action before precommit sync
goal satisfaction from actor success alone
new ranking or applicability implementation
```

If precommit construction or sync fails after preparation, transition-serving
must record a named shadow censor and still consume the same prepared object.
It cannot call the compatibility evaluator because that would evaluate twice.

The existing exact-Wave precommit remains observational and unchanged. It is
not S1C goal, action, or satisfaction authority.

## 8. Selected Action And Terminal Satisfaction

`PreparedResponseEvaluation` may expose only the capture evidence needed to
join its selected package to the already frozen K1 index. The source change may
add an opaque capture-only result carrying:

```text
selected_action_contract_root_sha256
opaque_execution_binding_root_sha256
observed_consequence_root_sha256
independent_runtime_verification_root_sha256
```

It must not expose package identity as public semantic identity. The first two
roots must be entries from the exact available-action/binding set committed
before execution. The latter two roots must be produced by the existing actor
plus independent verifier, not by the goal binder.

A `SelectedActionBindingReceiptV1` is valid only after the durable precommit and
only when:

- execution status is `Executed`;
- the selected package maps to one K1 action in the same index;
- the binding root belongs to the precommitted binding set;
- the existing independent runtime verifier passed;
- process epoch and monotonic/sequence constraints validate.

Goal satisfaction is computed by reproducing the frozen
`TypedGoalPredicateArtifactV1` against the independently observed consequence.
Actor success, HTTP 200, local accept, package match, or runtime parity cannot
stand in for predicate satisfaction.

An exact false predicate produces a durable receipt with `satisfied=false`; it
is valid negative evidence, not a censor. Missing terminal truth, verifier
failure, wrong horizon, or root mismatch is censored and cannot produce a
satisfaction receipt.

## 9. Persistence And Recovery

The exact base directory remains:

```text
/var/lib/nando-wave/transition/grounded-meaning-v1/
  decision-contract-precommits-v1/
```

S1C-2 may add separate framed-CBOR ledger prefixes inside that directory for:

```text
decision-precommit
selected-action-binding
goal-satisfaction
```

It may not silently migrate or rewrite the S1C-1 precommit ledger. Every append
used as evidence is followed by ledger sync before its receipt is reported.
Recovery validates every schema and root, rejects duplicate request/precommit
bindings, and projects only joins with this order:

```text
precommit -> selected action -> satisfaction
```

Torn final frames may be truncated only by the existing framed-ledger recovery
contract. Interior corruption, duplicate identity, root rebound, sequence
reversal, or cross-epoch join poisons the shadow evidence owner. Serving remains
available through the parent route.

No raw request, session, provider payload, response text, tool output, or
package payload is persisted in this directory.

## 10. Named Shadow Censors

The implementation must distinguish at least:

```text
CAPTURE_DISABLED
INELIGIBLE_TRAFFIC_PROVENANCE
MISSING_EXACT_GOAL
GOAL_INPUT_INVALID
AUTHORITY_SNAPSHOT_UNAVAILABLE
AUTHORITY_SNAPSHOT_MISMATCH
NO_APPLICABLE_K1_ACTION
ACTION_PROJECTION_INCOMPLETE
ACTION_CAPACITY_EXHAUSTED
PRECOMMIT_SEAL_FAILED
PRECOMMIT_SYNC_FAILED
SELECTED_ACTION_NOT_K1
SELECTED_ACTION_BINDING_FAILED
SELECTED_ACTION_SYNC_FAILED
TERMINAL_CONSEQUENCE_UNAVAILABLE
INDEPENDENT_VERIFIER_UNAVAILABLE
GOAL_PREDICATE_VERIFICATION_FAILED
SATISFACTION_SYNC_FAILED
```

Censors are diagnostic classifications, not synthetic episodes. If the
evidence ledger itself is unavailable, an in-memory counter or existing bounded
event may report the failure, but it cannot be cited as durable decision
evidence. S1C-4 owns the exact append-cursor denominator.

## 11. Frozen Budgets

S1C-1 budgets remain unchanged:

```text
precommit canonical bytes                 <= 32 KiB
available K1 actions                      <= 256
typed goal predicate                      <= 4 KiB
framed segment                            64 MiB
all grounded-decision ledgers combined    <= 2 GiB
raw request/session/provider payload      0 persisted bytes
no-goal p99 added latency                  <= 250 us
eligible sync p99 added latency            <= 5 ms
eligible sync hard max added latency       <= 20 ms
steady-state RSS delta                     <= 16 MiB
idle CPU delta                             <= 0.25% of one core
```

The no-goal path performs no S1C applicability evaluation and no synchronous
journal write. The eligible path may sync only the bounded evidence records
defined here. Quota exhaustion is a censor and cannot affect serving.

## 12. Allowed Source Slice

After this paper gate passes, S1C-2 implementation may touch only:

```text
crates/nando-transition-serving/src/lib.rs
crates/nando-transition-serving/src/grounded_decision_capture.rs
crates/nando-response-actor/src/package.rs
crates/nando-response-actor/src/lib.rs              re-export only if required
crates/nando-operator-learning/src/grounded_decision/pre_action.rs
crates/nando-operator-learning/src/grounded_decision/pre_action_tests.rs
crates/nando-operator-learning/src/grounded_decision/mod.rs
```

The operator-learning files may add only package-neutral censor or durability
contracts required by this document. They cannot add a goal inference policy,
model, learner, scheduler, certificate, or phase update.

Any required production config, service unit, dashboard, census, schema outside
this list, public API beyond the frozen capture evidence, or runtime owner beyond
transition-serving stops implementation as `PAPER_REVISION_REQUIRED`.

## 13. Tests And Evidence

The source candidate must pass:

1. goal allowlist and denylist tests, including hashes of forbidden text;
2. missing-goal fast-path and zero-evaluation tests;
3. exactly-one evaluation and same-prepared-object tests;
4. atomic cache publication, torn fingerprint, stale index, and missing index
   tests;
5. precommit-before-execute sequence and monotonic-time tests;
6. sync failure still consumes the same prepared object;
7. selected-action membership, root rebound, and wrong-epoch negatives;
8. exact true and exact false satisfaction receipts;
9. missing verifier, wrong horizon, and terminal-root mismatch negatives;
10. torn frame, duplicate, replay, restart recovery, quota, and poison tests;
11. capture-off byte/decision parity against the parent oracle;
12. capture-on serving parity for matched, no-match, ambiguous, actor-abstain,
    verifier-fail, and persistence-fail routes;
13. the three affected crate suites, strict Clippy, fmt, and split structural
    gates;
14. resource gates under the unchanged budgets.

No natural traffic, deployment, or live activation is required to accept the
source implementation. Such claims belong to S1C-3 and S1C-4.

## 14. Implementation Exit And Stop Verdicts

```text
S1C2_SOURCE_PASS
  exact source slice, all tests, parity, resources, and structural routes pass

MISSING_EXACT_GOAL
  valid runtime classification; not source failure and not S1C PASS

PAPER_REVISION_REQUIRED
  implementation needs a source, schema, owner, feature, or authority not frozen

VETO_GOAL_LEAKAGE
  forbidden or post-action field affects the goal

VETO_SECOND_EVALUATOR
  serving or evidence evaluates applicability/ranking twice

VETO_TORN_AUTHORITY
  executor and K1 index can originate from different snapshots

VETO_FALSE_DURABILITY
  evidence is reported before sync or cannot survive recovery

VETO_SERVING_DRIFT
  HTTP decision, bytes, fallback, false accepts, or parity changes

VETO_AUTHORITY_DRIFT
  shadow changes admission, certification, training, phase, or execution authority
```

Only `S1C2_SOURCE_PASS` permits an S1C-2 source commit. It still leaves:

```text
capture activation          false
deployment                  forbidden
S1C-3                       next paper-first slice
S1C-4 natural census        blocked
S2-S6                       blocked
K2 claim                    not proved
```

## 15. Paper Acceptance Gate

Before S1C-2 code begins:

- the adversarial critique is complete and all accepted repairs are present in
  this document;
- six independent NANDA packets pass without WATCH, conflict, foreign pull,
  owner conflict, negative hit, or repair queue;
- every packet remains `authority_ready=false`;
- a manifest hashes the final preregistration, critique, status-only plan
  changes, inputs, and results;
- the verification receipt binds the parent commit and parent receipt root;
- only the paper artifacts are committed and pushed;
- production, connector, services, and `graphify-out/` remain untouched.
