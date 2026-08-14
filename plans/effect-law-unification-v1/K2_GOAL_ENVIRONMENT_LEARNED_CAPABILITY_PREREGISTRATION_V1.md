# K2 Goal Environment Learned Capability Preregistration V1

Status: `FROZEN AFTER ADVERSARIAL REVIEW / PAPER AUTHORITY ONLY`

Date: `2026-08-14`

Architectural authority remains `ARCHITECTURE_CANON.md`. This document defines
one bounded generated-fixture experiment above the already implemented K2 goal
environment V1. It does not alter V1 schemas, reopen S1C-4, modify K1 natural
discovery, or grant production, K1, K2, deployment, or execution authority.

## 1. Decision And Plain-Language Objective

The V1 goal environment proved that a frozen exact goal can be used to choose,
execute, and independently verify one of two prepared fixture actions. It did
not learn the actions' effects: the target consequences were authored before
selection.

This experiment asks the next finite question:

> Can a separate deterministic learner infer two opaque actions' exact
> filesystem effects only from support pre/post manifests, transfer those
> effects to an unseen target manifest, and let the existing V1 route choose
> and verify the uniquely goal-satisfying action?

The exact route is:

```text
opaque action IDs
+ frozen support worlds and probe schedule
+ hidden executor-only action mapping
-> six isolated support executions through existing bwrap Law Lab
-> redacted action-ID + pre/post-manifest observations
-> separate hashed effect-learner binary
-> frozen unique learned effect-law set
-> unseen target pre-manifest, still without the target goal
-> learned prediction for every action
-> durable target prediction precommit
-> conversion to V1 fixture action references
-> existing V1 exact-goal episode
-> separate exact oracle
-> learned-capability receipt
```

The only positive verdict is:

```text
K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS
```

It means that this bounded fixture route learned and transferred exact effects.
It does not mean `NATURAL_K2`, `K2_MEANING_PASS`, `K2_LAW_PASS`, Wave grokking,
certificate-bound composition, or production usefulness.

## 2. Frozen Starting State

```text
K1 genuine LawCertificates                    1 / 3
Natural K2                                    NOT PROVED
K2 goal environment V1 fixture capability    PASS
K2 V1 authority                               false
production K1 scheduler                       unchanged / autonomous
production services                           out of scope
deployment                                    forbidden
```

The implementation may reuse only the authority-free generated-capability path
of V1 and the existing Law Lab bwrap executor. It must not reinterpret fixture
actions as genuine K1 vocabulary members.

## 3. Exact Claim Boundary

### 3.1 What a PASS proves

A PASS proves all of the following for one preregistered fixture family:

1. Two experiment-specific opaque action identities were exposed to the learner without operation
   names, operation plans, path hints, target goal, or expected target output.
2. Each action was independently executed in exactly three support worlds.
3. The learner received exactly six validated, redacted pre/post observations.
4. The learner found exactly one transferable bounded effect per action.
5. The learned law set was durably frozen before the target manifest was sent
   to the predictor.
6. The target world was absent from every support observation and had a novel
   manifest, source-content hash, removable-content hash, and distractor shape.
7. The predictor emitted a complete target terminal manifest for both actions
   without receiving the target goal or expected target manifest.
8. Every target prediction was durable before the target goal was passed to the
   V1 selection adapter or any target execution began.
9. Exactly one prediction matched the preregistered exact goal.
10. The selected opaque action's hidden plan executed in the existing bwrap
    Law Lab and produced the predicted manifest.
11. The separate V1 exact oracle accepted exact terminal-tree equality.
12. Restart replay reproduced every learning and target root byte-for-byte.
13. Required ablations failed closed.
14. Every authority bit remained false and no production path changed.

### 3.2 What a PASS does not prove

```text
natural evidence                              no
genuine K1 alternatives                       no
free-form program induction                   no
variable-role or variable-path generality     no
semantic goal understanding                   no
recursive composition                         no
Wave causal mechanism                         no
LawCertificate                                no
Epistemic Registry mutation                   no
CPU product authority                         no
deployment authority                          no
```

The learned effects are deliberately narrow fixed-path laws. Transfer is over
unseen contents and irrelevant tree structure, not arbitrary path variables.

## 4. Frozen Fixture Family

### 4.1 Opaque actions

Exactly two action IDs are derived with domain-separated SHA-256 from a
harness-provided 32-byte experiment commitment and sorted. The commitment is
owned by the integration harness, is not compiled into the learner, and is not
derived from either operation. Mapping assignment is independently frozen and
must not be a function of sorted action order.

