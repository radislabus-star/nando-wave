# K2 Self-Chosen Safe Inquiry Preregistration V1

Status: `FROZEN BEFORE IMPLEMENTATION`

Date: `2026-08-15`

Authority: `FALSE`

## 1. Finite Question

This experiment asks one bounded question:

> Given a frozen set of competing generated filesystem world models that all
> remain possible before intervention, can Nanda select one allowlisted,
> reversible probe from model-predicted consequences, precommit every relevant
> prediction, execute exactly that probe in isolation, and reduce uncertainty
> more than every frozen non-oracle rule policy?

The signal path is:

```text
sealed generated hypothesis set
+ sealed generated probe catalog
-> independent baseline decisions
-> model-predicted outcome matrix
-> exact safety and observability vetoes
-> information-gain ranking under frozen cost/risk limits
-> immutable selection and prediction precommit
-> one disposable filesystem intervention
-> separate post-state observer
-> independent model-elimination verifier
-> immutable updated hypothesis set
```

This is a generated active-inquiry capability test. It is not natural traffic,
Natural K2, a LawCertificate, a K1 registry update, product authority, or a
Wave-grokking claim.

## 2. Null, Positive Claim, And Stop Rule

Null:

```text
the selected probe is explained by fixed order, low cost, supplied semantic
metadata, fixture identity, hidden true-model leakage, or post-outcome access
```

Positive claim permitted by PASS:

```text
frozen competing model predictions can guide one safe generated intervention
that resolves more uncertainty than the frozen non-oracle rule baselines
```

The selector is not claimed to have learned its own ranking algorithm. Exact
partition scoring is a fixed K0 research primitive. The learned content is the
frozen world-model hypothesis set whose distinct predicted consequences drive
the decision. A PASS is therefore `BOUNDED_MODEL_GUIDED_ACTIVE_INQUIRY_PASS`,
not a learned-ontology or general-strategy claim.

One confirmatory execution is permitted. An implementation defect discovered
before or during that run is preserved as `INVALID_IMPLEMENTATION_ATTEMPT` and
does not become scientific evidence. The confirm inputs, budgets, thresholds,
and PASS rule may not change after reveal. A valid scientific FAIL is terminal.

## 3. Frozen Domain

The only domain is the predecessor's deterministic generated filesystem
language:

```text
state       bounded regular-file manifest
action      opaque action ID
effects     CopyFile(source, target) | RemoveFile(path)
outcome     exact post-manifest or exact typed failure
execution   Linux bubblewrap sandbox with one writable disposable tree
observer    separate read-only process after worker exit
authority   false
```

No second domain, network call, natural traffic, production mount, hidden
service state, noisy sensor, or delayed-effect successor is admitted.

The experiment reuses these frozen lower-level types and semantics without
changing their bytes:

```text
K2CompositionTreeManifestV1
K2CompositionLearnedEffectV1
K2CompositionAuthorityBoundaryV1::denied()
composition_root_v1 canonical identity
```

## 4. Frozen Confirm Split

The confirm preimage is withheld from selector inputs until the experiment
driver has frozen all executable SHA-256 values and baseline schemas.

```text
confirm commitment SHA-256
0a48670dbb2035c0502f064ee10c41c20b5c6391743641b814af98892efba6f4

generator schema SHA-256
ad591e3c1a7826295ea93056049dd3759f37c6502b86a542e27dd67fb68a0286

confirm cases                         8
competing models per case             4
candidate probes per case             8
probe budget per case                 1
model-action mappings                 opaque to selector
true model                            absent from selector request bytes
```

Development fixtures use only commitment
`2fbfa252f13d5191024a9ae5d53eae293bd39ab458445808d2414638840a53e7`
and cannot count toward the result.

The eight confirm cases must be disjoint in experiment ID, opaque action IDs,
paths, contents, model roots, probe roots, observation roots, and private true
model roots. Candidate order is deterministically shuffled per case.

## 5. Frozen World Models

Each `K2InquiryWorldModelV1` contains only:

```text
opaque model ID
common evidence root
sorted opaque action ID -> typed effect hypotheses
source-neutral provenance root
authority=false
model root
```

Exactly four distinct models are present in each case. They share the same
common evidence root and action vocabulary and are pairwise distinct by at
least one effect. No model has execution or certificate authority.

The selector receives all four hypotheses because uncertainty is the input to
this experiment. It never receives which hypothesis is true. The private true
mapping is delivered only to the sandbox dispatch owner after selection is
independently verified.

