# K2 Self-Formed Uncertainty V5 R8B Contract V3 Critique

Status: `VETO / DO NOT IMPLEMENT V3`

Date: `2026-08-20`

## 1. Verdict

V3 correctly moves Development split persistence into confirm-owner and removes
the second-dispatch/fixture-splice route. It is still unsafe to implement.

## 2. Findings

| Severity | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | V3 changes the meaning of `K2UncertaintyConfirmOwnerReceiptV1` in Development mode from absent split to present split. | Historical V1 Development receipts either become invalid under new validation or the same schema has two incompatible mode contracts. Confirm bytes may remain equal while historical receipt semantics silently drift. | Add a distinct `K2UncertaintyDevelopmentRehearsalOwnerReceiptV1`. Leave all V1 seal/validate/root functions byte-for-byte unchanged. The owner binary emits the response schema selected by request mode without a wrapper on stdout. |
| P0 | V3 does not persist enough information to reconstruct the exact owner receipt after a crash between split publication, journal append and stdout. | Recovery can avoid a second generator dispatch but cannot return the same receipt because the pipe receipt existed only in memory. A complete split can become an unrecoverable dead end. | Bind the complete non-secret pipe receipt into the Development split receipt. Persist or deterministically reconstruct a distinct Development owner receipt after `CasesGenerated`; replay of the same Development request returns the same receipt without redispatch. |
| P0 | V3 says the runner reads split metadata only, but the current `load_confirm_generator_split_receipt_v1` validates by reading every private resolver and truth file. | Reusing the loader would expose private truth to the public runner before the declared stage. | Add a Development metadata loader that validates only receipt bytes, schema, mode and descriptor roots. Confirm-owner owns full private-file validation; private children validate exact mounted payloads; cleanup verifier independently reopens final artifacts. |
| P0 | V3 permits unnamed "split-neutral downstream payload types" without an exact type and binding matrix. | A Confirm-only payload or a mode-free denominator could be accepted accidentally, enabling cross-mode substitution. | Freeze an explicit matrix of Development-only, common split-bearing, transitively bound reused, and immutable Confirm-only types. Add negative tests at every boundary. |
| P1 | V3 does not state whether the raw private batch is retained or how its root remains checkable after generator memory is gone. | The outer receipt could bind a private-batch root that no retained evidence can reconstruct. | Require the final-truth artifact set to reconstruct the sorted private cases and denominator commitment exactly, or retain a classified private-batch artifact. Bind the chosen route and test full root reconstruction. |
| P1 | Partial artifact paths exist after an injected fault, but retry/cleanup ownership is narrative only. | A replay could overwrite or reinterpret partial evidence, or cleanup could erase failure evidence. | Freeze per-prefix dispositions. The same attempt never overwrites; complete receipt permits idempotent recovery, incomplete receipt becomes `GeneratorResultIndeterminate`; cleanup removes only classified disposable temps and retains renamed failure artifacts. |
| P1 | "Confirm parity unchanged" is not bound to exact baseline bytes and hash-producing tests. | A shared-helper refactor could alter Confirm bytes while focused Development tests pass. | Preflight must hash current Confirm request, response, owner and split fixtures, map exact parity tests, and forbid edits to Confirm root formulas. |
| P1 | Cross-mode control denominator is left as an observed count. | A missing negative case can disappear without failing the contract. | Freeze the required named cross-mode matrix and exact count before implementation. |

## 3. Retained Good Decisions

The repaired contract should retain:

```text
one generator dispatch
unchanged Development generator wire bytes
split persistence inside confirm-owner
distinct Development top-level split schema
public/private filesystem separation
runner metadata-only boundary
no nonce, slot claim or sealed attempt
R8B structural-only claim boundary
```

## 4. Required Successor

V3 remains as rejected design evidence. A V4 contract must close every P0/P1,
then pass new structural, code-route and implementation-preflight gates. No V2
or V3 receipt may authorize implementation.
