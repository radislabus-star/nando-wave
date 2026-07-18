# Nando Wave Architecture Canon

Status: canonical architectural contract.

Audience: every human or AI agent changing this repository.

This file exists to prevent a locally reasonable refactor from destroying the
project's actual mechanism. Read it before editing discovery, learning, Wave,
actor, verifier, admission, serving, storage, or economics code.

## 1. What This Project Is

Nando Wave is a cache-conscious wave memory that learns transferable actions
from completed live traces and executes only independently verified actions on
CPU.

It is not primarily:

- a collection of hand-written profiles;
- a template selector;
- a DSL enumerator;
- a response cache;
- a conventional classifier with Wave terminology added afterward.

The central mechanism is:

```text
repeated signal
-> signed interference
-> stable center of mass / phase center
-> transferable action law
-> verifier-safe CPU execution
```

The Situn, Fourier, interference, lens, carrier, and counter-wave language is
not decoration. It records the design intuition: many small signed signals are
accumulated into a compact field; coherent structure reinforces itself, noise
is cancelled, and the stable result is stored as a wave center rather than as
a list of examples. The implementation is more exact than this intuition, but
a replacement that removes the signed field, phase, interference, or compact
center has changed the project, not refactored it.

## 2. The L1 -> L2 -> L3 Meaning

### L1: surface coherence

L1 converts text surfaces into a compact wave:

```text
byte 4-grams
+ boundary atoms
+ identities for short tokens
+ service-word atoms
-> signed ternary lane contributions
-> SurfaceWave4096
```

L1 does not understand an operator. It makes differently written observations
comparable and lets character fragments cohere into stable lexical surfaces.
The hot representation is deliberately small and sparse.

Current primary implementation:

```text
crates/nando-core/src/wave/surface_wave.rs
```

### L2: motifs, interference, and context

L2 combines active L1 lanes into reusable motifs and contextual centers.
Interference reinforces compatible motifs. Lens/carrier context constrains the
field. Counter-wave evidence suppresses noise and false attraction.

L2 is not a bag of labels. It is the intermediate structure that allows new
wording and layouts to reach the same law without exact lookup.

Relevant design and implementation references:

```text
docs/SYMBOL_CELL8_ARCHITECTURE.md
docs/NANDA_WAVE_THEOREM.md
crates/nando-core/src/wave/semantic_wave.rs
crates/nando-core/src/wave/semantic_extract.rs
```

### L3: transferable action

L3 binds roles, state transitions, and action structure. Here repeated
completed transitions can stabilize into a transferable action operator:

```text
same law across different surfaces
-> role/action centers
-> positive phase center
-> applicability subcenters and anti-centers
-> compact transferable operator
```

In this project, grokking means that examples no longer have to be retained as
the authority: their common law has stabilized into a compact center. At L3,
the important result is a transferable action, including its applicability
boundary, not merely a recognized phrase.

Relevant implementation:

```text
crates/nando-core/src/wave/l3_semantic_grokking.rs
crates/nando-core/src/wave/phase_center_runtime.rs
crates/nando-response-actor/src/online_subcenter.rs
crates/nando-response-actor/src/cegis.rs
```

### Optional JEPA-inspired research layer, not the system core

The phrase "hidden part" in the product architecture refers to a compact
JEPA-inspired latent world model, not to hidden Wave memory and not to another
name for L3 phase centers.

This is an optional future research layer. It is not required for extracting
repeatable operators from completed LLM traffic, is not part of the production
hot path, and must not block the CPU-coverage goal. The canonical core is the
bidirectional transferable Wave operator defined below.

Its conceptual contract is:

```text
current structural state
-> encoder -> latent state z_t

z_t + candidate action
-> action-conditioned predictor
-> predicted latent consequence z_t+1
```

For this repository the latent state is expected to describe structured
software/runtime consequences, such as diagnostic graphs, AST relations,
state deltas, or tool outcomes. It does not reconstruct a full textual world.
It predicts which candidate actions are promising before an expensive probe or
execution.

