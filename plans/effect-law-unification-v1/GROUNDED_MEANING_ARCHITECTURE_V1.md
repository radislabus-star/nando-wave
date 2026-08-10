# Grounded Meaning Architecture And Preregistered Execution Plan V1

Status: `CANONICAL PAPER PLAN / S0-S1B PASS / S1C NEXT`

Plan date: `2026-08-11`

Revision: `PRE-ACTION DECISION GROUNDING`

Plan structure gate: `PASS / SPLIT ROUTES 2 OF 2 / AUTHORITY FALSE`

Implementation authority: `FALSE`

This document is written and reviewed before the next implementation slice.
It is the canonical research and execution plan for the first grounded K2
experiment. It does not grant scientific, certification, admission, or runtime
authority.

Architectural authority remains `ARCHITECTURE_CANON.md`. Product execution and
economics remain owned by
`plans/nando-live-cpu-savings-v1/NANDO_LIVE_CPU_SAVINGS_MASTER_PLAN.md`.
The critical review and accepted repairs for this plan are recorded in
`GROUNDED_MEANING_PLAN_CRITIQUE_V1.md`.

Owners:

- K1 discovery, execution, and certification: existing K1 owners;
- pre-action decision evidence: the S1C decision-contract owner;
- K2 baselines and representation learning: cold `nando-operator-learning`;
- exact action and consequence truth: actor plus independent verifier;
- K2 certification: existing certificate authorities from complete evidence;
- execution authority: existing external admission only.

## 0. Paper-First Rule

No S1C-S6 behavior change starts until this plan and its critique are committed.
Every later slice requires a smaller immutable preregistration before code:

```text
SlicePreregistrationV1
|- parent_plan_commit
|- current_evidence_root
|- exact question and scoped claim
|- owner and dependency direction
|- allowed inputs and forbidden features
|- output schemas and authority flags
|- baseline and negative controls
|- frozen data split or acquisition watermark
|- runtime, memory, disk, and scan budgets
|- tests and evidence packet
|- rollback boundary
|- entry, exit, and stop verdicts
`- selected_at
```

The slice preregistration freezes before its implementation. A later discovery
may produce a new plan revision; it may not silently rewrite the old decision.
`WATCH`, `VETO`, `EMPTY`, and `INSUFFICIENT` do not become PASS through prose.

Required work order:

```text
paper plan
-> adversarial critique
-> accepted repairs
-> structural coherence gate
-> scoped implementation
-> tests and frozen evidence
-> status update
-> next slice preregistration
```

## 1. Decision And Finite Objective

Nando keeps two connected but independent routes:

```text
PRODUCT ROUTE
ordinary evidence
-> K1 operational law
-> BundleV4
-> verifier-safe CPU execution
-> exact global economics

GROUNDED MEANING ROUTE
pre-action typed goal + constraints + available K1 actions
-> durable decision contract before action selection
-> selected action + verified transition + satisfaction receipt
-> natural and laboratory decision episodes kept disjoint
-> frozen explicit baselines
-> optional learned hidden representation
-> goal, action, substitution, intervention, horizon, and composition tests
-> explicit K2 law candidate
-> independent natural future
-> MetaSkillPackage
-> the same verifier, certification, and admission boundary
```

The finite objective of this plan is one honest terminal result:

```text
K2_PRODUCT_PASS
or
EXPLICIT_PLANNER_SUFFICIENT
or
K2_ROUTE_INSUFFICIENT with an exact blocker and frozen denominator
```

The objective is not to keep a model training indefinitely. K1 laws remain
valuable if K2 fails: they execute real actions on CPU and increase verified
product coverage.

## 2. Evidence Freeze 0

The plan starts from this deployed, independently reproducible baseline:

```text
source commit                         663959064a37caf7eb917fc99dfedb6386355fa6
deployment receipt root              785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b
transition projection root           17ee90f188d5ea445d84007218713876b4abb6aecdd7fd1f53b4005715b4d52a
decision census report root          4a4bef8ec334676495851510a0ef6d5ed74039991f3b16b38e63c5893876e984
decision episode set root            f869a76401bb5319604dafd0315941b6759f94b419b06d71e0ec0ca746e676b4
```

Production census:

```text
durable CPU completions scanned      12,854
verified transitions projected        1,866
censored transitions                 10,988
distinct transition lineages             19

