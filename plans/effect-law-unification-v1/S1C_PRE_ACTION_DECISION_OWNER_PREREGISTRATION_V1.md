# S1C Pre-Action Decision Owner Preregistration V1

Status: `S1C-0 PAPER FREEZE PASS / NO RUNTIME CHANGE / AUTHORITY FALSE`

Date: 2026-08-11

Parent plan: `GROUNDED_MEANING_ARCHITECTURE_V1.md`

This document freezes the only permitted S1C implementation route. It does not
claim that a grounded decision episode exists, does not open K2, and does not
authorize S1C-1 code until the separate critique and structural verification
accept this contract.

## 1. Exact Objective

S1B proved a negative fact: production has transition evidence, but no durable
pre-action goal, alternative set, horizon, or satisfaction receipt. S1C may add
the missing observation contract. It may not infer meaning after the action or
change which response is served.

```text
pre-action typed goal already present
+ anchored K1-certified action vocabulary
+ existing admitted runtime snapshot
+ one deterministic applicability evaluation
-> durable decision contract
-> byte-identical existing selection and execution
-> independent consequence verification
-> grounded decision episode or named censor
```

The S1C-0 acceptance claim is only:

> The producer, identities, temporal order, persistence protocol, budgets,
> negative controls, deployment boundary, and finite terminal outcomes are
> frozen tightly enough to implement and falsify S1C without redesigning it
> during the experiment.

## 2. Evidence Freeze 0

The read-only production snapshot was collected on the mini-PC before this
document was written. No service, runtime file, registry, or journal was
changed.

```text
dashboard schema                         nando.control-dashboard-snapshot.v1
dashboard build                          2026.08.11-control-v12
snapshot generated_at_unix               1786432861

ordinary ingress since watermark
  requests                               68,902
  input tokens                           14,744,561,859
  complete since watermark               true

product registry
  ACTIVE packages                        2
  response registry schema               nando.response-registry.v6
  response registry revision             1600967834321909500
  response registry file bytes           595,600

K1 certification authority
  ledger revision                        4
  ledger root                            478dd06b...e0416b3
  K1-eligible latest packages            1
  laws / semantics / topologies          1 / 1 / 1
  K1 gate                                CLOSED

S1B decision census
  transition rows scanned                12,854
  transition rows projected              1,866
  goal-bound                             0
  alternative-bearing                    0
  decision episodes                      0
  distinct decision lineages             0
  verdict                                EMPTY_DECISION_SURFACE
  blocker                                missing_pre_action_goal
  report root                            4a4bef8e...e984

live safety
  false accepts                          0
  runtime parity failures                0
  CPU allowed                            true
  response package-counter overflow      0

transition-serving process
  PID                                    165670
  restart count                          0
  RSS sample                             168,432 KiB
  five-second CPU sample                 0.80% of one core

gateway-control process
  PID                                    1035203
  restart count                          0
  RSS sample                             67,056 KiB
  five-second CPU sample                 0.60% of one core

Nginx transport PID                      682430
Nginx restart count                      0
```

The two ACTIVE packages are product execution facts, not two K1 letters. The
latest anchored certification projection marks only the natural MS4 package as
`k1_unit_eligible=true`. The legacy scalar package must not manufacture a K2
alternative.

The accepted isolated no-capture evaluator baseline remains the three-run F8-D
receipt on the mini-PC:

```text
matched p99 ns                           645301 / 646618 / 648010
no-match p99 ns                          194403 / 194290 / 195340
hard max ns                              690741 / 659750 / 660128
budgets ns                               1000000 / 250000 / 2000000
```

S1C-1 must rerun the same current-checkout baseline before comparing candidate
code. Historical measurements are a frozen oracle, not a substitute for the
fresh A/B run.

## 3. Production Route And Owners

There is no separate response-actor service. The production owner is the
`nando-transition-serving` process; `nando-response-actor` is linked into that
binary.

Current exact route:

```text
provider request capture
  crates/nando-transition-serving/src/lib.rs:4820
-> existing pre-action topology commitment
  crates/nando-transition-serving/src/lib.rs:4850
-> response actor entry
  crates/nando-transition-serving/src/lib.rs:5172
-> admitted ResponseExecutor snapshot acquired
  crates/nando-transition-serving/src/lib.rs:5207
-> existing exact-Wave precommit
  crates/nando-transition-serving/src/lib.rs:5256
-> current combined applicability/ranking/selection/execution
  crates/nando-response-actor/src/package.rs:966
-> actual operator execution begins
  crates/nando-response-actor/src/package.rs:1131
```

