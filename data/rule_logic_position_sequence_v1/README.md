# rule_logic_position_sequence_v1

Persisted Step-12 corpus for ordered multi-token position binding.

Goal: force L3 to learn output-slot binding, not bag copy.

Rows contain:

- `state_before`: input sequence.
- `rule_action_example`: visible rule action example.
- `state_after_correct`: correct ordered sequence.
- `state_after_wrong`: same token bag, wrong order.

Hard gate:

- train/heldout token pools do not overlap.
- sequence lengths are 3, 4, 5, and 6.
- heldout rows carry context noise around the `sequence:` span.
- exact lookup must score `0`.
- bag-of-tokens shortcut must score `500` milli because correct and wrong share the same bag.
- Markov/bigram shortcut must stay below the rejection threshold.

Generate:

```bash
python3 data/rule_logic_position_sequence_v1/build_position_sequence_tasks.py
python3 data/rule_logic_position_sequence_v1/run_shortcut_gates.py
```
