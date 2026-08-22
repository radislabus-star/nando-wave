# K2 Self-Formed Uncertainty V5 R8B V8 Spectral Amendment V1

Status: PAPER ONLY. MOVE-ONLY SOURCE-SCOPE AMENDMENT.

This amendment extends only section 14 of
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md`. It does not change any
scientific denominator, process chronology, byte schema, authority owner,
negative test, execution boundary, or claim boundary.

## 1. Pinned Parent Contract And Checkpoint

```text
parent contract SHA-256
  1e6f44a88cbc2c38173e7b6dc0a1f272e0bfea074cbbe11caf5d1ff26eda1844

implementation HEAD
  0015d265506aa1da440fb33708124cf4789a62c8

baseline linked target
  19 passed / 0 failed / 1 ignored

ignored process
  M24 requires explicit R8B V8 execution authorization
```

The implementation worktree is intentionally dirty. The five oversized files
below are pinned by their current worktree bytes before this amendment is
implemented:

```text
r8b_model.rs
  678 lines / 26626 bytes / mode 0664
  6a5b959e4f1b596aa9b19c79ea55df7e4721880c1c9c57746781f3cf2ada802f

r8b_process_ledger.rs
  891 lines / 38243 bytes / mode 0664
  ee41c3c13e55e404839405408e4d2578f4ae82eaac8de58f218d9e8e05c50b8a

r8b_process_authorizer.rs
  898 lines / 44759 bytes / mode 0664
  f92f67919495382ee6885efeb0781080815700cfadb8ba4d862a2221485e5be1

r8b_support/process_runtime.rs
  1381 lines / 46016 bytes / mode 0664
  c2aa6d8b870f9f9368cd7cf0062f6b8d2e49a366c771588fb93b6c4a627744ce

r8b_linked_v1.rs
  1471 lines / 56953 bytes / mode 0664
  b1256a91e3ae51ee75ee75a882ec818e975303bc43f451647f8b31849e455ebe
```

## 2. Exact Blocker

The P00-P09 capability route is implemented and its pure linked target passes,
but final source-budget parity is blocked by five files that exceed the frozen
V8 limits. Raising every limit would erase the spectral gate. The repair must
therefore separate existing signal routes without changing behavior.

## 3. Frozen Signal Cuts

```text
non-process evidence models
  -> wrapper seal/decode helpers

process ledger
  -> streamed validation
  -> durable writer persistence

process authorization
  -> resource/event validation
  -> expected-plan/projection validation

test process runtime
  -> sandbox/materialization adapters
  -> raw command and timeout transport
  -> durable process-ledger binding

M24 child route
  -> child orchestration
  -> cleanup/publication helpers
```

No symbol may move across an authority boundary. The new files are private
submodules and may not create new public API, schema, process, environment
input, output object, decision owner, or test fixture authority.

## 4. Added Ownership Modules

The following six paths are added to the V8 source scope:

```text
crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_model/wrappers.rs
  wrapper validation, sealing and evidence-view decoding only

crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_process_ledger/stream.rs
  streamed header/event/seal validation only

crates/nando-operator-learning/src/k2_goal_environment/learned_composition/self_formed_uncertainty/r8b_process_authorizer/projection.rs
  expected producer-plan and observed-ledger projection equality only

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_support/process_runtime/command.rs
  bounded command, stdin/stdout/stderr and timeout transport only

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_support/process_runtime/ledger.rs
  test-owned durable process-ledger binding and recorded-process wrappers only

crates/nando-operator-learning/tests/k2_self_formed_uncertainty_confirm_r8b_linked_v1/child_cleanup.rs
  M24 cleanup, retained-tree census and child-owned publication helpers only
```

## 5. Amended Budgets

All unchanged V8 budgets remain binding. These route-local budgets are added:

```text
r8b_model/wrappers.rs                    <= 220 lines
r8b_process_ledger/stream.rs             <= 320 lines
r8b_process_authorizer/projection.rs      <= 420 lines
r8b_support/process_runtime/command.rs    <= 520 lines
r8b_support/process_runtime/ledger.rs     <= 560 lines
r8b_linked_v1/child_cleanup.rs            <= 400 lines
```

The original router-file budgets remain unchanged:

```text
r8b_model.rs                              <= 650 lines
r8b_process_ledger.rs                     <= 700 lines
r8b_process_authorizer.rs                 <= 750 lines
r8b_support/process_runtime.rs            <= 900 lines
r8b_linked_v1.rs                          <= 1200 lines
```

## 6. Move-Only Vetoes

The amendment is invalid if any cut changes:

```text
canonical serialized bytes
public symbol visibility
process argv or environment
timeout or failure classification
ledger event order or roots
producer-plan or projection semantics
P00-P09 transition order
M24/M25/M26/P09 execution state
test denominator or ignored-test status
```

No R8B suite, M24, M25, M26, P09, transient unit, deployment, dashboard
mutation, push, scientific claim, or runtime authority is authorized.

## 7. Required Checkpoint Gates

```text
amended implementation preflight READY_TO_IMPLEMENT
-> move-only extraction
-> cargo fmt --check
-> source-scope and line-budget parity
-> linked target: 19 pass / 0 fail / 1 ignored
-> R8B compile-only and pure negative/parity tests
-> observed-source code-route gate
-> separate explicit R8B execution authorization
```

Any changed result, public surface, canonical byte vector, route graph, or
`WATCH`/`VETO` stops the amendment before execution.
