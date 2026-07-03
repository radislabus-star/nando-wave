# WavePredictor Seed V1

This folder contains reviewed task candidates and references for WavePredictor
Task Factory work.

## Current Standard

Use `wave_task_v2`.

The canonical reference file is:

```text
reference_v2.jsonl
```

The schema and validator are:

```text
../wave_task_v2.schema.json
../validate_wave_task_v2.py
```

Old batch files were removed because they were candidate raw material, not
accepted training data.

## Required Fields

```text
schema_version
task_id
language
task_kind
domain_path
domain_tags
source_family
source_group
input
target
near_negative
operator_family
why_target_is_correct
why_negative_is_wrong
shortcut_risk
quality_status
```

## Field Rules

```text
schema_version = wave_task_v2
quality_status = candidate | reference | accepted | rejected
shortcut_risk = array of risks, not a single string
operator_family = operator family for balance and review, not answer authority
domain_path = hierarchical domain metadata
domain_tags = cross-domain/interference metadata
```

Domain metadata is for corpus control, balancing, heldout splitting, and
analysis. It must not be treated as runtime answer authority.

## Validation

```bash
python3 ../validate_wave_task_v2.py reference_v2.jsonl
```

## Reference Boundary

`reference_v2.jsonl` is an exemplar set for generation and review. It is not a
large accepted training dataset.
