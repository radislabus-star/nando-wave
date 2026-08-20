# Preimplementation Confirm Fixture Probe

Status: `PREFLIGHT TOOL ONLY / NO SCIENTIFIC EVIDENCE / NO AUTHORITY`

This source must be compiled against the repository workspace and its checked-in
`Cargo.lock`. A standalone Cargo manifest is intentionally absent because an
independent dependency resolution can change canonical JSON bytes.

For the frozen source checkout, copy `src/main.rs` temporarily to:

```text
crates/nando-operator-learning/examples/r8b_preimplementation_confirm_fixture_probe.rs
```

Then run from the repository root with:

```text
CARGO_BUILD_JOBS=20 cargo run --locked --release \
  -p nando-operator-learning \
  --example r8b_preimplementation_confirm_fixture_probe \
  -- <owner-executable> <generator-executable> <development-seed> <fresh-output-root>
```

The example is generation tooling, not product source. It must remain untracked
in the build checkout and must not be included in the implementation commit.
The retained evidence is this probe source plus the resulting canonical bytes,
hash manifest and execution log.

The probe performs no Confirm owner attempt, nonce creation, authorization-slot
claim or sealed execution. It captures:

- a non-sealed Confirm generator/split compatibility fixture;
- a historical Development-shaped `K2UncertaintyConfirmOwnerReceiptV1` and its
  nested pipe receipt.
