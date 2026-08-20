# Preimplementation Development Byte Vector Probe

Status: `PREFLIGHT MIRROR ONLY / NO RUNTIME OR SCIENTIFIC AUTHORITY`

This probe defines paper-only mirror structs from
`development-byte-contract.v1.json`. It verifies that the frozen field types,
field order and nested root tuples compile against the current canonical
serializer before product implementation exists.

Compile it only as a temporary workspace example with the checked-in root
`Cargo.lock`:

```text
cp src/main.rs \
  crates/nando-operator-learning/examples/r8b_preimplementation_development_byte_vector_probe.rs

CARGO_BUILD_JOBS=20 cargo run --locked --release \
  -p nando-operator-learning \
  --example r8b_preimplementation_development_byte_vector_probe \
  -- <historical-pipe-fixture> <fresh-output-root>
```

The temporary example is not product source and must not enter the
implementation commit. Its retained outputs are known-answer vectors only;
they grant no implementation, execution or claim authority.
