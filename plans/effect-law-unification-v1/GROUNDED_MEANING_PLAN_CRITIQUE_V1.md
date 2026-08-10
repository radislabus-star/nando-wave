# Grounded Meaning Plan Critical Review V1

Status: `ADVERSARIAL REVIEW / REPAIRS APPLIED TO CANONICAL PLAN`

Date: `2026-08-11`

Review target: the Grounded Meaning plan as it existed at commit
`663959064a37caf7eb917fc99dfedb6386355fa6`.

This review is intentionally separate from the final plan so weaknesses do not
disappear after editing. A repair marked `APPLIED` is present in
`GROUNDED_MEANING_ARCHITECTURE_V1.md`. Structural coherence is not scientific
or runtime authority.

## 1. Review Verdict

The scientific direction is worth testing, but the previous plan was not ready
for implementation beyond S1B. It correctly separated transition dynamics from
grounded decisions, yet it skipped the production owner that must create goals,
alternatives, and horizons before action. It also lacked a finite experiment
budget and could have become another open-ended infrastructure route.

Final position after repairs:

```text
research question             VALID
current decision evidence     EMPTY
hidden model work             FORBIDDEN NOW
next slice                    S1C PRE-ACTION OWNER ONLY
product K1 route              CONTINUES IN PARALLEL
plan authority                false
```

Structural review result:

```text
decision-evidence route       PASS / repair queue empty
proposal-authority route      PASS / repair queue empty
authority_ready               false by design
aggregate                     STRUCTURALLY_ACCEPTED_WITH_SPLIT
```

## 2. Critical Findings And Repairs

| Severity | Finding | Why it matters | Repair | Status |
|---|---|---|---|---|
| P0 | The plan jumped from an empty S1B census directly to S2. | No component owned goal/alternative creation before action. Waiting could never repair the denominator. | Added explicit S1C owner route and S1C-0 through S1C-4 slices. | APPLIED |
| P0 | Goal binding could become post-hoc semantic labeling. | Reading selected action or success would manufacture meaning. | Added strict pre-action allowlist, post-action denylist, temporal receipt ordering, and censor-on-late rules. | APPLIED |
| P0 | A free-text or LLM goal binder could inject the desired semantics by hand. | The experiment would measure the binder's labels rather than discovered meaning. | Natural goals must be exact typed protocol facts or mechanically reproducible bounded bindings; all other rows are censored. | APPLIED |
| P0 | A latent model could drift toward opaque runtime authority. | Good prediction is not execution truth. | Preserved explicit version space, verifier, certificate, external admission, and latent-authority veto. | APPLIED |
| P0 | Research capture could damage serving availability. | Scientific evidence is not allowed to break ordinary requests. | Evidence fails closed; serving follows the unchanged K1/upstream route. | APPLIED |
| P1 | `K1 1/3` cannot support a serious multi-action composition claim. | One law plus ABSTAIN is not a language of action compositions. | Added two-law and meaningful-alternative gates; K1 growth continues in parallel. | APPLIED |
| P1 | Admission and applicability were treated as one fact. | A package may be admitted globally but inapplicable to the current state. | External admission owns permission; a frozen deterministic evaluator owns the pre-action applicable set. | APPLIED |
| P1 | Two lineages alone are too weak for a broad meaning claim. | A tiny denominator can produce unstable or shortcut-driven results. | S2 must freeze exact denominators and lineage-disjoint surfaces before training; empty or underpowered surfaces stop. | APPLIED |
| P1 | `B4_TYPED_SEARCH` was named but not compute-matched. | A hidden model could appear superior only because the explicit planner received less budget or information. | Added matched K1 contracts, gas, information, metrics, and compute denominators. | APPLIED |
| P1 | Hidden training had no finite search budget. | Hyperparameter search could consume months and leak the holdout. | Added one bounded development route, a frozen architecture/budget, and one confirmatory run. | APPLIED |
| P1 | Full-archive scans could waste CPU while no evidence changed. | The first census already takes nontrivial time and currently has no decision rows. | Added append-cursor scans only after evidence delta and prohibited periodic full scans. | APPLIED |
| P1 | Natural traffic alone may not contain interventions. | Passive logs can show correlation but not distinguish goal/action causality. | Law Lab supplies bounded intervention evidence, kept disjoint from natural future and authority. | APPLIED |
| P1 | Product progress could be held hostage by K2 research. | K1 is already useful and funds the experiment. | Added an independent K1 product lane and global coverage target. | APPLIED |
| P2 | The old header said implementation had not started. | It contradicted deployed S1A/S1B evidence. | Replaced it with exact slice status and Evidence Freeze 0. | APPLIED |
| P2 | Bare L1/L2/L3 names remained ambiguous. | Internal Wave layers and recursive knowledge levels were being conflated. | Canonical plan uses W1-W3 and K0-K4 only. | APPLIED |

## 3. Hardest Remaining Risks

These risks cannot be repaired by wording:

1. Ordinary traffic may not expose an exact pre-action goal that can be bound
   without reading teacher text, selected action, or outcome.
2. The admitted K1 registry may rarely expose two meaningful actions under the
   same observation.
3. K1 currently has only one independently certified operational law, so a
   broad K2 composition claim remains scientifically premature.
4. Natural independent future has no honest delivery date.
5. `B4_TYPED_SEARCH` may solve the bounded problem exactly, leaving no need for
   a hidden representation.

The plan treats every one of these as a legitimate terminal result rather than
an invitation to weaken the evidence boundary.

## 4. Improvements Over The Previous Plan

```text
before
S1B EMPTY
-> vaguely wait for decision episodes
-> baselines
-> hidden model

after
S1B EMPTY
-> S1C pre-action owner with exact temporal contract
-> finite natural census
-> PASS / EMPTY / INSUFFICIENT / VETO
-> only then frozen baselines
-> only then one bounded hidden experiment
```

The improved plan has a finite terminal outcome at every stage. It separates
schema capability from production evidence, research proposal from authority,
natural evidence from laboratory intervention, and package-conditional success
from global CPU economics.

## 5. Review Acceptance Criteria

The final plan is acceptable only if the structural gate confirms all of these
relations without role swaps:

```text
pre-action evidence -> goal contract
external admission snapshot -> available action truth
actor + independent verifier -> consequence truth
cold census -> read-only evidence projection
meaning model -> proposal only
explicit meta-program -> certification candidate
external admission -> hot authority
ordinary verified receipt -> product economics
```

Any `WATCH`, `VETO`, or `ERROR` remains unresolved. A coherence-only PASS never
grants implementation or scientific authority.