missing pre-action topology           1,358
missing transport binding             9,407
ambiguous transport binding             209
identity mismatch                        14

goal-bound episodes                       0
alternative-bearing episodes              0
horizon-bound episodes                    0
satisfaction-verifiable episodes          0
GroundedDecisionEpisodeV1                 0
distinct decision lineages                0
```

Honest verdict:

```text
S1A transition projection          PASS
S1B decision census                PASS
classification                     DYNAMICS_ONLY
verdict                            EMPTY_DECISION_SURFACE
blocker                            missing_pre_action_goal
model training                     false
authority                          false
phase mutation                     false
```

This is not a failed hidden model. No hidden model has been trained. The result
proves that the current archive contains verified transitions but does not
contain the pre-action contracts needed to ask why an action was selected.

## 3. Namespace And Claim Contract

Use separate names for the representation axis and the executable-knowledge
axis:

```text
W1  surface-wave representation
W2  contextual motif/interference representation
W3  transferable action-induction representation

K0  source-neutral relation and execution primitives
K1  certified operational action laws
K2  grounded meanings and composition/equivalence laws over decisions
K3  verified strategies over K2 meanings
K4  methods that improve strategy and law discovery
```

`W1/W2/W3` describe how evidence is represented inside the Wave learner.
`K0/K1/K2/K3/K4` describe what executable knowledge has been proved. They are
not interchangeable.

Forbidden unqualified claims:

```text
"L1 understands"
"L2 is open"
"three letters form a language"
"a transition vector is meaning"
"a latent vector is meaning"
"K1 3/3 automatically proves K2"
```

The K1 minimum gate of three laws is only a seed for the first K2 experiment.
It is not a complete alphabet and does not limit K1 growth. K1 may eventually
contain hundreds or thousands of independently certified laws.

## 4. Scientific Question, Null, And Falsifiers

Question:

> Can Nando learn a compact goal-conditioned equivalence over states, actions,
> consequences, and horizons that transfers across unseen realizations and
> compositions better than source lookup, retrieval, explicit effect algebra,
> and bounded typed search?

Primary null:

```text
B4_TYPED_SEARCH is sufficient at matched information and compute.
```

Secondary nulls:

```text
surface identity or retrieval explains transfer
transition prediction ignores the goal
one action dominates because no meaningful alternative exists
uncertainty absorbs causal errors
the hidden representation compresses search but adds no predictive meaning
```

Predeclared falsifiers:

- no goal can be bound before action without reading selected action or outcome;
- no state exposes at least one meaningful nonselected K1 alternative;
- fewer than two independent operational laws exist for a composition claim;
- lineage-disjoint intervention surfaces remain empty;
- `M1_HIDDEN` does not beat `B4_TYPED_SEARCH` on the frozen meaning claim;
- source, package, cohort, episode, or future identity explains the result;
- the claimed consequence cannot be expressed by an exact verifier;
- product execution would require opaque latent authority.

Any falsifier gives a scoped terminal verdict. It does not revoke valid K1
laws or close the product route.

## 5. Operational Meaning, Dynamics, And Grounded Meaning

A K1 law already has extensional operational semantics:

```text
pre-action structural state
+ grounded roles
+ applicability boundary
+ typed action
-> independently verified consequence
```

That is real operational meaning. It is enough for deterministic execution and
verification. It does not prove a shared abstraction across different goals,
actions, implementations, and compositions.

Predicting `state + action -> next state` earns at most a dynamics claim. A K2
grounded meaning is defined operationally as:

> A compact goal-conditioned state/action/effect equivalence class that
> predicts which bounded action or composition reaches a pre-action typed goal,
> how relevant consequences change under causal intervention, and which
> changes are nuisance, across unseen realizations and independent natural
> future.

The claimed object is the tested equivalence class, not the coordinates of a
single model embedding and not subjective consciousness.

## 6. Evidence Contracts

### 6.1 Transition Fact

`GroundedTransitionEpisodeV1` remains a read-only fact projected from existing
actor, verifier, topology, transport, and certification receipts. It supports
K1 effect analysis and K2 dynamics only.

### 6.2 Pre-Action Goal

`TypedGoalContractV1` freezes before action selection:

```text
pre-action goal evidence root
typed success predicate root
outcome horizon contract root
observation mask root
feature exclusion root
independent goal verifier root
binder schema root
frozen sequence
```

The binder may inspect only pre-action evidence, K0 consequence types, and the
goal-predicate verifier contract. It may not inspect action ranking, selected
action, bundle/package identity, actor output, verifier outcome, or post-action
state.

Free-text LLM inference is not an authoritative goal binder. A natural goal is
eligible only when an exact typed goal is already present in the pre-action
protocol or can be mechanically derived from bounded source-neutral fields by a
frozen binder whose result an independent verifier can reproduce. Otherwise the
episode is `MISSING_EXACT_GOAL` and remains `DYNAMICS_ONLY`.

### 6.3 Available Actions

External admission owns which K1 packages are permitted to execute. A separate
deterministic applicability evaluator owns which admitted packages are
applicable under the frozen observation. `AvailableActionContractsV1` freezes
that complete set, plus `ABSTAIN`, before ranking. A meaningful alternative is
a nonselected applicable K1 action, not a renamed copy and not only `ABSTAIN`.

### 6.4 Decision Episode

`GroundedDecisionEpisodeV1` binds:

```text
pre-action observation
goal binding receipt
constraints and observation mask
available actions
selected action or bounded sequence
frozen outcome horizon
verified transition sequence
independent goal satisfaction receipt
lineage and provenance
terminal or censor disposition
```

Natural and laboratory episodes remain separate evidence classes. Law Lab may
provide exact alternatives and counterfactual outcomes. It may not manufacture
a natural choice, independent natural future, or authority.

### 6.5 Fail-Closed Evidence, Unchanged Serving

If any pre-action receipt is missing, late, invalid, ambiguous, or not durable:

```text
K2 evidence -> CENSORED
ordinary serving -> existing K1/upstream path unchanged
K1 authority -> unchanged
phase memory -> unchanged
```

Research capture must never turn an evidence failure into a user-visible
failure or an unsafe local accept.

## 7. Delivery Plan

Only one slice may be active at a time. Each slice ends in a commit and evidence
packet before the next slice starts.

### S0. Namespace And Paper Contract

Status: `PASS`

Delivered:

- W-axis and K-axis separated;
- dynamics, meaning, law, mechanism, and product claims separated;
- latent representation denied runtime authority.

### S1A. Grounded Transition Projection

Status: `PASS`

Exit evidence:

- 12,854 durable completions have an exact disposition;
- 1,866 verified transition episodes across 19 lineages;
- 10,988 censors have an exact reason ledger;
- report is read-only, deterministic, restart-stable, and authority false.

### S1B. Decision Schema And Census

Status: `PASS / EMPTY_DECISION_SURFACE`

Exit evidence:

- canonical goal, available-action, selected-action, satisfaction, and decision
  schemas exist and validate roots;
- production census is byte-stable and fail-closed;
- no post-hoc goal was invented;
- model training correctly remains false.

### S1C. Pre-Action Decision-Contract Owner

Status: `NEXT / NOT IMPLEMENTED`

Purpose: create the missing evidence before action without changing action
authority.

Logical route:

```text
ordinary pre-action observation
-> source-neutral goal binder
-> exact goal predicate + frozen horizon
-> current external-admission registry snapshot
-> deterministic applicability evaluation under the same observation
-> complete applicable K1 action set + ABSTAIN
-> atomic durable DecisionContractPrecommitV1
-> existing action selection and execution
-> selected-action binding
-> independent verified consequence at horizon
-> GoalSatisfactionReceiptV1
-> GroundedDecisionEpisodeV1 projection
```

Ownership boundary:

- pure contracts stay in `nando-operator-learning::grounded_decision`;
- the runtime producer attaches at the existing pre-action decision boundary,
  before candidate ranking or action selection;
- external admission remains the source of admitted-package truth;
- the frozen deterministic applicability evaluator owns applicability truth;
- actor plus independent verifier remain consequence truth;
- the cold census joins receipts and cannot create them retroactively.

`DecisionContractPrecommitV1` must bind the observation, typed goal, constraints,
horizon, admission registry revision/root, applicability evaluator schema/root,
complete available-action root, feature exclusions, sequence, and durable write
receipt. Reconstructing a different available set from the same commitment is a
hard identity failure.

S1C is split before code:

```text
S1C-0 route and ownership freeze
S1C-1 pure binder, predicate, journal, and temporal-order tests
S1C-2 shadow producer with authority=false
S1C-3 transactional deployment and restart parity
S1C-4 natural append-cursor census
```

S1C entry gate:

- exact producer insertion point and dependency direction documented;
- pre-action input allowlist and post-action denylist frozen;
- authoritative free-text or LLM goal inference forbidden;
- goal predicate is exact and independently verifiable;
- admission truth and applicability truth have separate owners;
- persistence, latency, disk, and rollback budgets frozen;
- no raw request/session payload is added to the K2 ledger;
- all existing CPU false-accept and parity gates are green.

S1C exit gate:

- receipt durability precedes action selection by sequence and monotonic time;
- tamper, replay, late write, restart, and identity-rebinding tests pass;
- unavailable goal or alternative becomes a named censor;
- serving behavior and authority are byte/decision equivalent with capture off;
- live false accepts and parity failures remain zero;
- the first natural census reports an exact denominator.

S1C terminal outcomes:

```text
PASS                        valid natural decision episodes exist
EMPTY_GOAL_SURFACE          no pre-action exact goal exists
EMPTY_ALTERNATIVE_SURFACE   no meaningful action alternative exists
INSUFFICIENT_LINEAGES       episodes exist but cannot support a split
VETO                        leakage, authority drift, or runtime regression
```

No S2 work starts unless S1C is PASS and at least two independent decision
lineages exist. A composition/equivalence claim additionally requires at least
two independently realized K1 laws and meaningful nonselected alternatives.

### S2. Frozen Explicit Baselines

Status: `BLOCKED BY S1C`

Before implementation, freeze `K2ExperimentPreregistrationV1` with exact
episode roots, lineage splits, intervention families, metrics, compute budget,
and minimum denominators. Then run:

| Baseline | Shortcut controlled |
|---|---|
| `B0_ID` | episode, package, source, cohort, stable-hash lookup |
| `B1_SURFACE` | lexical, length, formatting, topology frequency |
| `B2_RETRIEVAL` | nearest verified transition or exact replay |
| `B3_TYPED_ALGEBRA` | explicit roles, effects, applicability, composition |
| `B4_TYPED_SEARCH` | bounded exhaustive planning under identical K1 contracts and gas |

S2 exit gate:

- every baseline root and compute denominator is frozen;
- training/validation/confirmatory splits are lineage-disjoint;
- all required evaluation surfaces have a nonzero preregistered denominator;
- confirmatory sample size follows a frozen power/minimum-effect rule computed
  from support only, never from confirmatory outcomes;
- identity-only controls fail to explain the target;
- the confirmatory holdout remains unread.

If the decision surface cannot support this packet, stop at
`K2_ROUTE_INSUFFICIENT`. Do not train a hidden model on a weak denominator.

### S3. Frozen Hidden Representation Candidate

Status: `BLOCKED BY S2`

The first candidate is action-conditioned and JEPA-inspired:

```text
pre-action observation + roles -> z_s
typed goal + constraints + horizon -> z_g
source-neutral K1 action contract -> z_a
z_s + z_g + z_a + bounded uncertainty
-> predicted delta, satisfaction, and target representation
```

`MeaningModelSnapshotV1` binds model code, weights, training roots, split roots,
feature exclusions, objectives, baselines, interventions, and resource budget.

Training policy:

- heavy work runs only on the mini-PC;
- no LLM/model API calls are required;
- no background trainer or full-archive polling timer is installed;
- one bounded development split may debug the pipeline;
- architecture and hyperparameter budget freeze before one confirmatory run;
- confirmatory holdout outcomes remain hidden until the snapshot root exists;
- model output has proposal authority only.

If the model collapses, leaks identity, hides errors in uncertainty, or exceeds
its frozen budget, S3 terminates without S4 promotion.

### S4. Preregistered Evaluation

Status: `BLOCKED BY S3`

Required surfaces:

```text
SURFACE HOLDOUT
IMPLEMENTATION SUBSTITUTION
GOAL INTERVENTION
ACTION ALTERNATIVE
CAUSAL INTERVENTION
NUISANCE INTERVENTION
NOVEL COMPOSITION
HORIZON CHALLENGE
INDEPENDENT NATURAL FUTURE
```

Each surface reports exact eligible, scored, censored, positive, negative, and
lineage denominators. Empty surfaces are `WATCH`, never zero-error PASS.

S4 terminal interpretations:

```text
hidden < B4              EXPLICIT_PLANNER_SUFFICIENT
hidden = B4, less cost   K2_COMPRESSION_PASS only
hidden > B4 on frozen
meaning surfaces         K2_MEANING_PASS only
```

`K2_MEANING_PASS` is research evidence. It is not a law, mechanism certificate,
or execution permission.

### S5. One Explicit K2 Law Candidate

Status: `BLOCKED BY S4`

The hidden model may rank equivalence/composition hypotheses and safe probes.
It cannot remain the executor or authority.

```text
frozen K1VocabularySnapshotV1
+ GroundedDecisionEpisodeV1 roots
+ MeaningModelSnapshotV1 provenance
-> hidden proposal
-> bounded explicit meta-program version space
-> exact replay through the existing MS7 executor
-> semantic quotient
-> distinguishing Law Lab probe if needed
-> independent post-freeze natural future
-> exact-memory cleanup
-> explicit MetaSkillPackage candidate
-> LawCertificate evidence
```

Unknown or ambiguous composition is `ABSTAIN`. Every callee generation is
pinned and revocation propagates to dependants.

### S6. Product Crystallization

Status: `BLOCKED BY S5`

```text
explicit MetaSkillPackage
-> independent verifier
-> certification
-> external admission and bounded lease
-> ordinary CPU execution before upstream
-> exact upstream fallback on abstain or verification failure
-> exact global economics
```

S6 passes only with ordinary verified CPU work, zero active false accepts, zero
runtime parity failures, and a global denominator. Package-conditional 100%
does not substitute for global product coverage.

## 8. Frozen Evaluation And Claim Ladder

Claims remain independent:

```text
K2_DYNAMICS_PASS
  frozen unseen transition prediction only

