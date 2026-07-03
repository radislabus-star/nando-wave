# Rule Logic Binding Pressure v1

Purpose:

```text
Force L3 to learn transfer of X through an action trace.
Do not train on answer labels, proof_rule_id authority, or target_center_id.
```

Core pressure:

```text
state_before contains X
rule_action_example demonstrates generic movement
state_after_correct must contain the same X
state_after_wrong contains a plausible wrong Y
```

The train and heldout pools use disjoint bound values. A pass therefore cannot
come from exact variable lookup.

