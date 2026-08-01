# Three-Certificate Live Checkpoint

Date: 2026-08-01

## Result

```text
MS4 natural operator
|-- ExecutionCertificate       PASS
|   `-- Product Registry       YES
|-- LawCertificate             PASS
|   `-- Epistemic Registry     YES
|-- MechanismCertificate       COLLECTING / UNRESOLVED
`-- K1 unit eligibility        YES

K1 vocabulary gate             CLOSED
|-- law certificates           1 / 3
|-- semantic laws              1 / 3
|-- role topologies            1 / 2
|-- cleanup receipts           1 / 1
`-- false_bad_apply            0
```

Execution authority and epistemic membership are now independent. The
unresolved Wave mechanism certificate neither revokes CPU execution nor grants
K1 by itself.

## Cleanup Proof

The root-owned cleanup verifier restored and executed the immutable BundleV4
while the learner and transition state were inaccessible to the process.

```text
bundle_id
29eb219540fcdc4d7df3c574dd2ef85bf64971af50f75e838c158f87f473bfc8

receipt file sha256
e78b40ba78b712da78eee2950c11a8f5e3e3018bc9d8b1254905fea047b5548d

receipt root
a5de1874a59cdb914dfb283ee2bf3cded0e05085793bedd3be2aea72e248c61a

standalone restart root
cf7bda4a0e58d1b5543cace69eaea9a907a78e787c8e09b94fe46652c757dc12

verifier TCB root
3b9b6ec3c141d3f751cbf3c915e7d308565dcc50b71f4245046f6be93efeec2b

learner_state_absent             true
raw_evidence_absent              true
exact_example_authority_absent   true
```

## Monotonic Authority

```text
anchor revision                  3
anchor file sha256
18a3cd0a9eb39d2fb78104e95998050136d54b3cd26c041588524a358ba4843c

ledger root
7958b1f0b2f52607f025beb58fe3f6e5eb560e16e6433a58c3275d8aff61c059

journal events                   3
restart parity                   PASS
```

The topology migration event preserved the prior certificates. Safety
revocation remains ordered before topology migration when both are requested
at once.

## Deployment

```text
source commit
91fadee5860e4b50936680d214e663ed6c9c24df

serving binary sha256
5b296b547cd5ffc0e4711dca75cd22398f350f506a423e7eae2375c60e32ccc0

deployment receipt root
7bbf7951df6ee11fd454ec9e0fb3473ff1eb1a091ecb0b86e4650cb76d35cc19

rollback commit
ff07ba9fe05b012bc49557bfdc30e44dcb317f08
```

The hot serving PID `2166527` and Nginx PID `682430` did not change during
deployment or certification restart checks. The local connector PID `982927`
also remained active with zero service restarts.

## Verification

```text
nando-transition-serving tests   205 PASS / 0 FAIL / 9 ignored
Clippy -D warnings               PASS
Graphify update                  PASS
structural route 04              PASS 8 / 8
composite live gate              PASS 4 / 4 structural routes
M3                               WATCH
desktop horizontal overflow      false
mobile horizontal overflow       false
browser errors                   0
```

Exact-package Wave remains a separate collecting experiment. Natural L2 stays
closed until K1 contains at least three semantic laws across at least two
source-neutral role topologies.
