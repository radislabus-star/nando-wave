# Rule Logic Corpus v2

Purpose: remove the v1 answer-surface leak.

v1 was useful as a smoke test, but its `surface_family` shortcut reached
`623/1000` against the L3 heldout gate. That is too high for a serious L3
training corpus.

v2 changes the target contract:

```text
input: rule task + shuffled answer choices
target: choice=<label>
near_negative: choice=<wrong_label>
```

So the answer surface is the same across numbers, symbols, statuses, sets, and
mini business-style logic tasks. The model must inspect the rule and the option
content, not guess from the answer format.

## Files

- `build_seed_rule_tasks_v2.py`: deterministic solved task generator.
- `run_shortcut_gates.py`: strict shortcut audit.
- `accepted_rule_tasks_v2.jsonl`: generated tasks.
- `manifest.json`: generated metadata.
- `shortcut_gate_report.json`: generated gate report.

## Gate Target

```text
exact_lookup == 0
source_group_majority <= 300
proof_rule_id_majority <= 300
surface_family_majority <= 300
answer_status_majority <= 300
markov_choice <= 600
```

This still does not prove L3 grokking. It only proves the corpus is no longer
obviously broken by the dumb baselines we know today.
