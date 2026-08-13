# K1 Exact Experiment Opportunity Plan Critique V2

Status: completed critique of execution plan V1. Repairs are incorporated into
`K1_EXACT_EXPERIMENT_OPPORTUNITY_EXECUTION_PLAN_V2.md`.

Date: 2026-08-13.

## Verdict

The scientific decision is sound:

```text
exact deterministic identifier inputs
-> OpportunityRoot
-> at most one completed deterministic attempt per exact root
-> exact retained diagnostics
-> bounded research rate
```

Execution plan V1 is not safe enough as an executor handoff. It contains the
right mechanisms, but it leaves several decisions distributed across prose and
puts unrelated owners into one structural packet. An implementer could still:

- confuse receipt identity with causal experiment identity;
- preserve pieces of the rejected coarse-family patch;
- let learner proposals gain authority;
- append a terminal result before its diagnostic is durable;
- treat cooldown as missing evidence;
- deploy a V8 writer before compatible rollback readers exist;
- report scheduler progress as Law #2.

The repair is a staged V2 plan with explicit owner, input, output, invariant,
test, receipt, and exit gate for every boundary.

## Findings

| Severity | V1 weakness | Why it matters | V2 repair |
|---|---|---|---|
| P0 | One structural worksheet mixed identity, Wave execution, authority, persistence, deployment, budget, UI, and scientific claims | The gate returned `VETO`; owner conflicts and weak source/candidate pairing made the paper non-authoritative | Split into three coherent packets and require independent `PASS` for all three |
| P0 | The old dirty implementation is described as a refactor target but no removal inventory is frozen | Coarse family fields could survive in queue bytes, selection rank, authority parity, or dashboard even after the policy is declared rejected | V2 names every forbidden authority behavior and requires a pre-code diff inventory plus no-authority tests |
| P0 | `OpportunityRoot` and full Freeze V8 provenance remain easy to swap at the Raw Phase boundary | A timestamp, queue root, or generation number would turn one causal experiment into many different experiments | V2 gives the Raw Phase adapter its own implementation step and metamorphic parity gate |
| P0 | The terminal route description does not provide an explicit transaction state table | A crash between diagnostic and verdict could either lose the diagnosis or falsely mark a root attempted | V2 freezes event order, retry outcome, and expected restart projection at each fault point |
| P0 | Learner/authority parity is stated globally | An implementer may compare only final freeze roots while both processes share the same omitted input | V2 requires independently restored source-prefix, support, artifact, catalog, queue, causal-manifest, result, and diagnostic parity |
| P1 | Pure contracts, diagnostics, durable restoration, scheduler policy, and compatibility are broad phases without per-step artifacts | A phase may be called complete while its result exists only in code or logs | V2 requires a machine-readable receipt and explicit test list at every phase boundary |
| P1 | Exact deduplication, waiting semantics, and research rate are adjacent but not modeled as one deterministic state machine | Cooldown could be displayed as evidence starvation, or an exact repeat could consume budget | V2 defines ordered scheduler decisions and a transition table with no-event states |
| P1 | Legacy handling is described but not included in every relevant acceptance boundary | V1-V7 terminals might accidentally backfill the V8 attempt index | V2 requires zero legacy attempted roots in unit, replay, restart, and dashboard projections |
| P1 | Replay acceptance says worse-than-linear is a VETO but does not define how to compare current and 10x data | Measurement noise could be interpreted opportunistically | V2 freezes the input copies, measured fields, denominator, and decision receipt before replay |
| P1 | Deployment Phase A and B are correct but not tied to exact rollback fences and service ownership in one runbook | A rollback could install a pre-V8 reader after a natural V8 suffix exists | V2 binds rollback target, policy root, ledger prefix, anchor revision, binary hashes, and protected PIDs in deployment receipts |
| P1 | Dashboard work is a late phase but the backend-to-HTML parity contract is too small | The page could again show attractive but misleading aggregate counts | V2 limits the panel to decision facts, binds every value to an API field, and requires explicit unknown/zero semantics |
| P1 | There is no executor restart protocol after interruption | A new agent could repeat expensive discovery or silently continue from stale paper | V2 starts every phase with a compact resume check and forbids rerunning fresh gates unless their inputs changed |
| P2 | Commit boundaries are suggested, not required | A large mixed commit weakens review and rollback | V2 requires four implementation commits plus separate deployment receipts; UI cannot enter backend commits |
| P2 | The plan names useful outcomes but not a fixed engineering stop decision | Repeated classified failures could still lead to endless scheduler changes | V2 separates scheduler completion from natural scientific outcome and stops this route after one deployed, bounded, diagnostic-capable mechanism |

## Alternatives Rejected

### Keep the coarse family quotient as a cheap heuristic

Rejected. The current semantic signature is too coarse and may largely encode
only consequence type. Demoting a family can hide an unknown law. Coarse groups
may remain observation-only counters.

### Deduplicate by complete Freeze root

Rejected. Freeze metadata includes receipt and scheduling state that cannot
change the initial identifier result. It would manufacture novelty from time,
queue, and generation changes.

### Backfill OpportunityRoots for V1-V7 terminals

Rejected. Their signed records do not contain the complete causal manifest. A
reconstructed root would depend on mutable current state and could suppress a
natural experiment without preregistered evidence.

### Let the learner append its own diagnostic

Rejected. The learner owns execution proposals, not certification authority.
The authority must restore durable inputs, rerun the same pure evaluator, and
own final signed bytes.

### Skip the two-phase reader migration

Rejected. Existing readers deny unknown event variants and fields. Once a V8
event is naturally appended, a pre-V8 binary is not a valid rollback target.

### Promise Law #2 after scheduler deployment

Rejected. This work can expose a unique class or a precise defect, but Law #2
still requires independent future, BundleV4, admission, verified CPU execution,
economics, cleanup, and a passing LawCertificate.

## Residual Risks

These are real measurements for implementation and replay, not reasons to
weaken the contract:

1. Natural exact roots may almost never repeat. The mechanism is still useful
   only if diagnostics are classified and bounded; otherwise deployment is
   vetoed.
2. Authority replay may be expensive. The 256-row wake bound, content-addressed
   archive, and 10x replay gate must prove bounded behavior.
3. Stable rejection codes may be too coarse. `internal_unclassified` never
   grants deterministic suppression and blocks deployment if it dominates.
4. Mutable collection checkpoints can race selection. Double-read parity must
   return `STALE_BEFORE_FREEZE` with no journal event.
5. A correct scheduler cannot create missing natural evidence or guarantee a
   unique semantic class. Its success is truthful search, not a guaranteed law.

## Required Paper Decision

Implementation may begin only when:

```text
identity_and_raw_phase structural packet        PASS
authority_and_persistence structural packet     PASS
compatibility_budget_claim structural packet    PASS
code-route design                               PASS
implementation preflight                        READY_TO_IMPLEMENT
safe_to_implement                               true
```

`WATCH`, `VETO`, or `BLOCKED_BEFORE_CODE` means repair paper, not code.
