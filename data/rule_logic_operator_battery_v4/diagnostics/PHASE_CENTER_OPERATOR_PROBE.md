# Phase Center Operator Probe

Date: 2026-07-02

## Question

Can v4 operator transitions be recognized by a center-of-mass of relation
waves, without epochs and without extracting an explicit slot-map program?

## Method

Script:

```text
data/rule_logic_operator_battery_v4/run_phase_center_operator_probe.py
```

The probe converts each candidate transition into relation-wave atoms:

```text
out slot i received source slot j
output length
source length
marker insertion when applicable
```

Train correct transitions form a circular phase center. Train wrong transitions
form an anti-center. Heldout correct and wrong candidates are scored by:

```text
coherence(candidate, correct_center) - coherence(candidate, wrong_center)
```

No epoch repair is used.

## Result

Verdict:

```text
PHASE_CENTER_OPERATOR_PROBE_WATCH
```

Action-key center:

```text
compiled_phase_centers: 380
heldout_rows: 5312
heldout_accuracy_milli: 1000
wrong_wins: 1
median_margin: 0.745695339668544
p10_margin: 0.24661838083430487
median_positive_center_gap: 0.5309988081981409
p10_positive_center_gap: 0.16924771234438507
```

No-action key ablation:

```text
compiled_phase_centers: 40
heldout_accuracy_milli: 738
wrong_wins: 1392
```

By class:

```text
{
  "composed": {
    "rows": 960,
    "correct": 960
  },
  "conditional": {
    "rows": 768,
    "correct": 768
  },
  "edit": {
    "rows": 1536,
    "correct": 1536
  },
  "order": {
    "rows": 2048,
    "correct": 2047,
    "wrong_wins": 1
  }
}
```

## Interpretation

This is the intended "three knobs -> center of mass" diagnostic:

```text
many relation waves
-> common phase center
-> correct heldout transition closer than same-bag wrong transition
```

It does not show epoch-based learning. It shows that the operator signal can be
represented as a phase center over transition relations.

## Claim Boundary

Allowed:

```text
The current v4 battery has a zero-epoch phase-center operator signal.
```

Not allowed:

```text
semantic grokking proven
production Wave runtime solved
flat CPU parity proven by this probe
```

Next:

```text
compile these phase centers into actual Wave field/readout weights
and compare against current epoch repair.
```
