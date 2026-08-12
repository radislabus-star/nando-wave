# S1C-3D Capture Route Repair Preregistration V1

Status: `PAPER FROZEN / CRITIQUE PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Date: `2026-08-12 Europe/Tallinn`

Parent evidence:

- `S1C3C_CAPTURE_INSTALLATION_TERMINAL_REPORT_2026-08-12.md`;
- `S1C3C_CAPTURE_INSTALLATION_TERMINAL_STATUS.json`;
- postmortem root
  `5daeb142e7b5782d330a6aeca1166afcfae0f96ba00cd163a283bcc1990e60fd`.

## 1. Exact Question

Can the already implemented pre-action decision capture route be installed
without changing action authority after repairing the parity-oracle input
identity and separating performance targets from correctness and safety
vetoes?

S1C-3D is a new prospective engineering repair. It does not reinterpret,
delete, rerun, or promote S1C-3C.

```text
S1C-3C evidence                 immutable terminal RESOURCE_VETO
S1C-3D engineering repair       new identities and receipts
S1C-4 natural census            closed until installation PASS
S2 grounded meaning             blocked until S1C-4 evidence
```

## 2. Failure Decomposition

S1C-3C proved that the transaction, resource measurement, fail-closed stop,
and no-mutation boundary execute. Its three reported failures have different
meanings and must not remain one undifferentiated veto:

```text
parity oracle PermissionDenied
  correctness / evidence-access defect
  hard blocker before installation

settlement p99 5.097076 ms and 6.104611 ms
  performance target deviations
  recorded as OPTIMIZATION_WATCH when hard safety ceilings pass

production mutation false
  rollback and pre-mutation boundary worked
  preserved as positive operational evidence only
