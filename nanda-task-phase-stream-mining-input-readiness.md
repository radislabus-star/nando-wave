# NANDA Task: Phase Stream Mining Input Readiness

## query

Verify the bounded structural claim that current traces are not ready for new
route-family phase-center mining because missing-shadow rows do not carry
request-side atoms.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| mining_readiness_report | llm_call_boolean_rows | `17000` | report JSON |
| mining_readiness_report | missing_shadow_rows_with_request_side_atoms | `0` | report JSON |
| mining_readiness_report | route_family_mining_ready_now | `false` | report JSON |
| next_artifact | should_be | `real_traffic_phase_atom_trace_v1` | report JSON |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| mining_readiness_report | llm_call_boolean_rows | `17000` | report JSON |
| mining_readiness_report | missing_shadow_rows_with_request_side_atoms | `0` | report JSON |
| mining_readiness_report | route_family_mining_ready_now | `false` | report JSON |
| next_artifact | should_be | `real_traffic_phase_atom_trace_v1` | report JSON |

## rejected_boundary

Do not run route-family mining over boolean-only traces. Do not inspect raw
response text, use target/proof labels, promote, serve, local-accept, or claim
market money from this readiness audit.
