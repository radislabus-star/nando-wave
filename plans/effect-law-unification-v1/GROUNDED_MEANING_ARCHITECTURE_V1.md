# Grounded Meaning Architecture V1

Status: `CANONICAL RESEARCH CONTRACT / IMPLEMENTATION NOT STARTED`

Date: `2026-08-10`

Revision: `DECISION-GROUNDING REPAIR`

Owners:

- K1 execution and certification: existing operator owners;
- K2 representation learning and hypothesis proposal: cold
  `nando-operator-learning` research plane;
- exact action and consequence truth: actor plus independent verifier;
- execution authority: existing external admission only.

This contract corrects a naming and architecture error. The repository has
historically used bare `L1/L2/L3` both for internal Wave representation layers
and for recursive levels of executable intelligence. Those are different axes.
Bare level names are no longer sufficient in canonical claims.

## 1. Decision

Nando keeps two independent but connected routes:

```text
PRODUCT ROUTE
ordinary evidence
-> K1 operational law
-> BundleV4
-> verifier-safe CPU execution
-> exact economics

GROUNDED MEANING ROUTE
pre-action typed goals + available K1 actions
-> verified natural and laboratory decision episodes
-> explicit baselines + learned hidden representations
-> goal, action, intervention, horizon, and substitution tests
-> explicit K2 law candidate
-> independent future
-> MetaSkillPackage
-> the same verifier and admission boundary
```

K1 laws are valuable even if K2 fails. They execute real actions on CPU and
grow product coverage. K2 is not another name for a larger `CompositionDag`.
It asks whether Nando can discover a representation that preserves the
meaning of actions across different realizations, interventions, and novel
compositions.

The minimum K1 gate of three laws is a seed for the first K2 experiment. It is
not a completed alphabet, a semantic breakthrough, or a limit on K1 growth.
K1 may contain hundreds or thousands of independently certified laws.

## 2. Namespace Contract

Use these names in new canonical documentation, reports, and dashboards:

```text
W1  surface-wave representation
W2  contextual motif/interference representation
W3  transferable action-induction representation

K0  source-neutral relation and execution primitives
K1  certified operational action laws
K2  grounded meanings and composition laws over verified decisions
K3  verified strategies over K2 meanings
K4  methods that improve strategy and law discovery
```

`W1/W2/W3` describe how evidence is represented inside the Wave learner.
`K0/K1/K2/K3/K4` describe what level of executable knowledge has been proved.
They are not interchangeable. A K1 law may be discovered with W1-W3 machinery;
that does not make it a K3 epistemic law.

Forbidden unqualified claims:

```text
"L1 understands"
"L2 is open"
"three letters form a language"
"a latent vector is meaning"
```

Required replacements name the axis, for example `W2 motif transfer`, `K1
LawCertificate`, or `K2 grounded-meaning experiment`.

## 3. Operational Meaning, Dynamics, And Grounded Meaning

A K1 law has extensional operational semantics:

```text
pre-action structural state
+ grounded roles
+ applicability boundary
+ typed action
-> independently verified consequence
```

That is real meaning. It is enough for deterministic execution and exact
verification. It does not by itself prove that the machine has formed an
abstraction shared by different actions or implementations.

Predicting a transition is not yet grounded meaning. A model can learn
`state + action -> next state` while remaining blind to why the action was
selected, which alternatives were possible, and whether the result satisfied
the intended goal. That earns at most a dynamics claim.

For this project, a K2 grounded meaning is defined operationally:

> A compact goal-conditioned state/action/effect equivalence class is a
> grounded meaning only when it predicts which bounded action or composition
> reaches a pre-action typed goal, how relevant consequences change under
> causal intervention, and which changes are nuisance, across unseen
> realizations and independent natural future.

This definition avoids a philosophical authority claim. Nando does not prove
subjective understanding. It can prove cross-realization predictive transfer
that cannot be explained by IDs, retrieval, surface similarity, or the current
explicit effect algebra.

The claimed object is the equivalence class that survives those tests, not the
coordinates of one learned vector. Two implementations may share meaning in a
declared scope; the same action under different goals may not.

## 4. The Unit Is A Decision Episode, Not An Operator Vector

Meaning does not belong to `COUNT`, `FILTER`, or one BundleV4 in isolation. It
is conditioned on state, roles, goal, constraints, available actions, selected
action, horizon, and consequence.

