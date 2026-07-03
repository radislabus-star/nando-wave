# rule_logic_position_sequence_v3

Reserved clean path for the next ordered-sequence corpus.

Current status: generated and tested.

Result:

- shortcut gate: `VALID_POSITION_SEQUENCE_V3_CANDIDATE`
- runtime gate: `FAIL_CURRENT_ARCHITECTURE_ON_V3`
- reason: field and flat readout agree exactly, but current action/role binding does not separate the dense rule/length/output-slot matrix.

Do not copy v2 files here without changing:

- schema to `position_sequence_v3`
- manifest/report schema names
- balanced generation matrix
- shortcut gates
- per-group diagnostics

See `PLAN.md`.
See `baseline_v3_report.json` for the measured failure.
See `static_diagnostics_report.json` for action-separability and folded-collision pressure.
See `DIAGNOSTIC_RUNS.md` for reproducible commands and sweep bookkeeping.
