# Rule Logic Corpus v3

Purpose: train L3 on action traces, not answer labels.

Core contract:

```text
state_before
rule_action_example
state_after_correct
state_after_wrong
```

Training interpretation:

```text
active wave = state_before + rule_action_example
target delta = state_before -> state_after_correct
negative delta = state_before -> state_after_wrong
```

This is the first corpus that matches the current L3 guard:

```text
L3 grokking object = invariant transition over wave state
```

Fields named `proof_rule_id` and `proof_rule_family` are proof/audit metadata
only. They must not become model authority.
