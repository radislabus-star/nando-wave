# K1 Exact Experiment Opportunity V2 - P5 Verification

Date: 2026-08-14

## Verdict

`P5 BOUNDED EXACT-WAKE SCHEDULER: IMPLEMENTED_AND_VERIFIED_OFF_PRODUCTION`

The authority now owns one bounded Queue V4/Freeze V8 decision per cold wake.
It distinguishes inactive writer, active generation, missing evidence, exhausted
novel evidence, research cooldown, frozen candidate, and open K1 vocabulary.
Every no-event decision preserves the signed ledger revision and root.

This receipt grants no deployment authority and makes no Law #2 claim.

## Bound Contract

| Artifact | SHA-256 |
|---|---|
| Execution plan V2 | `0b120cb26a0a377863fca69160567b564596c343e457e895f3a4eccdb5db0155` |
| Plan critique V2 | `3cb5c25c8ab6ba4305c77505dc78c364bf381ebdf744ff4a98e070b0639794f1` |
| P5 implementation preflight | `f11722dbbf66f389ee22c6b66d8bfa28690d67a6934bc2b5c97d3ae32a2bd1ba` |
| P5 preflight receipt | `50c7f64d633c4e730dcf938495e68c6364d576ad55c4db2f976161eb6d5eee48` |
| Observed-source route receipt | `1c2be169201c7278f8b384294cb4644ec2ea8cca91f21a6a21b61475aa943802` |

Implementation base: `6bd4590`.

Preflight verdict: `READY_TO_IMPLEMENT`, zero blockers.

## Implemented Route

```text
cold runtime wake
-> typed exact-wake request
-> authority restores signed scheduler and durable evidence
-> authority rebuilds exact opportunities
-> operator-blind bounded selection
-> zero events or one signed Freeze V8
-> typed exact-wake status
-> durable runtime report
```

The installed policy freezes all limits: one new freeze per wake, 300 seconds
between freezes, 48 freezes per trailing 24 hours, and 256 readiness rows per
wake. Budget state is reconstructed from signed Freeze V8 history after restart;
there is no mutable side counter.

## Verified Boundaries

1. Writer OFF preserves the complete legacy candidate lifecycle.
2. Empty durable evidence returns typed `WaitingForEvidence`, not an operational
   error.
3. Active generation and pending terminal transfer prevent another generation.
4. No evidence, no novel evidence, cooldown, and candidate-ready are ordered and
   distinct.
5. No-event and active-generation wakes preserve ledger bytes, root, and
   revision.
6. A terminal completion cannot open another generation in the same wake.
7. `K1VocabularyOpen` cannot carry a research-cooldown timestamp.
8. The runtime report stores the authority-issued status root; it cannot invent
   candidate counts, blockers, timestamps, or authority.

## Verification

All Rust work ran on the mini-PC with `CARGO_BUILD_JOBS=20` and `-j 20`.

| Check | Result |
|---|---:|
| Focused scheduler suite | 102 PASS, 2 ignored |
| Exact status deadline boundary | PASS |
| Writer OFF and empty-evidence no-event | PASS |
| Active-generation read-only wake | PASS |
| Signed-history budget restart parity | PASS |
| `cargo fmt --all -- --check` | PASS |
| `git diff --check` | PASS |
| Observed-source structural route gate | PASS, 22/22 evidence bindings |

The two ignored tests require explicit production-prefix paths or a private key;
P5 intentionally did not cross that boundary. P7 owns production-copy replay.

## Claim Boundary

```text
P5 bounded exact-wake scheduler    PASS
P6 compatibility and rollback     NOT STARTED
V8 production writer              OFF
Production                        UNTOUCHED
Dashboard                         UNTOUCHED
Synthetic evidence                NONE
Law #2                            NOT PROVED
K1                                1/3
```

The next authorized stage is P6: byte-exact legacy compatibility, isolated V8
reader fixtures, fault injection, and the post-V8 rollback fence. P6 may not
deploy, enable the production writer, update the dashboard, or manufacture
future evidence.
