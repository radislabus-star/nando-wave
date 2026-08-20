# K2 Self-Formed Uncertainty V5 R7J Contract V2

Status: `REVISED AFTER ADVERSARIAL CRITIQUE / PENDING STRUCTURAL GATES / NO CODE AUTHORITY`

Date: `2026-08-20`

Supersedes: `K2_SELF_FORMED_UNCERTAINTY_V5_R7J_CONTRACT_V1.md`

Critique: `K2_SELF_FORMED_UNCERTAINTY_V5_R7J_CRITIQUE_V1.md`

## 1. Exact Scope

R7J implements three independent evaluation components:

```text
manifested frozen case evidence + post-observation private mapping
-> bounded oracle and frozen-baseline evaluator

R7K-owned process-outcome rows
-> scope-separated control evaluator

rooted evaluation receipts + route/accounting receipts
-> conjunctive terminal evaluator
```

R7J does not execute K1-K12, perform cleanup, create a Confirm nonce, claim an
authorization slot, run a sealed attempt, publish a scientific result, touch
production, mutate K1 or claim Natural K2.

```text
sealed attempts              0 / 1
Confirm nonce                ABSENT
R7K                          LOCKED until R7J component PASS
production authority         false
Natural K2 authority         false
```

## 2. Executable Owners

The following application-owned executables are pairwise distinct and verify
their current executable SHA-256 against each request:

```text
nando-k2-self-formed-oracle-baseline
  complete frontier reconstruction, actual-outcome parity, oracle and baselines

nando-k2-self-formed-control-evaluator
  closed-scope process-outcome validation only

nando-k2-self-formed-terminal-evaluator
  receipt validation, exact aggregation and terminal precedence only
```

The oracle may reuse canonical JSON, SHA-256, manifest, typed-effect and public
model value types. It cannot import or call closure planner, selector, final
verifier, terminal, control or their ranking, elimination and partition helpers.
The control evaluator cannot execute a control. The terminal evaluator cannot
read raw frontier, observation, resolver, final-truth or nonce bytes.

The integration harness transports requests and compares responses. It owns no
evaluation predicate and cannot manufacture a PASS conjunct.

## 3. Oracle Transport And Manifest

`K2UncertaintyOracleBaselineCaseDescriptorV1` is a compact canonical stdin
schema containing only:

```text
experiment_id_sha256
public_batch_root_sha256
batch_precommit_root_sha256
all_cases_precommitted_root_sha256
case_id_sha256
case_sequence
public_case_root_sha256
prepared_case_root_sha256
closure_plan_root_sha256
baseline_summary_root_sha256
observation_vector_root_sha256
final_verifier_receipt_root_sha256
private_truth_artifact_root_sha256
case_evidence_manifest_root_sha256
oracle_evaluator_executable_sha256
```

The descriptor must remain below 1 MiB. Large evidence is mounted read-only in
one case tree whose closed `K2UncertaintyOracleCaseEvidenceManifestV1` lists
every relative path, byte length, mode, content SHA-256, semantic root and kind.
The tree contains exactly:

```text
public case vocabulary and support
complete learned four-model set and semantic partition
all 1,792 raw frontier dispositions in frozen pages
frontier class census
prepared closure plan and its public preverification
four frozen baseline decisions
ordered observation vector
independent final-verifier receipt
one matching private truth artifact
```

Every file is verified against the manifest before semantic decoding. Missing,
extra, duplicate, symlinked, mutable, foreign or hash-mismatched material is an
infrastructure failure. The evaluator has no resolver executable, learner
channel, mutable workspace, nonce, authorization ledger, cleanup tree or
network access.

The private artifact is mounted only after the matching observation vector and
final-verifier receipt are durable. Oracle logic may use only case/public
bindings and the action-to-effect mapping. Topology-family and matched-pair
labels are forbidden inputs to ranking, survival and true-class derivation.

## 4. Independent Frontier Reconstruction

The evaluator validates all 1,792 raw dispositions and independently rebuilds
the exact equivalence partition from canonical fields:

```text
four predicted observable outcome equalities
eligibility disposition derived from source fields
safety disposition derived from robust accounting
risk and cost
applicability, dependency and cleanup hints
```

It rejects caller-supplied equivalence keys that do not equal the reconstruction.
For each equivalence class, the canonical representative is the member with the
lexicographically smallest probe root. Exact member coverage is mandatory:

```text
raw members in classes        1,792 / 1,792
members in more than one class            0
unclassified members                       0
representatives                  class count
representative omissions/additions         0
```

This reconstructed ordered representative set `R` is the sole oracle frontier.

## 5. Outcome And True-Class Semantics

Each probe carries its own frozen `initial_manifest`. Probe workspaces are
independent; no filesystem state crosses probe ordinals.

