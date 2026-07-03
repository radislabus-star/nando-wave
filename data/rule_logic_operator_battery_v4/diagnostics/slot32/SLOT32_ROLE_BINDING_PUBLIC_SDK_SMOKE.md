# Slot32 Role-Binding Public SDK Smoke

Date: 2026-07-03

Verdict:

```text
SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE_PASS
```

Command:

```text
cargo test -p nando-core --test wavepredictor_role_binding_sdk_public -- --nocapture
```

Companion check:

```text
cargo clippy -p nando-core --test wavepredictor_role_binding_sdk_public -- -D warnings
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-role-binding-sdk-boundary.md --task-id slot32-role-binding-sdk-boundary --domain code
verdict: PASS
complexity_score: 29
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-boundary.trace.json
```

Scope:

```text
public Rust API surface:
  nando_core::WavePredictorRoleBindingOffloadRuntime
  nando_core::WavePredictorRoleBindingOffloadPolicy
  nando_core::WavePredictorRoleBindingEvalTask
  nando_core::WavePredictorRoleBindingDecision
  nando_core::WavePredictorRoleBindingOffloadSummary

package format:
  `.nwrb` serialized role-binding runtime

tested operations:
  inspect_package_bytes
  from_package_bytes
  score_task / decide_task through offload_summary_into
  invalid package rejection
  invalid policy rejection
```

Result:

```text
running 2 tests
test public_sdk_rejects_invalid_role_binding_package_bytes_and_policy ... ok
test public_sdk_loads_role_binding_package_and_routes_local_vs_fallback ... ok

test result: ok. 2 passed; 0 failed
```

Interpretation:

```text
The `.nwrb` role-binding package is no longer only a test-local byte artifact.
An external Rust consumer can inspect package bytes, load a runtime from bytes,
and route local-vs-fallback decisions through the exported nando_core API.
```

Boundary:

```text
This is a public Rust SDK smoke for the role-binding `.nwrb` package path.
It is not the phase-center `.nwpc` package path, not a CLI package command,
not a daemon/API registry proof, not raw-language action parsing, and not a
new claim about broad workflow reasoning or text generation.
```
