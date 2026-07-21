# R4 Clippy Fingerprint Retirement

Status: `BOUNDED_SUBTRACTION_ONLY`

The immutable R0 fingerprint remains unchanged. R4 moved the actor runtime and
VM executor into a clean owner crate, where three pre-existing mechanical
diagnostics were fixed under `-D warnings`:

- one VM needless borrow;
- one runtime needless borrow;
- one runtime unused lifetime.

`RETIRED_CLIPPY_DIAGNOSTICS.tsv` must be an exact sorted subset of the R0
fingerprint. The remote STOP runner subtracts only that declared subset and
still rejects an unknown disappearance or any new diagnostic. The retirement
file and its SHA-256 are recorded in the machine receipt.

This mechanism does not waive compiler, test, or Clippy failures in a new owner
crate. `nando-operator-runtime` must remain fully clean.
