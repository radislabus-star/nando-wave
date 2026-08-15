# K2 Self-Chosen Safe Inquiry Critical Review V1

Status: `ADVERSARIAL REVIEW COMPLETE / P0-P1 REPAIRS APPLIED`

Date: `2026-08-15`

Target: `K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md`

## 1. Verdict

The experiment is worth implementing after repair. It can establish that
frozen causal hypotheses are useful for choosing a safe intervention, but it
cannot establish that the ranking rule, ontology, or world-model family was
invented by Nanda.

```text
finite falsifiable question                  yes
sealed generated future                      yes
strong non-learning heuristic                included
exact information and safety accounting      included
natural K2                                   forbidden
production authority                         false
unresolved P0/P1 after repair                 0
```

## 2. Findings And Repairs

| Severity | Finding | Failure mode | Applied repair |
|---|---|---|---|
| P0 | "Learned inquiry" overstated the mechanism. | A fixed information-gain algorithm could be presented as a learned strategy. | Claim narrowed to model-guided inquiry; the fixed scorer is named K0 and learning of the ranking rule remains unproved. |
| P0 | Beating an oracle is impossible by definition. | A contradictory PASS contract could be satisfied only by redefining the oracle after reveal. | Oracle is an upper-bound audit; PASS requires equality with it and strict superiority only to four non-oracle rules. |
| P0 | The true model could leak through fixture IDs or request bytes. | Selector reads the answer instead of resolving uncertainty. | Private true-model root is absent from selector/baseline bytes, checked bytewise before dispatch, and opaque-ID/path permutation controls are mandatory. |
| P0 | A selector could inspect the observed outcome before precommit. | Post-outcome selection masquerades as active inquiry. | Selection and all model predictions are journaled and independently verified before any worker dispatch. |
| P0 | Safety metadata could be advisory. | Unsafe high-information probe wins. | Eligibility is conjunctive and independently recomputed; unsafe, non-reversible, delayed, ambiguous, unknown, malformed, and over-budget probes are VETO. |
| P0 | The policy could report a small matrix while evaluating hidden candidates. | Accounting denominator becomes unverifiable. | Exactly 4 models x 8 probes are present per case; every pair has one disposition and the verifier reconstructs all 32. |
| P0 | Baselines could receive weaker inputs. | Claimed superiority comes from unequal information. | All rule baselines receive the identical public model/catalog bytes; only oracle runs after reveal and is reported separately. |
| P0 | The explicit baseline was too weak. | Learned-model route only beats arbitrary hash and cost rules. | Added frozen applicability/dependency/cleanup heuristic with exact weights and tie-breaks. |
| P0 | Driver-owned observation could echo worker output. | No independent fact establishes the filesystem transition. | Observer is a distinct executable that scans a read-only post-state mount and receives no models, effects, predictions, or worker stdout. |
| P0 | Worker request could execute a different action than selected. | Correct selection and unrelated mutation are spliced together. | Dispatch request binds selection root, selected probe root, private effect resolution, worker executable, and initial manifest; verifier checks the binding. |
| P0 | Retrying after a crash could create multiple outcomes. | One-probe budget is silently violated. | Journal dispatch is durable before spawn; a dispatch without outcome is indeterminate and same-identity redispatch is rejected. |
| P1 | A single confirm case would be fragile. | One lucky partition or root order passes. | Eight ID/path/content/model-disjoint cases are frozen. |
| P1 | Candidate order could carry the label. | Selector chooses a privileged array position. | Canonical sorting plus candidate-order shuffle control. |
| P1 | Paths or opaque IDs could encode probe roles. | Identity lookup replaces causal prediction. | Semantic invariance under opaque-ID permutation and path bijection is required. |
| P1 | Cheapest, safer probes may reasonably collect less evidence. | Comparing only information ignores cost. | One-probe budget is primary; surviving models and exact cost/risk are both reported. PASS requires unique resolution and does not claim cost dominance. |
| P1 | Generated cases are deliberately discriminative. | Benchmark success may not transfer to natural uncertainty. | Claim remains generated-only; natural, delayed, noisy, partial, and second-domain inquiry stay explicitly unproved. |
| P1 | Prepared competing models do not prove model induction. | Input construction is promoted as learning. | Model-set formation is declared out of scope; predecessor evidence is not recounted as this experiment's result. |
| P1 | Shared transition code could make selector and verifier agree on a bug. | False PASS from correlated implementation. | Verifier has an independent transition/prediction implementation and focused value/path regressions; it may share only canonical data types and hashing. |
| P1 | A failed implementation could consume the single scientific attempt. | Coding defects become evidence against the hypothesis or invite threshold changes. | Development fixtures are commitment-disjoint; invalid implementation attempts are preserved separately without changing confirm bytes or budgets. |
| P1 | Cleanup could erase unfavorable evidence. | Terminal result disappears with the workspace. | Receipts and journal roots are preserved; only disposable trees are removed after observer publication. |

