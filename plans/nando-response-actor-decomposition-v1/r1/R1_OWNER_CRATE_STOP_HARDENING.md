# R1 Owner-Crate STOP Hardening

Status: `PASS`

Date: 2026-07-21

The first `nando-operator-kernel` STOP run exposed two assumptions inherited
from the monolithic response-actor baseline:

1. a successful test log has no `failures:` section;
2. new owner crates must have empty failure and Clippy fingerprints instead of
   matching the response actor's accepted legacy debt.

The runner now treats these cases separately:

```text
nando-response-actor  -> exact frozen R0 failure and Clippy fingerprints
other owner crate     -> zero test failures and zero Clippy diagnostics
```

The Clippy location parser also accepts any `crates/<owner>/src/` path. This is
runner-only hardening: product code, evidence, thresholds, authority, services,
and F5-B remain unchanged.

Proof:

```text
shellcheck ops/dev/nando-remote-gate                 PASS
nando-operator-kernel STOP compile                   PASS
nando-operator-kernel STOP tests                     PASS
nando-operator-kernel STOP Clippy                    PASS
fingerprint_verdict                                  PASS
new_background_build_processes                       0
```