S1C-1 may introduce these ownership boundaries only:

| Owner | New responsibility | Forbidden responsibility |
|---|---|---|
| `nando-operator-learning::grounded_decision` | Pure typed contracts, exact mechanical goal binder, predicate artifacts, validation | Hot execution, admission, certification, IO |
| `nando-response-actor::ResponseExecutor` | One opaque pre-action evaluation and consumption of that exact evaluation | Goal inference, certification, persistence, K2 claims |
| `nando-transition-serving::grounded_decision_capture` | Join frozen owners, append and sync precommit, emit censors and counters | Change selection, grant authority, train, mutate phase |
| External response admission | Permitted execution snapshot | K1 epistemic eligibility, applicability |
| Anchored operator-certification ledger | Latest K1 epistemic eligibility | Runtime applicability, action selection |
| Existing runtime evaluator | Applicability and ranking under the current observation | Goal creation, LawCertificate issuance |
| Actor plus independent verifier | Actual consequence and runtime parity | Retroactive goal or available-set creation |
| Cold grounded-decision census | Append-cursor join and terminal projection | Backfill, relabel, execute, certify |

Dependency direction is fixed:

```text
operator-learning contracts
        ^
        |
response-actor pure evaluation
        ^
        |
transition-serving orchestration + persistence

operator-admission authority ----------read-only snapshot----^
operator-certification authority ------read-only projection--^
```

No new service, network hop, scheduler, model, periodic worker, or second
applicability implementation is permitted.

Registry, admission, and certification inputs are published off-path as one
immutable `DecisionAuthoritySnapshotV1` beside the cached `ResponseExecutor`.
The refresh owner reads fingerprints before and after validation and publishes
only when both reads match. A torn or stale refresh is rejected; request threads
never read certification files or rebuild this index.

## 4. Goal Surface

### 4.1 Eligible Natural Goal

A natural S1C goal exists only when it is available before evaluation and is
either:

1. an exact typed goal already present in an authenticated protocol contract;
2. an exact typed consequence mechanically derived from bounded source-neutral
   pre-action fields by a frozen deterministic binder; or
3. an externally supplied content-addressed typed-goal envelope whose capture
   provenance proves that transition-serving did not inject it.

The binder is not an LLM and does not classify free text. A hash of free text is
still free-text authority and remains forbidden.

Allowed pre-action input classes:

```text
authenticated protocol kind and projection kind
provider capture commitment and pre-action topology roots
bounded typed role/value artifacts already present before action
explicit typed tool-choice and tool-result contract, when exact
externally supplied typed-goal envelope with capture provenance
canonical consequence type
frozen exact predicate schema
frozen same-request terminal horizon schema
```

Allowed consequence types remain:

```text
scalar | record | collection | boolean | rendered_sequence
```

S1C V1 supports only the same-request terminal horizon. Multi-request plans and
compositions belong to later stages and cannot enter through a broad timeout.

### 4.2 Exact Predicate Artifact

`TypedGoalContractV1` already binds predicate and verifier roots. S1C-1 must add
a resolvable `TypedGoalPredicateArtifactV1`, capped at 4 KiB canonical bytes,
with no executable code and one of these exact comparators:

```text
typed_value_root_equals
record_projection_root_equals
collection_multiset_root_equals
collection_count_equals
boolean_equals
rendered_sequence_root_equals
```

The artifact stores typed target roots, not raw request text or raw session
payload. The independent verifier must reproduce the predicate result from the
same artifact and terminal consequence.

### 4.3 Absolute Denylist

None of the following may affect the goal contract or binder result:

```text
request free text, embeddings, or LLM classification
package ID, bundle ID, package origin, or proof counters
applicable-package count, routing margins, rank, tie, or selected action
actor output, tool output, verifier result, success, or failure
post-action state, terminal receipt, or upstream response
teacher labels, generated fixtures, Law Lab output, or synthetic traffic
future session rows or later topology
K1 scheduler candidate, identifier result, or phase state
```

Any required field outside the allowlist produces `MISSING_EXACT_GOAL`; it does
not trigger a permissive fallback binder.

## 5. Certified Action Identity

### 5.1 Product Admission Is Not K1 Eligibility

An action enters the S1C available set only if all conditions hold in the same
frozen snapshot:

```text
package is present in validated external response admission
+ latest anchored certification entry has k1_unit_eligible=true
+ certification entry binds the same package execution payload
+ package validates under the current registry and runtime contract
+ existing evaluator finds it applicable to this pre-action observation
```

A product-only, legacy, partial-law, revoked, stale, or certification-mismatched
package remains executable under its existing authority but is excluded from K2
evidence.

