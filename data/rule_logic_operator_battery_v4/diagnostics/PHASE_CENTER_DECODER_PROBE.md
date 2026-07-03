# Phase Center Decoder Probe

Date: 2026-07-02

## Question

Can phase centers generate heldout output slots, not merely judge correct vs
wrong candidates?

## Method

Script:

```text
data/rule_logic_operator_battery_v4/run_phase_center_decoder_probe.py
```

Training creates relation phase centers for each operator/output slot:

```text
out_i receives src_j
out_i receives marker
```

Heldout decoding receives only:

```text
state_before source tokens
rule_action_example
condition flag
```

It enumerates possible source-slot/marker relations and selects the relation
closest to the phase center. The stronger mode also uses a learned capacity
cleanup profile from train, so the decoded sequence must use the right number
of source/marker factors. It does not choose between provided correct/wrong
candidates.

## Result

Verdict:

```text
PHASE_CENTER_DECODER_PROBE_WATCH
```

Action-key decoder:

```text
compiled_operator_keys: 380
compiled_slot_centers: 4741
heldout_rows: 5312
decoded_rows: 4800
heldout_accuracy_milli: 873
wrong_exact_rows: 0
same_bag_output_milli: 1000
median_min_slot_margin: 0.787044150945043
p10_min_slot_margin: 0.2922552609863498
```

Local-only decoder boundary:

```text
heldout_accuracy_milli: 807
wrong_exact_rows: 0
duplicate_source_rows: 1024
```

No-action key ablation:

```text
compiled_operator_keys: 40
compiled_slot_centers: 508
heldout_accuracy_milli: 15
wrong_exact_rows: 0
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
    "correct": 864,
    "beam_empty": 512,
    "other_wrong": 160
  },
  "order": {
    "rows": 2048,
    "correct": 2048
  }
}
```

## Interpretation

This is stronger than the previous phase-center judge:

```text
phase centers -> output slots -> learned capacity cleanup -> full sequence
```

It tests whether the center can act as a decoder/generator for the transition,
not only as a scorer for an already supplied candidate.

## Claim Boundary

Allowed:

```text
The current v4 battery has a zero-epoch phase-center decoder signal.
```

Not allowed:

```text
semantic grokking proven
production Wave runtime solved
enterprise benchmark complete
```

Next:

```text
compile the phase-center decoder into Rust/Wave flat runtime and add ablations:
without action center, without role center, shuffled centers, and reduced cells.
```
