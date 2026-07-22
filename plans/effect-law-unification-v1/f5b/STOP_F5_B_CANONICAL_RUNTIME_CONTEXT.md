# STOP-F5-B Canonical Runtime Context

Status: `PASS / F5_C_UNLOCKED_NOT_STARTED`

Implementation: `a237c3cd73ab43247d32ea03a4d8530b4bbe9e0d`

Authority: `false`

## Result

```text
Frozen pre-action evidence
  -> nando-operator-kernel shared structural walker
  -> FrozenCandidateRelationGraphV1
  -> canonical frozen-view adapter
                              \
                               -> CanonicalRuntimeStructuralViewV3
                              /
Live provider request
  -> bounded shallow synopsis
  -> the same kernel walker
  -> CanonicalRuntimeRequestV3
  -> ExtractionReceiptV3
```

F5-B now has one source-neutral structural language. Learning does not own a
second extractor, and runtime does not reinterpret the frozen graph. The live
object borrows the provider payload; durable views and receipts contain only
typed structure, hashes, counts, and fail-closed verdicts.

This stage does not dispatch a mode, bind an operator role, execute an actor,
verify an action, persist a package, or grant authority.

## STOP Matrix

```text
direct vs wrapped surface canonical parity          PASS
renamed/reordered surface canonical parity          PASS
teacher/action leakage scan                         PASS
one extraction per request                          PASS
budget exhaustion                                   ABSTAIN
raw durable payloads                                0
production callers                                  0
authority                                           false
```

The runtime caps are enforced before F5-C:

```text
request text                         <= 16 KiB
JSON nodes visited                   <= 4096
payload text visited                 <= 64 KiB
recent events                        <= 32
role candidates                      <= 64
relations                            <= 256
advertised capabilities              <= 64
```

Overfull capability sets, wide events, truncated candidate searches, and
oversized request text return an extraction receipt with
`AbstainBudgetExhausted`. They do not return an earlier plausible candidate.

## P0 Closed During Review

The first implementation bounded the main candidate walker but still called
recursive `canonical_shape` and `collect_shape_stats` pre-passes and scanned
the complete input/capability arrays for a synopsis. That would have made the
4096-node receipt mathematically false on wide requests.

The final implementation removes those routes:

```text
candidate extraction
+ event topology
+ event class
= one recursive walker and one shared node counter

runtime synopsis
= bounded shallow recent-window scan
+ bounded capability declarations and argument roles
```

The adversarial tests prove that a 1000-member event stops at exactly the
configured node cap and that a 1000-capability request abstains after the
bounded declaration prefix.

## Verification

All remote STOP gates ran on exact HEAD with no overlay or untracked input:

```text
nando-operator-kernel       13 PASS / 0 FAIL / Clippy PASS
nando-operator-runtime      13 PASS / 0 FAIL / Clippy PASS
nando-operator-learning    198 PASS / 0 FAIL / Clippy PASS
owner total                224 PASS / 0 FAIL

nando-response-actor       287 PASS / 26 known FAIL
response fingerprint       PASS, no new test or Clippy diagnostics
workspace check            PASS
scoped rustfmt             PASS
Graphify                    25,728 nodes / 57,818 edges / 1,188 communities
```

The full workspace `cargo fmt --check` remains blocked by pre-existing drift
in `crates/nando-core/src/wave.rs` and
`crates/nando-operator-runtime/src/runtime.rs`; neither file belongs to F5-B.

NANDA owner routes:

```text
kernel structural language       PASS
learning frozen adapter          PASS
runtime context owner            PASS
authority_ready                  false
```

Graphify resolves the live extractor to `extract_structural_surface_v3` in one
call hop and `PreActionBindingSurfaceV1` to the same function in two hops. The
shared-owner route is present in the code graph rather than only in this plan.

The live composite gate remains fail-closed:

```text
gate verdict                     PASS
eligible_for_local_accept        false
response ACTIVE packages         0
M3                               WATCH
false accepts                    0
runtime parity failures          0
```

Both service `InvocationID` values are byte-identical to STOP-R9 and
`NRestarts=0`. No deployment or restart occurred.

## Artifacts

```text
STOP_F5_B_CANONICAL_RUNTIME_CONTEXT.json
STOP_F5B_KERNEL_REMOTE.json
STOP_F5B_RUNTIME_REMOTE.json
STOP_F5B_LEARNING_REMOTE.json
STOP_F5B_RESPONSE_REMOTE.json
STOP_F5B_LIVE_GATE.json
STOP_F5B_SYSTEMD_STATE.txt
NANDA_F5B_KERNEL.md
NANDA_F5B_LEARNING.md
NANDA_F5B_RUNTIME.md
f5b-kernel.trace.json
f5b-learning.trace.json
f5b-runtime.trace.json
```

## Next Boundary

Only F5-C is unlocked:

```text
ProtocolModeSetV2
-> existing RoleGraph / OperatorCircuit vocabulary
-> immutable structural dispatch index
-> complete pre-phase mapping report
-> ABSTAIN on cap, hidden frontier, or overfull bucket
```

F5-C must consume `CanonicalRuntimeRequestV3`. It may not rescan the provider
payload, add another predicate language, call the legacy
`bind_raw_pre_action_components`, or grant runtime authority.