### 5.2 Source-Neutral Projection

S1C-1 must define `K1ActionContractProjectionV1`. Its public evidence contains
only:

```text
schema
semantic_law_id_sha256
role_topology_id_sha256
program_semantic_class_id_sha256
effect_contract_root_sha256
applicability_contract_root_sha256
verifier_contract_root_sha256
pinned_callee_set_root_sha256
consequence_type
action_contract_root_sha256
```

The projection must reuse the existing
`ProgramSemanticClassDescriptorV1` components: effect law, role schema,
protocol-mode set, executable behavior, and verifier contract. It must not
create a second semantic classifier.

The public `action_contract_root_sha256` excludes package ID, bundle ID,
registry revision, admission lease timestamps, proof counts, ranking margin,
phase coordinates, request surface, and source identity. Two packages with the
same tested action semantics quotient to one available action.

An internal `OpaqueActionExecutionBindingV1` binds that source-neutral root to:

```text
execution payload root
external admission package-binding root
latest certification entry root
frozen registry root and revision
frozen certification ledger root and revision
```

Only its root enters the decision precommit. Package IDs remain in their
existing authority/runtime receipts and are reconstructed during the cold join;
they are not K2 model inputs.

If any semantic-class, effect, role, applicability, verifier, callee, execution,
admission, or certification root cannot be independently reproduced, the action
is `ACTION_PROJECTION_INCOMPLETE`. Missing roots are never synthesized from a
package ID or display label.

### 5.3 Complete Available Set

`AvailableActionContractsV1` contains the sorted unique applicable certified
action roots plus the canonical ABSTAIN root. The hard capacity is 256 action
roots.

```text
applicable certified actions == 0  -> NO_APPLICABLE_K1_ACTION
applicable certified actions == 1  -> valid set, no meaningful alternative
applicable certified actions >= 2  -> alternative-bearing after quotient
applicable certified actions > 256 -> CAPACITY_EXHAUSTED
```

On capacity exhaustion K2 evidence is censored. Ordinary serving still uses the
unchanged top-8 selection behavior.

## 6. One Evaluation, One Execution

The current `execute_inner` combines applicability, top-8 ranking, tie handling,
selection, and execution. S1C must not copy it.

Frozen API shape:

```text
ResponseExecutor::evaluate_pre_action(request, provider_payload, k1_index)
-> PreparedResponseEvaluation
   |- complete K1 action roots or named evidence censor
   |- existing top-8 candidates and current diagnostics, private
   `- request/snapshot identity, private

durable DecisionContractPrecommitV1

ResponseExecutor::execute_prepared(prepared)
-> exact current RoutedResponseExecution
```

`PreparedResponseEvaluation` is opaque, non-serializable, non-cloneable, and
consumed exactly once. Package IDs, margins, and winner are inaccessible to the
goal binder and decision ledger. `execute_prepared` rejects a different
executor snapshot, request digest, provider digest, or reused prepared object.

The evaluator may compute its existing top-8 ranking internally in the same
bounded pass, but no selected action is published and no action is executed
until the precommit has synced. The goal contract is already frozen before this
evaluation. This is the exact temporal boundary that preserves one evaluator
without allowing ranking or outcome to create the goal.

The existing `ResponseExecutor::execute` remains a compatibility wrapper:

```text
evaluate_pre_action without K2 capture
-> execute_prepared
```

Every current caller therefore retains one decision path.

The `k1_index` is the immutable component of `DecisionAuthoritySnapshotV1`.
Changing registry, external admission, certification ledger, revocation state,
or runtime contract creates a new snapshot root; prepared evaluations cannot
cross that root.

## 7. Decision Contract And Temporal Order

`DecisionContractPrecommitV1` must bind:

```text
schema and precommit root
request-event identity root
process-epoch root
pre-action observation and topology roots
TypedGoalContractV1 root
PreActionGoalBindingReceiptV1 root
constraint contract root
outcome horizon contract root
observation mask and feature-exclusion roots
response registry schema, revision, and canonical root
external admission authority root
certification ledger revision and root
K1 vocabulary gate root
applicability evaluator schema and runtime-contract root
AvailableActionContractsV1 root
opaque execution-binding-set root
journal sequence
action_selection_not_before_sequence
precommit monotonic nanos
authority_ready=false
phase_mutation_allowed=false
```

The write order is fixed:

```text
capture pre-action observation
-> freeze exact goal and horizon
-> evaluate applicability once
-> seal precommit
-> append framed CBOR record
-> sync_data returns success
-> derive durability receipt from verified frame coordinates
-> existing exact-Wave precommit, if open
-> consume prepared evaluation
-> selected-action/runtime-verifier receipts
-> terminal goal-satisfaction receipt
```

The precommit cannot contain its own physical frame coordinates without a
circular root. `DecisionContractDurabilityReceiptV1` is therefore a deterministic
read/recovery projection over:

```text
precommit root + segment ID + offset + payload length + payload digest
```

Only a frame returned by `sync_data` and later reproduced by the ledger reader
is durable. No second self-referential write receipt is invented.

Sequence rules:

```text
journal_sequence > 0
action_selection_not_before_sequence > journal_sequence
selected-action receipt references the precommit root
selected-action sequence >= action_selection_not_before_sequence
goal-satisfaction receipt references the same goal and horizon roots
```

Monotonic time is scoped by `process_epoch_root`. A crash after durable
precommit but before selection produces `PRECOMMIT_WITHOUT_SELECTION`; the
action is never replayed merely to complete evidence.

S1C-1 must add `SelectedActionBindingReceiptV1` with:

```text
precommit root
selected source-neutral action-contract root or ABSTAIN root
opaque execution-binding root
runtime verification receipt root
selected-action sequence and monotonic nanos
process-epoch root
```

It is sealed only after consuming the prepared evaluation. The cold census
rejects a selected action not present in the precommitted available set or
bound to a different snapshot.

If append, sync, root validation, capacity, or lock acquisition fails:

```text
K2 evidence                  named censor
prepared serving decision    consumed normally
ordinary output              unchanged
K1 authority                 unchanged
phase memory                 unchanged
```

## 8. Persistence And Retention

The journal owner is `nando-transition-serving`. The frozen path is:

```text
/var/lib/nando-wave/transition/grounded-meaning-v1/
  decision-contract-precommits-v1/
