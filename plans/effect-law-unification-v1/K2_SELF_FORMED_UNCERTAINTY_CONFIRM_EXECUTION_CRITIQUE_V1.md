# Critique Of K2 Self-Formed Uncertainty Confirm Execution Addendum V1

Status: `V1 REJECTED / REPAIR REQUIRED BEFORE PREFLIGHT`

Date: `2026-08-16`

Target: `K2_SELF_FORMED_UNCERTAINTY_CONFIRM_EXECUTION_ADDENDUM_V1.md`

## Verdict

V1 identifies the missing owners and preserves the one-attempt boundary, but it
is not implementation-ready. It leaves enough ambiguity to recreate the same
R9 failure under new names.

## Findings

| Severity | Finding | Why It Matters | Required Repair |
|---|---|---|---|
| P0 | The authorization receipt is named but not defined. | A free-form environment flag could impersonate R10 authorization. | Freeze a local-procedural authorization receipt containing exact user text, session ID, timestamp, predecessor freeze root and denied authority. Bind its root into the attempt descriptor. |
| P0 | The private resolver is allowed to open the full private batch. | It would see topology and true-class material not needed for safety dispatch. That creates a hidden answer route before observation. | Generator must emit a resolver-only table containing case ID, opaque action ID and effect, separate from final private truth. Resolver cannot open final truth. |
| P0 | V1 does not specify who may open final private truth or when. | The coordinator could read true classes before plans or observations freeze. | Final truth is mounted only into the independent final verifier, per case, after that case's complete observation vector is durable. Coordinator receives only the verifier receipt. |
| P0 | The exact confirm owner to generator wire format is missing. | The owner could persist the nonce-bearing request or pass it through argv/env. | Use one anonymous stdin pipe, clear child environment, close the parent write end after one request, forbid argv/env/path nonce transport and scan public artifacts for nonce bytes. |
| P0 | The terminal PASS evaluator is narrative, not a schema with exact counters. | A partial batch or missing control can be reported as PASS. | Define a sealed terminal request and receipt with every V4 conjunct, exact denominators and zero-valued veto counters. Independent terminal evaluator executable must recompute the verdict. |
| P0 | V1 inherits V2's one-probe oracle equality after V4 introduced two-probe plans. | A genuine two-probe closure cannot equal the residual count of a non-closing one-probe oracle. The conjunctive PASS is internally impossible for those cases. | V5 must replace the oracle with a complete bounded one-or-two-probe oracle and define exact independent joint-plan ranking before code. |
| P0 | V1 does not bind the coordinator executable into the source route independently from its manifest entry. | Another test-owned or changed binary could still orchestrate the attempt. | Freeze coordinator source, executable hash and command descriptor; the process must verify its current executable hash against the descriptor. |
| P0 | Dry-run readiness can bypass the nonce owner entirely. | The same boolean-capability defect can recur. | Readiness must exercise descriptor validation, process isolation, public/private split, all-case precommit, execution, terminal evaluation and cleanup through the exact coordinator and terminal binaries. Only CSPRNG creation and Confirm split are replaced by the frozen development input. |
| P1 | Execution order is not explicitly derived or bound. | Reordering after outcomes could adapt later cases. | Public generator output must freeze the nonce-derived case order; the batch precommit and attempt descriptor bind it before execution. |
| P1 | OS-level process isolation and mount policy are unspecified. | Same-user file modes alone do not stop accidental private reads or network access. | Use `bwrap --unshare-all`, no network namespace access, cleared environment, exact read-only/read-write mounts, process/RSS/CPU/file limits and per-owner mount tests. Custody remains LOCAL_PROCEDURAL. |
| P1 | Failure before `NONCE_COMMITTED` is ambiguous. | A failed nonce write could lead to a silent second creation. | Exclusive attempt-directory creation consumes the authorization slot. Any later failure is immutable terminal; no second directory or nonce under that authorization. |
| P1 | Retention and cleanup have no independent census owner. | Cleanup could erase failed evidence or leave disposable state. | Freeze before/after path manifests, classify every path and verify zero unclassified or disposable residue independently. |
| P1 | Existing development output parity is asserted without an exact golden root. | Confirm support could silently change the already proven development behavior. | Pin all 16 development public/private roots and process result root before edits; require byte parity after repair. |
| P1 | Negative controls are not bound to the sealed implementation revision. | Old development control results could be reused after confirm-route changes. | Rebuild and rerun all 32, V3 four and V4 sixteen controls from the successor commit; bind their binary/source/log roots into R9. |
| P1 | Baseline superiority aggregation is unspecified. | The terminal owner cannot prove the preregistered baseline conjunct. | Define exact per-policy per-case outcome, risk and cost comparison and aggregate counters before code. |
| P2 | Owner count and executable identities are inconsistent. | V1 names seven roles but does not say which may share a process. | Freeze an explicit executable topology. Confirm owner, coordinator, private resolver, terminal evaluator and cleanup verifier are distinct from each other and from all existing owners. |
| P2 | New orchestration risks becoming another 880-line monolith. | Proof, process IO, persistence and domain decisions would mix again. | Split model, persistence, process sandbox, public preparation, private dispatch, execution, terminal and cleanup modules with one-way dependencies. |

## Required V2 Shape

```text
authorization receipt
-> immutable attempt descriptor
-> exclusive attempt slot
-> nonce commitment
-> anonymous pipe to confirm-capable generator
-> public batch + resolver table + final truth split
-> public all-case preparation and precommit
-> resolver-only safety dispatch
-> worker and observer
-> final truth mounted only to independent verifier
-> independent terminal evaluator
-> independent cleanup verifier
-> immutable terminal and cleanup receipts
```

V2 must define exact schemas, executable topology, mount permissions, failure
states, denominators, development parity roots and pre-nonce tests. Until then,
the implementation preflight must return `BLOCKED_BEFORE_CODE`.

Because the defect changes a preregistered scientific comparator, a mere
execution addendum is insufficient. The repaired canonical contract must be a
new preregistration delta, V5, reviewed before implementation.
