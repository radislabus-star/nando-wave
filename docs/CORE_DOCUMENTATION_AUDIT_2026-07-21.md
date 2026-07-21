# Core Documentation Audit

Date: 2026-07-21 Europe/Tallinn.

Scope: documentation and status ownership only. Runtime code, services,
deployment, packages, and execution authority were not changed.

## Finding

The repository had enough architectural material, but no single current-core
entrypoint. Active plans also disagreed about the B1B/F4 boundary:

```text
controlled B1B receipt said F4 UNLOCKED
post-implementation ownership review said F4 BLOCKED
historical progress documents looked current by filename
```

The scientific evidence was not wrong. Its authority interpretation was stale.

## Resolution

```text
ARCHITECTURE_CANON.md
  owns invariant architectural meaning

docs/CORE.md
  owns the current machine, owner graph, status, and blocker

docs/CURRENT_CORE_DECISION.md
  owns the active product-core decision

active roadmap headers
  own mechanism-specific work status

STOP reports and JSON receipts
  preserve checkpoint evidence; they do not silently grant later authority
```

The current boundary is now identical across active documents:

```text
B1B controlled fixture                CONTROLLED_FIXTURE_PASS
independent physical truth            BLOCK
AcceptedBindingLawEvidence            BLOCK
F4 Protocol Compiler                  BLOCKED
production authority                  false
```

The original `STOP_B1B_ADJUDICATION.json` remains byte-preserved. Its
`f4_status=UNLOCKED_NOT_STARTED` field is explicitly historical.

## Structural Audit

An aggregate NANDA packet over the entire path returned `VETO` because it put
proof, compiler, runtime, verifier, and admission under mixed owner groups.
That is the failure mode this refactor is meant to prevent.

The packet was then split by linked owner group:

```text
semantic law owner          PASS
binding evidence owner      PASS
physical trial owner        PASS
trusted resolver owner      PASS
causal adjudicator owner    PASS
F4 compiler owner           PASS
runtime grounder owner      PASS
verifier owner              PASS
admission owner             PASS
controlled fixture owner    PASS

owner-local routes          10 / 10 PASS
authority_ready             false
```

This is a documentation-coherence result only. It cannot promote a package or
unlock F4.

## Mechanical Validation

```text
relative Markdown links     11 active files PASS
active F4 contradictions    0
git diff --check            PASS
Graphify update             PASS
Graphify graph              24453 nodes / 55960 edges / 1050 communities
```

Graphify reported 127 non-code source files with zero AST nodes. That warning
does not affect this documentation audit and remains a separate graph-extractor
observation.

## Next Authorized Work

```text
split binding_evidence_adjudication proof ownership
-> independently observed PhysicalTrialReceipt
-> separate TrustedLabelResolver
-> CausalAdjudicator
-> private AcceptedBindingLawEvidence | INSUFFICIENT
-> STOP review
-> only then F4
```

Opcode expansion, predictor work, Wave-threshold changes, deployment, and
authority changes remain out of scope until this boundary closes.
