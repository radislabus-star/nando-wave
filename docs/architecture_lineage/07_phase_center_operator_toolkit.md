# Position 7: Phase-Center Operator Toolkit

Anchor:

```text
data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_OPERATOR_PROBE_C32.md
```

## Central Tool Set

This is the working literature toolkit for the next Nando Wave step.

```text
Nanda / Fourier grokking:
  progress measures, phase/Fourier ablation, mechanism trace.
  reference: https://arxiv.org/abs/2301.05217

Kuramoto / circular statistics:
  phase center, order parameter r, coherence as confidence.
  reference: https://doi.org/10.1103/RevModPhys.77.137

Plate HRR and HDC/VSA:
  role/filler binding, superposition, noisy unbinding, cleanup memory.
  references:
    https://redwood.berkeley.edu/wp-content/uploads/2020/08/Plate-HRR-IEEE-TransNN.pdf
    https://arxiv.org/abs/2111.06077

Resonator networks:
  factorization of distributed structures into operator/role/filler factors.
  reference: https://arxiv.org/abs/2007.03748

Modern Hopfield / associative memory:
  energy retrieval, attractor cleanup, prototype memory.
  reference: https://arxiv.org/abs/2008.02217
```

## Nando Wave Mapping

The next mechanism must be stated in this language:

```text
relation waves
-> circular/phase center of mass
-> correct transition closer than same-bag wrong transition
-> Wave runtime readout
-> cleanup only if readout is noisy
```

The current strongest diagnostic signal:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_OPERATOR_PROBE_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/phase_center_operator_probe_c32_report.json

phase cells: 32
compiled_phase_centers: 380
heldout_rows: 5312
heldout_accuracy_milli: 1000
wrong_wins: 0
median_margin: 0.7671
p10_margin: 0.3130

no-action ablation:
  heldout_accuracy_milli: 782
  wrong_wins: 1156
```

Rust proof-runtime reproduction:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_RUNTIME_PROBE_C32.md

test:
  operator_battery_v4_phase_center_runtime_probe_report

result:
  verdict: PHASE_CENTER_RUNTIME_PROBE_PASS
  cells: 32
  action_heldout_accuracy_milli: 1000
  action_wrong_wins: 0
  action_p10_margin: 0.312965
  no_action_heldout_accuracy_milli: 782
  no_action_wrong_wins: 1156
  epoch_repair_used: false
```

Capacity and ablation:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_CAPACITY_ABLATION_C8_C64.md

test:
  operator_battery_v4_phase_center_capacity_ablation_report

result:
  verdict: PHASE_CENTER_CAPACITY_ABLATION_PASS

  C8 wrong_wins: 11
  C16 wrong_wins: 1
  C32 wrong_wins: 0
  C64 wrong_wins: 0

  C32 top16 train-cell ablation:
    accuracy_milli: 999
    wrong_wins: 5
    p10_margin: 0.157739
```

Flat runtime:

```text
artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_FLAT_RUNTIME_C32.md

test:
  operator_battery_v4_phase_center_flat_runtime_report

release result:
  verdict: PHASE_CENTER_FLAT_RUNTIME_PASS
  flat_accuracy_milli: 1000
  flat_wrong_wins: 0
  flat_sign_parity_mismatches: 0
  flat_margin_parity_mismatches: 0
  flat_records: 380
  flat_runtime_bytes_estimate: 407360
  flat_eval_p99_latency_ns: 506
```

Interpretation:

```text
The operator signal exists as a compact phase-center relation structure.
This is closer to the original Wave/Fourier goal than epoch table repair.
```

## Priority Order

Use this order before adding new architecture:

```text
1. phase-center relation-wave compiler
2. phase ablation and capacity curve: C8 / C16 / C32 / C64
3. weak-action and no-action probes
4. compile phase centers into Rust/Wave flat runtime
5. prove strict slot readout and sequence energy parity
6. only then allow epoch/error-driven repair if the Wave gate is red
```

## Hard Boundary

This is not allowed as proof:

```text
target_id
proof_rule_id authority
concrete_x_lookup
manual local_out_t
fixed frame_id
hidden answer table
hand-coded bind(X)
```

This is also not enough:

```text
Python phase-center diagnostic only
accuracy without ablation
energy-only judge without strict readout
single seed without robustness
```

## Next Proof Debt

```text
Build a Rust phase-center runtime diagnostic:
  fixed-size phase center records
  same train/heldout split
  correct-vs-wrong score parity with Python
  no-action ablation
  phase cell capacity curve
  byte/latency estimate
  forbidden flags false
  epoch_repair_used false
```

The next green claim is not "full LLM reasoning".

The next green claim should be:

```text
Nando Wave can induce compact transferable phase centers for operator
transitions on v4, and the signal survives same-bag wrong negatives plus
action ablation, before any epoch repair.
```
