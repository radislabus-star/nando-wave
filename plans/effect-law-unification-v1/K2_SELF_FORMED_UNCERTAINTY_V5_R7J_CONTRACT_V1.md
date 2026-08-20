# K2 Self-Formed Uncertainty V5 R7J Contract V1

Status: `DRAFT FOR ADVERSARIAL CRITIQUE / NO CODE AUTHORITY`

Date: `2026-08-20`

## 1. Claim Boundary

R7J closes only the independent evaluation component boundary:

```text
frozen public case and plan
+ durable observation vector
+ independent final-verifier receipt
+ one matching private final-truth case
-> independent bounded oracle and frozen-baseline evaluation
-> scope-separated control-receipt evaluation
-> independent conjunctive terminal evaluation
```

R7J does not run a sealed attempt, create a Confirm nonce, claim an
authorization slot, execute K1-K12 against a real attempt, clean an attempt
tree, publish an overall result, touch production, or prove Natural K2.

```text
sealed attempts              0 / 1
Confirm nonce                ABSENT
Natural K2 authority         false
production authority         false
R7K                          LOCKED until R7J component PASS
```

## 2. Owner Separation

Three application-owned executables are pairwise distinct and self-hash-bound:

```text
nando-k2-self-formed-oracle-baseline
  reads one case final truth plus frozen public evidence
  computes oracle and baseline consequences
  cannot import closure ranking, final-verifier elimination or terminal code

nando-k2-self-formed-control-evaluator
  verifies complete, scope-labelled control outcome records
  cannot execute learner, planner, worker, oracle or terminal logic
  does not generate the R7K K1-K12 fault scenarios

nando-k2-self-formed-terminal-evaluator
  consumes roots, receipts, exact counters and resource measurements only
  cannot read nonce, resolver mapping, final-truth or raw observation bytes
  cannot recompute oracle, controls, cleanup or result publication
```

The integration harness may transport requests and compare receipts, but it is
not an evaluation owner and cannot manufacture a PASS conjunct.

## 3. Oracle Input Contract

`K2UncertaintyOracleBaselineCaseRequestV1` is a closed canonical schema. It
contains:

```text
experiment_id_sha256
public_batch_root_sha256
batch_precommit_root_sha256
case_id_sha256
public_case_root_sha256
public_preverification_root_sha256
frozen model-guided plan root and ordered probe roots
complete representative probe dispositions
complete four-model set and semantic partition
four baseline decisions frozen before private reveal
observation_vector_root_sha256
final_verifier_receipt_root_sha256
expected_final_truth_root_sha256
oracle_evaluator_executable_sha256
```

The matching `K2UncertaintyConfirmFinalTruthCaseV1` is not serialized into the
request. It is mounted read-only at `/private/final-truth.json` only after the
observation vector and final-verifier receipt for that case are durable.

The evaluator receives no resolver table, nonce, authorization ledger, learner
output channel, mutable workspace, closure-planner executable, terminal
receipt, or cleanup state.

## 4. Independent Outcome Semantics

The evaluator validates the public model set and private mapping independently.
For one representative probe:

```text
probe action ID
-> private mapping action ID
-> actual typed effect
-> canonical effect application to the frozen initial manifest
-> actual observable outcome root
```

For each syntactic model, the evaluator applies that model's effect for the same
action to the same initial manifest. A model survives a probe exactly when its
observable outcome root equals the actual outcome root. A semantic class
survives exactly when at least one of its syntactic members survives. No score,
hint, risk, cost, model ID, selected-plan identity or final-verifier Boolean may
alter survival.

For a two-probe plan, the evaluator applies both probes in frozen order to the
same evolving state for actual truth and for each surviving model. The second
outcome is not evaluated against the original state. A model eliminated by the
first probe cannot re-enter.

The oracle implementation may reuse canonical typed-effect and manifest byte
types. It must implement its own bounded plan enumeration, sequential state
transition and residual partition code. It may not call the closure planner,
selector, final verifier, or their elimination/ranking helpers.

## 5. Complete Bounded Oracle

Let `R` be the complete canonical representative probe set reopened from the
frozen public frontier. The evaluator enumerates exactly:

```text
all [p] for p in R
all [p, q] for p in R, q in R, p != q
```

Each candidate must satisfy per-step safety eligibility and cumulative frozen
risk/cost budgets. The evaluator records the exact enumerated, eligible and
rejected denominators. An empty eligible set is an infrastructure failure, not
an oracle result.

For each eligible plan it computes sequential residual semantic classes. The
winner is ordered by:

```text
minimum residual semantic classes
-> minimum plan length
-> minimum cumulative risk
-> minimum cumulative cost
-> lexicographic ordered probe roots
```

The receipt records the winning plan, residual count, retained true class,
enumeration denominator and rank components. Oracle equality for one case is:

```text
model_guided_residual_classes == 1
&& oracle_residual_classes == 1
```

It is never reported as superiority over the oracle.

## 6. Frozen Baseline Evaluation

The evaluator does not select baseline probes after reveal. It reopens the four
decisions committed by the public coordinator before private execution:

```text
passive observation
stable-root order
cheapest first
explicit applicability/dependency/cleanup heuristic
```

For each case and policy it records selected probe or `None`, actual residual
semantic classes, true-class retention, executed risk and executed cost. The
passive baseline records the pre-probe semantic-class count and zero execution.

`K2UncertaintyOracleBaselineBatchReceiptV1` contains exactly sixteen case
receipts. For each baseline policy independently it records:

