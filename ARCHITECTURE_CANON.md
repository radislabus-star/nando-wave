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

The later latent/hidden action-state work augments this layer. It may recover a
law whose decisive state is not directly named in the surface. It must not
replace the observable Wave path or bypass proof.

Relevant implementation:

```text
crates/nando-core/src/wave/l3_semantic_grokking.rs
crates/nando-core/src/wave/phase_center_runtime.rs
crates/nando-response-actor/src/online_subcenter.rs
crates/nando-response-actor/src/cegis.rs
```

## 3. Canonical Learning Path

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

## 4. The Required Feedback Loop

The Wave must receive both reinforcement and contradiction:

```text
verified positive
-> reinforce center / law

verified counterexample
-> invalidate unsafe winner
-> derive an action-neutral distinguishing relation
-> form anti-center or split into applicability subcenters
-> begin a new frozen generation
```

This is the current decisive research/product boundary. A system that counts a
negative but does not feed it back into the field is not the intended learning
loop. A clean law needs both a positive center and a learned boundary against
negative states.

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

## 5. Runtime Boundary

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

## 6. Non-Negotiable Invariants

1. Completed action/response is allowed and required as a training label.
2. Future action/response is forbidden in runtime routing and guards.
3. Field names, function names, and manual family IDs are not semantic
   authority; transfer must survive renamed surfaces.
4. Negatives must update anti-centers, applicability subcenters, or CEGIS
   repair. Merely recording a rejection is insufficient.
5. `false_accepts = 0` is a hard requirement.
6. `runtime_parity_mismatches = 0` is a hard requirement.
7. Every local accept has an independent verifier receipt.
8. The miner emits evidence-bearing candidates; external admission grants
   authority.
9. Frozen future is event-time independent from support. Never fabricate or
   backfill it from support.
10. Potential, shadow, ACTIVE, and real CPU coverage are different numbers.
11. State is bounded and compact. Normal startup must not rescan unbounded
    history, and the hot path must not append unbounded payloads.
12. Serving and learning remain streaming, event-driven Rust with low idle CPU.
13. One algorithmic mechanism changes at a time; refactoring and scoring changes
    are separate commits.

Accounting identities must have no silent loss:

```text
admission_ready_cohorts
= emitted_candidates + explicitly_blocked_candidates

collection_observations
= executable + ambiguous + irreducible

local_accepts
= independently_verified_accepts
```

## 7. Truthful Proof and Economics

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

## 8. Behavioral Oracle

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

## 9. Known Destructive Failure Modes

These mistakes have already damaged coverage and must not be repeated:

- Calling the completed-trace teacher label "leakage" and removing it from
  training. Result: structural fragmentation and support split across many
  weak groups.
- Making exact template or DSL selection the center of discovery. Result:
  surface-bound programs, support such as 12/32 or 13/32, and loss of transfer.
- Counting counterexamples without feeding them into Wave repair. Result: no
  applicability boundary and no live anti-center.
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

## 10. Required Protocol Before Core Changes

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

## 11. Supporting Documents

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
