# NANDA Task: phase-stream-live-self-mining-loop-evidence

## query

Check that the live self-mining loop reads verifier-bound phase atom traffic,
ranks action-family classes, compiles quarantine .nwpc candidates, and reports
shadow metrics without local accept.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_self_mining_loop | reads | phase_atom_trace_rows | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#total_rows |
| live_self_mining_loop | groups | action_families | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#action_families_seen |
| live_self_mining_loop | ranks | high_value_classes | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#high_value_classes |
| live_self_mining_loop | compiles | quarantine_nwpc_candidates | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#compiled_quarantine_candidates |
| quarantine_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.candidate_package_path |
| live_self_mining_loop | shadow_scores | heldout_events | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.heldout_events |
| live_self_mining_loop | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#aggregate_unique_cpu_accepts_over_exact_cache |
| live_self_mining_loop | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.false_accepts |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_self_mining_loop | reads | phase_atom_trace_rows | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#total_rows |
| live_self_mining_loop | groups | action_families | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#action_families_seen |
| live_self_mining_loop | ranks | high_value_classes | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#high_value_classes |
| live_self_mining_loop | compiles | quarantine_nwpc_candidates | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#compiled_quarantine_candidates |
| quarantine_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.candidate_package_path |
| live_self_mining_loop | shadow_scores | heldout_events | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.heldout_events |
| live_self_mining_loop | reports | unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#aggregate_unique_cpu_accepts_over_exact_cache |
| live_self_mining_loop | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-v1.report.json#classes.0.false_accepts |
