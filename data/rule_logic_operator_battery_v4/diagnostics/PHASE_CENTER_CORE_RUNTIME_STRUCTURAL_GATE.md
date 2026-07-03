# Phase Center Core Runtime Structural Gate

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| phase_center_core_runtime | exports | nando_core::PhaseCenterFlatRuntime | PHASE_CENTER_CORE_RUNTIME_C32.md |
| phase_center_core_runtime | proves_route | C32 phase-center scorer parity | PHASE_CENTER_CORE_RUNTIME_C32.md |
| phase_center_core_runtime_release_gate | passed_with | zero wrong wins and zero parity mismatches | release gate output |
| phase_center_core_runtime | forbidden_flags | false | release gate output |
| phase_center_core_runtime_boundary_strict_decoder | excludes | strict ordered decoder | PHASE_CENTER_CORE_RUNTIME_C32.md boundary line: full strict ordered decoder |
| phase_center_core_runtime_boundary_multiseed | excludes | multi-seed strict robustness | PHASE_CENTER_CORE_RUNTIME_C32.md boundary line: multi-seed strict robustness |
| train_per_cell_2_multiseed | verdict | RED_MULTI_SEED_CURRENT_SCOPE | multiseed_train_per_cell_2/MULTISEED_ROBUSTNESS_REPORT.md |
| train_per_cell_2_conditional | remains | strict readout red | multiseed_train_per_cell_2/multiseed_summary.json |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| phase_center_core_runtime | exports | nando_core::PhaseCenterFlatRuntime | src/wave/phase_center_runtime.rs |
| phase_center_core_runtime | proves_route | C32 phase-center scorer parity | operator_battery_v4_phase_center_core_runtime_report |
| phase_center_core_runtime_release_gate | passed_with | zero wrong wins and zero parity mismatches | release gate output |
| phase_center_core_runtime | forbidden_flags | false | release gate output |
| phase_center_core_runtime_boundary_strict_decoder | excludes | strict ordered decoder | report boundary line: full strict ordered decoder |
| phase_center_core_runtime_boundary_multiseed | excludes | multi-seed strict robustness | report boundary line: multi-seed strict robustness |
| train_per_cell_2_multiseed | verdict | RED_MULTI_SEED_CURRENT_SCOPE | regenerated summary |
| train_per_cell_2_conditional | remains | strict readout red | regenerated summary |