## 3. Strongest Alternative Explanation

The exact information-gain scorer is sufficient once humans provide a complete
candidate model set and probe catalog. A PASS therefore does not show a hidden
representation inventing experiments. It shows something narrower:

```text
prepared but answer-blind competing causal hypotheses
-> exact predicted outcome partitions
-> safe probe chosen without true-model leakage
-> one real intervention resolves the hypothesis set
```

This remains important because it closes the loop from uncertainty to an
evidence-producing action. It is not yet a strategy learner.

## 4. Why The Baselines Are Fair But Limited

Passive, stable-order, cheapest-first, and the explicit heuristic are frozen
before reveal and use the same public bytes. The heuristic is materially
stronger than a random order because it receives applicability, dependency,
and cleanup hints.

None computes exact model-predicted partitions. Consequently, beating them
establishes the value of model-specific consequence predictions, not universal
superiority to every possible hand-written planner. The oracle checks whether
the selected intervention reached the best possible one-probe elimination.

## 5. Implementation Risks To Catch Before Confirm

```text
outcome identity accidentally includes model identity
failure and unchanged-state outcomes accidentally collapse
observer receives worker stdout or private effect bytes
stable-hash tie-break runs before information score
unsafe probe remains in the eligible denominator
heuristic hints leak into model-guided score
private true-model root appears in selector serialization
same probe is spawned twice after journal recovery
workspace cleanup precedes observer publication
verifier calls selector implementation instead of reconstructing it
```

Each item must map to a focused test or the implementation preflight remains
`BLOCKED_BEFORE_CODE`.

## 6. Repaired PASS Interpretation

If every conjunct passes, the permitted statement is:

> In eight sealed generated filesystem cases, Nanda used frozen competing
> causal-model predictions to precommit and execute one safe reversible probe,
> independently reduced each four-model set to one, strictly outperformed four
> frozen rule policies on remaining uncertainty, and matched the one-probe
> oracle with authority false.

The following statement remains forbidden:

> Nanda learned a general strategy, understands natural meaning, or can safely
> experiment in production.

## 7. Post-Result Obligation

After PASS or terminal FAIL, write a separate critique that reports:

```text
all denominators and costs
every invalid implementation attempt
baseline survivor counts per case
oracle agreement
tie count
safety and veto dispositions
process and verifier independence
remaining prepared-answer surfaces
the narrowest justified next experiment
```

No successor starts until that review is complete.

## 8. Exact Structural Evidence Index

```text
exact eligibility gate vetoes unsafe ambiguous delayed and unknown probes
dispatch request binds verified selected probe and private resolved effect
sandbox worker mutates only one disposable generated work tree
observer executable scans read-only post-state after worker exit
observer request excludes models predictions effects and worker stdout
cleanup removes workspace only after observer publication
independent verifier reconstructs predictions without selector calls
observed outcome eliminates every inconsistent frozen model
positive result requires eight singleton updated model sets
opaque identity permutation preserves semantic selection disposition
path bijection preserves semantic selection disposition
candidate order shuffle preserves selected semantic probe role
```
