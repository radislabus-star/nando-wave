# K2 Self-Formed Uncertainty V5 R8B Implementation Inventory Critique

Status: `CRITIQUE COMPLETE / THREE DEFECTS REPAIRED / PREFLIGHT STILL REQUIRED`

Date: `2026-08-20`

## Verdict

The first implementation inventory was directionally coherent but not exact
enough to authorize preflight. Three P1 defects were repaired before generating
an implementation-preflight receipt.

## Findings And Repairs

| Severity | Finding | Consequence | Repair |
|---|---|---|---|
| P1 | F02 described a retained lock file, while V5 requires a nonblocking advisory lock on the canonical lab-root directory whose ownership ends at process death. | An implementation could create a stale durable lock artifact and block legal recovery. | State explicitly that the directory lock creates no artifact and is released by process death. |
| P1 | The code budget counted the cfg-test Development module as runtime and undercounted integration test files. | The claimed ownership and size budget did not match the listed paths. | Freeze seven runtime modules, one cfg-test module and five integration test files. |
| P1 | Cleanup interruption and actual-tree classification had a required test ID but no dedicated file owner. | The linked-route test could silently absorb another proof route and become a mixed monolith. | Add a bounded R8B cleanup integration test owning classification, retention, deletion, residue and interruption. |

## Source Feasibility Checks

The existing journal already exposes the descriptor, events, projection and
validated append operations needed for D1 through D5. Its transition table
already permits:

```text
ArtifactsFrozen -> GeneratorDispatched
GeneratorDispatched -> CasesGenerated
GeneratorDispatched -> GeneratorResultIndeterminate
```

Therefore V5 recovery can remain a Development-owner concern and
`confirm_attempt_journal.rs` does not need to change.

The existing cleanup model accepts a closed relative-path registry and separates
retained, disposable and superseded evidence. The R8B cleanup route can reuse
that implementation while constructing an exact test-owned registry over the
actual linked attempt tree. Existing cleanup owner and verifier bytes remain
frozen.

The existing Confirm artifact reader follows symlinks and uses ordinary rename,
but V5 does not reuse it for Development publication. The new
`immutable_publication.rs` route must use `O_NOFOLLOW`, regular-file and link
count checks, same-filesystem no-clobber publication and exact fault controls.
Confirm bytes and behavior remain a separate compatibility denominator.

## Remaining Boundary

This critique proves only that the planned file ownership and failure topology
are internally implementable against the observed source. It does not prove the
new code, recovery, R8B execution, resource limits, production survival or any
scientific claim. Rust edits remain forbidden until the fresh manifest returns
exactly `READY_TO_IMPLEMENT` with zero blockers.
