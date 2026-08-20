# K2 Self-Formed Uncertainty V5 R8B Contract V4 Critique

Status: `VETO / V4 MUST NOT AUTHORIZE CODE OR EXECUTION`

Date: `2026-08-20`

Reviewed contract:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4.md`

Reviewed source commit:
`bdcae5351c7de75f325b0ebe752804066823cc38`

## 1. Verdict

V4 repairs the four defects found in V3, but it still does not define one
crash-safe, single-owner Development route. It is therefore a useful rejected
design, not implementation authority.

The decisive source facts are:

```text
confirm_owner.rs:35-94
  one public function returns ConfirmOwnerReceiptV1 for both modes

confirm_owner.rs:96-125
  every existing attempt enters recovery and then returns an error

confirm_owner.rs:127-186
  Development dispatches once, appends CasesGenerated(response root),
  persists no split and returns the historical shared owner receipt

confirm_owner.rs:256-282
  only Confirm publishes the existing private split

confirm_attempt_journal.rs:278-316
  Development ReadyForGeneratorDispatch is not resumed by recovery
```

R7I and R7J do not close this gap. They construct a separate Confirm fixture
from a fixed nonce and then test downstream components. They do not consume the
Development response produced by confirm-owner.

## 2. Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | V4 does not retire the existing Development branch of `execute_self_formed_confirm_owner_v1`. | The new linked owner and the old response-only owner can coexist as two Development producers. A test or future caller can still bypass the new split and recovery contract. | Split the internal APIs by mode. The Confirm API must reject Development. The Development API must return only the new receipt. The process may dispatch by validated input schema/mode, but stdout contains the selected concrete receipt without a wrapper. |
| P0 | V4 preserves `R7H 9/9` while the current R7H Development test requires `split_receipt_root_sha256 = None`. | The old test and the new required Development split cannot both remain behaviorally true. Keeping both would force a competing legacy path. | Retain frozen R7H evidence at its historical commit, replace the current successor assertion with an explicitly named R7H-invariant compatibility test, and record the intentional supersession. |
| P0 | No single-writer rule protects one attempt from two live owner processes. | A second process can treat an in-progress attempt as a crashed attempt and append `GeneratorResultIndeterminate` while the first generator is still running. | Acquire one nonblocking exclusive owner lock before inspecting or mutating an attempt and hold it through receipt-byte preparation. A concurrent caller must fail without journal or file mutation. |
| P0 | The recovery table omits the valid Development phase `ArtifactsFrozen -> ReadyForGeneratorDispatch`. | A crash before dispatch cannot resume even though no generator side effect occurred. Current source simply returns an error. | Freeze every pre-dispatch, dispatched, split, journal and owner-receipt state. Resume only the state that proves no dispatch event exists; never redispatch after `GeneratorDispatched`. |
| P0 | Binding a pipe receipt into the split is not enough to prove that the 34 files reconstruct the exact response bytes observed on stdout. | Recovery can bind roots that are individually valid but not prove equality with `response_bytes`, `response_bytes_sha256` and the response root in the pipe receipt. | Reconstruct the ordered private batch and complete Development response, serialize canonically, and compare exact root, byte length and SHA-256 with the pipe receipt before `CasesGenerated`. |
| P1 | Artifact descriptors omit a private-case ordinal and V4 does not freeze an exact reconstruction formula. | `PrivateBatchV1` is order-sensitive while its validator does not require sorted cases. The current generator sorts, but recovery authority would depend on an implementation habit rather than persisted structure. | Add an ordinal to each private descriptor, bind exact ordinal coverage `0..15`, and reconstruct in ordinal order. Independently verify the current generator's sorted order. |
| P1 | V4 names three new structs but omits the distinct Development artifact-kind enum and exact schema/root formulas. | A superficially compatible implementation can pass prose review while changing the byte contract. | Freeze schema strings, every root input and exact logical identity constraints in the paper before preflight. |
| P1 | The 14-case mode matrix omits the historical Development-shaped `ConfirmOwnerReceiptV1`, the reverse owner-receipt substitution, a foreign pipe receipt and a historical `CasesGenerated(response root)` journal. | The exact legacy objects most likely to be confused with V4 remain untested. | Expand and name the matrix. Keep positive Confirm byte fixtures and Development recovery fixtures as separate denominators. |
| P1 | V4 says files are exact but does not forbid symlink, hard-link or path-escape substitution during recovery and mounting. Existing Confirm reads follow symlinks. | A descriptor hash can match while physical custody points outside the attempt tree. | New Development readers must use bounded no-follow regular-file checks, canonical parent checks, exact mode and link count. Add path-substitution negatives. Confirm bytes remain unchanged. |
| P1 | `rename` is described as no-overwrite publication, but ordinary POSIX rename can replace an existing destination. | The paper promises stronger durability than the named operation provides. | Use a no-clobber publication primitive for immutable Development files and define restart handling for a linked final plus leftover temp. Journal replacement remains a separate single-writer operation. |
| P1 | Fault coverage has no exact denominator and mixes pure persistence faults with real process restart claims. | A few selected tests could be reported as complete coverage, or exhaustive generator reruns could exceed the resource contract. | Separate all immutable-publication boundaries from a small exact process-restart matrix and report both denominators. |
| P1 | Cleanup does not enumerate the 34 payloads, split receipt, owner receipt, temporary files and failed initialization states. | A failed route can leave unclassified private payloads or residue while still presenting unrelated cleanup PASS. | Freeze successful and failed-attempt cleanup classes and require the actual linked tree in the cleanup census. |

## 3. Atomic Repair Obligations

```text
O01 Confirm API accepts Confirm only.
O02 Development API accepts DevelopmentRehearsal only.
O03 Process stdout selects one concrete receipt and adds no wrapper.
O04 One exclusive live owner protects each Development attempt.
O05 A rejected concurrent owner performs zero durable mutation.
O06 Pre-dispatch recovery may perform exactly one dispatch.
O07 Post-dispatch recovery may never redispatch.
O08 Pure publication faults and process restart faults are separate denominators.
O09 Control evaluator and terminal evaluator remain separate unchanged owners.
O10 Historical Development owner and journal shapes are rejected by R8B loaders.
```

## 4. What V4 Got Right

The following V4 decisions remain valid and are carried into V5:

```text
distinct Development top-level receipts
unchanged Confirm schemas and validators
complete pipe receipt persisted through the Development split
owner-only full private validation
public/metadata-only runner loader
one linked process route before any R8B aggregate
R9B, R10B and R11B remain locked
```

## 5. Required Successor

V5 must pass, in order:

```text
source-grounded V5 critique repair
-> six owner-bounded structural routes
-> one V5 design code-route gate
-> exact-byte implementation preflight
-> READY_TO_IMPLEMENT
```

No V2, V3 or V4 receipt grants code authority.