`GroundedTransitionEpisodeV1` remains the atomic fact projection derived from
existing receipts:

```text
GroundedTransitionEpisodeV1
|- episode_root
|- evidence_class                 NATURAL | LAB
|- pre_action_state_root
|- observed_constraint_root       OPTIONAL, absence is explicit
|- grounded_role_environment_root
|- k1_law_id
|- bundle_id
|- action_binding_root
|- verified_delta_root
|- post_action_state_root
|- independent_verifier_root
|- lineage_root
|- capture_generation_root
|- censor_or_terminal_disposition
`- provenance_root
```

It can support K1 effect analysis and a K2 dynamics baseline. It cannot alone
support a grounded-meaning claim. The K2 learning and evaluation unit is:

```text
GroundedDecisionEpisodeV1
|- decision_episode_root
|- evidence_class                    NATURAL | LAB
|- pre_action_observation_root
|- typed_goal_contract_root          REQUIRED
|- goal_binding_receipt_root         PRE-ACTION / INDEPENDENT
|- constraint_contract_root
|- observation_mask_root
|- available_action_contracts_root   includes ABSTAIN
|- selected_action_or_sequence_root
|- frozen_outcome_horizon_contract
|- transition_episode_roots
|- verified_delta_sequence_root
|- goal_satisfaction_receipt_root
|- alternative_probe_manifest_root   OPTIONAL / LAB only
|- independent_verifier_root
|- lineage_root
|- capture_generation_root
|- censor_or_terminal_disposition
`- provenance_root
```

`TypedGoalContractV1` is captured before action. It must be source-neutral,
bounded, independently recoverable from pre-action evidence, and exactly
verifiable at the frozen horizon. A post-hoc LLM summary, teacher concept name,
package label, or successful outcome cannot manufacture a goal.

```text
TypedGoalContractV1
|- goal_contract_root
|- pre_action_goal_evidence_root
|- typed_success_predicate_root
|- outcome_horizon_contract_root
|- observation_mask_root
|- feature_exclusion_root
|- independent_goal_verifier_root
|- binder_schema_root
`- frozen_at_sequence
```

The goal binder may inspect pre-action evidence, K0 consequence types, and the
goal-predicate verifier contract. It may not inspect the selected action, action
ranking, candidate action verifier, bundle/package identity, actor output, or
post-action state. Its receipt freezes before action selection; otherwise goal
satisfaction is circular.

`available_action_contracts_root` binds the admitted K1 actions that were
actually applicable under the same pre-action observation plus `ABSTAIN`. If
the system cannot recover a goal or at least one meaningful alternative, the
episode is classified `DYNAMICS_ONLY` and excluded from K2 meaning evidence.

Rules:

- the view is derived only from already durable evidence;
- goal, constraints, observation mask, available actions, and horizon freeze
  before prediction;
- post-action fields are unavailable to the pre-action predictor;
- source names, package labels, cohort IDs, tool names, and episode IDs cannot
  become semantic features;
- censored outcomes may train channel-availability diagnostics only;
- laboratory and natural episodes remain disjoint evidence classes;
- Law Lab may supply exact action alternatives and counterfactual outcomes, but
  never a natural choice, natural future, or authority;
- the derived view cannot mutate K1, Wave phase memory, economics, or serving.

## 5. Hidden Representation Contract

The first candidate mechanism is action-conditioned and JEPA-inspired, but
JEPA is a hypothesis, not an architectural truth. A latent predictor is not a
meaning engine until goal and action interventions survive. Factor state, goal,
action, and uncertainty:

```text
pre-action observation + grounded roles
-> context encoder
-> z_s                         candidate world-state representation

typed goal + constraints + frozen horizon
-> goal encoder
-> z_g                         candidate goal representation

source-neutral K1 action contract
-> action encoder
-> z_a                         action representation

z_s + z_g + z_a + u_t
-> action-conditioned predictor
-> predicted delta, goal satisfaction, and target representation

verified transition sequence + GoalSatisfactionReceipt
-> target encoder
-> observed target representation

