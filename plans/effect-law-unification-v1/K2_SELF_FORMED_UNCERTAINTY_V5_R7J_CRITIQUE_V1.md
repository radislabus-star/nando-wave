# Critique Of K2 Self-Formed Uncertainty V5 R7J Contract V1

Status: `V1 REJECTED / PAPER REPAIR REQUIRED / NO CODE AUTHORITY`

Date: `2026-08-20`

Target: `K2_SELF_FORMED_UNCERTAINTY_V5_R7J_CONTRACT_V1.md`

## Verdict

V1 identifies the three missing evaluation owners and preserves the zero-attempt
boundary. It is not implementation-ready. The draft assigns the oracle a
different two-probe state semantics from the frozen R7I execution route, places
an unbounded frontier inside a 1 MiB protocol, and asks the terminal owner to
recompute facts from opaque roots without the rooted receipt bytes.

No Rust implementation may start from V1.

## Findings

| Severity | Finding | Failure If Left Open | Required Repair |
|---|---|---|---|
| P0 | V1 applies both probes to one evolving state. R7I executes every selected probe against that probe's own frozen `initial_manifest` in a distinct workspace. | The oracle would evaluate a different experiment from the one whose observations and final-verifier receipt it consumes. | Define sequentiality as survivor-set filtering in frozen probe order. Each probe outcome is independently computed from that probe's own precommitted initial manifest. No filesystem state crosses probe workspaces. |
| P0 | The oracle request says it contains all representative dispositions while stdin is capped at 1 MiB and the frozen maximum is 1,792 representatives. | A valid complete frontier can be rejected by transport or silently narrowed to fit. | Keep stdin as a compact descriptor. Mount a read-only, content-manifested case evidence tree containing the paged raw frontier, public model set, prepared plan, baseline decisions and observation vector. Verify every file byte and root before evaluation. |
| P0 | V1 trusts a caller-supplied representative set. | Omitting a stronger representative can manufacture oracle equality. | Reconstruct the exact representative set independently from all 1,792 raw dispositions and the canonical equivalence classes. Require one canonical minimum-root member per class and exact membership coverage before enumeration. |
| P0 | The true syntactic model and true semantic class are not defined. | An evaluator can retain any singleton class and still report equality. | Match the private action-to-effect mapping against all four syntactic models, require exactly one matching syntax root, then require exactly one containing semantic class. Every evaluated plan must retain that class. |
| P0 | The oracle can compute counterfactual outcomes without checking that they are the outcomes actually observed on the model-guided route. | Oracle and final-verifier receipts can each be valid for different executions. | Independently apply the private effect for every selected probe, compare the resulting manifest outcome to the matching ordered observation, and bind the observation vector and final-verifier receipt before baseline/oracle aggregation. |
| P0 | The terminal contract says it consumes roots only, but also says it recomputes denominators and predicates. A SHA-256 root is opaque. | Terminal PASS can reduce to trusting caller booleans and totals. | Supply canonical nested receipt bodies or a read-only receipt tree plus a complete content manifest. The terminal reseals every receipt and recomputes only facts represented in those receipts. Raw private truth and observations remain forbidden. |
| P0 | Control rows do not define evidence that the named control process actually ran. | A caller can submit the expected disposition as both expected and observed. | Define a process-outcome row with runner/test executable roots, request root, normal-exit state, exit code, bounded stdout bytes/root, stderr root, timeout/panic flags and exact decoded disposition. R7J evaluates rows; R7K owns their execution. |
| P1 | Complete ordered enumeration can reach `n + n(n-1) = n^2`, or 3,211,264 plans for one maximal case. V1 records only a count. | An optimized or interrupted search can skip candidates while retaining the expected count. | Stream candidates in canonical plan-root order into a domain-separated hash chain, use checked arithmetic, and record enumerated, eligible and each rejection denominator. Tests compare the stream with an independent small-frontier exhaustive oracle. |
| P1 | V1 names safety eligibility but does not say whether caller booleans or source fields own it. | A planner-produced `eligible=true` can enter the oracle unchanged. | Recompute exact per-probe eligibility from reversible, exact-immediate, robust accounting and frozen limits; then apply checked cumulative risk and cost limits. |
| P1 | Baseline decisions are described by labels rather than exact pre-reveal provenance. | A baseline can be reselected after truth reveal. | Reopen the four decisions from the prepared public case, bind their original baseline owner and `ALL_CASES_PRECOMMITTED` roots, and reject any derived or replacement decision. |
| P1 | Import prohibition alone does not establish evaluator independence inside one Rust crate. | Shared elimination or ranking helpers can make verifier and oracle fail identically. | Freeze a prohibited dependency list, keep the evaluator module free of closure/final-verifier modules, run a source-route gate, and add adversarial parity tests built from independently calculated expected rows. Shared canonical value types and hashing are allowed. |
| P1 | Verdict precedence does not classify false accepts, failed controls, missing evidence and scientific comparator failures individually. | The same failure can become `INFRASTRUCTURE_FAIL` or `SCIENTIFIC_FAIL` depending on branch order. | Freeze a complete failure-class table. Ambiguous irreversible dispatch is indeterminate; malformed/missing/foreign evidence, controls, resources and forbidden effects are infrastructure failures; complete but false model, oracle or baseline predicates are scientific failures. |
| P1 | R7J fixture tests can construct a terminal-shaped PASS before R7K supplies real K1-K12 outcomes. | A component fixture can be published as Development rehearsal evidence. | Persist no terminal PASS in R7J. Component tests use explicitly non-authoritative fixture provenance. `DEVELOPMENT_REHEARSAL_PASS` requires R7K-owned executable control receipts and remains unreachable in R7J evidence. |
| P1 | V1 does not bind the richer private final-truth fields away from oracle ranking. | Topology family or matched-pair labels can become a shortcut to the true class. | Oracle semantics may read only case bindings and the action-to-effect mapping. Add a forbidden-field/source gate and prove permutation invariance for topology labels and matched-pair labels. |
| P2 | The four baselines remain one-probe while model-guided and oracle routes may use two probes. | PASS may be narrated as superiority over equally budgeted adaptive baselines. | Preserve the frozen comparator, but repeat the V5 claim exclusion in every receipt and result: no superiority claim over adaptive baselines with the same two-probe budget. |

## Repaired Route

```text
compact case descriptor
+ read-only manifested public evidence tree
+ post-observation private mapping
-> independent complete-frontier reconstruction
-> canonical representative reconstruction
-> actual outcome and observation parity
-> complete one/two-probe survivor-set oracle
-> frozen one-probe baseline evaluation
-> rooted per-case receipt
-> exact sixteen-case aggregate receipt

R7K process outcomes
-> scope-separated control evaluator
-> rooted control receipt

aggregate receipt + control receipts + compact route receipts
-> terminal receipt-body verification
-> deterministic precedence
```

## Gate Boundary

V2 may enter owner-bounded structural review only after every repair above is
explicit. Any `WATCH`, `VETO`, `BLOCKED_BEFORE_CODE`, missing 1 MiB size proof or
unsafe route decision returns the work to paper. R7K, sealed attempts, nonce
creation, production, K1 and Natural K2 remain locked.