This layer has no authority to execute. A latent prediction is search evidence,
not truth. A real actor and independent verifier must still evaluate the
consequence. Prediction error updates latent dynamics; it must not be silently
converted into an applicability anti-center.

The existing file:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hidden_state.rs
```

does **not** by itself implement this JEPA-like contract. It derives bounded
cross-layer and combination atoms for subcenter/quarantine refinement. Its
historical `hidden_state` name must not be used as evidence that an
action-conditioned latent consequence predictor already exists.

Primary conceptual references:

```text
https://ai.meta.com/blog/v-jepa-2-world-model-benchmarks/
https://arxiv.org/abs/2506.09985
```

## 3. Canonical System Core: Bidirectional Transferable Operator

The core of the whole system is a generational, bidirectional Wave operator:

```text
state / observation
-> ForwardWave
-> transferable law instance
-> actor execution
-> independent typed receipts
-> VerifiedDeltaReceipt
-> BackwardWave
-> candidate generation g+1
```

The symbolic representation expresses a law. `ForwardWave` transfers that law
to the current structural state. `BackwardWave` converts independently verified
consequences into a bounded change of the operator field. Neither direction may
bypass independent proof.

### TransferableOperatorV2

The canonical operator representation is:

```text
TransferableOperatorV2
+-- RoleGraph
|   +-- structural roles
|   `-- binding constraints
+-- RelationProgram
|   +-- equality / delta / cardinality
|   +-- selection
|   `-- frame preservation
+-- TypedTransform
|   +-- projection
|   +-- computation
|   +-- filtering
|   `-- aggregation
+-- CompositionDag
|   `-- ordered dependent transformations
+-- RendererContract
|   `-- response form without field-name or exact-surface authority
+-- VerifierContract
+-- ForwardWave
`-- BackwardWave
```

This representation must be rich enough for one law to combine independent
surfaces that currently fragment into separate exact programs. The miner finds
repeated evidence; the operator is responsible for expressing and transferring
the law.

### ForwardWave

```text
state_before + current observation
-> structural role binding
-> relation evaluation
-> transform and composition
-> renderer
-> predicted relation frame
-> actor result
```

`ForwardWave` does not mean unconstrained prediction. It instantiates a known
operator law on the current state. Runtime receives no target response,
`state_after`, or future action.

### Independent typed execution trace

The actor cannot describe its own success. Each stage emits a bounded receipt:

```text
RoleBindingReceipt
-> RelationEvaluationReceipt
-> TransformReceipt
-> CompositionReceipt
-> RendererReceipt
-> VerifierReceipt
-> VerifiedDeltaReceipt
```

Every receipt binds at least the generation, operator, input relation hashes,
output relation hash, stage result, event time, evidence source, and previous
receipt hash.

In production, the residual is derived without teacher authority:

```text
observed_relation_frame - predicted_relation_frame
-> typed residual wave
```

The observed frame must come from independent tool output, state transition, or
verifier evidence. A teacher response may be used for support and development
experiments after a trace is complete, but it is not production runtime
authority. With no independent observation, the outcome is censored `UNKNOWN`.

### BackwardWave

`BackwardWave` is a typed transformation from verified residual to a bounded
phase update, not a metaphor and not a discrete rejection table:

```text
zero verified residual
-> phase-aligned reinforcement

repeatable applicability residual
-> phase-inverted counter-wave
-> distributed anti-center / narrower applicability field

hard structural residual
-> localize RoleGraph / RelationProgram / Transform / Composition / Renderer
-> center bifurcation, repair, split, or revoke

censored outcome
-> no semantic phase update
-> optional bounded availability/uncertainty accounting only
```

### Generation firewall

An ACTIVE operator is immutable. Feedback never mutates production authority in
place:

```text
ACTIVE generation g                         immutable
-> verified feedback
-> bounded BackwardWave accumulator
-> candidate generation g+1                 shadow
-> replay + frozen future + causal ablation
-> external admission
-> ACTIVE generation g+1                    immutable
```

This firewall preserves the proof for generation `g`, prevents feedback
oscillation, and gives every behavioral change a new evidence lineage.

### Canonical matched-capacity experiment

The system core is accepted only through four frozen contours:

```text
A. old operator
   current baseline

