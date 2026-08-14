# K2 Goal-Bearing Law Lab Environment Preregistration V1

Status: `FROZEN AFTER ADVERSARIAL REVIEW / PAPER AUTHORITY ONLY`

Date: `2026-08-14`

Architectural authority remains `ARCHITECTURE_CANON.md`. This document defines
one isolated research substrate. It does not reopen S1C-4, alter K1 discovery,
or grant K2, product, execution, or deployment authority.

## 1. Plain-Language Objective

Production traffic answered the previous question conclusively:

```text
1,024 / 1,024 ordinary requests classified
exact machine-readable pre-action goals  0
S1C-4 terminal verdict                  EMPTY_GOAL_SURFACE
```

Waiting for more rows inside that immutable window cannot create a goal that
was absent before action. Retrospective goal inference would manufacture the
property under study.

The next route is therefore separate:

```text
isolated goal-bearing environment
-> exact typed goal frozen before action
-> simultaneously available K1 actions
-> bounded authority-free choice
-> isolated safe probe
-> exact independent oracle
-> durable decision episode
```

The finite engineering objective is to prove that this route can execute and
record an exact goal-conditioned decision without touching production. It is
not yet to prove that Nando learned meaning.

## 2. Frozen Starting State

```text
K1 certified operational laws       1 / 3 minimum seed
K1 product execution                LIVE / unchanged
Law Lab sandbox                     CAPABILITY PASS / RUNTIME OFF
S1C-4 natural goal census           TERMINAL EMPTY_GOAL_SURFACE
natural exact pre-action goals      0 / 1,024
K2 scientific evidence             0 episodes
K2 execution authority             false
implementation authority           false
deployment authority               false
```

Meaningful composition requires at least two independently certified K1 laws.
The preferred K1 experiment seed remains three laws. Because only one genuine
K1 law exists now, the first executable example may be a generated capability
self-test only. Its evidence namespace and claims must remain disjoint from
certificate-bound research and natural evidence.

## 3. Exact Scientific Boundary

The canonical K2 target remains:

> A compact goal-conditioned state/action/effect equivalence class that
> predicts which bounded action or composition reaches a pre-action typed goal,
> survives causal intervention, and transfers across unseen realizations.

This slice can establish only:

```text
K2_GOAL_ENVIRONMENT_CAPABILITY_PASS
```

It cannot establish any item in the scientific or product ladder:

```text
K2_DYNAMICS_PASS
K2_COMPRESSION_PASS
K2_MEANING_PASS
K2_LAW_PASS
K2_MECHANISM_PASS
K2_PRODUCT_PASS
```

No lower claim implies a higher claim.

## 4. Three Evidence Classes

### 4.1 Generated Capability Self-Test

Purpose: verify serialization, temporal order, selector plumbing, sandbox
execution, exact evaluation, journal replay, and authority denial.

```text
generated environment         allowed only in capability namespace
fixture action descriptors    allowed only in capability namespace
LawCertificate contribution   forbidden
K1/K2 evidence contribution   forbidden
natural future contribution   forbidden
product economics credit      forbidden
```

### 4.2 Certificate-Bound Laboratory Research

Purpose: run bounded interventions over genuine immutable K1 action identities.
Every action must be bound to the same frozen Epistemic Registry snapshot and
to its independently issued LawCertificate, BundleV4, execution certificate,
applicability guard, effect contract, semantic class, and role topology roots.

Laboratory outcomes may distinguish hypotheses. They remain laboratory
evidence and cannot substitute for independent natural future.

### 4.3 Natural Evidence

Natural evidence remains owned by the existing ordinary-traffic capture,
future, verifier, cleanup, certification, and admission routes. This new
environment cannot write into those ledgers or represent a laboratory event as
ordinary traffic.

## 5. Frozen Typed Contracts

### 5.1 `K2GoalEnvelopeV1`

The goal is sealed before action selection and contains:

