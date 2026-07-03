# Rule Logic Operator Battery v4

Purpose:

```text
Build proof-gated operator corpora beyond pure ordered same-bag permutation.
```

This battery is separate from the frozen v3 paged-u32 16-slot regression:

```text
data/rule_logic_position_sequence_v3/REGRESSION_LOCK.md
```

Operator classes:

```text
order        = same-bag position permutation
edit         = insert / delete / duplicate / replace / move state edits
conditional  = branch chosen from state condition
composed     = two-step operator chains
```

Protocol per class:

```text
1. build corpus
2. run shortcut gates
3. run training/runtime gate when the runtime supports the class
4. run ablations
5. verify field/flat parity
6. write report
```

Rules:

```text
No success claim if lookup, target_id, proof_rule_id authority,
concrete_x_lookup, manual local_out_t, or hidden hardcode is used.
```

Architecture rule:

```text
No architecture change until a red gate is reproduced and diagnosed.
```

Current first step:

```text
build_operator_battery_v4.py
run_shortcut_gates.py
```

Current mechanism contract:

```text
NEXT_MECHANISM_CONTRACT.md
```

This contract is the handoff from the first v4 battery run to the next code
step. It records which classes are green/red, which mechanisms are allowed, and
which shortcut substitutions remain forbidden.
