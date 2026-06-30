# Triad Packet Contract

Use this format for the first real NANDA structural gate.

## Minimal JSON

```json
{
  "task_id": "local-task-id",
  "domain": "customs|contract|crm|code|general",
  "query": "short user request",
  "triads": [
    {
      "id": "t1",
      "subject": "supplier",
      "relation": "supplies",
      "object": "goods",
      "evidence": "source line or file reference",
      "confidence": 0.8,
      "subject_role": "supplier",
      "object_role": "goods",
      "route": "delivery"
    }
  ],
  "candidate_triads": [
    {
      "id": "c1",
      "subject": "supplier",
      "relation": "supplies",
      "object": "goods",
      "evidence": "candidate answer",
      "confidence": 0.8,
      "subject_role": "supplier",
      "object_role": "goods",
      "route": "delivery"
    }
  ],
  "candidate_answer": "optional answer text to verify"
}
```

## Markdown Input

`nanda-pack-from-md` accepts two Markdown sections:

```markdown
## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|
| t1 | Alpha   | supplies | Goods  | doc.md:1 |        0.9 | supplier     | goods       | goods | deal1 |

## candidate_triads

| id | subject | relation | object | evidence         | confidence | subject_role | object_role | route | group            |
|----|---------|----------|--------|------------------|------------|--------------|-------------|-------|------------------|
| c1 | Alpha   | supplies | Goods  | candidate_answer |        0.9 | supplier     | goods       | goods | candidate-answer |
```

## Required Semantics

- `subject`, `relation`, and `object` are role-bearing fields, not plain tokens.
- `evidence` should point to the file, line, message, or paragraph that supports
  the triad.
- `confidence` is extractor confidence, not NANDA confidence.
- `triads` are source/evidence triads.
- `candidate_triads` are extracted from the candidate answer and checked against
  source triads.
- `group` binds triads that belong to the same deal, route, case, or local
  structural context. It is required for route-splice detection.
- Missing evidence should lower the final verdict to `WATCH`.

## First Wave Encoding Target

The first real checker should map each triad into:

```text
role(subject) bind entity(subject)
relation(relation)
role(object) bind entity(object)
composite(subject, relation, object)
```

The acceptance condition is not "tokens are similar". A candidate is accepted
only when local bindings and the composite triad mode agree.

## Minimal Output

```text
verdict: PASS | WATCH | VETO
complexity_score: 12
stable:
  - t1
weak:
  - t2
conflict:
  - t3 role/object mismatch
notes:
  - evidence missing for t4
route_coherence:
  weak:
    - candidate-deal
baseline_summary:
  exact_matches:
    - c1
  reversed_hits: []
```
