# Operator Grokking Probe

Date: 2026-07-02

## Question

Can the v4 operator battery be compressed into compact reusable operator
programs without epoch-based repair?

This probe tests a one-pass induction path:

```text
train transitions -> compact operator program -> heldout application
```

It is a diagnostic stand, not a Wave runtime proof.

## Method

Script:

```text
data/rule_logic_operator_battery_v4/run_operator_grokking_probe.py
```

Report:

```text
data/rule_logic_operator_battery_v4/diagnostics/operator_grokking_probe_report.json
```

For each train row, the probe induces a compact program:

```text
out_i <- src_j
out_i <- marker
```

The probe groups programs by:

```text
operator_class
sequence_length
condition_flag
normalized_rule_action_example
```

It does not use these fields for the key or program:

```text
proof_rule_id
source_group
task_id
state_after_correct
state_after_wrong
why_target_is_correct
why_negative_is_wrong
```

## Result

```text
rows: 10624
train_rows: 5312
heldout_rows: 5312
compiled_operator_programs: 380
operator_program_conflicts: 0
skipped_train_rows: 0
skipped_heldout_rows: 0
heldout_accuracy_milli: 1000
heldout_wrong_match_rows: 0
same_bag_output_milli: 1000
```

By class:

```text
order:       2048 / 2048
edit:        1536 / 1536
conditional:  768 / 768
composed:     960 / 960
```

By surface:

```text
business: 1328 / 1328
network:  1328 / 1328
ru_words: 1328 / 1328
symbols:  1328 / 1328
```

By noise:

```text
clean:             1328 / 1328
distractor:        1328 / 1328
instruction_noise: 1328 / 1328
prefix_suffix:     1328 / 1328
```

## Interpretation

The result supports the operator-compiler direction:

```text
operator weights do not have to come only from epoch repair;
the corpus contains compact transition structure that can be induced one-pass.
```

This does not replace Nando Wave. It says the next mechanism can be:

```text
one-pass compact operator induction
-> compile into Wave weights / energy / cleanup
-> optional repair only if gates stay red
-> frozen flat CPU runtime
```

## Claim Boundary

Allowed:

```text
The v4 corpus admits compact one-pass operator program induction.
For this diagnostic, 5312 train rows compressed into 380 reusable programs and
recovered all 5312 heldout rows.
```

Not allowed:

```text
semantic grokking proven
Wave runtime replaced
multi-seed product proof complete
enterprise benchmark complete
```

Important caveat:

```text
The current rule_action_example is explicit and identifies the operator action.
This probe does not parse the target slot-map from it, but it does use the
normalized action text as the operator key. A harder next probe should test
weaker action descriptions and compile the induced programs into the actual
Wave field/readout path.
```

## Next Step

Compare three compile paths:

```text
1. current multi-epoch repair
2. one-pass induced operator program
3. one-pass induced program compiled into Wave weights + cleanup
```

Metrics:

```text
strict slot
sequence energy
flat parity
ablation collapse
compile time
runtime latency
edge count
failure breakdown
```
