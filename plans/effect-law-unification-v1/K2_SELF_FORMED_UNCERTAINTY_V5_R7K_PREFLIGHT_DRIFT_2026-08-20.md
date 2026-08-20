# K2 Self-Formed Uncertainty V5 R7K Preflight Drift

Status: `STOPPED BEFORE R7K RESULT / V3 PRESERVATION MISMATCH`

Date: `2026-08-20`

## Finding

R7K V3 preflight bound the R7J integration test at:

```text
path      crates/nando-operator-learning/tests/
          k2_self_formed_uncertainty_confirm_r7j_v1.rs
SHA-256   9a549403805a1b1a34e675dd5fe4a73515d94b00b86eeca5b403a34e707a747b
bytes     46,013
policy    immutable_predecessor_test
```

The R7K prototype changes that harness to export already validated R7J
Development receipts when an explicit persistence environment variable is
present. The current file is 49,850 bytes with SHA-256
`94461707b09e1bb81118fb3d3dad4591a25bc46c19d3502781f98dff2e371bfc`.

The R7J evaluator, terminal evaluator and normal no-export decision path remain
unchanged, but exact byte preservation is false. V3 therefore cannot authorize
an R7K component PASS.

## Additional Gap

The exporter writes a hash manifest, but the R7K integration prototype decodes
the packet without first validating every manifested path and byte hash. A
partially stale or substituted packet could reach schema validation before the
transport discrepancy is named.

## Stop Boundary

Until V4 passes:

```text
R7K component result              forbidden
R8B unlock                        forbidden
sealed attempt                    forbidden
scientific verdict                forbidden
deployment                        forbidden
production effect                forbidden
```

The existing prototype and completed test runs are diagnostic evidence only.
They are not an R7K result receipt.

## Required Repair

1. Freeze an explicit transport-only R7J export contract.
2. Keep the default R7J path byte-for-byte free of persistent side effects.
3. Require a fresh empty export root with mode `0700`.
4. Publish exported files as `0400` plus a closed hash manifest.
5. Validate the complete manifest before any R7K packet decode.
6. Prohibit exported R7J rows from satisfying K1-K12 process outcomes.
7. Re-run R7J both without export and with export followed by R7K.
8. Create and pass a new implementation-preflight revision before the R7K
   result is published.
