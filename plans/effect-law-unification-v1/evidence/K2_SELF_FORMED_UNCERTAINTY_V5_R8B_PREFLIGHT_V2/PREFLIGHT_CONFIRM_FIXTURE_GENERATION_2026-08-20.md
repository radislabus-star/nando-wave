# R8B Preimplementation Confirm Fixture Generation

Status: `PASS / COMPATIBILITY FIXTURE ONLY / NO R8B OR SCIENTIFIC AUTHORITY`

## Source

```text
source HEAD         bdcae5351c7de75f325b0ebe752804066823cc38
source diff         empty for Cargo.lock, package manifest and product src
Cargo.lock SHA-256  9328508784d6d5a560f8d1b3c4af446c20fb3aac2e556771d93a7d763fd97f08
probe SHA-256       37925c0ce536d34d73bcb3d10be7dfdec67cce9d000c5a4989ebcfa2cc75c714
machine             e@192.168.3.94 / x86_64 / 20 cores
rustc               1.97.1 (8bab26f4f 2026-07-14)
cargo               1.97.1 (c980f4866 2026-06-30)
profile             release / --locked / CARGO_BUILD_JOBS=20
```

The source commit reached the mini-PC through a verified complete Git bundle.
The dirty historical mini-PC checkout was not used. The probe source was
mounted as an untracked workspace example so the checked-in root `Cargo.lock`
remained the dependency authority.

## Result

```text
probe exit                         0
canonical fixture files          41
typed decode-reserialize equal   41 / 41
Confirm owner attempts            0
Development owner attempts        1
sealed attempts                    0
authorization slot claims         0
```

The Confirm fixture consists of one non-sealed deterministic generator request
and response, the complete historical Confirm split receipts and all 35 stored
artifact descriptors. The owner fixture is the historical
Development-shaped `K2UncertaintyConfirmOwnerReceiptV1` with its nested pipe
receipt, one dispatch, no split, no nonce and zero sealed attempts.

`FIXTURE_MANIFEST.json` freezes the typed roots and execution boundary.
`SHA256SUMS` freezes every canonical JSON file. These fixtures prove only the
preimplementation bytes accepted by source `bdcae535...`; they do not prove the
future Development implementation or an R8B result.