For one probe and one mapping:

```text
probe action ID
-> mapped typed effect
-> apply effect to that probe's initial manifest
-> canonical post manifest
-> exact observable outcome root
```

For each syntactic model, the evaluator independently finds the model effect for
the same action and applies it to the same probe initial manifest. A syntactic
model survives exactly when its outcome root equals the private-mapping outcome.
A semantic class survives when at least one member survives.

The two-probe semantics is sequential evidence elimination, not sequential
filesystem mutation:

```text
all four syntactic models
-> filter by outcome of probe p in p's frozen world
-> filter survivors by outcome of probe q in q's frozen world
-> quotient surviving syntax roots by the frozen semantic partition
```

Eliminated models cannot re-enter. No score, hint, model ID, plan identity,
final-verifier Boolean or topology label participates in survival.

The evaluator derives the true model by exact equality between the private
action-to-effect mapping and every action in each syntactic model. Exactly one
syntax root must match. Exactly one semantic class must contain that syntax
root. Every model-guided, oracle and baseline result records whether this true
class survived.

For every selected model-guided probe, the independently computed private
outcome must equal the corresponding ordered observation-vector outcome. The
case is invalid before oracle aggregation if any ordinal, plan, manifest,
observation or final-verifier binding differs.

## 6. Complete Bounded Oracle

For `n = |R|`, the evaluator enumerates in canonical ordered probe-root order:

```text
all [p]                       n
all [p, q], p != q            n * (n - 1)
total                         n^2
```

All arithmetic is checked. A maximal case has at most 3,211,264 plans. Every
candidate contributes a domain-separated row to a streaming enumeration hash
chain, including plan roots, source ordinals, eligibility, rejection reason,
residual syntax roots, residual semantic-class roots, true-class retention,
risk and cost. The final receipt records the chain root and exact totals:

```text
enumerated
eligible
rejected_probe_ineligible
rejected_risk_budget
rejected_cost_budget
```

Eligibility is independently recomputed. Every step must be reversible,
exact-immediate, publicly safe and within frozen per-probe limits. Cumulative
risk and cost must fit the frozen plan limits. An empty eligible set is an
infrastructure failure.

Eligible plans are ranked by:

```text
minimum residual semantic classes
-> minimum plan length
-> minimum cumulative risk
-> minimum cumulative cost
-> lexicographic ordered probe roots
```

The winner records its full rank tuple and retained true class. Case oracle
equality is exactly:

```text
model_guided_residual_semantic_classes == 1
&& model_guided_true_class_retained
&& oracle_residual_semantic_classes == 1
&& oracle_true_class_retained
```

Equality is never narrated as superiority over the oracle.

## 7. Frozen Baselines

The evaluator reopens, but never reselects, the four decisions already rooted
in the prepared public case and `ALL_CASES_PRECOMMITTED`:

```text
passive observation
stable-root order
cheapest first
explicit applicability/dependency/cleanup heuristic
```

The original baseline owner executable root, decision roots and precommit root
must match exactly. Each policy remains the frozen one-probe comparator. For
each case and policy the evaluator records selected probe or `None`, actual
residual semantic classes, true-class retention, risk and cost. Passive records
the pre-probe class count and zero execution.

The batch receipt contains exactly sixteen ordered case receipts. For each
policy independently it recomputes:

```text
model_sum
policy_sum
strict_model_improvement_cases
aggregate_superiority = model_sum < policy_sum
threshold_pass = strict_model_improvement_cases >= 12
```

Risk and cost are diagnostics and break no residual tie. Missing, duplicate,
foreign or post-reveal decisions invalidate the batch. This comparator does not
prove superiority over adaptive baselines receiving the same two-probe budget.

## 8. Oracle Receipts

`K2UncertaintyOracleBaselineCaseReceiptV1` binds:

```text
descriptor and manifested-tree roots
reconstructed frontier and representative roots
exact n and n^2 denominator
enumeration chain and rejection counters
private mapping and derived true syntax/class roots
model-guided plan, observation-parity and residual result
oracle winner and residual result
four frozen baseline results
final-verifier receipt root
evaluator executable root
authority false
```

`K2UncertaintyOracleBaselineBatchReceiptV1` contains exactly sixteen unique
ordered case receipt bodies, recomputes all case and baseline aggregates, and
binds their roots. Caller-supplied aggregate booleans have no authority.

## 9. Control Evaluation

Scopes and exact denominators are disjoint:

```text
SuccessorStaticLegacy     32
SuccessorStaticV3          4
SuccessorStaticV4         16
DevelopmentRehearsalV5    12
SealedAttemptV5            12
```

Each `K2UncertaintyControlProcessOutcomeV1` contains:

