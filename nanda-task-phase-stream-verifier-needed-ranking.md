# NANDA Task: Phase Atom Verifier-Needed Ranking

## query

Verify the bounded structural claim for `phase_atom_verifier_needed_ranking_v1`.
The report may rank action families for verifier/result capture, but it must
not claim `.nwpc` compile eligibility, local accept, or market savings.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| verifier_needed_report | rows_with_action_family | `4936` | report JSON |
| verifier_needed_report | rows_with_verifier_label | `0` | report JSON |
| verifier_needed_report | compile_allowed | `false` | report JSON |
| verifier_needed_report | local_accept_enabled | `false` | report JSON |
| run_check_family | recommended_verifier_capture | `capture_tool_output_status_verifier` | report JSON |
| edit_or_build_family | recommended_verifier_capture | `capture_git_diff_or_file_change_verifier` | report JSON |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| verifier_needed_report | rows_with_action_family | `4936` | report JSON |
| verifier_needed_report | rows_with_verifier_label | `0` | report JSON |
| verifier_needed_report | compile_allowed | `false` | report JSON |
| verifier_needed_report | local_accept_enabled | `false` | report JSON |
| run_check_family | recommended_verifier_capture | `capture_tool_output_status_verifier` | report JSON |
| edit_or_build_family | recommended_verifier_capture | `capture_git_diff_or_file_change_verifier` | report JSON |

## rejected_boundary

Do not compile `.nwpc`, promote, serve, local-accept, claim CPU10, or claim
market money from this ranking. It only orders verifier/result capture work
for future verifier-bound phase-center discovery.
