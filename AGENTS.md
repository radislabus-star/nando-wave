# Nando Wave Agent Entry Point

Before reading or changing miner, Wave, runtime, verifier, admission, storage,
or economics code, read:

```text
/home/ubu/projects/nando-wave/ARCHITECTURE_CANON.md
```

That document is the architectural authority for this repository. Existing
research notes and reports are supporting evidence; they do not override the
canon silently.

## Required Change Protocol

Before editing core behavior:

1. Draw the affected signal path and mark the exact blocker.
2. Record a live baseline for coverage, savings, candidates, ACTIVE packages,
   false accepts, parity failures, latency, memory, and disk growth.
3. Compare behavior with the pre-refactor oracle named in the canon when the
   change touches discovery, grouping, Wave feedback, or transferable actions.
4. Change one mechanism at a time. Do not combine a refactor with a scoring or
   learning change.
5. Keep the independent verifier, external admission, and fallback boundary.
6. Run focused checks, the required live transition gate, and a live runtime
   check before claiming success.
7. Commit the scoped change.

Do not replace the Wave discovery core with a template/DSL selector. Do not
remove completed-trace teacher labels from training. Do not use future action
or response data at runtime.