```text
schema
goal_envelope_root_sha256
environment_root_sha256
goal_kind
goal_predicate_root_sha256
expected_goal_manifest_root_sha256
expected_goal_store_snapshot_root_sha256
horizon
constraints_root_sha256
oracle_contract_root_sha256
created_at_unix_ms
```

V1 supports one exact terminal goal kind:

```text
workspace_tree_root_equals(expected_tree_root_sha256)
```

The expected manifest is copied from a read-only, content-addressed goal store
before predictions. The horizon is one disposable sandbox execution. Free
text, model output, selected action, action ranking, post-action state, and
terminal outcome are forbidden inputs to the goal envelope.

### 5.2 `K2K1ActionRefV1`

Each genuine action reference contains:

```text
action_root_sha256
law_certificate_root_sha256
epistemic_registry_member_root_sha256
bundle_v4_root_sha256
execution_certificate_root_sha256
applicability_guard_root_sha256
effect_contract_root_sha256
semantic_class_root_sha256
role_topology_root_sha256
```

Generated capability actions use a separate fixture descriptor and may not set
any certificate-bound field.

### 5.3 `K2K1VocabularySnapshotV1`

```text
schema
snapshot_root_sha256
provenance
epistemic_registry_revision
epistemic_registry_root_sha256
ordered action references
captured_at_unix_ms
```

Certificate-bound mode requires at least two action references. Capability
mode requires at least two fixture action descriptors so the route actually
contains a choice rather than a single scripted action.

Immediately before the decision freeze, the freeze owner re-reads the external
Epistemic Registry projection and requires its revision and root to equal the
snapshot. A mismatch emits `STALE_BEFORE_FREEZE`; no decision freeze or probe is
created. This check observes registry authority and cannot mutate it.

### 5.4 `K2AlternativeSetV1`

Every alternative is available against the same frozen environment root and
contains one action root, one applicability witness root, one bounded operation
plan root, and one predicted consequence schema. The applicability witness
must bind that exact shared environment root. Action roots are unique and
sorted. At least two alternatives are required.

Certificate-bound alternatives must be pairwise distinct in LawCertificate,
semantic-class, effect-contract, and predicted-consequence roots. Capability
alternatives must have pairwise-distinct fixture effect roots and predicted
consequence roots. Different names or hashes over otherwise equivalent actions
do not create a meaningful alternative.

The alternative set is frozen before ranking. Candidate order, source identity,
human labels, and stable hash may not grant semantic preference. Stable hash is
tie-break only.

### 5.5 `K2DecisionFreezeV1`

The immutable pre-action freeze binds:

```text
schema
decision_freeze_root_sha256
episode_id_sha256
provenance
goal_envelope_root_sha256
vocabulary_snapshot_root_sha256
alternative_set_root_sha256
initial_environment_root_sha256
selector_contract_root_sha256
oracle_contract_root_sha256
budget_root_sha256
deterministic_seed_sha256
previous_journal_entry_root_sha256
frozen_at_unix_ms
authority=false
```

The durable freeze must complete before predictions, ranking, selection, or
sandbox execution. Journal ordinal and durable publication order establish the
temporal claim; wall-clock timestamps are descriptive only.

### 5.6 Predictions And Selection

`K2AlternativePredictionSetV1` records one prediction for every frozen
alternative:

```text
action_root_sha256
predicted_terminal_tree_root_sha256
predicted_goal_satisfied
prediction_evidence_root_sha256
```

The set also binds predictor schema, predictor contract or executable root,
provenance, all input roots, and the journal sequence at creation. Outcome,
oracle receipt, terminal manifest, and post-action roots are forbidden inputs.

The full prediction set is durably precommitted before selection. The
`PreparedCapabilitySelectorV1` chooses an action only when exactly one
alternative predicts goal satisfaction. Otherwise it returns
`NO_UNIQUE_SELECTION` and executes nothing. Its receipts carry
`learned=false` and are ineligible for K2 compression or meaning datasets.

This prepared selector is an explicit baseline. Its success proves route
capability, not learned K2 meaning.

### 5.7 `K2ProbePlanV1`

