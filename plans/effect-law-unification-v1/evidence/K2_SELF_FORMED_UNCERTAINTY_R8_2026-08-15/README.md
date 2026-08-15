# K2 Self-Formed Uncertainty R8 Evidence

Status: `PASS / AUTHORITY FALSE`

This append-only directory binds the full non-sealed R8 verification of commit
`5d01a893a097270ccbd5b9d9f184c051abfa5ce7`.

```text
release package tests                 465 PASS / 0 FAIL / 8 ignored
library tests                         445 PASS
legacy controls                       32 / 32 PASS
V3 controls                            4 / 4 PASS
V4 controls                           16 / 16 PASS
development cases                    16 / 16 PASS
one-probe / two-probe split             8 / 8
independent final verification       16 / 16 PASS
maximum final request bytes       913356 / 1048576
release process duration             164.26 / 1200 seconds
false accepts                             0
strict Clippy                           PASS
fmt and diff checks                     PASS
structural routes                       PASS
authority                              FALSE
```

`ARTIFACT_SHA256SUMS` covers raw logs and rerun structural receipts. The
receipt records its SHA-256. No production source, service, traffic, dashboard,
K1 state, package, certificate, or sealed input was touched.