Replaying one commitment is byte-identical. A separately frozen
hardcode-resistance control uses a second commitment and therefore different
action IDs without changing the effect language.

The learner-visible catalog contains only:

```text
schema
catalog_root_sha256
action_ids_sha256[2]
```

Human labels, operation names, source paths, target paths, effect classes,
expected deltas, and operation-plan roots are forbidden in this catalog.

### 4.2 Hidden executor mapping

The orchestrator privately freezes exactly one mapping before any support
execution:

```text
opaque action A -> CopyFile {
  source_path: "input.bin",
  target_path: "selected.bin"
}

opaque action B -> RemoveFile {
  path: "obsolete.bin"
}
```

The mapping is represented with existing Law Lab operations:

```text
CopyFile   -> CopySourceFile { source_path, work_path: target_path }
RemoveFile -> RemoveWorkPath { work_path: path }
```

Before the experiment freeze, the complete canonical mapping bytes are
published as one immutable private artifact by temp write, file sync,
no-replace rename, and directory sync. The freeze binds both artifact root and
mapping root. Restart must reopen and validate those exact bytes before any
continuation. Neither the private artifact root, mapping root, mapping bytes,
nor any operation-plan root may enter a learner request or learner outcome.

### 4.3 Support worlds

Exactly three immutable support source trees are frozen. Every tree contains:

```text
input.bin       file, target source
obsolete.bin    file, removal target
one or more distractor files/directories
selected.bin    absent
```

Across the three worlds:

- `input.bin` content hashes and byte lengths are pairwise distinct;
- `obsolete.bin` content hashes and byte lengths are pairwise distinct;
- complete tree roots are pairwise distinct;
- distractor topology is not constant;
- no other file has the same `(byte length, content hash, executable)` tuple as
  `input.bin` within its world;
- all relative paths satisfy existing Law Lab V1 constraints.

No support world may be changed after the experiment freeze.

### 4.4 Probe schedule

The exact six-entry schedule is frozen before execution:

```text
support world 0 x opaque action A
support world 0 x opaque action B
support world 1 x opaque action A
support world 1 x opaque action B
support world 2 x opaque action A
support world 2 x opaque action B
```

The stable order is by support-world ordinal, then sorted opaque action ID.
Each pair executes once from the corresponding immutable source snapshot.
No adaptive support probe is allowed in V1.

### 4.5 Unseen target world

One immutable target source tree is committed in the experiment freeze but is
not included in the learning request. It has the same applicability surface
and these holdout conditions:

```text
target tree root             distinct from all support roots
input.bin content hash       distinct from all support input hashes
input.bin byte length        distinct from all support input lengths
obsolete.bin content hash    distinct from all support obsolete hashes
obsolete.bin byte length     distinct from all support obsolete lengths
distractor topology          distinct from every support topology
selected.bin                 absent
```

The exact target goal is the target tree after action A copies `input.bin` to
`selected.bin`, preserving `obsolete.bin` and every distractor. It is frozen in
the read-only V1 goal store before support execution, but its manifest and root
are forbidden from learner learning and prediction requests. The enforceable
claim is about exact serialized process input, not about what the orchestrator
could know in memory.

## 5. Frozen Typed Contracts

All structs use canonical JSON, `deny_unknown_fields`, explicit schemas, sorted
collections, nonzero lowercase SHA-256 roots, checked integer arithmetic, and
exact root recomputation on validation.

### 5.1 `K2OpaqueActionCatalogV1`

```text
schema
catalog_root_sha256
action_ids_sha256[2]
```

Validation requires exactly two unique sorted roots.

### 5.2 `K2LearnerPublicContextV1`

This is the only experiment-level context identity accepted by learner
protocols:

```text
schema
public_context_root_sha256
public_experiment_id_sha256
catalog_root_sha256
support_set_root_sha256
support_probe_schedule_public_root_sha256
allowed_effect_language_root_sha256
learner_manifest_root_sha256
learner_executable_sha256
learner_budget_root_sha256
```

It excludes the private freeze root, harness commitment, hidden mapping and
artifact roots, target holdout commitment, target manifest, target goal,
selector identity, operation plans, and expected outcomes. Learner-visible
artifacts bind this public root, never the private freeze root.

### 5.3 `K2HiddenActionMappingV1`

```text
schema
mapping_root_sha256
catalog_root_sha256
entries[2] {
  action_id_sha256
  operation: CopyFile | RemoveFile
  operation_plan_root_sha256
}
```

This value belongs only to the fixture orchestrator. It is never serialized
inside learner-visible artifacts.

