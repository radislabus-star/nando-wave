# P8 Independent Implementation Critique

Date: 2026-08-14

Verdict: `PASS`, with one explicitly narrowed performance claim.

## Seven Required Questions

| Question | Answer | Evidence |
|---|---|---|
| Can a coarse group suppress a new root? | No. Legacy family fields remain decode-only/zero for V8, selection authority rejects the coarse queue schema, and exact attempt state is keyed by `OpportunityRoot`. | `selection_authority.rs:20`, `queue.rs:382`, `selection.rs:542` |
| Can receipt-only metadata perturb Raw Phase? | No. V8 resolves the identification domain from the validated causal manifest's `OpportunityRoot`; generation, queue, score, and time remain outside that root. | `identification.rs:614`, `opportunity.rs:482`, OpportunityRoot metamorphism tests |
| Can a client forge a diagnostic or terminal disposition? | No. Generic append rejects diagnostics and exact deterministic terminals. Authority restores frozen inputs, reruns the evaluator, owns the timestamp, and seals both events. | `authority.rs:226`, `authority.rs:1091`, authority negative tests |
| Can an operational failure mark an attempt complete? | No. Attempt projection requires V8, `AcquisitionFail`, a deterministic allowlisted blocker, a deterministic disposition, and exact diagnostic/verdict evidence roots. | `state.rs:542`, `projection.rs:190`, disposition tests |
| Can a crash lose diagnosis or duplicate a terminal? | No within the signed transaction model. A durable diagnostic is reused byte-identically after restart; only the missing verdict is appended. Duplicate completion returns the existing projection. | `authority.rs:324`, `authority.rs:340`, `diagnostic_crash_retry_appends_only_one_matching_verdict` |
| Can a pre-V8 binary be selected after a V8 suffix exists? | No. Any V8 candidate event forces the minimum reader schema to V8 and rejects a pre-V8 rollback reader. | `authority.rs:1051`, `first_v8_suffix_permanently_fences_pre_phase_a_readers` |
| Can the dashboard imply Law #2? | Not from this backend. Replay seals `law_2_proved = false`, K1 remains ledger-derived, and quality remains `UNKNOWN`. P12 must preserve these fields byte-for-byte in HTML. | `k1_exact_opportunity_replay.rs:195`, `k1_exact_opportunity_replay.rs:198`, P12 display contract |

## P7 Scaling Critique

The execution-plan phrase `10x concatenated copy` and the accepted P7 preflight
are not identical experiments. P7 ran ten independent processes over the same
immutable 1x bytes. It therefore measures aggregate system work, contention,
per-process RSS, and deterministic output over a 10x aggregate denominator. It
does not prove single-process input-size complexity on a physically concatenated
10x state.

The release claim is narrowed accordingly:

```text
system-level 10x aggregate replay             MEASURED
ten identical semantic roots                  PASS
current production-copy resource bound        PASS
single-process 1x -> 10x input complexity     NOT MEASURED
```

This limitation does not weaken authority, false-accept, parity, wire, or
current production-copy bounds. No algorithmic-complexity claim is used to
assert Law #2, K1 progress, or answer quality.

## Decision

No unresolved authority or correctness issue remains in the P1-P7 backend
scope. P8 authorizes P9 source fixation and push. Deployment still requires the
separate Phase A and Phase B transactions.
