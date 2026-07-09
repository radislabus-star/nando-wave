# NANDA Task: Phase Stream Phase Atom Trace Builder

## query

Verify the bounded structural claim for `real_traffic_phase_atom_trace_v1`.
The builder may write atom-trace rows from existing telemetry, but it must not
claim route-family mining readiness, product local accept, or market savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| phase_atom_trace_report | output_rows | `17000` | report JSON |
| phase_atom_trace_report | rows_ready_for_route_family_mining | `0` | report JSON |
| phase_atom_trace_report | rows_ready_for_existing_shadow_scoring | `374` | report JSON |
| phase_atom_trace_report | local_accept_enabled | `false` | report JSON |
| phase_atom_trace_report | market_money_claim_allowed | `false` | report JSON |
| next_recorder | must_populate | `explicit_state_action_atoms` | report JSON |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| phase_atom_trace_report | output_rows | `17000` | report JSON |
| phase_atom_trace_report | rows_ready_for_route_family_mining | `0` | report JSON |
| phase_atom_trace_report | rows_ready_for_existing_shadow_scoring | `374` | report JSON |
| phase_atom_trace_report | local_accept_enabled | `false` | report JSON |
| phase_atom_trace_report | market_money_claim_allowed | `false` | report JSON |
| next_recorder | must_populate | `explicit_state_action_atoms` | report JSON |

## rejected_boundary

Do not compile, promote, serve, local-accept, claim CPU10, or claim market
money from this builder. Do not use raw response text, target/proof authority,
`concrete_x_lookup`, manual `local_out_t`, legacy `.nwrb`, or the old
role-binding backend. The result only proves that a phase atom trace format now
exists and that the current trace pool still lacks explicit state/action atoms
for new route-family mining.