```

Use the existing framed-CBOR ledger format and crash-tail recovery. Do not add a
JSONL hot-path writer or a file per request.

Budgets:

```text
canonical precommit payload              <= 32 KiB
available action roots                   <= 256
active segment                           64 MiB
sync cadence                             every eligible precommit
active journal hard quota                2 GiB
raw request/session/provider payload     0 bytes
periodic full-archive scans              0
idle polling workers                     0
```

The cold census advances only from a durable `(segment_id, offset)` cursor after
filesystem change notification or an explicit post-deployment check. It does
not wake to rescan an unchanged archive.

Retention may delete a sealed segment only after every record is represented by
an anchored census checkpoint and no active experiment references it. The cold
census owns eligibility; transition-serving owns physical deletion. Quota
exhaustion disables new K2 capture and leaves serving running. It never evicts
unsettled evidence to make room.

Persisted bytes are scanned in tests with unique raw markers. Presence of raw
request text, session text, provider body, tool arguments, actor output, or
upstream output is a hard VETO.

## 9. Resource And Behavior Budgets

All builds, tests, benchmarks, scans, and calculations run on the mini-PC.

```text
ordinary output parity                    byte-identical
status/reason/package selection parity    exact
false accepts                             0
runtime parity failures                   0
no-goal incremental p99                   <= 250 us
no-goal hard incremental ceiling          <= 2 ms
eligible sync-path incremental p99        <= 5 ms
eligible sync-path hard ceiling           <= 20 ms
hot RSS delta                             <= 16 MiB
idle CPU delta, 60-second average         <= 0.25% of one core
request-path unbounded allocation         forbidden
request-path archive scan                 forbidden
```

The sync-path budget is separate because honest durability has an IO cost. It
cannot be hidden in the no-goal denominator. Missing the budget returns VETO;
the budget is not relaxed after measurement.

## 10. Required Tests And Oracles

### 10.1 Contract And Identity

- canonical roundtrip, unknown-field rejection, root tamper, and all size caps;
- action roots stable across package rename, registry order, and lease renewal;
- action roots change for effect, role, applicability, verifier, callee, or
  consequence changes;
- duplicate implementations quotient to one action;
- product-only and non-K1 packages never enter the K2 action set;
- anchored certification rollback, stale entry, revocation, and payload mismatch
  censor evidence.

### 10.2 Leakage And Temporal Negatives

- changing only free text cannot create or change a goal;
- changing rank, margin, selected package, output, verifier result, or outcome
  cannot change the precommitted goal;
- late goal binding, selected action without durable precommit, reused prepared
  evaluation, different request, and different executor snapshot are rejected;
- crash after append header, payload, sync, precommit, and selection yields the
  frozen terminal/censor result after restart;
- persisted-byte scan proves raw payload absence.

### 10.3 Serving Parity

The parity oracle compares current `execute_inner` behavior with both:

```text
compatibility execute wrapper
prepared evaluation -> execute_prepared
```

It covers no authority, no match, grounded bind failure, applicability guard
failure, ties, exact threshold, top-8 overflow, more than 256 applicable
packages, actor abstain, verifier failure, and successful local execution.
Every observable `RoutedResponseExecution` field must match.

### 10.4 Persistence And Resource

- partial-tail recovery, duplicate request identity, segment rotation, cursor
  restart parity, quota exhaustion, and retention ownership;
- release A/B latency on pinned CPU with at least 4,096 no-goal and 4,096 matched
  cases per run, three runs;
- isolated sync-path benchmark with at least 1,024 durable records, three runs;
- RSS at load, warmup, evaluation, append, and recovery;
- 60-second idle CPU observation with unchanged input files.

## 11. Slice Boundaries

### S1C-1: Pure Code And Tests

Permitted:

- pure contracts, goal predicate artifact, certified action projection;
- prepared evaluator split with compatibility wrapper;
- journal implementation and fault-injection tests;
- current/candidate parity and resource receipts on the mini-PC.

Forbidden: deployment, live capture, dashboard claims, model training.

### S1C-2: Shadow Producer

Feature flag defaults off. With capture on:

```text
authority_ready=false
model_training_allowed=false
phase_mutation_allowed=false
local accept authority unchanged
```

Any capture failure is visible in counters but never changes serving output.

### S1C-3: Transactional Deployment

Only `nando-transition-serving` is an owning runtime binary. If its linked code
changes, one explicit transactional hot restart is required with old/new binary
hashes, PID change, rollback binary, config hash, journal root, health, and
15-second survival in the deployment receipt.

Nginx, connector, gateway-control, cold learner, admission authority, and
certification authority remain untouched unless a later separately frozen slice
changes their binaries. Rollback disables capture and restores the previous
serving binary; it never deletes forward evidence.

### S1C-4: Finite Natural Census

The natural window terminates at the first of:

```text
10,000 ordinary pre-action surfaces after the deployment watermark
72 hours after the deployment watermark
hard safety VETO
```

The terminal denominator is `ordinary_decision_boundary_seen`: ordinary
requests that reach the response pre-action decision boundary after the
deployment watermark. Total gateway ingress remains a separate context
denominator and is never substituted for it. The report must expose:

```text
ordinary_gateway_ingress
ordinary_decision_boundary_seen
exact_goal_eligible
goal_missing
authority_snapshot_ready
applicability_evaluated
no_applicable_k1_action
action_projection_incomplete
capacity_exhausted
single_action_only
alternative_bearing
durable_precommits
selected_actions
settled_goal_receipts
complete_decision_episodes
distinct_decision_lineages
all named censors
```

No synthetic or manually targeted request counts toward this window.

Terminal classification is evaluated in this strict, mutually exclusive order:

```text
VETO
  leakage, identity drift, false accept, parity failure, durability failure,
  resource budget breach, or serving regression occurs.