The probe plan binds the decision freeze, complete prediction set, selected
action, exact Law Lab sandbox request, source tree, worker manifest,
deterministic seed, and budgets. It is durable before execution.

The selected action cannot modify the goal, alternatives, predictions, oracle,
or budget. One episode executes at most one probe.

### 5.8 `K2LawLabBindingV1`

The adapter emits one canonical binding before dispatch over:

```text
episode identity
decision freeze root
goal envelope root
vocabulary snapshot root
alternative set root
prediction set root
selected action root
Law Lab request root
source tree root
worker manifest root and worker SHA-256
deterministic seed root
budget root
```

The sandbox receipt must validate against that exact request. A valid receipt
from a different candidate, goal, source tree, worker, seed, or episode is a
replay and is rejected.

The first source slice does not change `LawLabSandboxRequestV1` or its frozen
purposes. It uses the existing `GeneratedCapabilitySelfTest` purpose internally
and keeps certificate-bound sandbox dispatch closed until a later separately
preregistered integration slice.

### 5.9 Independent Exact Oracle

The exact oracle receives only:

```text
frozen goal predicate
validated Law Lab execution receipt
terminal workspace tree manifest
```

It must not read selector scores, selected-action rationale, expected candidate
output, or model text. The candidate action and selector cannot act as oracle.
The oracle emits `K2ExactGoalReceiptV1`, binding the decision, probe, terminal
tree root, exact boolean satisfaction, and oracle contract root.

The frozen oracle manifest binds its schema and executable SHA-256. Its identity
must differ from the selector and sandbox worker identities. The oracle's
complete input type contains only the validated goal predicate, exact sandbox
binding/receipt, and terminal tree manifest; negative tests reject additional
selector or prediction fields.

Capability PASS requires that executable to run as a separate process through
the canonical `K2ExactOracleRequestV1 -> K2ExactOracleOutcomeV1` protocol. A
manifest label without execution of the hashed binary is invalid. The binary
has no network, sandbox execution, journal, selection, or authority API.

### 5.10 Terminal Outcome And Episode Seal

`K2DecisionOutcomeReceiptV1` is created before the terminal journal event and
binds:

```text
decision freeze root
prediction set root
probe plan root or null
sandbox receipt root or null
exact goal receipt root or null
terminal verdict
authority boundary
```

The complete canonical outcome receipt is embedded in the terminal event. Its
root is therefore bound by the event without an external payload lookup.

After publication, `K2DecisionEpisodeSealV1` is derived deterministically from:

```text
outcome receipt root
terminal journal event root
final deterministic projection root
authority boundary
```

The episode seal is never fed back into the terminal event and is never an
event payload. This one-way order avoids a circular hash. Exactly one terminal
outcome event exists per episode; every reader derives the same episode seal.

## 6. Temporal Order

```text
environment snapshot
-> goal envelope sealed
-> vocabulary snapshot sealed
-> alternatives sealed
-> decision freeze append + sync
-> predictions append + sync
-> selection/probe plan append + sync
-> probe-dispatched marker append + sync
-> sandbox execution
-> independent exact oracle
-> terminal receipt append + sync
```

No step may be reconstructed from a later outcome. A missing durable pre-action
step makes the episode ineligible.

## 7. Safe Probe Contract

The implementation must reuse the existing Law Lab sandbox isolation instead
of creating another executor:

```text
network                       off
production state mounts       forbidden
production writes             forbidden
secrets                       absent
host PID namespace            absent
source snapshot               read-only
workspace                     disposable
shell interpretation          forbidden
deterministic seed            required
cleanup receipt               required
```

Existing Law Lab generation, wall, CPU, memory, disk, input, output, process,
model-call, and model-token ceilings are upper bounds. This K2 slice may only
reduce them.

## 8. Durable Episode Journal

The journal is an append-only directory with one canonical event file per
ordinal. Publication is `temp file -> file sync -> no-replace rename ->
directory sync`. A published event is never rewritten, truncated, or compacted.
Every event binds:

