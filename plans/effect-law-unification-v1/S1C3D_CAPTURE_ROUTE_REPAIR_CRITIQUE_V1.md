# S1C-3D Capture Route Repair Critique V1

Status: `ADVERSARIAL REVIEW PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Reviewed artifact: `S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md`

## Findings And Repairs

| Priority | Finding | Risk | Applied repair |
|---|---|---|---|
| P0 | Calling S1C-3D a rerun could erase S1C-3C. | Post-result relabelling would corrupt the evidence history. | S1C-3C roots and terminal status remain immutable parents; every S1C-3D epoch has new identities and an append-only receipt. |
| P0 | Making the live registry broadly readable would fix the symptom by weakening authority isolation. | Unrelated processes could read or replace authority data. | Root creates a bounded transaction-local `root:e 0440` snapshot inside a `root:e 0550` directory; live ownership and mode remain unchanged. |
| P0 | An `e:0400` snapshot would be writable after an owner-controlled chmod or replaceable through an e-writable directory. | The oracle identity could mutate its own purportedly frozen input. | Root retains file and directory ownership; group `e` receives read and traverse only, while negative tests require chmod, write, unlink, and rename denial. |
| P0 | Snapshot bytes could drift from the authority source. | Both oracles could agree on the wrong registry. | Receipt binds source and snapshot SHA-256 for registry and admission; mismatch is a hard veto. |
| P0 | Treating every latency target as advisory could admit an unbounded hot-path stall. | A working route could still damage production. | The 5 ms p99 values are optimization targets, while 20 ms per-operation hard maxima, hot disabled-path budgets, health, and rollback remain hard safety gates. |
| P0 | Ignoring return code 101 could hide a real panic. | A correctness failure might be laundered as latency noise. | Only the frozen legacy 5 ms assertion may map to `legacy_target_assertion`, and only with complete metrics, exact executable identity, a target deviation, and all hard maxima passing. |
| P0 | Installation PASS could be reported as grounded meaning. | Infrastructure availability could be mistaken for K2 evidence. | S1C-4 starts at `COLLECTING` with a post-install cursor; scientific authority, training, and phase mutation remain false. |
| P1 | Unlimited retries could select a lucky performance run. | Optional stopping could manufacture a latency PASS. | Performance is reported on every epoch and never needed for a scientific claim. Each immutable candidate gets at most one production mutation attempt; repairs require new identities and preserve failures. |
| P1 | A transaction could mutate before parity verification. | Rollback could become the first evidence that parity was broken. | Local and remote predeployment verification must match byte-for-byte before rollback is armed and before any installed byte changes. |
| P1 | A passing snapshot parity could still differ from live runtime authority. | Fixture parity would not prove production identity. | Snapshot source roots bind the authority-owned live artifacts, and post-restart runtime parity/false-accept gates remain independently required. |
| P1 | The route could restart Nginx or the connector for convenience. | User traffic and evidence continuity would be disrupted. | Both are forbidden mutations; identity drift is a safety veto. |

## Rejected Alternatives

```text
change the S1C-3C receipt or verifier
  rejected: rewrites a consumed experiment

chmod the live registry 0644
  rejected: weakens the authority boundary

raise every latency threshold until PASS
  rejected: hides measurements instead of separating target and safety

drop latency from the receipt
  rejected: prevents later optimization and masks regressions

start S1C-4 from archived transitions
  rejected: creates retroactive pre-action evidence
```

## Verdict

The repair is coherent if implementation preserves all four distinctions:

```text
old evidence vs new repair
live authority artifact vs read-only parity snapshot
performance target vs hard operational safety
capture installation vs scientific meaning
```

Structural coherence can authorize implementation of the repair tools. It
cannot authorize production mutation; that authority comes only from the
frozen predeployment verifier packet.

The first parity and authority worksheets returned `VETO` because each group
mixed two decision owners. The final packets split snapshot creation from
parity adjudication and predeployment mutation from post-install census entry.
All four final packets pass without `WATCH`, conflicts, foreign pull, negative
hits, or repair items; `authority_ready` remains false.