```text
scope and control ID
experiment/freeze/attempt roots required by scope
runner and test executable roots
control request root
normal_exit
exit_code
bounded stdout bytes and SHA-256
stderr SHA-256
timed_out
panicked
decoded named disposition
source/log artifact roots
outcome root
authority false
```

The evaluator accepts a control only when the scope contains the exact expected
ID once, process execution ended normally, timeout and panic are false, exit
code and bounded stdout exactly encode the preregistered disposition, and all
roots reseal. Parse errors, expected text supplied without process evidence,
wrong scope, missing/duplicate rows and denominator substitutions fail.

R7J implements schemas, evaluation and adversarial substitutions. R7K owns and
runs the K1-K12 fault scenarios, including K12 cleanup. Therefore R7J cannot
publish a K1-K12 PASS receipt or complete Development rehearsal terminal.

## 10. Terminal Evidence

The terminal executable accepts two distinct closed schemas, never an untagged
shared envelope:

```text
K2UncertaintyDevelopmentRehearsalTerminalRequestV1
K2UncertaintySealedTerminalRequestV1
```

Each request carries canonical receipt bodies or a read-only receipt tree plus a
complete content manifest. Roots without bytes are insufficient. The evaluator
validates and reseals every nested receipt, checks pairwise owner identities and
recomputes all representable counters. It cannot open raw observations, private
mapping, frontier pages, nonce bytes or cleanup state.

Development rehearsal requires zero authorization slots, nonces and sealed
attempts, `scientific_verdict_requested = false`, one complete oracle/baseline
batch receipt, all three successor-static control receipts, one R7K-owned
DevelopmentRehearsalV5 receipt and exact route receipts. Only then can it emit
`DEVELOPMENT_REHEARSAL_PASS`.

The sealed request requires one exact authorization-slot projection, one nonce
commitment projection, one sealed attempt, attempt-bound evaluation receipts,
the attempt-bound SealedAttemptV5 control receipt, route receipts and resource
measurements. R7J exercises this schema with in-memory non-authoritative
fixtures only and creates no durable terminal artifact.

## 11. Failure Classes And Precedence

Malformed bytes or an invalid closed schema are protocol errors and emit no
terminal receipt. For a valid schema, precedence is:

```text
INDETERMINATE
  irreversible dispatch with a durable missing or ambiguous result

INFRASTRUCTURE_FAIL
  missing, duplicate, foreign, malformed or hash-mismatched evidence
  owner, scope, denominator, control or route failure
  resource violation, forbidden execution, authority promotion or network effect

SCIENTIFIC_FAIL
  complete model-guided route does not retain one true semantic class
  bounded oracle does not retain one true semantic class
  any frozen baseline aggregate or 12/16 threshold predicate is false

K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS
  every exact infrastructure and scientific conjunct is true
```

`false_accepts > 0` is an infrastructure safety failure. No weighted score,
majority vote, missing-row default, cleanup result or narrative override exists.
The scientific verdict is immutable, authority-denied and separate from later
cleanup and publication.

## 12. Independence And Size Gates

Before implementation PASS:

```text
oracle compact descriptor serialized size          < 1 MiB
control request serialized size                     < 1 MiB
terminal request or receipt-manifest descriptor     < 1 MiB
all bounded stdout                                  < 1 MiB
mounted evidence files                    individually manifested
```

Source-route checks forbid oracle dependencies on closure planner, selector,
final verifier, control and terminal modules. Label permutation, model-root
permutation, representative omission, plan omission, outcome substitution,
cross-scope receipt, root-only terminal and count-overflow controls must fail.

Small-frontier tests compare the streaming oracle to a separate explicit
Cartesian-product reference. Maximum-frontier tests prove checked `n^2`
accounting and bounded memory without requiring all candidate rows in RAM.

## 13. R7J Acceptance

```text
critique incorporated                                  PASS required
owner-bounded structural routes                        PASS required
implementation preflight                 READY_TO_IMPLEMENT required
three distinct self-hash-bound executables              3 / 3
frontier reconstruction                            1,792 / case
representative omission/addition/duplication                 0
oracle plan denominator                                  n^2 exact
model-guided actual-observation parity                 16 / 16
one-probe and two-probe oracle equality                 8 / 8
true class retained                                    16 / 16
four baseline aggregate predicates                       4 / 4
four baseline 12/16 thresholds                           4 / 4
control substitution negatives                         all PASS
terminal missing/cross-mode negatives                   all PASS
legacy V4 and R7G/R7H/R7I regressions                   all PASS
library/check/Clippy/fmt/diff                            all PASS
false accepts                                                   0
network/production/K1/dashboard effects                         0
sealed attempts                                               0/1
```

R7J component PASS unlocks only R7K. It does not unlock R8B, R9B, R10B or R11B
and does not prove Natural K2.