u_t                            latent uncertainty / missing factors
```

The model-induced relation `m(state, goal, action, horizon)` is only a candidate
equivalence relation. `z_s`, `z_g`, and `z_a` are coordinates, not meaning;
`u_t` is uncertainty, not meaning. A model must not hide prediction error
inside unconstrained uncertainty.

The immutable owner is `MeaningModelSnapshotV1`, which binds:

```text
model schema and weights root
encoder and predictor roots
training episode roots
frozen split roots
feature exclusion root
objective and metric roots
baseline roots
intervention manifest root
resource budget
created_at
```

No learned vector is serialized into BundleV4, used as `law_id`, or admitted as
runtime authority. Representations belong to a model snapshot and a decision
episode.
Changing model weights changes the snapshot root without changing a certified
K1 operator.

## 6. Strong Baselines Before Latent Claims

Every K2 experiment compares the candidate against all preregistered baselines:

| Baseline | Shortcut it controls |
|---|---|
| `B0_ID` | episode, package, source, cohort, and stable-hash lookup |
| `B1_SURFACE` | lexical, length, formatting, and topology-frequency similarity |
| `B2_RETRIEVAL` | nearest verified transition or exact replay |
| `B3_TYPED_ALGEBRA` | explicit role, effect, applicability, and composition contracts |
| `B4_TYPED_SEARCH` | bounded exhaustive planning over the same K1 contracts and gas |
| `M1_HIDDEN` | candidate learned hidden representation |

`B4_TYPED_SEARCH` is the strongest scientific baseline. The hidden route must
beat it on preregistered heldout prediction at matched information and compute
to earn a meaning claim. Matching exact quality with a preregistered material
search reduction earns only `K2_COMPRESSION_PASS`. If the explicit planner is
sufficient, the honest result is
`EXPLICIT_PLANNER_SUFFICIENT`, not semantic grokking.

## 7. Frozen Evaluation Surfaces

Model and baseline roots freeze before any scored outcome. Splits are by
lineage and realization, never random rows from one repeated episode.

Required surfaces:

```text
SURFACE HOLDOUT
  new wording and layout, same verified transition

IMPLEMENTATION SUBSTITUTION
  different K1 programs with the same consequence

GOAL INTERVENTION
  same state and action, different typed goal; satisfaction must change

ACTION ALTERNATIVE
  same state and goal, intervene on one available action

CAUSAL INTERVENTION
  change one effect-relevant relation; prediction must change

NUISANCE INTERVENTION
  change a preregistered irrelevant surface; prediction must remain stable

NOVEL COMPOSITION
  heldout K1 combination and ordering

HORIZON CHALLENGE
  same immediate delta, different preregistered delayed consequence

NATURAL FUTURE
  post-freeze ordinary episode with durable precommitted prediction
```

The scored packet reports exact denominators for every surface. Splits keep
goals, lineages, realizations, and intervention families disjoint. Empty
surfaces are `WATCH`, not zero-error PASS.

## 8. K2 Claim Ladder

Claims remain separate:

```text
K2_DYNAMICS_PASS
  a frozen model predicts unseen verified transitions
  -> effect-model evidence only; no grounded-meaning claim

K2_COMPRESSION_PASS
  M1_HIDDEN matches exact bounded planning with materially less frozen compute
  -> search-efficiency evidence only; no grounded-meaning claim

K2_MEANING_PASS
  M1_HIDDEN beats the strongest surviving baseline on goal intervention,
  action alternatives, substitution, causal/nuisance intervention, horizon,
  and novel composition holdouts
  -> research evidence only

K2_LAW_PASS
  one explicit composition/equivalence law survives version-space quotient,
  independent natural future, exact-memory cleanup, and LawCertificate
  -> Epistemic Registry member

K2_MECHANISM_PASS
  removing or shuffling the hidden representation destroys the K2 transfer
  while matched explicit and capacity controls do not explain the result
  -> MechanismCertificate evidence

K2_PRODUCT_PASS
  crystallized MetaSkillPackage executes ordinary traffic before upstream,
  independent verifier passes, exact economics is recorded
  -> Product Registry member