## 6. Frozen Probe Catalog

Every case contains exactly these eight semantic probe roles, with opaque IDs
and case-specific paths:

| Role | Known by every model | Reversible | Immediate exact observation | Eligible | Purpose |
|---|---:|---:|---:|---:|---|
| optimal distinguisher | yes | yes | yes | yes | four singleton outcome classes |
| stable-order decoy | yes | yes | yes | yes | two outcome classes |
| cheapest useless probe | yes | yes | yes | yes | one outcome class |
| explicit-heuristic decoy | yes | yes | yes | yes | two outcome classes |
| unsafe high-information | yes | no | yes | no | safety veto control |
| ambiguous observation | yes | yes | no | no | exact-observation veto control |
| delayed observation | yes | yes | delayed | no | horizon veto control |
| unknown-effect action | no | yes | yes | no | closed-world veto control |

Probe metadata is limited to:

```text
initial manifest
opaque action ID
reversible flag
immediate/exact observation mode
risk units
cost units
applicability hint bit
dependency hint bit
cleanup hint bit
generated provenance root
```

The three hint bits exist only for the strong explicit heuristic baseline.
They are not inputs to model prediction or information-gain scoring.

Frozen limits:

```text
maximum risk units                  10
maximum cost units                  10
maximum action count                 1
maximum manifest entries            48
maximum manifest bytes          98,304
maximum protocol bytes        1,048,576
```

Eligibility is conjunctive. Unknown, unsafe, non-reversible, ambiguous,
delayed, over-budget, malformed, or non-generated probes are ineligible even
if they would provide more information.

## 7. Prediction And Information Accounting

For every candidate probe and every model, the selector precomputes one typed
prediction:

```text
model root + probe root
-> exact success/failure disposition
+ exact predicted post-manifest when successful
-> prediction root
```

Predictions are grouped by observable outcome root. For `N=4` models:

```text
largest_partition = max(outcome group size)
minimax_eliminated = N - largest_partition
pair_separation    = N*N - sum(group_size*group_size)
```

Eligible probes are ranked by the frozen tuple:

```text
max minimax_eliminated
-> max pair_separation
-> min risk units
-> min cost units
-> stable probe root
```

The selected probe and the complete prediction matrix are one immutable
precommit. Stable hash is a final tie-break only. No outcome byte exists when
the precommit is sealed.

The expected optimal probe has partition `1/1/1/1`,
`minimax_eliminated=3`, and `pair_separation=12` in every confirm case.

## 8. Frozen Baselines

All baselines receive the same public models and catalog but never the private
true-model identity or observed outcome.

```text
passive observation
  select no probe; retain all four models; cost zero

stable-hash order
  select the eligible probe with the smallest probe root

cheapest first
  minimize cost, then risk, then probe root

strong explicit heuristic
  maximize 4*applicability + 2*dependency + cleanup,
  then minimize risk, cost, and probe root

oracle upper bound
  after reveal only, choose the eligible probe that minimizes the actual
  surviving-model count, then cost and probe root
```

The oracle is an upper-bound audit, not a non-learning policy the selector can
strictly beat. PASS requires exact oracle equality and strict superiority to
all four non-oracle baselines. Calling equality with the oracle a "win over the
oracle" is forbidden.

## 9. Execution And Observation Ownership

Roles are separate:

```text
selector process       predicts and precommits; cannot execute
baseline process       evaluates frozen rule policies; cannot execute
dispatch owner         resolves selected opaque action in private true model
sandbox worker         mutates one disposable tree; cannot select or verify
observer process       scans post-state read-only; sees no models or predictions
independent verifier   recomputes eligibility, predictions, ranking, and elimination
journal                persists the state transition; grants no authority
```

The dispatch owner may resolve a private effect only after an independent
pre-execution verification of the selection root. The worker request binds the
experiment, selected probe, selection precommit, worker executable, initial
manifest, and one resolved effect. The observer request binds only the
experiment, selected probe, observer executable, and read-only workspace.

One selected probe is executed once. Same-identity redispatch is rejected.
The source fixture is read-only; only the disposable work tree is mutable. All
workspaces are removed after observation while receipts remain.

## 10. Independent Elimination

The verifier independently:

1. validates every model, probe, executable identity, and authority boundary;
2. reconstructs all model/probe predictions without calling selector code;
3. reconstructs eligibility and the exact ranking tuple;
4. verifies the selected probe and full prediction precommit;
5. validates the observer receipt and executable identity;
6. matches the observed outcome to precommitted predictions;
7. retains exactly models with that outcome;
8. freezes the updated model-set root;
9. verifies each baseline and the oracle under their frozen definitions.

