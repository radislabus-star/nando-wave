# CPU Call Catalog

Status: review-only working catalog for CPU80.

Source report:

```text
target/nando-wave/real-traffic-shadow/cpu-call-catalog-business-value-v1.report.json
```

The catalog is a product filter before building another operator profile. It
answers one question:

```text
Which real call class can add unique verified CPU accepts over exact cache?
```

It is not a market claim. It does not enable local accepts. It does not count
candidate, scoreable, or broad-route rows as savings.

## Business Value Gate

A profile row passes the gate only when all conditions are true:

```text
call_class appears in non-synthetic real trace
non-exact candidate calls exist
deterministic verifier hook is ready
expected unique CPU accepts over exact cache > 0
false_accepts = 0
```

Anything else goes to a shelf:

```text
PROVEN          already has unique verified CPU accepts
CANDIDATE       payload/verifier evidence exists, but expected unique accepts are not proven
WATCH           low support, singleton-only, no verifier, or exhausted support
REJECT_FOR_NOW  broad or risky route; split before more work
```

## Current Snapshot

Window:

```text
total_llm_calls: 1000
exact_cache_hits: 53
current_verified_cpu_accepts: 26
current_incremental_unique_cpu_accepts_over_exact_cache: 25
business_value_gate_passed_rows: 7
proven_profile_rows: 7
candidate_profile_rows: 4
watch_profile_rows: 12
rejected_profile_rows: 6
```

PROVEN rows:

| rank | call class | candidates | non-exact | expected unique | status note |
| ---: | --- | ---: | ---: | ---: | --- |
| 1 | `role_binding_mixed_map_seed0` | 99 | 96 | 7 | Support is already covered; improve payload/evidence before another promote. |
| 2 | `role_binding_agent_control_seed0` | 111 | 74 | 5 | Duplicates/exact-cache overlap constrain unique value; split broader tool-state subfamilies. |
| 3 | `git_control` | 18 | 18 | 4 | Current safe support is exhausted; improve command outcome evidence or split. |
| 4 | `role_binding_conditional_branch_seed0` | 88 | 87 | 3 | Verifier-ready, but policy support is exhausted; split stronger conditional subfamily. |
| 5 | `metrics_report_readout` | 55 | 55 | 3 | Current robust metrics support is exhausted; split stronger report subfamily. |
| 6 | `serving_ops` | 25 | 25 | 3 | Current serving support is exhausted; split stronger daemon/health subfamily. |
| 7 | `role_binding_edit_marker_length_seed0` | 92 | 92 | 1 | Low support; improve edit evidence before another promote. |

CANDIDATE rows:

```text
uncatalogued / resource_pressure_budget
read_inspect
style_brevity
resource_pressure_budget
```

These do not count as savings. They are work candidates only if expected
unique accepts can be raised by verifier evidence or a narrower split.

REJECT_FOR_NOW rows:

```text
answer_or_explain
project_context_dialogue
agent_continue_execute
```

These are intentionally blocked as broad routes. Work only narrow
artifact-backed subfamilies, never the route as a whole.

## Next Engineering Rule

Do not build the next profile because it is interesting.

Build it only if the catalog row shows one of these:

```text
expected_unique_cpu_accepts_over_exact_cache > 0
or
clear deterministic verifier evidence that can raise expected unique accepts
```

The current highest-leverage pattern is not another generic profile. It is:

```text
split high-volume REJECT_FOR_NOW routes into narrow artifact-backed call classes
or improve evidence geometry for exhausted PROVEN routes
```

Immediate safe targets:

```text
metrics_report_readout split
git_control split
serving_ops split
read_inspect verifier/evidence
test_output_parse if found in real trace
```

Blocked for now:

```text
answer_or_explain as a whole
project_context_dialogue as a whole
agent_continue_execute as a whole
IME singleton-only routes
resource_pressure without verifier-true evidence
```

## Claim Boundary

Allowed:

```text
On the current non-synthetic 1000-call Codex trace, the CPU call catalog finds
7 proven call classes and 25 incremental unique verified CPU accepts over exact
cache, with false_accepts=0.
```

Not allowed:

```text
Nando saves 80%
Nando saves market traffic
scoreable rows are savings
broad answer routes are safe
candidate rows are verified CPU accepts
```