B. rich oracle operator without BackwardWave
   tests whether TransferableOperatorV2 is expressive enough
   claim_authority = false

C. rich automatically induced operator without BackwardWave
   tests automatic operator induction

D. rich automatically induced operator with BackwardWave
   tests Wave self-correction
```

Interpretation:

```text
B - A = expressive-capacity gain
C / B = automatic-induction quality relative to the oracle ceiling
D - C = causal contribution of BackwardWave
```

Freeze the stream, teacher groups, support/future partition, verifier, top-k,
hypothesis budget, runtime budget, and package budget. Required BackwardWave
controls are shuffled residual phase, magnitude-only residual, discrete
anti-center only, and no backward feedback. Product authority belongs only to
the automatically induced contour.

Primary measurements are independent surfaces per law, frozen-future
executions, eliminated exact checks, package bytes, p99, and wrong accepts.
Potential coverage is secondary and is never counted as actual CPU savings.

### Implementation status boundary

This section is the canonical target core, not a claim that every component is
already complete. The repository already contains phase centers, negative Wave
training, anti-wave scoring, CEGIS repair, typed actor/verifier pieces, frozen
future, and external admission. `TransferableOperatorV2`, the complete typed
receipt chain, production `VerifiedDeltaReceipt`, immutable generational
`BackwardWave`, and the four-contour proof remain implementation obligations
until verified by artifacts and live runtime evidence.

## 4. Canonical Learning Path

Training may inspect a completed trace, including the action and answer that
actually occurred. That is the teacher signal. This is legitimate
self-training, not runtime leakage.

```text
completed live trace
-> post-action teacher signal
-> structural alignment and grouping
-> L1/L2 relation-wave representation
-> positive phase center
-> counterexamples
-> anti-center, applicability subcenter, or repaired law
-> transferable action operator
-> compact deterministic actor/program
-> independent frozen-future receipts
-> external admission
-> ACTIVE registry
```

Current ownership:

```text
completed trace and teacher join
  crates/nando-transition-serving/src/session_stream.rs

streaming teacher/student state
  crates/nando-response-actor/src/online.rs
  crates/nando-response-actor/src/online_state.rs
  crates/nando-response-actor/src/online_checkpoint.rs

structural grouping and semantic equivalence
  crates/nando-response-actor/src/semantic_alias.rs
  crates/nando-response-actor/src/online_subcenter.rs

counterexamples, repair, anti-centers, winning laws
  crates/nando-response-actor/src/cegis.rs

ordinary structured-result induction
  crates/nando-response-actor/src/online_collection.rs
  crates/nando-response-actor/src/collection_synthesis.rs

program and execution
  crates/nando-response-actor/src/program.rs
  crates/nando-response-actor/src/runtime.rs

independent verification and admission
  crates/nando-response-actor/src/verifier.rs
  crates/nando-response-actor/src/online_admission.rs
  crates/nando-response-actor/src/bin/nando-response-admission.rs
```

## 5. The Required Feedback Loop

Every evaluated outcome must first be classified. It is incorrect to collapse
all non-PASS outcomes into one negative class:

```text
verified positive
-> reinforce center / law

repeated applicability negative
-> accumulate evidence from independent sessions
-> derive an action-neutral distinguishing relation
-> form an anti-center
-> narrow the route without changing the actor law

hard actor / verifier / teacher contradiction
-> invalidate the unsafe winner
-> split into applicability subcenters, repair, or revoke
-> begin a new frozen generation

