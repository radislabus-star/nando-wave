# Critique Of K2 Self-Formed Uncertainty V5 R7K Contract V1

Status: `V1 REJECTED / REPAIR REQUIRED BEFORE STRUCTURAL GATES`

Date: `2026-08-20`

Target: `K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V1.md`

## Verdict

V1 preserves the zero-sealed-attempt boundary and separates cleanup authority,
mutation and proof. It is not implementation-ready. Its process-outcome and
crash semantics contain three impossible transitions and several incomplete
bindings that could manufacture a clean rehearsal receipt.

## Findings

| Severity | Finding | Why It Matters | Required Repair |
|---|---|---|---|
| P0 | V1 requires the invalid target process itself to exit zero with normalized control stdout. | Existing fail-closed owners return typed errors/nonzero process exits. R7J intentionally rejects nonzero outcomes, so the contract cannot be implemented without changing every target or fabricating stdout. | Freeze one non-authoritative Development control-case adapter inside the integration test executable. It must call the actual target API, match one exact error, emit the frozen two-field stdout, and bind target function/source/request/error roots. The R7J evaluator remains the only control authority. |
| P0 | Deletion is journaled after mutation. | A crash after `unlink` and before append leaves an absent unrecorded path. V1 then declares terminal failure although the intended mutation may have completed, so exact restart parity is impossible. | Use per-path `DELETE_INTENT_FROZEN -> unlink/rmdir -> parent fsync -> DELETE_COMPLETE_FROZEN`. Restart may complete an intent only after matching either the pre-delete identity or the already-absent postcondition. |
| P0 | Cleanup journals and receipts are written inside the tree whose before/after census requires no new paths. | Correct cleanup would create unexpected residue and fail its own verifier. Ignoring those files would create an unclassified namespace. | Put cleanup control-plane state in a sibling root bound by the rehearsal descriptor. Give it a separate retained manifest. The cleanup owner receives the governed attempt root read-write and control root write-only by explicit role. |
| P0 | K12 merges retained-file deletion and residue omission without requiring both branches. | A runner could test only the easier branch and still emit the single expected disposition. | K12 becomes a two-subcase aggregate. Both `retained_delete` and `residue_omission` must independently reach the same named rejection and be bound into one K12 outcome root. |
| P1 | The caller supplies artifact kinds and the authorizer's static policy is not root-bound. | A changed or substituted policy could relabel retained evidence disposable. | Bind the closed artifact registry root, classification-policy source root and authorizer executable root into the classified manifest and authorization receipt. Independently reject any row whose class differs from the registry. |
| P1 | The complete manifest has no entry, path-length or aggregate-size bounds. | A malformed rehearsal tree could exhaust memory or exceed the 1 MiB protocol after the experiment has already terminated. | Stream a bounded manifested evidence tree: compact descriptor below 1 MiB, at most 8,192 entries, relative path at most 4,096 bytes, regular file at most the frozen artifact limit, and aggregate census bytes reported separately. |
| P1 | K3 and K4 are described as canary searches but the matching algorithm and allowed representations are not frozen. | Encoding, hashing or path normalization can hide a leak while the control claims PASS. | Bind raw bytes, lowercase/uppercase hex, base64 and SHA-256 canary forms. Scan argv, environment, path components, persisted request bytes and all public regular-file bytes. Record denominator and zero misses. |
| P1 | K5 and K6 prove logical validation but not the frozen mount boundary. | A process could reject early while still receiving private files, leaving a latent leakage path. | Run each control twice: closed-schema early-state rejection and mount census proving the forbidden private path is absent from the child namespace. Both subreceipts are required. |
| P1 | Directory and symlink semantics are incomplete. | Deepest-first deletion can cross a symlink or fail nondeterministically on non-empty directories. | Reject all symlinks at census and before every mutation. Delete regular disposable files first, then disposable directories in descending depth. Retained directories are never removed. |
| P1 | Development and future sealed result publication share a narrative entrypoint. | A Development terminal could be relabeled as a scientific result after cleanup. | Define disjoint tagged request and receipt schemas. R7K implements only `DevelopmentRehearsalComplete`; the sealed publisher path is decode/rejection coverage only. |
| P1 | V1 does not bind the exact current R7J commit and executable set before implementation. | Later source drift could make the K1-K12 evidence refer to a different evaluator than the published R7J component. | Implementation preflight must measure the current source commit/tree, R7J test roots and all three R7J executable hashes from the clean mini-PC build. |
| P2 | V1 says the supervisor owns path census without defining whether it may read private bytes. | A broad recursive reader would violate R7I private-content separation. | Census metadata and regular-file hashing run in a dedicated authority-denied function after terminal; the supervisor transports only the resulting descriptor. The function may read bytes only for hashing and cannot decode private schemas. |
| P2 | The spectral budget omits the control adapter and sibling control-plane model. | The implementation could exceed the frozen ownership plan without review. | Keep the adapter inside the R7K integration test binary and add no production wrapper. Permit at most six owner-specific modules while retaining exactly four new wrappers. |

## Required V2 Shape

```text
actual invalid owner call
-> exact typed error
-> non-authoritative control-case adapter
-> measured process outcome
-> R7J independent control evaluator
-> Development terminal
-> hash-only bounded attempt census
-> root-bound artifact registry classification
-> external cleanup control root
-> DELETE_INTENT_FROZEN
-> exact deletion + parent fsync
-> DELETE_COMPLETE_FROZEN
-> independent after census
-> CleanupFrozen
-> DevelopmentRehearsalComplete
```

V2 must also freeze the multi-form leakage denominator, K5/K6 mount-census
subcontrols, both K12 branches, and disjoint Development/scientific result
schemas. Until V2 passes the structural and implementation-preflight routes,
R7K code remains unauthorized.