```

## 3. Owner Route

```text
authority-owned live registry + admission
-> root reads each artifact once
-> immutable transaction-local parity snapshot
-> source and snapshot SHA-256 equality receipt
-> owner root:e, mode 0440, parent root:e mode 0550 during oracle runs
-> baseline oracle and candidate oracle read the same snapshot
-> exact output rows compared byte for byte
```

The live registry is never made world-readable and its ownership or mode is
not relaxed. The snapshot is bounded to the transaction, contains no request
payload, and is removed with the transaction work directory after receipts are
sealed. Both oracles receive the exact same path and bytes.

Snapshot requirements:

```text
registry source root == registry snapshot root
admission source root == admission snapshot root
snapshot owner         root:e
snapshot mode          0440
snapshot directory     root:e 0550
oracle return code     0 / 0
oracle row count       16 / 16
payload rows           byte-identical
```

Any source/snapshot mismatch, writable snapshot, nonzero oracle exit, malformed
row, or payload mismatch is a hard correctness veto.

## 4. Three Verdict Axes

The repair reports three independent classes.

### Correctness

Hard requirements:

- parity snapshot binding and byte identity pass;
- journal recovery and exact record counts pass;
- candidate, source, executable, config, and receipt identities match;
- final verifier independently reproduces every bound root;
- capture-disabled actor/runtime parity remains exact;
- `false_accepts = 0` and runtime parity failures remain zero.

### Operational Safety

Hard requirements:

- hot capture-disabled budgets remain unchanged;
- every durability operation has `hard_max <= 20,000,000 ns`;
- RSS delta remains at most 16 MiB;
- idle CPU remains at most 0.25% of one core;
- services stay active with stable identities except the preregistered
  transition-serving replacement;
- connector PID/restarts survive and route receipt failures remain zero;
- rollback restores exact prior binary/config and restart parity.

### Optimization

Targets, not authority vetoes:

```text
single-ledger p99       target <= 5,000,000 ns
precommit p99           target <= 5,000,000 ns
settlement p99          target <= 5,000,000 ns
```

A target deviation is emitted as `OPTIMIZATION_WATCH` with the exact value and
ratio. It does not override a correctness or safety failure, and it cannot be
silently omitted. It may coexist with installation PASS when all hard gates
pass. Optimization remains a later product task with the same measurements as
its baseline.

The inherited candidate test binary may exit `101` because its frozen test
asserts the old 5 ms target. S1C-3D may classify that exact exit as
`legacy_target_assertion` only when:

- the metric line is complete;
- denominator and CPU affinity are exact;
- at least one named target p99 exceeds 5 ms;
- every corresponding hard maximum is at most 20 ms;
- the content-addressed test executable and source assertion are the frozen
  S1C-3C identities.

Every other nonzero exit remains an instrument failure.

## 5. Frozen Production Boundary

Allowed mutation after all predeployment gates pass:

```text
transition-serving binary
transition-serving capture config
transition-serving intentional restart
S1C capture journal root
deployment and rollback receipts
control-plane terminal/live projection
```

Forbidden:

```text
Nginx restart
connector restart
response package or admission mutation
K1 certification mutation
phase memory mutation
synthetic or targeted natural evidence
retroactive S1C-4 rows
model training
K2 authority
```

The production transaction must be crash-atomic and rollback-armed before the
first installed byte changes.

## 6. Attempt And Repair Discipline

S1C-3D is an engineering installation route, not an independent scientific
future. It therefore uses immutable repair epochs instead of pretending that
one infrastructure defect closes the architecture forever.

For each epoch:

```text
paper/root revision
-> committed implementation and verifier identities
-> local schema and negative tests
-> pushed source freeze
-> at most one production mutation attempt for that identity
-> append-only terminal receipt
```

A correctness or safety failure consumes that identity. A later repair must
use a new source commit, root, and transaction ID and preserve all prior
receipts. Performance `OPTIMIZATION_WATCH` alone does not consume or veto an
otherwise passing installation.

## 7. Required Tests

Before production mutation:

- parity snapshot and its directory deny chmod, write, unlink, and rename to
  the oracle user;
- source/snapshot hash mismatch is rejected;
- registry and admission snapshot swap is rejected;
- nonzero oracle exit and unequal rows are rejected;
- `5.097076 ms` and `6.104611 ms` classify as optimization watches;
- any hard maximum above 20 ms classifies as safety veto;
- unrelated test panic remains instrument veto;
- missing metric, denominator, affinity, or monitor row fails closed;
- predeployment local and remote verifier outputs are byte-identical;
- rollback interruption tests pass at every mutable state.

## 8. Result Matrix

```text
correctness FAIL
-> CORRECTNESS_VETO
-> no mutation or exact rollback

operational safety FAIL
-> SAFETY_VETO
-> no mutation or exact rollback

correctness PASS + safety PASS + optimization WATCH
-> DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH
-> capture installed
-> S1C-4 opens COLLECTING at a new append cursor

all three PASS
-> DEPLOYMENT_PASS
-> capture installed
-> S1C-4 opens COLLECTING at a new append cursor
```

Neither deployment verdict proves that natural goals, alternatives, decision
episodes, grounded meaning, or K2 exist.

## 9. Exit Evidence

Required final packet:

- paper, critique, implementation, executable, parity-input, resource,
  deployment, rollback, connector, and service roots;
- exact target deviations and hard-gate values;
- installed/release hashes and service identities;
- post-restart journal projection parity;
- live dashboard projection;
- explicit `scientific_authority=false`, `model_training=false`, and
  `phase_mutation=false`.

## 10. Structural Paper Gate

Four owner-local routes passed after two initial owner-conflict VETOs were
repaired by separating snapshot creation from parity verdict ownership and
predeployment mutation authority from post-install census ownership:

```text
repair identity             PASS
parity snapshot             PASS
target versus safety        PASS
installation authority      PASS
WATCH                       none
repair queue                empty
authority_ready             false
```

Receipts are stored under
`evidence/S1C3D_CAPTURE_ROUTE_REPAIR_PAPER_V1/`. Structural PASS authorizes
implementation inside this contract only. It does not authorize production
mutation.