timeout / unavailable environment / missing evaluator / not evaluated
-> censored UNKNOWN
-> do not reinforce a center
-> do not create an anti-center
-> do not count as evidence against the law
```

This is the current decisive research/product boundary. A system that counts a
repeatable applicability negative but does not feed it back into the field is
not the intended learning loop. A clean law needs both a positive center and a
learned boundary against negative states. Conversely, poisoning that boundary
with infrastructure failures or unknown outcomes is also a learning failure.

An anti-center means repeatable *non-applicability*, not arbitrary failure. It
requires independent-session evidence and an observable pre-decision relation
that distinguishes the negative surface from positive support. A hard semantic
contradiction means that the current operator law or its partition is unsafe;
it must cause repair, split, or revocation rather than being hidden behind a
broader anti-center.

The next meaningful live progression is:

```text
live counterexample
-> live anti-center or clean subcenter
-> growing clean frozen future
-> independent receipts
-> first ACTIVE ordinary project/status/count/filter/compose law
```

If no action-neutral pre-decision distinction exists, the correct result is
`ABSTAIN`. Never invent a discriminator from the future action.

### Three complementary intelligence levels

These are distinct responsibilities, not competing implementations:

```text
JEPA-inspired latent consequence model
  encodes state z_t and predicts z_t+1 under a candidate action

self-correcting Wave operator
  binds a known law to an actor and learns when it is applicable

external causal law discovery (for example, MICRO-12 research)
  investigates unresolved contradictions and proposes genuinely new actions
```

Their loop is:

```text
latent predictor ranks candidate consequences
-> external discovery probes and proposes a new verified law
-> Wave compresses repeated experience into centers
-> actor executes the law
-> verifier classifies the consequence
-> prediction error updates latent dynamics
-> applicability evidence updates Wave boundaries
-> unexplained hard contradiction returns to causal discovery
```

The external researcher is not part of the hot runtime and has no execution
authority. The latent predictor does not authorize actions. Wave phase centers
remain a separate compact memory for recognition and applicability; they are
not the JEPA latent state.

## 6. Runtime Boundary

Runtime is intentionally narrower than training:

```text
state_before + current observation
-> Wave route
-> exact applicability boundary
-> deterministic actor
-> independent verifier
-> ACCEPT

any uncertainty or disagreement
-> ABSTAIN
-> upstream model
```

Runtime must not read the future action, final teacher response, `state_after`,
or proof-only training atoms. The actor cannot authorize itself. The miner
cannot grant execution authority. Admission is external.

Typed programs and renderers are an execution and proof language around a law
discovered by the Wave. They are useful, but they are not the intelligence
core. Never turn discovery into selection among a few pre-named programs.

## 7. Non-Negotiable Invariants

1. Completed action/response is allowed and required as a training label.
2. Future action/response is forbidden in runtime routing and guards.
3. Field names, function names, and manual family IDs are not semantic
   authority; transfer must survive renamed surfaces.
4. Repeatable applicability negatives must update anti-centers. Hard semantic
   contradictions must trigger applicability subcenters, CEGIS repair, or
   revocation. Merely recording either outcome is insufficient.
5. Censored outcomes such as timeout, unavailable environment, missing
   evaluator, or `NOT_EVALUATED` are unknown evidence. They must never train a
   positive center or anti-center.
6. `false_accepts = 0` is a hard requirement.
7. `runtime_parity_mismatches = 0` is a hard requirement.
8. Every local accept has an independent verifier receipt.
9. The miner emits evidence-bearing candidates; external admission grants
   authority.
10. Frozen future is event-time independent from support. Never fabricate or
   backfill it from support.
11. Potential, shadow, ACTIVE, and real CPU coverage are different numbers.
12. State is bounded and compact. Normal startup must not rescan unbounded
    history, and the hot path must not append unbounded payloads.
13. Serving and learning remain streaming, event-driven Rust with low idle CPU.
14. One algorithmic mechanism changes at a time; refactoring and scoring changes
    are separate commits.
15. JEPA-like latent predictions, Wave applicability evidence, and verifier
    truth are three different signals. They must have separate state and update
    rules; none may masquerade as another.
16. ACTIVE generations are immutable. Verified feedback can only construct a
    separately proven candidate generation.
17. BackwardWave updates require a typed `VerifiedDeltaReceipt` whose observed
    side is independent from the actor.

Accounting identities must have no silent loss:

```text
admission_ready_cohorts
= emitted_candidates + explicitly_blocked_candidates

collection_observations
= executable + ambiguous + irreducible

