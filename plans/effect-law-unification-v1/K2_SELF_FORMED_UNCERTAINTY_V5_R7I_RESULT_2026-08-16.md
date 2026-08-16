# K2 Self-Formed Uncertainty V5 R7I Result

Status: `R7I PASS / COMPONENT ROUTE FROZEN / NO SCIENTIFIC ATTEMPT`

Date: `2026-08-16`

## Result

R7I now has executable process ownership for the public preparation and private
effect boundary:

```text
public batch + denominator commitment
-> application-owned public coordinator
-> sixteen complete public frontiers and frozen one-or-two-probe plans
-> final-verifier material present before private execution
-> ALL_CASES_PRECOMMITTED with zero private mounts
-> one resolver table + one frozen ordinal
-> application-owned private resolver emits exactly one effect
-> independent safety binding
-> durable dispatch
-> isolated worker
-> read-only observer with exact parity
-> frozen observation vector
-> final truth mounted read-only only for independent final verification
```

The first diagnostic exposed one concrete packaging defect: the final verifier
received the two `final-v2` material files but not the frozen frontier artifacts
they referenced. The repair publishes the content-bound final material inside
each public probe evidence root before `ALL_CASES_PRECOMMITTED`. The protocol
limit remained 1 MiB and verifier semantics were not weakened.

Implementation commit:
`3147df774e154e815a435f56658d1c841d841bc7`

Implementation tree:
`d0577a57f2b377d33ffb5eebf1e80bf50aa0d020`

## Verification

All compiled checks ran on the isolated mini-PC checkout
`/home/e/build/nando-wave-k2-self-formed-r7i-final` with twenty Cargo jobs.

```text
fresh R7I release route                 2 / 2 PASS in 311.20 s
fresh R7I cases                       16 / 16 PASS
one-probe / two-probe plans             8 / 8
legacy V4 process regression            1 / 1 PASS in 231.04 s
R7H authorization and restart parity    9 / 9 PASS
R7G generator parity                    3 / 3 PASS
library tests                         448 / 448 PASS
all-target Cargo check                         PASS
strict Clippy -D warnings                     PASS
cargo fmt --check                            PASS
git diff --check                             PASS
NANDA structural worksheets              2 / 2 PASS
observed-source route evidence          23 / 23 PASS
false accepts                                  0
```

## Scope Boundary

```text
changed crate files                         14
largest new file                         621 lines
files outside nando-operator-learning        0
network, serving, K1 or dashboard calls       0
production deployment effects                 0
natural traffic reads or writes               0
```

The application-owned public coordinator and private resolver are real
executables. The complete R7I DevelopmentRehearsal sequence is still driven by
the integration harness. This is an explicit boundary, not a hidden PASS: R8B
and R9B must run the exact route through the full confirm-owner dry-run before
new R10 authorization can exist.

## Authority Boundary

```text
real authorization slot claims   0
real Confirm nonce               ABSENT
sealed scientific attempts       0 / 1
scientific verdict               ABSENT
Natural K2 authority             false
deployment authority             false
```

R7I proves the component process boundary and complete non-scientific rehearsal
route. It does not prove the scientific hypothesis, Natural K2, independent
custody, or product value.

Machine-readable receipt:
`evidence/K2_SELF_FORMED_UNCERTAINTY_V5_R7I_2026-08-16/R7I_RECEIPT_V1.json`

Receipt SHA-256:
`0210b211cefdf3b1180757b6d8ba060420a21986f7660d18f6f0b225e50570f5`

The next permitted stage is R7J: bounded oracle and baseline owners, V5 control
evaluation, and the independent terminal evaluator. R7K remains locked until
R7J passes.
