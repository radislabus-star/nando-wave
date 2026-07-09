# NANDA Task: Codex History Phase Atom Ingest

## query

Verify the bounded structural claim for `codex_history_phase_atom_trace_v1`.
The ingest may turn real Codex request history into request/state/action atoms,
but it must not claim verifier-bound operator mining, local accept, or market
savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| codex_history_phase_atom_report | output_rows | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_request_atoms | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_action_atoms | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_verifier_label | `0` | report JSON |
| codex_history_phase_atom_report | rows_ready_for_route_family_mining | `0` | report JSON |
| codex_history_phase_atom_report | local_accept_enabled | `false` | report JSON |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| codex_history_phase_atom_report | output_rows | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_request_atoms | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_action_atoms | `5000` | report JSON |
| codex_history_phase_atom_report | rows_with_verifier_label | `0` | report JSON |
| codex_history_phase_atom_report | rows_ready_for_route_family_mining | `0` | report JSON |
| codex_history_phase_atom_report | local_accept_enabled | `false` | report JSON |

## rejected_boundary

Do not compile, promote, serve, local-accept, claim CPU10, or claim market
money from this ingest. The output has request/state/action atoms but no
verifier/result labels. The next step is verifier/result capture or a verifier
needed ranking, not `.nwpc` promotion.