### 5.4 `K2PrivateExperimentContractV1`

The canonical private artifact contains:

```text
schema
private_contract_root_sha256
experiment_id_sha256
harness_commitment_sha256
public_context_root_sha256
hidden_action_mapping
support_source_manifest_roots_sha256[3]
target_pre_manifest
target_expected_goal_manifest
target_goal_store_snapshot_root_sha256
```

It is durable before experiment freeze, path-independent, and readable only by
the test orchestrator/restart validator. Its complete bytes are forbidden in
learner stdin. The target manifests remain withheld from both learner calls
except that `target_pre_manifest` is later copied into the prediction request
after the learned law set is durable.

### 5.5 `K2SupportWorldV1`

```text
schema
world_root_sha256
world_ordinal
source_manifest
fixture_provenance_root_sha256
```

The world root binds the complete manifest, not a path to mutable bytes.

### 5.6 `K2SupportProbePlanV1`

```text
schema
plan_root_sha256
experiment_id_sha256
catalog_root_sha256
support_set_root_sha256
hidden_mapping_root_sha256
ordered_probes[6] {
  probe_ordinal
  support_world_root_sha256
  action_id_sha256
  deterministic_seed_sha256
}
```

No operation or expected result appears in an ordered probe.

### 5.7 `K2LearnedCapabilityFreezeV1`

The pre-execution freeze binds:

```text
schema
freeze_root_sha256
experiment_id_sha256
provenance = generated_capability_self_test
public_context_root_sha256
private_contract_artifact_root_sha256
catalog_root_sha256
support_set_root_sha256
support_probe_plan_root_sha256
hidden_mapping_root_sha256
target_holdout_commitment_root_sha256
learner_manifest_root_sha256
learner_executable_sha256
independent_verifier_contract_root_sha256
selector_executable_sha256
sandbox_executor_manifest_root_sha256
sandbox_worker_sha256
exact_oracle_manifest_root_sha256
exact_oracle_executable_sha256
budget_root_sha256
deterministic_seed_sha256
frozen_at_unix_ms
authority = all false
```

Learner, V1 selector, worker, and oracle executable hashes must be pairwise
distinct. The target commitment binds target pre-manifest and expected-goal
roots while withholding both values from learner learning requests and the
goal from target prediction requests. The freeze validates the exact durable
private artifact bytes and the public context, but the learner receives only
the latter.

### 5.8 `K2SupportDispatchV1`

Each support dispatch is durable before process creation and binds:

```text
experiment freeze root
probe ordinal
world root
opaque action ID
hidden operation-plan root
exact Law Lab request root
worker and executor roots
deterministic seed
```

This artifact is orchestrator-private. A dispatch without an exact validated
outcome is never retried under the same experiment identity.

### 5.9 `K2SupportObservationV1`

After the orchestrator validates the exact Law Lab receipt against the hidden
request, it emits this learner-visible redaction:

```text
schema
observation_root_sha256
public_context_root_sha256
probe_ordinal
support_world_root_sha256
action_id_sha256
source_manifest_root_sha256
pre_work_manifest
post_work_manifest
sandbox_receipt_root_sha256
```

Before redaction, the orchestrator independently validates the exact sandbox
request, receipt, worker outcome, source manifest, pre-work manifest, and
post-work manifest. `pre_work_manifest` is exactly
`worker_outcome.pre_work_manifest`; `post_work_manifest` is exactly
`worker_outcome.post_work_manifest`; `source_manifest_root_sha256` is exactly
the frozen support source root. Any mismatch invalidates the complete support
set before learning.

Forbidden fields include operation, operation plan, operation result, mutation
path hint, expected delta, goal, target root, selector evidence, and target
output. The receipt root is opaque evidence identity only.

### 5.10 `K2EffectLearningRequestV1`

```text
schema
request_root_sha256
public_context
catalog
ordered_support_observations[6]
minimum_support_worlds_per_action = 3
allowed_effect_language_root_sha256
```

The request contains no target artifact and no hidden mapping. The allowed
language root commits only to the grammar, not to a preferred hypothesis:

```text
CopyFile { source_path, target_path }
RemoveFile { path }
```

Strict request decoding rejects every private experiment field, including a
private freeze root, mapping or mapping root, target commitment, target
manifest, goal, operation plan, selector evidence, and expected output.

### 5.11 Learned effect language

```text
K2LearnedEffectV1 =
  CopyFile { source_path, target_path }
  RemoveFile { path }
```

