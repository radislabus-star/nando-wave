# K2 Self-Formed Uncertainty V5 R8B V7 Implementation Preflight Critique V1

Date: 2026-08-21
Scope: manual adversarial review of `implementation-preflight.v7.json`
Machine receipt reviewed: `d501bbe48a5a00787e47a55235c8cb997bcc78d4b1159a790db104f127846a53`

## Verdict

REPAIR REQUIRED BEFORE ACCEPTING MACHINE READY.

The deterministic preflight returned `READY_TO_IMPLEMENT`, but that result is
schema and baseline readiness only. Manual review found one P0 execution-boundary
defect and one P1 request-channel ambiguity. No code, test attempt, deployment or
publication is authorized by the pre-critique receipt.

## Findings

| Severity | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | `IMPLEMENTATION_READY` flowed directly into pre-execution validation and eventually `R8B_FROZEN`. The V7 contract requires a separate explicit R8B execution boundary after implementation and static tests. | The implementation receipt could be misread as authority to launch the scientific route. | Insert a non-mutating `require-separate-r8b-execution-authorization` transition. Its positive state must be distinct from implementation readiness; absence of separate authorization terminates as `R8B_EXECUTION_NOT_AUTHORIZED`. Add an independent boundary test and invariant. |
| P1 | Suite and M24-child requests bound semantic roots and output directories, but the preflight did not freeze how canonical request bytes reach a libtest process. | Environment/path substitution could change selector, output path or evidence kinds while leaving the output-channel checks apparently valid. | Send one bounded canonical request on stdin to the exact libtest selector. Bind stdin SHA-256 in `ChildStarted`; require the child to decode once, verify route, selector, current executable SHA-256, allowed evidence paths and output directory before any write. Keep stdout/stderr diagnostic-only. Add positive parity and substitution/extra-byte negatives. |

## Checks That Passed Manual Review

- Inventory and donor parity are exact: `7 modified + 16 new = 23` paths, with
  no path present on only one side.
- All sixteen current donor files remain within their V7 line budgets. The
  tightest headroom is 119 lines in `r8b_authorizer.rs` and 140 lines in
  `r8b_model.rs`; implementation must remain focused.
- All invariant, identity and state-machine test references resolve; no IDs are
  duplicated.
- P06 excludes M25/M26 and P09; P09 remains diagnostics and cannot change M25 or
  M26 bytes.
- `/usr/bin/strace` is pinned by absolute path, mode, length and SHA-256.

## Next Legal Action

Retain the pre-critique manifest and receipt unchanged, repair the canonical V7
preflight, rerun the deterministic validator, then perform a second manual
critique. Implementation remains forbidden until both reviews close.