```text
schema
episode_id_sha256
sequence
previous_entry_root_sha256
event_kind
event_payload_root_sha256
canonical event payload
entry_root_sha256
written_at_unix_ms (descriptive only)
```

Each mutating pre-action publication is durable before the next state
transition. Startup uses one pure deterministic projector over ordinal files
and validates canonical bytes, filename/sequence identity, payload/root parity,
hash chain, sequence continuity, legal state transitions, root references, and
at-most-one terminal outcome. Unknown files, missing ordinals, stale temp files, and noncanonical or
partial events fail closed; published evidence is never silently repaired.

V1 permits at most 16 events and 1 MiB of canonical event bytes per episode,
64 KiB per event, and 64 retained capability episodes under one configured lab
root. The journal has no retention authority. It may not truncate, compact,
rewrite, delete, or merge episodes.

## 9. State Machine And Terminal Outcomes

```text
EMPTY
-> CONTRACT_FROZEN
-> PREDICTIONS_PRECOMMITTED
-> PROBE_PLANNED
-> PROBE_DISPATCHED
-> PROBE_EXECUTED
-> OUTCOME_VERIFIED
-> TERMINAL
```

Allowed terminal outcomes:

```text
CAPABILITY_PASS
LAB_GOAL_SATISFIED
LAB_GOAL_NOT_SATISFIED
INSUFFICIENT_K1_VOCABULARY
CERTIFICATE_BOUND_RUNTIME_CLOSED
STALE_BEFORE_FREEZE
NO_MEANINGFUL_ALTERNATIVES
NO_UNIQUE_SELECTION
SANDBOX_VERIFICATION_FAIL
ORACLE_MISMATCH
BUDGET_EXHAUSTED
SAFETY_VETO
INDETERMINATE_AFTER_CRASH
```

No result releases execution or certificate authority. Capability provenance
may terminate only as `CAPABILITY_PASS` or a failure verdict; it may not emit a
scientific or certificate-bound laboratory goal claim. Every projection repeats
provenance and the all-false authority block rather than relying on display
context.

## 10. Authority Matrix

| Component | Freeze goal | Propose | Select | Probe | Verify | Certify | Execute hot |
|---|---:|---:|---:|---:|---:|---:|---:|
| Goal environment owner | yes, pre-action | no | no | no | no | no | no |
| K1 snapshot reader | no | no | no | no | no | no | no |
| Prepared selector | no | predictions only | authority-free | no | no | no | no |
| Law Lab sandbox | no | no | no | isolated only | no | no | no |
| Exact oracle | no | no | no | no | exact lab outcome only | no | no |
| Episode journal | no | no | no | no | no | no | no |
| Existing certification/admission | no | no | no | no | external evidence only | unchanged | unchanged |

All new receipts carry:

```text
law_certificate_issued=false
package_activated=false
execution_authority_granted=false
k1_registry_mutated=false
k2_claim_granted=false
phase_memory_mutated=false
product_economics_credited=false
natural_holdout_satisfied=false
```

## 11. Failure And Restart Rules

- Failure before durable decision freeze creates no episode.
- Failure after freeze but before a durable probe plan terminalizes without
  execution.
- The writer appends and syncs `PROBE_DISPATCHED` before process creation.
- A dispatched episode without an exact validated execution receipt is
  `INDETERMINATE_AFTER_CRASH`; it is never silently rerun under the same
  episode identity.
- Sandbox or cleanup verification failure is terminal and cannot be converted
  into a goal outcome.
- Oracle disagreement or malformed receipt is terminal `ORACLE_MISMATCH`.
- Restart never changes roots, budgets, predictions, selection, or provenance.
- A failed capability episode may be repeated only under a new episode identity;
  it cannot overwrite the failed episode.

## 12. Implementation Slice V1

Only the following source work is in scope after paper and preflight PASS:

```text
nando-operator-learning
  -> pure K2 goal/vocabulary/alternative/freeze contracts
  -> prepared exact selector
  -> exact terminal-tree oracle protocol and isolated oracle binary
  -> append-only episode journal and restart projection
  -> adapter validation for existing Law Lab sandbox receipts
  -> capability fixtures and focused tests
```