The learner enumerates all exact hypotheses supported by the observed deltas.
For `CopyFile`, every observation for an action must add exactly one file at a
stable target path, preserve every other entry, and have exactly one pre-state
file at a stable source path whose full file tuple equals the added file. The
source contents must vary across support worlds.

For `RemoveFile`, every observation for an action must remove exactly one file
at a stable path and preserve every other entry. Removed contents must vary
across support worlds.

No-op, constant-output, content-hash lookup, world-specific, path-varying,
multi-delta, metadata-only, or ambiguous-source hypotheses are admitted.

### 5.12 `K2LearnedEffectLawSetV1`

```text
schema
law_set_root_sha256
learning_request_root_sha256
learner_manifest_root_sha256
learner_executable_sha256
support_observation_set_root_sha256
allowed_effect_language_root_sha256
laws[2] {
  action_id_sha256
  effect
  supporting_world_roots_sha256[3]
  supporting_observation_roots_sha256[3]
  enumerated_candidate_count
  enumerated_candidate_roots_sha256[]
  rejected_candidate_count
  rejection_counts_by_reason
  version_space_size = 1
  law_root_sha256
}
learned = true
authority = all false
```

The learner must enumerate the complete bounded grammar against all three
observations for each action. Candidate and rejection counts must reconcile,
candidate roots are unique and sorted, and at most 32 candidates may be
enumerated per action. Exactly one survivor per opaque action is required. The
set is durably frozen before the target pre-manifest is sent to any prediction
process.

### 5.13 `K2TargetIndependenceReceiptV1`

Before target prediction, an independent holdout checker emits:

```text
schema
receipt_root_sha256
support_set_root_sha256
target_pre_tree_root_sha256
support_tree_roots_pairwise_distinct = true
target_tree_root_novel = true
target_input_hash_novel = true
target_input_length_novel = true
target_obsolete_hash_novel = true
target_obsolete_length_novel = true
target_distractor_topology_novel = true
target_absent_from_learning_request = true
```

All fields must be true and are recomputed from complete manifests and exact
canonical learner request bytes. The receipt is durable before the target
manifest enters the predictor.

### 5.14 `K2TargetPredictionRequestV1`

```text
schema
request_root_sha256
public_context_root_sha256
catalog_root_sha256
learned_law_set
target_pre_manifest
```

The request excludes target goal, expected target manifest, hidden mapping,
selector evidence, action preference, and operation plans.

### 5.15 `K2LearnedTargetPredictionSetV1`

```text
schema
prediction_set_root_sha256
target_prediction_request_root_sha256
learner_manifest_root_sha256
learner_executable_sha256
learned_law_set_root_sha256
target_pre_tree_root_sha256
predictions[2] {
  action_id_sha256
  learned_law_root_sha256
  predicted_terminal_manifest
  prediction_root_sha256
}
learned = true
authority = all false
```

Both predictions are computed by exact complete-manifest transformation. They
are durably published before the target goal object is passed to the V1
selection adapter and before any target sandbox process starts.

### 5.16 `K2LearnedEffectVerificationReceiptV1`

A library-owned verifier independently checks the external learner outcome:

```text
schema
verification_root_sha256
verifier_contract_root_sha256
public_context_root_sha256
support_observation_set_root_sha256
learned_law_set_root_sha256
target_prediction_set_root_sha256
verified_support_laws = 2
verified_target_predictions = 2
wrong_laws = 0
wrong_predictions = 0
authority = all false
```

The verifier uses a separate exhaustive delta checker and complete-manifest
transformer. It may share canonical data types and hashing helpers, but it must
not call the learner's candidate enumeration, law construction, prediction,
or selection functions. For every target prediction it compares the complete
sorted entry vector, total bytes, tree root, and exact preservation of every
unaffected entry.

### 5.17 V1 conversion binding

`K2LearnedToV1BindingV1` binds every learned target prediction to exactly one
generated-capability `K2K1ActionRefV1`:

```text
opaque action ID
learned law root
predicted terminal tree root
V1 fixture action root
hidden operation-plan root
V1 predicted consequence root
```

The predicted roots must be identical. Operation-plan roots come only from the
pre-frozen hidden mapping and are not inputs to the learner. Existing V1
schemas and validators remain byte-for-byte unchanged. V1 continues to mark
its prepared prediction and selection receipts `learned=false`; only the
higher-level V2 binding may claim that their consequence roots originated in
the separately frozen learned prediction set.

### 5.18 `K2LearnedCapabilityOutcomeV1`

The terminal outcome binds:

