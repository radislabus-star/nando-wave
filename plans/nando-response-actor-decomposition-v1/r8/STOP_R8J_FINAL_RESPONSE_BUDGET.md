# STOP-R8J: Final Response Actor File Budget

Status: `PASS / MOVE_ONLY / AUTHORITY_FALSE`

Date: 2026-07-21

## Boundary

The last two test-only soft-budget files were split without changing their
assertions or the immutable R0 failure set. Tests that belong to the historical
26-failure fingerprint remain in their original module; only passing families
move behind the new child modules.

```text
online_collection_tests.rs                 3187 -> 2207
online_collection_tests/scored.rs             0 ->  986
lib_tests.rs                               2661 -> 1871
lib_tests/projection.rs                       0 ->  796
online_collection test functions                 42/42
lib test functions                               51/51
historical failure names                         26/26
```

The response-miner sibling bridge was also checked through all targets. Its
helpers are visible only inside `response_miner::app`; this repairs the binary
test route missed by the earlier library-only R8H gate without widening the
crate API.

## Final Budget

```text
tracked response-actor Rust lines             56,804
largest production file                        2,476
production hard VETO files (>2500)                 0
test soft WATCH files (>2500)                      0
generic helper/common modules                      0
```

## Proof

```text
remote clean compile                         PASS
all-target Clippy fingerprint                PASS
historical tests                       287 PASS / 26 known FAIL
historical failure fingerprint               PASS
new test failures                                0
new Clippy diagnostics                           0
new remote background builds                     0
execution authority                          false
deploy/restart                               not run
```

Machine receipt: `R8J_FINAL_RESPONSE_BUDGET_STOP.json`.
