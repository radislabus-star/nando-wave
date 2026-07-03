# Phase Center Capacity And Ablation C8-C64

Date: 2026-07-02

## Question

Is the C32 phase-center result a lucky single setting, or does it show a real
capacity boundary and a removable phase mechanism?

## Command

```text
cargo test -p nando-core --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_phase_center_capacity_ablation_report --nocapture
```

## Method

Rust test:

```text
operator_battery_v4_phase_center_capacity_ablation_report
```

The test runs the same phase-center runtime path at:

```text
C8 / C16 / C32 / C64
```

Then it selects phase cells by train-only positive-vs-negative center
separation and removes the top cells from the C32 scorer.

No heldout answers are used for choosing ablated cells.
No epoch repair is used.

## Capacity Result

```text
C8:
  action_accuracy_milli: 998
  action_wrong_wins: 11
  action_p10_margin: 0.214822
  no_action_accuracy_milli: 710
  no_action_wrong_wins: 1543

C16:
  action_accuracy_milli: 1000
  action_wrong_wins: 1
  action_p10_margin: 0.246618
  no_action_accuracy_milli: 738
  no_action_wrong_wins: 1392

C32:
  action_accuracy_milli: 1000
  action_wrong_wins: 0
  action_p10_margin: 0.312965
  no_action_accuracy_milli: 782
  no_action_wrong_wins: 1156

C64:
  action_accuracy_milli: 1000
  action_wrong_wins: 0
  action_p10_margin: 0.416739
  no_action_accuracy_milli: 799
  no_action_wrong_wins: 1066
```

## Ablation Result

Baseline C32:

```text
accuracy_milli: 1000
wrong_wins: 0
median_margin: 0.767109
p10_margin: 0.312965
```

Train-only phase-cell ablation:

```text
top4:
  accuracy_milli: 1000
  wrong_wins: 0
  median_margin: 0.704321
  p10_margin: 0.276477

top8:
  accuracy_milli: 1000
  wrong_wins: 0
  median_margin: 0.634195
  p10_margin: 0.248453

top16:
  accuracy_milli: 999
  wrong_wins: 5
  median_margin: 0.498671
  p10_margin: 0.157739
```

## Verdict

```text
PHASE_CENTER_CAPACITY_ABLATION_PASS
```

## Interpretation

```text
C8/C16 are not enough for strict zero-wrong proof.
C32 is the first zero-wrong compact phase-center rung.
C64 increases margin reserve.
Train-only top-cell ablation predictably weakens the operator signal and
eventually creates wrong wins.
```

Allowed claim:

```text
The v4 phase-center signal has a measurable capacity boundary and a measurable
phase-cell mechanism inside the Rust proof runtime.
```

Still not allowed:

```text
full strict ordered decoder solved
production flat runtime solved
general semantic grokking solved
```

## Next Proof Debt

```text
1. Compile the C32/C64 phase centers into final flat CPU records.
2. Add latency/bytes report for the phase-center runtime path.
3. Add phase-cell ablation by operator class.
4. Keep epoch repair as fallback only.
```
