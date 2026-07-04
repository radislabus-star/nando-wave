# Test Output Parse Command Split Gate

Query:

```text
Verify that the test_output_parse real-traffic command block was split out of
the role_binding_runtime_cmd.rs monolith without changing CLI dispatch, runtime
behavior, admission policy, verifier authority, or CPU80 claims.
```

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| test_output_parse_command_split | extracted_route_block | test_output_parse_real_traffic_public_commands | crates/nando-cli/src/role_binding_runtime_cmd/test_output_parse.rs |
| role_binding_runtime_cmd | includes | role_binding_runtime_cmd/test_output_parse.rs | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| test_output_parse_command_split | preserves_module_scope | include_from_role_binding_runtime_cmd | crates/nando-cli/src/role_binding_runtime_cmd.rs |
| test_output_parse_command_split | moved_lines | 1632 | git diff --stat |
| test_output_parse_command_split | changed_cli_dispatch | false | crates/nando-cli/src/main.rs unchanged |
| test_output_parse_command_split | changed_help_surface | false | crates/nando-cli/src/help.rs unchanged |
| test_output_parse_command_split | changed_runtime_math | false | mechanical route-command include split only |
| test_output_parse_command_split | changed_admission_policy | false | mechanical route-command include split only |
| test_output_parse_command_split | target_labels_used_for_runtime | false | existing test_output_parse command claim boundaries |
| test_output_parse_command_split | proof_labels_used_for_runtime | false | existing test_output_parse command claim boundaries |
| test_output_parse_command_split | cargo_fmt_check | pass | cargo fmt --check |
| test_output_parse_command_split | cargo_check_nando_cli | pass | cargo check -p nando-cli |
| test_output_parse_command_split | cargo_clippy_nando_cli | pass | cargo clippy -p nando-cli -- -D warnings |
| current5k_catalog | total_llm_calls | 5000 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| current5k_catalog | current_incremental_unique_cpu_accepts_over_exact_cache | 110 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| current5k_catalog | verified_gap_to_80_calls | 3888 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| current5k_catalog | business_value_gate_passed_rows | 7 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| current5k_catalog | proven_profile_rows | 7 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |
| current5k_test_output_parse_row | current_status | PROVEN | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=route_gap_test_output_parse_profile_v1 |
| current5k_test_output_parse_row | expected_unique_cpu_accepts_over_exact_cache | 91 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=route_gap_test_output_parse_profile_v1 |
| current5k_test_output_parse_row | false_accepts | 0 | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json#row=route_gap_test_output_parse_profile_v1 |
| boundary_economics | no_size_only_split_policy | true | nanda-boundary-economics crates/nando-cli/src --find-refactors --format json |
| boundary_economics | large_new_api_split_permission | false | boundary refactor finder returned KEEP pressure, not SPLIT_STRONG |
| cpu80_status | achieved | false | target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json |

## Claim Boundary

This packet allows the narrow claim:

```text
test_output_parse public real-traffic commands were moved into a route-owned
include file while preserving the original module scope and passing format,
check, clippy, and structural-gate verification.
```

It does not allow:

```text
CPU80 achieved
new local accepts
new market savings
changed admission/verifier authority
large public API split approved
```