EMPTY_GOAL_SURFACE
  exact-goal count is zero at the bounded terminal.

EMPTY_ALTERNATIVE_SURFACE
  exact goals exist, but no episode has at least two distinct applicable
  certified action roots after semantic quotient.

INSUFFICIENT_LINEAGES
  alternative-bearing complete episodes exist, but fewer than two independent
  decision lineages exist at the bounded terminal.

PASS
  alternative-bearing complete natural episodes exist in at least two
  independent decision lineages, and every safety and parity gate passes.
```

S2 additionally requires two independently realized K1 laws in those episodes.
A single K1 action plus ABSTAIN is not a composition vocabulary.

## 12. Frozen Stop Rules

Stop S1C immediately on any of:

```text
goal depends on free text, selected action, ranking, or outcome
product package treated as K1 without latest anchored certification
second applicability evaluator or recomputed serving decision
raw payload persistence
precommit not durable before selected-action publication and execution
action identity changes under package rename or lease renewal
ordinary serving output differs from the parity oracle
false accepts > 0 or runtime parity failures > 0
resource or disk budget breach
structural gate WATCH or VETO
```

No budget, denominator, allowlist, predicate family, horizon, or terminal window
may be widened after seeing S1C evidence. A required change creates V2 and a new
post-change watermark.

## 13. Immediate Next Action

The independent adversarial critique and seven split fail-closed structural
routes passed. S1C-1 may begin from the current serving behavior oracle. No
runtime code or deployment is part of S1C-0.