```text
experiment freeze root
all six support dispatch and observation roots
support evidence set root
learning request and learned law-set roots
target independence receipt root
target prediction request and set roots
independent verification receipt root
learned-to-V1 binding root
V1 decision freeze root
V1 prediction set root
V1 selection root
V1 Law Lab binding and execution receipt roots
V1 exact-goal receipt root
V1 terminal outcome and episode seal roots
ablation receipt root
support worlds = 3
support executions = 6
learned laws = 2
target predictions = 2
wrong predictions = 0
verdict = K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS
authority = all false
```

The outcome is valid only after exact cross-object replay. A post-event seal
binds the immutable terminal event root and final projection without creating
a cyclic root dependency.

## 6. Learner Executable Boundary

The separate binary is named `nando-k2-effect-learner`. Its canonical protocol
accepts exactly one tagged request on stdin and emits exactly one canonical
outcome on stdout:

```text
learn_effects   -> K2EffectLearningRequestV1 -> K2LearnedEffectLawSetV1
predict_target  -> K2TargetPredictionRequestV1 -> K2LearnedTargetPredictionSetV1
```

Requirements:

- executable SHA-256 is frozen before support execution;
- stdin and stdout are canonical JSON with no trailing bytes;
- stderr is empty on success;
- environment is cleared except deterministic locale settings;
- the learner runs under a dedicated bwrap wrapper with all namespaces
  unshared, no network route, an empty writable tmpfs, no repository, fixture,
  home, secret, private-contract, or target-store mount, and only read-only
  runtime libraries plus the exact hashed learner executable;
- no host filesystem path, clock, randomness, environment variable, private
  freeze root, target goal, expected output, selector receipt, or hidden
  mapping is an input;
- the process cannot issue authority or write evidence;
- the caller validates every returned byte and the independent verifier checks
  every law and prediction without calling learner inference functions;
- learner failure terminalizes the experiment without target execution.

The orchestrator is not allowed to substitute an in-process learned result for
the output of the frozen executable.

## 7. Temporal Order And Durability

The learning journal uses immutable canonical ordinal event files with temp
write, file sync, no-replace publication, and directory sync. Published events
are never rewritten or truncated. Ordinal order, not wall-clock time, proves
the sequence.

Every `K2LearnedCapabilityEventV1` binds:

```text
schema
experiment_id_sha256
sequence
event_kind
payload_schema
payload_root_sha256
previous_entry_root_sha256
entry_root_sha256
recorded_at_unix_ms (descriptive only)
```

One pure deterministic projector is the only state owner. It validates every
payload's canonical bytes and replays exact references from freeze through
support dispatch/observation pairs, evidence set, law set, independence
receipt, predictions, independent verification, V1 conversion, V1 episode,
ablations, and terminal outcome. A post-terminal seal binds outcome root,
terminal event root, and final projection root; the seal is never written back
into a journal payload.

Required order:

```text
01 experiment freeze
02 support dispatch 0
03 support observation 0
04 support dispatch 1
05 support observation 1
...
12 support dispatch 5
13 support observation 5
14 support evidence set frozen
15 learned effect-law set frozen
16 target independence receipt frozen
17 target prediction set frozen
18 independent verification receipt frozen
19 learned-to-V1 binding frozen
20 V1 target episode seal observed
21 ablation receipt frozen
22 terminal learned-capability outcome
```

The target goal object may be passed to the V1 conversion/selection adapter only
after event 17 is durable. Target process creation requires event 19 and the V1
`PROBE_DISPATCHED` event to be durable.

Restart validates canonical bytes, contiguous ordinals, hash chain, event
state transitions, and all cross-event roots. Fresh and restarted projections
must be identical.

### 7.1 Crash rules

```text
crash before support dispatch publication
-> that probe did not execute; same process may append dispatch later

crash after support dispatch publication but before exact observation
-> INDETERMINATE_AFTER_SUPPORT_DISPATCH
-> no same-identity retry and no PASS

crash after support observation
-> replay validated prefix and continue with next scheduled probe

crash after learned-law freeze
-> target prediction may resume from exact frozen laws

crash after independence receipt
-> target prediction may resume from exact frozen laws and holdout receipt

crash after target predictions but before V1 dispatch
-> V1 episode may continue from exact frozen predictions

crash after V1 dispatch without exact V1 execution receipt
-> existing V1 INDETERMINATE_AFTER_CRASH rule; no retry

crash before learned terminal event
-> no learned-capability PASS exists
```

No recovery path reconstructs missing post-action evidence or silently retries
an already dispatched action.

## 8. Frozen Budgets

