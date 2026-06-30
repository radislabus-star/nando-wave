# Hierarchical Gate Reference

Use this reference when a packet is too large for one honest NANDA verdict.
The goal is not to raise global limits. The goal is to keep hard limits as
role-mixing guardrails and build a proof tree:

1. global skeleton gate;
2. local subgates;
3. claim-boundary gate;
4. aggregate decision.

## When To Split

Split when the global packet returns `WATCH` only because of size, entity,
triad, route, or evidence-count limits.

Do not split when the global packet has:

- semantic-conflict `VETO`;
- unresolved evidence gaps;
- non-empty foreign pull;
- Codex failure-field block;
- missing source material.

Those remain unresolved or repair-required. A split does not turn weak evidence
into strong evidence.

## Naming Subgates

Name subgates by the relation that could be confused, not by arbitrary file or
paragraph position.

Recommended names:

- `global_skeleton_gate`;
- `subgate_<route>`;
- `subgate_<quantity_type>`;
- `subgate_<body_group>`;
- `subgate_<source_family>`;
- `subgate_remaining_debt`;
- `claim_boundary_gate`.

For physics/R&D packets, useful split dimensions are:

- `physics_route`;
- `quantity_type`;
- `body_group`;
- `source_family`;
- `claim_boundary`;
- `remaining_debt`;
- `validation_claim`.

## Aggregating Verdicts

The aggregate may be interpreted as:

```text
STRUCTURALLY_ACCEPTED_WITH_SPLIT
```

only when all conditions are true:

- global packet verdict is `WATCH`;
- global `WATCH` is size-only;
- every required subgate is `PASS`;
- `claim_boundary_gate` is `PASS`;
- no required local branch is missing or truncated.

Aggregation rules:

- any subgate `WATCH` => `REVIEW_REQUIRED`;
- any subgate `VETO` => `REPAIR_REQUIRED`;
- global semantic-conflict `VETO` remains `VETO`;
- missing evidence remains `WATCH`;
- size-only global `WATCH` is never called "almost PASS".

Correct wording:

```text
global WATCH was size-only; route-level subgates passed; aggregate status =
STRUCTURALLY_ACCEPTED_WITH_SPLIT
```

Incorrect wording:

```text
WATCH almost PASS
```

## Physics-Style Template

```yaml
global_skeleton_gate:
  checks:
    - previous node -> current node
    - operator frozen or changed
    - total coverage before/after
    - unresolved debt count
    - no validation overclaim

subgate_source_rate:
  checks:
    - source-rate rows
    - plume-rate rows
    - pickup-rate rows

subgate_dose_power:
  checks:
    - dose rows
    - particle-power rows
    - environment proxy rows

subgate_sputtering:
  checks:
    - sputtering-yield rows
    - source-rate rows
    - whether the source is lab, model, observation, or proxy

subgate_remaining_debt:
  checks:
    - what is intentionally not resolved
    - which rows stay outside numeric acquisition
    - which claims require later validation

claim_boundary_gate:
  checks:
    - source-rate != lab sputtering yield
    - dose proxy != species-resolved particle flux
    - particle power != local flux density
    - numeric coverage != external validation complete
```

## Hard-Environment Example

```yaml
hard-environment-v1:
  global_skeleton:
    coverage: "13/27 -> 23/27"
    remaining_debt: 4
    role_operator: frozen

  source_rate_group:
    bodies:
      - Io
      - Europa
      - Enceladus

  dose_power_group:
    bodies:
      - Ganymede
      - Callisto
      - Rhea
      - Dione

  remaining_debt_group:
    debts:
      - Ganymede sputtering
      - Callisto sputtering
      - Rhea sputtering
      - Dione sputtering

  claim_boundary:
    - not all rows are species-resolved fluxes
    - not all rows are lab sputtering yields
    - external validation not complete
```

Report this case as accepted only if the global skeleton, all local groups, and
the claim-boundary gate pass. Otherwise keep `WATCH` or `VETO` visible.