K2_COMPRESSION_PASS
  exact typed-search quality with materially lower frozen compute

K2_MEANING_PASS
  beats the strongest baseline on goal/action/substitution/intervention/
  horizon/composition transfer

K2_LAW_PASS
  explicit transferable meta-law survives quotient, independent future,
  cleanup, and LawCertificate

K2_MECHANISM_PASS
  latent ablation destroys transfer while matched controls do not

K2_PRODUCT_PASS
  admitted MetaSkillPackage completes ordinary verified CPU work
```

None implies another.

## 9. Authority Matrix

| Component | Create goal after outcome | Propose | Probe | Certify | Execute hot | Admit |
|---|---:|---:|---:|---:|---:|---:|
| Pre-action goal binder | no | no | no | no | no | no |
| Decision projector/census | no | no | no | no | no | no |
| K1 Scheduler | no | cohort only | no | no | no | no |
| Meaning model | no | yes | rank only | no | no | no |
| Law Lab | no | no program hints | bounded isolated | no | no | no |
| Existing identifier | no | explicit hypotheses | no | candidate only | no | no |
| Actor + independent verifier | no | no | execute/verify | receipt only | admitted only | no |
| Certification authority | no | no | no | complete evidence only | no | no |
| External admission | no | no | no | no | bounded lease | yes |

Hard vetoes:

```text
outcome -> invented goal
selected action -> inferred goal
latent prediction -> local accept
embedding -> law identity
transition prediction -> meaning claim
lab probe -> natural future
surface similarity -> semantic equivalence
prepared DAG -> natural K2 discovery
teacher label -> K2 authority
```

## 10. Runtime, Data, And Work Budgets

Before S1C implementation, freeze measured no-capture latency, RSS, disk, and
failure baselines. The slice budget must satisfy all of these:

- serving success and fallback behavior unchanged;
- evidence capture fail-closed while serving remains available;
- no raw request/session payload persisted in the K2 ledger;
- append-only bounded records and exact retention ownership;
- no periodic full-archive scan;
- scans advance by durable append cursor and run only after evidence changes;
- no sustained idle CPU worker;
- no unbounded model, probe, hyperparameter, or context budget;
- heavy builds, tests, scans, and training only on the mini-PC;
- hot restart only when the owning binary changes, transactionally, with an
  explicit rollback receipt;
- Nginx and unrelated services remain untouched.

Every added runtime byte and tool transition must buy a measurable increase in
evidence quality, safety, product coverage, or experiment decisiveness.

## 11. Parallel K1 Product Route

K2 must not block K1 growth:

```text
Law #2 -> ordinary verified CPU -> incremental global coverage
Law #3 -> minimum K1 experiment basis
more K1 laws -> larger action and intervention vocabulary
target -> stable >=10% verified current-epoch savings
```

K1 remains `1/3` at Evidence Freeze 0. With one independently realized law,
Nando can study dynamics and action-versus-abstain, but it cannot honestly
claim general composition or equivalence over multiple operational actions.

## 12. Stop Conditions

Stop or narrow the grounded-meaning route when:

- goals cannot be captured before action without post-hoc invention;
- meaningful alternatives or exact satisfaction predicates do not exist;
- fewer than two independent decision lineages exist;
- fewer than two independently realized K1 laws exist for a composition claim;
- a required evaluation surface is empty;
- source/package/episode/future identity leaks;
- hidden representation does not beat `B4_TYPED_SEARCH` for the frozen claim;
- the exact verifier cannot express the consequence;
- product execution would require opaque latent authority;
- no new heldout predictive value appears within the frozen compute budget;
- product coverage does not justify the additional runtime and maintenance cost.

Stopping the hidden route may produce `EXPLICIT_PLANNER_SUFFICIENT`. That is a
useful scientific result, not a failure of Nando. Stopping K2 never revokes a
valid K1 CPU law.

## 13. Evidence Matrix

| Claim | Required artifact | Missing verdict |
|---|---|---|
| Goal existed before action | goal contract + pre-action binding receipt | DYNAMICS_ONLY |
| Alternative was real | admission snapshot + applicable action roots | DYNAMICS_ONLY |
| Goal was satisfied | frozen horizon + independent satisfaction receipt | UNKNOWN |
| Decision surface exists | exact census + independent lineages | EMPTY/INSUFFICIENT |
| Hidden model predicts dynamics | frozen snapshot + unseen transition split | NOT EVALUATED |
| Hidden representation adds meaning | B0-B4 + all intervention surfaces | NOT PROVED |
| Explicit K2 law exists | quotient + future + cleanup + certificate | NOT PROVED |
| Hidden mechanism is necessary | matched ablation packet | UNRESOLVED |
| Product saves CPU | admitted ordinary receipt + global economics | UNKNOWN |

## 14. Current Honest State And Next Command

```text
K1 certified operational laws       1 / 3 minimum seed
K1 product execution                LIVE
Law Lab substrate                   CAPABILITY PASS / RUNTIME OFF
S0 paper architecture               PASS
S1A transition projection           PASS
S1B decision census                 PASS / EMPTY_DECISION_SURFACE
S1C pre-action owner                NOT IMPLEMENTED
S2 frozen baselines                 BLOCKED
S3 hidden representation            BLOCKED
S4 evaluation                       BLOCKED
S5 explicit K2 law                  BLOCKED
S6 K2 product                       BLOCKED
K2 execution authority              false
```

The next permissible engineering action is only `S1C-0`: freeze the exact
pre-action insertion point, allowlist/denylist, persistence protocol, budgets,
test oracle, and rollback boundary. No model training and no K2 claim are
permitted before natural decision episodes pass the S1C and S2 gates.

Structural review packets:

```text
.nanda/nanda-task-grounded-meaning-plan-evidence-v1.md
.nanda/nanda-task-grounded-meaning-plan-authority-v1.md
```