```text
opaque actions                         exactly 2
support worlds                         exactly 3
support probes                         exactly 6
target worlds                          exactly 1
target probes                          at most 1
main learner process invocations       exactly 2
main exact-oracle invocations          exactly 1
entries per fixture tree               at most 32
bytes per fixture tree                 at most 64 KiB
learned effects per action              exactly 1 on PASS
candidate effect hypotheses per action at most 32
learning request canonical bytes       at most 512 KiB
learner outcome canonical bytes        at most 128 KiB
target prediction request bytes        at most 256 KiB
target prediction outcome bytes        at most 128 KiB
learning journal events                at most 24
canonical bytes per event              at most 128 KiB
learning journal bytes                 at most 3 MiB
retained experiment identities         test-local only, at most 8
learner stdin bytes per invocation      at most 512 KiB
learner stdout bytes per invocation     at most 128 KiB
learner stderr bytes per invocation     at most 4 KiB, empty on success
learner wall time per invocation        at most 2,000 ms
learner CPU time per invocation         at most 1,000 ms
learner address space                   at most 256 MiB
learner visible processes               at most 2
ablation learner invocations            at most 8
ablation sandbox probes                 exactly 1
ablation oracle invocations             exactly 1
ablation canonical bytes                at most 2 MiB total
```

Every sandbox execution inherits the stricter existing Law Lab V1 limits.
Main and ablation counters are reported separately. Budget exhaustion is
fail-closed and cannot be promoted to PASS.

## 9. Required Negative Controls And Ablations

All required controls operate on copied in-memory artifacts or disposable
test stores. They cannot mutate the main successful journal.

### 9.1 Support-count ablation

Remove one world's two observations. The learner must return
exactly `INSUFFICIENT_SUPPORT`, not a law.

### 9.2 Action-identity shuffle

Swap opaque action IDs for one complete world's observations. The learner must
return exactly `NON_TRANSFERABLE_DELTA`.

### 9.3 Ambiguous copy source

Add a second pre-state file with the same full file tuple as `input.bin` in all
support worlds. The copy law must return exactly `AMBIGUOUS_SOURCE_MATCH`.

### 9.4 Constant-output ablation

Replace one copied post-state file tuple with a tuple from another world. Exact
learning must return exactly `NON_TRANSFERABLE_DELTA`.

### 9.5 Outcome-dependence ablation

Replace all six post-work manifests with their corresponding pre-work
manifests. The learner must return exactly `NON_TRANSFERABLE_DELTA`. This
demonstrates that action IDs alone cannot produce laws.

### 9.6 Dynamic-ID control

Derive a second opaque catalog from a separately frozen harness commitment,
rebind copied valid learner views by the same private mapping permutation, and
rerun learning. Exactly two laws with the same two effects and different
action IDs must be learned. Reusing either first-experiment action ID is
`FIXED_ACTION_ID_DEPENDENCY`.

The copied adversarial observation sets in 9.2-9.6 use a dedicated
`generated_capability_ablation` provenance and newly sealed roots. They are not
represented as Law Lab receipts and cannot enter the main support evidence
set.

### 9.7 Holdout alias rejection

Reuse any support pre-manifest as the target. The high-level validator must
return exactly `TARGET_NOT_INDEPENDENT` before selection or target execution.

### 9.8 Support provenance mismatch

Bind a valid post-work manifest to another probe's source, pre-work, or sandbox
receipt root. Redaction validation must return exactly
`SUPPORT_EVIDENCE_INVALID` before learner invocation.

### 9.9 Target-goal leakage rejection

A learner request containing a goal root, expected target root, hidden mapping,
private freeze root, operation plan, or selector field must return exactly
`LEARNER_REQUEST_PRIVATE_FIELD_REJECTED` through strict protocol decoding.

### 9.10 Prediction tamper

Alter either predicted target manifest after freeze. V1 conversion or restart
replay must return exactly `TARGET_PREDICTION_ROOT_MISMATCH` before target
dispatch.

### 9.11 Wrong-action exact oracle

Execute the non-selected hidden action in a disposable target copy. The exact
oracle must return `goal_satisfied=false`; constructing a learned PASS from
that receipt must return exactly `EXACT_GOAL_UNSATISFIED`.

### 9.12 Cross-experiment replay

Rebind a valid support observation, learned law set, target prediction, V1
episode seal, or ablation receipt from another experiment identity. Restart
validation must return exactly `CROSS_EXPERIMENT_REPLAY`.

### 9.13 Authority tamper

Setting any authority field true must invalidate the containing artifact and
terminal outcome with exactly `AUTHORITY_BOUNDARY_VIOLATED`.