local_accepts
= independently_verified_accepts
```

## 8. Truthful Proof and Economics

Always report these levels separately:

```text
discovered optimistic upper bound
shadow executions
independently verified frozen future
admission-ready candidates
ACTIVE authority
actual local CPU accepts
independently verified input-token savings
```

Do not call potential coverage savings. Do not call shadow traffic CPU accepts.
Do not call a laboratory proof product completion.

Product M3 means all of the following, not an architecture score:

```text
verified input-token saving share >= 50%
for three consecutive independent windows
false_accepts = 0
runtime parity mismatches = 0
economics hard gate = YES
```

## 9. Behavioral Oracle

For changes to discovery, grouping, Wave feedback, or transferable actions,
compare behavior with the preserved pre-refactor tree:

```text
/home/ubu/projects/rsmod/worktrees/nando-wave-pre-refactor-2026-07-10
HEAD 6071708bbdd15f5df0be31f68379986d796e24b1
```

This is a behavioral oracle, not code to copy blindly. Preserve the current
independent verifier, admission, fallback, parity, storage, and runtime safety
shell. Restore useful learning behavior inside that shell, one mechanism at a
time, and keep an improvement only when live coverage grows without a safety
regression.

## 10. Known Destructive Failure Modes

These mistakes have already damaged coverage and must not be repeated:

- Calling the completed-trace teacher label "leakage" and removing it from
  training. Result: structural fragmentation and support split across many
  weak groups.
- Making exact template or DSL selection the center of discovery. Result:
  surface-bound programs, support such as 12/32 or 13/32, and loss of transfer.
- Counting counterexamples without feeding them into Wave repair. Result: no
  applicability boundary and no live anti-center.
- Treating timeout, unavailable environment, or unevaluated work as negative
  knowledge. Result: a poisoned anti-center that learns infrastructure noise.
- Hiding a hard actor/verifier/teacher contradiction inside a broad anti-center.
  Result: the unsafe operator survives instead of being split or revoked.
- Merging all actions globally without preserving structural role alignment.
  Result: inconsistent roles and unsynthesizable families.
- Deduplicating packages only by actor text while ignoring phase centers,
  anti-centers, predicates, margin, or verifier authority.
- Improving dashboards, gates, or infrastructure while ordinary CPU coverage
  remains unchanged, then reporting the technical PASS as product progress.
- Performing a broad academically motivated rewrite before understanding and
  reproducing the original L1/L2/L3 behavior.
- Mixing a move-only refactor with a learning/scoring change, making regressions
  impossible to attribute.
- Mutating an ACTIVE center in place from live feedback. Result: destroyed proof
  lineage, oscillation, and behavior with no frozen generation boundary.
- Calling a counter or discrete reject list `BackwardWave` without proving a
  typed residual-to-phase update and its causal phase ablation.

## 11. Required Protocol Before Core Changes

Every agent must do this before editing core behavior:

1. Read this canon and the directly relevant implementation files.
2. Draw the current signal tree in plain language and mark the exact blocker.
3. Name the touched boundary: capture, discovery, Wave, CEGIS, actor, verifier,
   admission, runtime, storage, or economics.
4. Record a short live baseline: actual CPU share, verified token savings,
   teacher pools, winners, future, candidates, ACTIVE packages, false accepts,
   parity failures, latency, memory, and disk growth.
5. Compare with the behavioral oracle when recovery or core learning is
   involved.
6. Make one scoped change. Do not add a manual operator class as a shortcut.
7. Run focused checks, then one release build, the mandatory live transition
   gate, and a real runtime check.
8. Report actual wall time for long commands and do not disappear silently.
9. Commit the change with a narrow message.
10. Update this canon only when the architecture itself changes.

## 12. Supporting Documents

The canon is short by design. Deeper evidence and implementation detail live
here:

```text
docs/NORTH_STAR.md
docs/NANDA_WAVE_THEOREM.md
docs/NANDO_WAVE_SIGNAL_PATH_L1_TO_OPERATOR.md
docs/SYMBOL_CELL8_ARCHITECTURE.md
docs/LEXICON_FOUNDATION_V1.md
docs/RISKS.md
```

If a supporting document contradicts this canon, stop and resolve the
contradiction explicitly. Do not silently choose the interpretation that makes
the planned refactor easier.
