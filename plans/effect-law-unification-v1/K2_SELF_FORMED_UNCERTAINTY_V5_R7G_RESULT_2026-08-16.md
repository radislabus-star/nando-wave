# K2 Self-Formed Uncertainty V5 R7G Result

Status: `R7G PASS / R7G FROZEN / NO NONCE AUTHORITY`

Date: `2026-08-16`

## Result

R7G implements the V5 ownership split without changing the historical
Development wire contract:

```text
Development request
-> exact historical schema
-> shared deterministic generator core
-> byte-identical public and private artifacts

Confirm request
-> separate closed schema
-> disjoint experiment and case domains
-> split-aware public and private artifacts

public coordinator
-> separate self-hashing closure-planner process
-> independent closure verifier
```

The generator process dispatches only after matching the exact request schema.
No network or production callsite was added.

## Development Parity

```text
implementation commit       8f916ddff33180b8c94f6b41c50b06fd67fccdd5
generator tests             3 / 3 PASS
historical request root     c285343b9aa9c5146a1f512cf3c2f412a0e538dd803665429b42035e81430588
historical response root    10264f4a25e3ad22156a30c49dc1f53aee1de5754d6dd0f1b77c54929b3531cf
public batch root           9fbdd35627b2b5265b8e35274412e7dc2a0cce576066022d99b8c67f13b8ad8a
private batch root          5ed5436d0c78e5e62f58bbb4efa34cda73a50af296fdf03dfab16014f1c274e5
denominator root            011ea03be71e80d520101ce2e7b8897be2a2b88e4285d0c8ca905dbb715026dc
response bytes SHA-256      26e509ae09fd68b97323e9e9d0ee1bce9bfee7b0dbd75be5aca2015675027c6e
identity-only rebinding     PASS
```

## Process Verification

All heavy checks ran on the mini-PC checkout
`/home/e/build/nando-wave-k2-self-formed-r2` with twenty Cargo jobs.

```text
release compile --no-run    PASS
case preverification        16 / 16 PASS
case execution              16 / 16 PASS
process test                1 / 1 PASS in 165.08 s
maximum verifier request    913356 / 1048576 bytes
strict Clippy -D warnings   PASS in 27.19 s
```

The Confirm process test used fixed static test material. It did not call a
CSPRNG, claim an authorization slot, dispatch a sealed attempt, or create a
scientific verdict.

Machine-readable receipt:
`evidence/K2_SELF_FORMED_UNCERTAINTY_V5_R7G_2026-08-16/R7G_RECEIPT_V1.json`

Receipt SHA-256:
`c0eb3979d12ab07be218cd290b7aa9cac4e6898ce69d2d1c3ef5a0fbca8abb7c`

## Authority Boundary

```text
Confirm nonce              ABSENT
authorization slot claims  0
sealed attempts            0
production effects         0
Natural K2 authority       false
```

R7G proves split ownership and Development parity only. It does not prove
Natural K2, authorize Confirm execution, mutate K1 or phase memory, activate a
package, deploy production code, or update the dashboard.

The next permitted transition is R7H: global authorization-slot uniqueness,
one-shot nonce and dispatch journaling, and atomic output splitting. R7H must
remain non-sealed and must not create a real nonce.
