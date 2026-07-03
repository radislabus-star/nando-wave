# Slot32 Role-Binding CLI Inspect Rung

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PACKAGE_INSPECT_V1_PASS
ROLE_BINDING_PACKAGE_VERIFY_V1_PASS
```

Commands:

```text
cargo run -p nando-cli --release -- role-binding-package-inspect-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
cargo run -p nando-cli --release -- role-binding-package-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
```

Report:

```text
target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
```

Scope:

```text
package format: .nwrb role-binding runtime
public runtime path: nando_core::WavePredictorRoleBindingOffloadRuntime
CLI commands:
  role-binding-package-inspect-v1
  role-binding-package-verify-v1
```

Core metrics:

```text
schema_version: nando_role_binding_package_inspect_report_v1
package_magic: NWRB0001
package_bytes: 26468
action_base: 131072
action_count: 16384
role_base: 0
role_stride: 4096
slot_scoped_action_page_bits: 12
slot_scoped_action_source_bits: 5
edge_count: 2202
serialized_len: 26468
payload_bytes: 26424
package_fingerprint64: 365065097387925697
magic_matches: true
serialized_len_matches: true
nonzero_fingerprint: true
nonempty_runtime: true
sdk_load_matches_inspect: true
report_matches_package: true
```

Verification after recording:

```text
cargo fmt --check
git diff --check -- <touched role-binding CLI/docs/report files>
cargo check -p nando-cli
cargo clippy -p nando-cli -- -D warnings
cargo run -p nando-cli --release -- role-binding-package-inspect-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
cargo run -p nando-cli --release -- role-binding-package-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Interpretation:

```text
The role-binding `.nwrb` package is now inspectable and verifiable from the
nando-cli product surface, not only through Rust integration tests. The CLI
route reads package bytes, inspects the fixed header, reloads through
WavePredictorRoleBindingOffloadRuntime, checks the SDK-loaded package info
against the inspected header, writes a product-proof JSON report, and verifies
the saved report against package bytes.
```

Boundary:

```text
This closes CLI inspect/verify for `.nwrb` role-binding package artifacts.
It is not `.nwrb` CLI scoring, not `.nwrb` daemon/registry routing, not
phase-center `.nwpc`, not raw-language action parsing, not broad workflow
reasoning, and not text generation.
```