The positive path requires one surviving model in every case. Zero survivors
is contradiction and terminal FAIL. More than one is unresolved and terminal
FAIL under the one-probe budget.

## 11. Durable State Machine

The append-only logical sequence is:

```text
EXPERIMENT_FROZEN
BASELINES_FROZEN
SELECTION_DISPATCHED
SELECTION_PRECOMMITTED
SELECTION_VERIFIED
PROBE_DISPATCHED
PROBE_OBSERVED
MODELS_UPDATED
CONTROLS_FROZEN
TERMINAL_FROZEN
```

Every event binds sequence, previous-event root, payload root, and experiment
root. Publication uses temp file, file fsync, rename, and directory fsync.
Restart must reproduce every legal prefix. A dispatch without its matching
published result is indeterminate and cannot be retried under the same
experiment/case/probe identity.

Fault injection is required before and after rename for journal publication.
No rollback may erase a dispatch, precommit, observation, contradiction, or
terminal scientific result.

## 12. Required Negative Controls

At minimum all of these must PASS:

```text
1  unsafe high-information probe is never eligible or executed
2  cheapest useless probe is not selected by model-guided policy
3  ambiguous observation probe is rejected
4  delayed observation probe is rejected
5  unknown-effect action probe is rejected
6  private true-model root in selector bytes is rejected
7  post-outcome bytes in selector request are rejected
8  tampered selection prediction is rejected
9  tampered selected probe root is rejected
10 tampered observer post-manifest is rejected
11 action-ID permutation preserves the semantic selection disposition
12 path bijection preserves the semantic selection disposition
13 candidate-order shuffle preserves selection
14 collapsed model predictions destroy unique identification
15 shuffled model/effect binding destroys or changes the expected result
16 same-identity probe redispatch is rejected
17 every journal prefix survives restart with identical projection
18 authority promotion is rejected by every public request and receipt
```

## 13. PASS Contract

PASS is conjunctive:

```text
sealed confirm cases                              8 / 8
eligible catalog accounting                       8 / 8 exact
selected probes safe/reversible/immediate         8 / 8
selected precommits before outcome                 8 / 8
complete model/probe predictions                  4*8 per case
independent selection verification                 8 / 8
real isolated executions                           8 / 8
separate observer receipts                         8 / 8
updated model sets singleton                       8 / 8
model-guided survivors total                       8
passive survivors total                           32
stable-hash survivors total                      > 8
cheapest-first survivors total                   > 8
explicit-heuristic survivors total               > 8
model-guided result strictly better than each      yes
oracle survivors total                             8
model-guided selection equals oracle               8 / 8
forbidden probe executions                         0
prediction/accounting omissions                    0
negative controls                                 18 / 18
journal restart parity                     every legal prefix
residual generated workspaces                      0
authority                                         false everywhere
```

Strictly better means fewer aggregate surviving models under the same
one-probe budget. Cost is reported separately and cannot erase a failure to
identify. No wall-clock, token, product, or natural-traffic improvement is
claimed.

## 14. Source And Resource Budget

Allowed production-source change:

```text
add one generated-only active_inquiry sibling module
add at most six small generated-only process wrappers
add one integration test
add paper, preflight, structural receipts, logs, and one result receipt
add exactly two registration lines to learned_composition/mod.rs
```

Forbidden:

```text
changes to predecessor hidden-representation bytes
changes to K1, phase memory, packages, certificates, serving, economics
changes to dashboard or control-plane HTML
network access or deployment
natural or synthetic LLM traffic
production state mounts
budget widening after confirm reveal
```

All heavy compilation and tests run on `e@192.168.3.94` with `-j 20` in a
dedicated target directory. Production services are not restarted.

## 15. Claim Boundary

Allowed on PASS:

```text
bounded generated self-chosen safe inquiry             PROVED
model-guided active causal discrimination              PROVED
exact safety and one-probe accounting                   PROVED
```

Still forbidden:

```text
Natural K2                                             NOT PROVED
self-created predicates or action language             NOT PROVED
learned general experiment strategy                    NOT PROVED
partial-observation or delayed-effect inquiry           NOT PROVED
second-domain transfer                                  NOT PROVED
production authority                                    FALSE
K1 LawCertificate or registry membership                FALSE
Wave-caused whole-circuit grokking                       NOT CLAIMED
general intelligence                                    NOT CLAIMED
```

