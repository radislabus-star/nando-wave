# K2 Self-Formed Uncertainty V5 R7K Implementation Discrepancy

Status: `STOPPED BEFORE CODE / V2 PROCESS ADAPTER IMPOSSIBLE AS WRITTEN`

Date: `2026-08-20`

## Finding

R7K Contract V2 requires the non-authoritative control-case adapter to run as a
mode of the Cargo integration-test executable while R7J requires the measured
child stdout to be exactly one canonical `K2UncertaintyControlStdoutV1`.

The standard Rust test harness writes its own framing and terminal output. A
parent could extract the embedded JSON, but those extracted bytes would not be
the measured child stdout committed by `K2UncertaintyControlProcessOutcomeV1`.
That would recreate synthetic process evidence.

## Repair

Use one dedicated authority-denied test-support binary with a closed K1-K12
request schema. It calls the actual target library API, accepts only one exact
typed rejection per frozen subcase and writes only the canonical two-field
stdout. It has no aggregate control, terminal, cleanup, result or scientific
authority.

The four R7K owner wrappers remain exactly:

```text
cleanup authorizer
cleanup owner
cleanup verifier
result publisher
```

The adapter is separately counted as one test-support process artifact and is
forbidden from appearing in a sealed executable manifest. Contract V3 and a new
preflight revision are required before code.