Certificate-bound contracts are validated in this slice, but their executor
returns `INSUFFICIENT_K1_VOCABULARY` while fewer than two genuine K1 actions are
present and `CERTIFICATE_BOUND_RUNTIME_CLOSED` otherwise. Opening that dispatch
requires a later paper contract with real registry baselines.

Out of scope:

```text
transition-serving integration
K1 scheduler changes
generation 606 mutation
ordinary traffic capture
model training
hidden representation
automatic composition search
BundleV4 creation
LawCertificate issuance
phase mutation
package activation
dashboard changes
deployment or service restart
```

## 13. Required Tests

```text
goal bytes are canonical and changing post-action data cannot change the goal
goal is durable before predictions and selection
every alternative has exactly one precommitted prediction
zero or multiple predicted satisfiers execute nothing
fixture and certificate-bound namespaces cannot cross-decode or cross-promote
duplicate/aliased action roots are rejected
tampered K1 certificate binding is rejected
registry revision/root drift returns STALE_BEFORE_FREEZE without an episode
selector inputs cannot influence oracle evaluation
hashed oracle binary is actually executed and its canonical outcome verified
sandbox receipt from another K2 episode is rejected as replay
sandbox receipt must bind the exact frozen probe and terminal tree
network/secret/production-write authority remains false
journal rejects gaps, forks, duplicate terminal receipts, and noncanonical rows
journal terminal outcome and derived episode seal have no circular root
journal rejects partial events, stale temp files, and byte/event budget overflow
restart projection is byte/root stable
crash after durable dispatch cannot rerun the scientific episode
capability PASS leaves every authority flag false
existing Law Lab and K1 scheduler tests remain unchanged
fmt and strict Clippy PASS
```

All builds and tests run on the mini-PC with `-j20`. No deployment is authorized.

## 14. Stop Rules

Stop before code on any of:

```text
structural gate WATCH or VETO
implementation preflight not READY_TO_IMPLEMENT
K1 fixture identity can be mistaken for a certificate-bound action
two aliases can satisfy the meaningful-alternative denominator
goal can depend on selection or outcome
oracle can read selector-private evidence
selector, oracle, or worker executable identities are equal
sandbox receipt can be replayed across episodes
sandbox reuse requires production mounts, network, or hot changes
journal cannot distinguish pre-action durability from post-action append
any route writes natural, K1, certification, economics, or phase authority
```

Stop the episode on any budget, safety, isolation, binding, canonicalization,
cleanup, or exact-oracle failure.

## 15. Success Condition

The implementation slice is complete only when an isolated generated
capability episode demonstrates:

```text
typed goal frozen and synced before choice
-> at least two authority-free fixture alternatives
-> complete prediction precommit
-> unique prepared selection
-> existing Law Lab sandbox execution
-> exact independent terminal-tree evaluation
-> restart-stable terminal episode
-> every authority flag false
```

The displayed verdict must remain exactly:

```text
K2 GOAL ENVIRONMENT  CAPABILITY PASS
K1 ALTERNATIVES      FIXTURE ONLY
K2 SCIENTIFIC CLAIM  NOT EVALUATED
RUNTIME AUTHORITY    FALSE
DEPLOYED             NO
```

## 16. Applied Adversarial Review

The preserved review is
`K2_GOAL_ENVIRONMENT_CRITIQUE_V1.md`. All P0 and P1 repairs are incorporated in
this frozen version: atomic registry staleness, semantic alternative diversity,
exact K2-to-sandbox binding, durable pre-dispatch state, executable oracle
identity, immutable ordinal journal files, storage ceilings, predictor
provenance, exact goal-store snapshot binding, deterministic restart projection,
a non-circular terminal outcome/episode seal, and a closed certificate-bound
executor in the first source slice.

Implementation remains forbidden until split NANDA gates pass and the exact
implementation preflight returns `READY_TO_IMPLEMENT`.