The ablation receipt is PASS only when every required negative control reaches
its exact preregistered result. It records each control's input root, expected
verdict, observed verdict, process counts, and canonical outcome root.

## 10. State Machine And Terminal Verdicts

```text
FROZEN
-> SUPPORT_RUNNING
-> SUPPORT_COMPLETE
-> LAWS_FROZEN
-> HOLDOUT_VERIFIED
-> TARGET_PREDICTIONS_FROZEN
-> PREDICTIONS_VERIFIED
-> TARGET_EPISODE_COMPLETE
-> ABLATIONS_COMPLETE
-> TERMINAL
```

Positive terminal:

```text
K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS
```

Every terminal projection also carries one class:

```text
CAPABILITY_PASS
LEARNING_NEGATIVE
INFRASTRUCTURE_FAILURE
INDETERMINATE_AFTER_DISPATCH
```

An infrastructure or persistence failure is never reported as evidence
against the learning hypothesis.

Negative terminals:

```text
FREEZE_INVALID
SUPPORT_EXECUTION_FAILED
INDETERMINATE_AFTER_SUPPORT_DISPATCH
SUPPORT_EVIDENCE_INVALID
INSUFFICIENT_SUPPORT
AMBIGUOUS_SOURCE_MATCH
NON_TRANSFERABLE_DELTA
NON_UNIQUE_VERSION_SPACE
LEARNER_PROTOCOL_INVALID
LEARNED_LAW_SET_INVALID
TARGET_NOT_INDEPENDENT
TARGET_PREDICTION_INVALID
NO_UNIQUE_SELECTION
TARGET_EXECUTION_FAILED
TARGET_PREDICTION_MISMATCH
EXACT_GOAL_UNSATISFIED
ABLATION_FAILED
BUDGET_EXHAUSTED
REPLAY_INVALID
CROSS_EXPERIMENT_REPLAY
AUTHORITY_BOUNDARY_VIOLATED
FIXED_ACTION_ID_DEPENDENCY
```

There is no automatic retry under the same identity after a dispatch-side
indeterminate state. A new attempt requires a new preregistered experiment ID.

## 11. Authority And Side-Effect Matrix

| Capability | Allowed |
|---|---:|
| Read immutable generated fixture trees | yes |
| Write disposable test-local journals/workspaces | yes |
| Execute existing bwrap Law Lab worker | yes |
| Execute separate hashed learner and oracle binaries | yes |
| Pass target goal to V1 selector after learned predictions are durable | yes |
| Read production traffic or natural evidence | no |
| Write natural evidence or K1 scheduler state | no |
| Issue LawCertificate or registry member | no |
| Activate a package or route CPU traffic | no |
| Mutate Wave/phase memory | no |
| Access secrets or network | no |
| Control services or deploy | no |

Every new artifact repeats the existing all-false `K2AuthorityBoundaryV1`.

All disposable action workspaces must carry existing Law Lab cleanup receipts.
The learner bwrap tmpfs disappears with the process. On test completion, the
fixture owner removes private artifacts, journals, source snapshots, goal
store, and ablation stores, then records a test-only cleanup receipt proving
every temporary path absent. Only explicitly copied canonical outcome roots
may remain in test output; no automatic evidence retention is allowed.

## 12. Implementation Slice

Allowed source changes are limited to:

```text
crates/nando-operator-learning/src/k2_goal_environment/learned_capability.rs
crates/nando-operator-learning/src/k2_goal_environment/learned_journal.rs
crates/nando-operator-learning/src/k2_goal_environment/mod.rs
crates/nando-operator-learning/src/bin/nando-k2-effect-learner.rs
crates/nando-operator-learning/tests/k2_goal_environment_learned_v1.rs
plans/effect-law-unification-v1/K2_GOAL_ENVIRONMENT_LEARNED_*.md
.nanda/nanda-task-k2-goal-environment-learned-*.md
```

One minimal export adjustment in `crates/nando-operator-learning/src/lib.rs`
is allowed only if the existing module export does not already expose the new
public types.

Forbidden changes include V1 K2 schemas, Law Lab V1 schemas, production
scheduler, serving, admission, certification, economics, dashboard, transport,
natural evidence, deployment scripts, service definitions, and `graphify-out/`.

No deployment is authorized by this slice.

## 13. Required Tests

### 13.1 Unit and contract tests