```

None of these verdicts implies another.

## 9. From Hidden Proposal To Explicit Execution

The hidden model may rank hypotheses, identify equivalence candidates, and
select safe distinguishing probes. It cannot supply authority or remain an
opaque hot executor.

```text
frozen K1VocabularySnapshotV1
+ GroundedDecisionEpisodeV1 roots
+ MeaningModelSnapshotV1
-> candidate equivalence / composition relation
-> bounded explicit meta-program version space
-> exact replay through the existing MS7 executor
-> semantic quotient
-> distinguishing Law Lab probe when needed
-> independent post-freeze natural future
-> explicit MetaSkillPackage
-> existing ExecutionCertificate / LawCertificate / MechanismCertificate
-> external admission
```

The latent model root may appear in proof provenance. The admitted package must
contain a typed, bounded, unfoldable program and an independent verifier. Every
callee generation is pinned; revocation propagates to dependants. Unknown or
ambiguous composition is `ABSTAIN`.

## 10. Authority Matrix

| Component | May create goal after outcome | May propose | May probe | May certify | May execute hot | May admit |
|---|---:|---:|---:|---:|---:|---:|
| Pre-action goal binder + decision projector | no | no | no | no | no | no |
| K1 Scheduler | no | cohort only | no | no | no | no |
| Meaning model | no | yes | rank only | no | no | no |
| Law Lab | no | no program hints | bounded isolated | no | no | no |
| Existing identifier | no | explicit hypotheses | no | candidate only | no | no |
| Actor + independent verifier | no | no | execute/verify | receipt only | only admitted program | no |
| Certification authority | no | no | no | yes from complete evidence | no | no |
| External admission | no | no | no | no | grants bounded lease | yes |

Hard vetoes:

```text
latent prediction -> local accept
model embedding -> law identity
post-action outcome -> invented goal
selected action -> inferred goal
transition prediction -> grounded-meaning claim
lab probe -> natural holdout
surface similarity -> semantic equivalence
prepared DAG -> natural K2 discovery
teacher concept label -> K2 authority
three K1 laws -> automatic K2 PASS
```

## 11. Delivery Slices

Implement one slice at a time:

```text
S0   namespace and dashboard correction                         THIS CONTRACT
S1A  read-only GroundedTransitionEpisodeV1 projection           no model
S1B  TypedGoalContractV1 + GroundedDecisionEpisodeV1 census     no model
S2   B0-B4 baseline runner over frozen decision episodes        shadow only
S3   MeaningModelSnapshotV1 candidate                            shadow only
S4   preregistered goal/action/substitution/intervention eval
S5   one explicit K2 candidate through version space/future
S6   MetaSkillPackage through existing certification/admission
```

S1-S4 must not restart hot serving or change CPU authority. S1B publishes exact
counts for goal-bound, alternative-bearing, dynamics-only, censored, and
lineage-independent episodes. If any required surface is empty, model training
does not start. Heavy builds, training, archive scans, and scored evaluation run
only on the mini-PC. Model calls are not required for the baseline or
representation learner.

## 12. Stop Conditions

Stop or narrow the research route when:

- fewer than two independently realized operational laws exist for a claimed
  equivalence or composition test;
- a pre-action typed goal, frozen horizon, or applicable action set cannot be
  recovered without post-hoc labels;
- the dataset contains only single-action transitions and no intervention can
  distinguish alternatives;
- a split leaks source, package, cohort, episode, or future identity;
- the hidden model does not beat `B4_TYPED_SEARCH` under the frozen claim;
- uncertainty absorbs intervention errors;
- representation collapse or nuisance sensitivity survives repair;
- the exact verifier cannot express the claimed consequence;
- product execution would require the latent model as authority;
- the route produces no new heldout predictive value within its frozen budget.

Failure of `M1_HIDDEN` does not close Nando. It means explicit typed planning is
currently the better K2 mechanism. Failure of a K2 experiment does not
revoke valid K1 CPU laws.

## 13. Current Honest State

```text
K1 certified operational laws       1 / 3 minimum seed
K1 product execution                LIVE
Law Lab exact-outcome substrate     CAPABILITY PASS / RUNTIME OFF
GroundedTransitionEpisodeV1         NOT IMPLEMENTED
TypedGoalContractV1                 NOT IMPLEMENTED
GroundedDecisionEpisodeV1           NOT IMPLEMENTED
B0-B4 frozen baseline packet        NOT IMPLEMENTED
MeaningModelSnapshotV1              NOT IMPLEMENTED
K2 dynamics claim                   NOT EVALUATED
K2 grounded-meaning claim           NOT EVALUATED
K2 natural law                      BLOCKED BY BASIS AND EVIDENCE
K2 execution authority              false
```

The next architectural implementation is S1A and S1B, not model training:
derive leakage-audited transition facts, then prove whether pre-action goals,
available alternatives, frozen horizons, actor truth, verifier truth, evidence
class, and lineage roots can form honest decision episodes. No denominator, no
model.

Structural review packet:
`.nanda/nanda-task-grounded-meaning-architecture-v1.md`.