```text
sum(model-guided residual classes)
sum(policy residual classes)
strict per-case model-guided improvements
aggregate_superiority = model_sum < policy_sum
per_case_threshold_pass = strict improvements >= 12 / 16
```

Risk and cost are diagnostics only and cannot break a residual-class tie.
Missing case/policy rows, duplicate rows, a post-reveal baseline decision, or a
different sixteen-case denominator invalidates the receipt.

## 7. Control Receipt Evaluator

Control scopes are disjoint:

```text
SuccessorStaticLegacy    exactly 32 named outcomes
SuccessorStaticV3         exactly 4 named outcomes
SuccessorStaticV4        exactly 16 named outcomes
DevelopmentRehearsalV5   exactly K1-K12, sealed_attempts = 0
SealedAttemptV5           exactly K1-K12, sealed_attempts = 1
```

`K2UncertaintyControlEvaluationRequestV1` carries one scope, expected exact
control IDs, expected dispositions, observed dispositions, source/test roots,
owner executable root, experiment/freeze roots appropriate to that scope and
an exact denominator. The evaluator accepts a row only when execution exited
normally and returned the exact named disposition. Parse errors, panic,
timeout, missing rows, duplicate rows and wrong dispositions are failures.

R7J implements the closed schemas, evaluator and adversarial substitution tests.
R7K implements the isolated fault-scenario runners for K1-K12, including K12
cleanup behavior. Therefore R7J may prove that the evaluator rejects malformed
or cross-scope receipts, but it may not claim `K1-K12 PASS` or a complete
DevelopmentRehearsal terminal.

## 8. Terminal Requests

The same terminal executable accepts two distinct canonical schemas without an
untagged shared envelope.

### Development Rehearsal

`K2UncertaintyDevelopmentRehearsalTerminalRequestV1` requires:

```text
sealed_attempts = 0
authorization_slots = 0
nonce_count = 0
scientific_verdict_requested = false
oracle/baseline component receipt
scope-separated successor static receipts
DevelopmentRehearsalV5 control receipt
exact route counters and zero forbidden effects
```

It can emit only `DEVELOPMENT_REHEARSAL_PASS` or a named rehearsal failure. R7J
tests the evaluator with frozen component fixtures; full route PASS remains
locked until R7K supplies executable K1-K12 receipts.

### Sealed Scientific Attempt

`K2UncertaintySealedTerminalRequestV1` requires one exact authorization slot,
one committed nonce, one sealed attempt, the attempt-bound oracle/baseline
receipt, successor static receipts, the `SealedAttemptV5` receipt, exact route
counters and resource measurements. R7J tests this schema only with
non-authoritative fixtures. No real request is created.

## 9. Verdict Precedence

For a canonically valid terminal request:

```text
any durable ambiguity after irreversible dispatch
  -> INDETERMINATE

otherwise any owner/hash/binding/denominator/resource/route failure
  -> INFRASTRUCTURE_FAIL

otherwise any complete scientific conjunct false
  -> SCIENTIFIC_FAIL

otherwise every exact conjunct true
  -> K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS
```

Malformed bytes or an invalid closed schema are rejected as protocol errors and
cannot yield any verdict receipt. No missing-row default, weighted score,
majority vote, narrative override or cleanup state participates in the verdict.

The sealed scientific verdict is immutable and denied all production, K1,
LawCertificate, package, phase-memory and Natural K2 authority.

## 10. Exact Terminal Conjuncts

The terminal evaluator checks, rather than trusts, exact fields for:

```text
attempt and authorization counts appropriate to request mode
16 complete cases and four-model sets
28,672 raw probe dispositions
114,688 raw predictions
16 frozen closure plans of length one or two
derived exact selected-execution denominator
16 independent preverification receipts
one safety, worker/observer parity and observation per selected execution
16 final-verifier PASS receipts
16 singleton model-guided residuals with retained private true class
16 bounded-oracle singleton equalities
4 baseline aggregate superiority tests
4 baseline per-case thresholds
32 + 4 + 16 successor static controls
12 scope-correct V5 controls
false accepts = 0
forbidden executions = 0
authority promotions = 0
production/network effects = 0
resource violations = 0
```

Every derived denominator is recomputed from immutable plan and receipt roots.
Caller-supplied totals are cross-checks, never authority.

## 11. Persistence And Limits

R7J receipts use canonical JSON, `deny_unknown_fields`, denied authority and
content-addressed roots. Process stdin/stdout remains bounded by the frozen
1 MiB protocol limit. Receipts contain roots and exact counters, not raw private
mapping or nonce bytes.

No R7J component mutates the attempt journal or scientific artifacts. Receipt
publication in later orchestration remains the confirm owner's crash-atomic
responsibility. R7J tests byte-identical decode/reseal parity and rejects
foreign roots, reordered rows and count overflow.

## 12. R7J Acceptance

```text
paper critique incorporated
owner routes structurally PASS
implementation preflight READY_TO_IMPLEMENT
three distinct application executables self-hash correctly
oracle enumeration independently covers all valid one/two-probe plans
8 one-probe and 8 two-probe model-guided cases reach oracle equality
all four baseline aggregate and per-case predicates evaluate exactly
control evaluator rejects every scope/denominator/disposition substitution
terminal evaluator rejects missing conjuncts and cross-mode receipts
rehearsal and sealed schemas cannot be confused
legacy V4, R7G, R7H and R7I regressions PASS
library, check, Clippy, fmt and diff checks PASS
false accepts 0
network, production, K1 and dashboard effects 0
sealed attempts 0 / 1
```

Only an R7J component PASS unlocks R7K. It does not unlock R8B, R9B, R10B or
R11B directly.