- canonical round-trip and root tamper rejection for every new type;
- exact two-action catalog and hidden mapping coverage;
- support-set diversity and target holdout independence;
- support observation redaction and unknown-field rejection;
- exact bounded hypothesis enumeration for copy and remove;
- ambiguous-source and non-transferable-delta rejection;
- exact target manifest simulation for both effects;
- independent verifier parity without calls into learner inference functions;
- public learner context contains no private commitment or target field;
- four executable identities are pairwise distinct;
- learner bwrap mount/network/environment and process-budget checks;
- dynamic-ID and outcome-dependence controls;
- all-false authority validation;
- journal legal transitions, budgets, tamper, gap, duplicate, and cross-root
  rejection;
- restart parity at every legal prefix;
- crash-after-dispatch no-rerun behavior;
- V1 conversion equality without V1 schema changes.

### 13.2 Real-process integration

On the mini-PC, with the actual release binaries:

```text
3 support worlds x 2 actions             6 / 6 exact bwrap executions
learner learn_effects process             PASS
frozen learned laws                       2 / 2 unique
learner predict_target process            2 / 2 exact predictions
independent support/target verification   4 / 4 exact
target unique selection                   1 / 2
target bwrap execution                    PASS
separate exact oracle                     PASS
ablation learner processes                within 8 / all exact verdicts
wrong-action ablation bwrap + oracle       1 + 1 / goal unsatisfied
restart replay                            byte-identical
disposable fixture cleanup                verified absent
authority                                 false
```

The test must print the terminal learned outcome root, terminal seal root,
learned law-set root, target prediction-set root, V1 outcome root, and V1 seal
root.

### 13.3 Regression and source fences

- all existing K2 V1 tests remain unchanged and pass;
- existing real bwrap Law Lab tests pass;
- full `nando-operator-learning` package tests pass;
- `cargo fmt --check` passes;
- strict `cargo clippy --all-targets -- -D warnings` passes;
- source diff contains no forbidden production path;
- seven frozen baseline artifact SHA-256 values from V1 remain unchanged.

All Rust builds and tests run on the mini-PC with `CARGO_BUILD_JOBS=20` and the
frozen remote target directory. Local work is limited to source editing,
paper gates, Git, and Entire provenance.

## 14. Stop Rules

Implementation must not begin when any of these is true:

```text
adversarial critique has an unresolved P0 or P1
any split NANDA packet is WATCH or VETO
implementation preflight is not READY_TO_IMPLEMENT
baseline bytes or modes drift
learner receives operation or target-goal information
support or target fixture fails independence constraints
V1 schema change is required
production or deployment path would be touched
```

During implementation, any scientific mismatch stops promotion of the claim,
but a repair to code that violates this already frozen contract is allowed
only when the repair does not alter fixture evidence or success thresholds. A
contract change requires a new preregistration revision and rerun of all gates.

## 15. Exact Success Verdict

The experiment may print
`K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS` only when:

```text
support schedule frozen before execution                 yes
support bwrap outcomes exact                             6 / 6
learner-visible observations operation-free              6 / 6
unique learned laws                                      2 / 2
learned law set durable before target prediction         yes
target independent from support                          yes
target predictions durable before V1 goal adapter        2 / 2
independent law and prediction verification              4 / 4
unique predicted goal-satisfying action                  exactly 1
target bwrap outcome equals learned prediction           yes
separate exact oracle accepts frozen goal                yes
required ablations                                       all PASS
fresh/restart roots                                      identical
false authority bits                                     0
production mutations                                     0
V1 schema mutations                                      0
temporary paths remaining                                0
```

Any missing item yields a named negative terminal or no verdict. A structurally
valid receipt is evidence only for this generated capability experiment.

## 16. Applied Adversarial Review

The separate critique is preserved in
`K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_CRITIQUE_V1.md`. Every P0/P1 repair is
now represented in this frozen contract:

```text
learner-public/private context split                    applied
provable no-goal serialized-input boundary              applied
independent effect and target verifier                   applied
durable private mapping artifact                         applied
four-way executable identity separation                 applied
dynamic action IDs and outcome-dependence control       applied
exact Law Lab pre/post provenance                        applied
typed journal, projector, and acyclic seal               applied
learner process limits and bwrap isolation               applied
complete bounded version-space accounting                applied
typed target independence receipt                        applied
exact per-ablation verdicts                              applied
separate main/ablation budgets                           applied
disposable fixture cleanup receipt                       applied
complete unaffected-entry preservation                  applied
terminal evidence-class separation                       applied
```

This paper freeze still grants no source authority by itself. Three split
NANDA packets must return `PASS`; then an implementation preflight must return
`READY_TO_IMPLEMENT` and `safe_to_implement=true` before any Rust edit.
