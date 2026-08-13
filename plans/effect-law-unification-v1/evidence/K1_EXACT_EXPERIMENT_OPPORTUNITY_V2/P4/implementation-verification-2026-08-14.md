# K1 Exact Experiment Opportunity V2 - P4 Verification

Date: 2026-08-14

## Verdict

`P4 DURABLE AUTHORITY AND PERSISTENCE: IMPLEMENTED_AND_VERIFIED_OFF_PRODUCTION`

P4 satisfies the frozen objective: client proposals are non-authoritative, an
exact attempt is reconstructed from authority-owned immutable inputs, and a
completed deterministic attempt survives crash and restart as a pure signed
event projection.

This receipt grants no deployment authority and makes no Law #2 claim.

## Bound Contract

| Artifact | SHA-256 |
|---|---|
| Execution plan V2 | `0b120cb26a0a377863fca69160567b564596c343e457e895f3a4eccdb5db0155` |
| Plan critique V2 | `3cb5c25c8ab6ba4305c77505dc78c364bf381ebdf744ff4a98e070b0639794f1` |
| P4 implementation preflight V2 | `e89fd96cfbb0d36c5e0358db7f54051419f9b97f307bb5b5757a288c635ee31d` |
| P4 preflight receipt | `d0fb6e01b21dd57b147f76b8800eb9606ddbc1c57eb466e25ff0536e804248f7` |
| Observed-source route receipt | `5296b25e055940653e0e848bc6a9e2290cc460a485ffb6f486b565464b6631dc` |

Implementation base: `8b90bc8137c7f738a621b344f4973a158f0a1bfe`.

Preflight verdict: `READY_TO_IMPLEMENT`, zero blockers.

## Implemented Route

```text
authority source snapshot
-> exact support and artifact projection
-> authority-owned Queue V4 and Freeze V8
-> immutable object and manifest publication
-> authority-owned exact identifier rerun
-> TerminalDiagnosticV1
-> matching terminal verdict
-> signed ExactAttemptIndexV1 projection
-> restart recovery
```

The mechanism and epistemic ledgers remain separate. A clean state creates
independent genesis anchors for both lanes. The final freeze CAS rereads policy,
registry, active protocol modes, and durable source roots before append.

## Critique Repairs

1. Clean-start recovery now creates the epistemic genesis anchor unconditionally.
2. Final wake CAS rereads installed policy and the active protocol-mode root.
3. Exact terminal mutation is restricted to the epistemic lane.
4. Signed verdict replay requires Freeze V8 and the exact `IdentifierResultRoot`.
5. A direct policy-change fault test proves `STALE_BEFORE_FREEZE`, revision zero,
   and zero scheduler events after a valid ON-to-OFF race.

## Verification

All Rust work ran on the mini-PC with `CARGO_BUILD_JOBS=20` and `-j 20`.

| Check | Result |
|---|---:|
| `nando-operator-learning` tests | 433 PASS |
| `nando-response-actor` tests | 386 PASS, 2 ignored |
| `nando-transition-serving` tests | 345 PASS, 11 ignored |
| Focused scheduler suite | 49 PASS |
| Policy/source CAS fault test | PASS |
| Strict Clippy for all three crates | PASS |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |
| Observed-source structural route gate | PASS, 14/14 evidence bindings |

The structural receipt confirms four separate routes: terminal execution,
terminal authority, restart execution, and restart proof. It does not by itself
claim runtime or scientific correctness; the mapped tests above close the P4
implementation gate.

## Claim Boundary

```text
P4 durable authority machinery     PASS
P5 bounded scheduler               NOT STARTED
V8 production writer               OFF
Production                         UNTOUCHED
Dashboard                          UNTOUCHED
Synthetic evidence                 NONE
Law #2                             NOT PROVED
K1                                 1/3
```

The next authorized stage is P5: scheduler policy and bounded work. It must not
deploy P4, update the dashboard, issue a LawCertificate, or manufacture future
evidence.
