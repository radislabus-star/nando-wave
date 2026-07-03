# Rule Logic Corpus v1

Purpose: build solved rule-transfer tasks for L3.

This is not a general text corpus. It is a proof corpus for hidden operators:

```text
example/context -> hidden rule -> new application -> checkable answer
```

The model must not receive `proof_rule_id` as training authority. It is present
only for dataset audit, heldout construction, and proof reports.

## Current Layers

```text
L1: text -> n-grams -> lanes
L2: lanes -> motifs
L3: motifs/state -> rule operator -> verified next state
```

## Status Labels

```text
PROVEN: rule and answer are mechanically checkable
LIKELY: plausible hypothesis, not enough proof
UNSETTLED: required variable is missing
CONFLICT: evidence/rule collision
```

## Good Task

A task is useful only when shortcuts fail:

```text
exact lookup: fail
source-group majority: fail
rule-name leakage: fail
Markov/bigram surface guess: fail or weak
near-negative: plausible but wrong
```

## Files

- `rules.json`: rule operator inventory.
- `rule_task_v1.schema.json`: row contract.
- `build_seed_rule_tasks.py`: deterministic solved-task generator.
- `validate_rule_tasks.py`: structural JSONL validator.
- `run_shortcut_gates.py`: shortcut baseline audit.
- `external_sources.json`: external public corpora to review/import later.
- `accepted_rule_tasks_v1.jsonl`: generated solved seed corpus.
- `manifest.json`: generated build metadata.
- `shortcut_gate_report.json`: generated gate report.

## External Corpora Policy

External datasets are not imported blindly. Each source must pass:

```text
license_ok
format_adapter_ready
answer_verifiable
near_negative_available_or_generated
shortcut_gate_passed
```

The seed corpus is procedural and local. External sources are candidates for the
next ingestion phase, not runtime authority.
