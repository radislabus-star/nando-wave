# Nando Remote Development Gate

`nando-remote-gate` runs decomposition checks on the 20-thread mini-PC while
leaving local live services untouched.

```bash
ops/dev/nando-remote-gate fast \
  --filter effect_law_v3 \
  --scope crates/nando-response-actor/src/effect_law_v3.rs

ops/dev/nando-remote-gate stop \
  --json-out plans/nando-response-actor-decomposition-v1/r1/STOP_R1_RUN.json \
  --graphify

ops/dev/nando-remote-gate release
```

The remote clone is reset to exact local `HEAD` before every run. A dirty file
is transferred only when its repository-relative path is explicitly passed as
`--scope`; this prevents unrelated local work from contaminating proof runs.

Profiles:

```text
fast     bounded target-dev, incremental, direct filtered test-binary runs
stop     clean target-proof, full lib + Clippy fingerprint comparison
release  non-incremental release test build and binary inventory
```

The runner closes its SSH ControlMaster, waits for every Cargo/Rustc child, and
fails if it leaves a new remote build process. It never starts a daemon or
touches local systemd services.
