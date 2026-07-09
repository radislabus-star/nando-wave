# NANDA Task: phase-stream-live-self-mining-loop-multifamily-evidence

## query

Check that the live self-mining loop handles a broader verifier-bound
phase-atom input with more than one action family, ranks classes, compiles
quarantine .nwpc candidates, and shadow-scores each class safely.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| multifamily_self_mining_loop | reads | multi_trace_rows | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#total_rows |
| multifamily_self_mining_loop | reports | action_family_count | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#action_families_seen |
| multifamily_self_mining_loop | ranks | high_value_classes | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#high_value_classes |
| multifamily_self_mining_loop | compiles | quarantine_nwpc_candidates | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#compiled_quarantine_candidates |
| tool_status_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.0.candidate_package_path |
| run_check_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.1.candidate_package_path |
| tool_status_candidate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.0.false_accepts |
| run_check_candidate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.1.false_accepts |
| multifamily_self_mining_loop | reports | aggregate_unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#aggregate_unique_cpu_accepts_over_exact_cache |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| multifamily_self_mining_loop | reads | multi_trace_rows | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#total_rows |
| multifamily_self_mining_loop | reports | action_family_count | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#action_families_seen |
| multifamily_self_mining_loop | ranks | high_value_classes | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#high_value_classes |
| multifamily_self_mining_loop | compiles | quarantine_nwpc_candidates | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#compiled_quarantine_candidates |
| tool_status_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.0.candidate_package_path |
| run_check_candidate | stores | nwpc_package | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.1.candidate_package_path |
| tool_status_candidate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.0.false_accepts |
| run_check_candidate | reports | safety_zero_counts | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#classes.1.false_accepts |
| multifamily_self_mining_loop | reports | aggregate_unique_cpu_accepts_over_exact_cache | target/nando-wave/streaming/phase-atom-live-self-mining-loop-multifamily-v1.report.json#aggregate_unique_cpu_accepts_over_exact_cache |
