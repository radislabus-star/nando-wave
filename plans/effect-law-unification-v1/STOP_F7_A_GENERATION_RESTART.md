# STOP-F7-A Generation Restart

Date: `2026-07-22`

Verdict: `COMPLETE_CONTROLLED_PROOF_PASS`

Authority: `false`

## Result

```text
F6 executable artifact set
        |
        v
kernel-owned artifact-set digest        PASS
        |
        v
CanonicalGenerationId                   PASS
  sequence + parent + seven roots
        |
        v
canonical restart bundle                PASS
  manifest + sorted artifacts only
        |
        v
decode + artifact validation            PASS
        |
        v
dispatch rebuilt from artifacts         PASS
        |
        v
artifact/index root convergence         PASS
        |
        v
byte-identical restart                  PASS
        |
        v
generation-owned evidence ledger        F7-B / NOT STARTED
        |
        v
generation-bound F6 receipt envelope    F7-C / NOT STARTED
        |
        v
admission                               F8 / BLOCKED
```

F7-A creates no second generation key. Kernel, runtime and proof converge on
the exact same artifact-set root. Restore never trusts a persisted dispatch
index; it rebuilds the index from canonical artifacts and checks it against the
manifest.

## Proven Boundary

- `32` artifacts maximum per generation.
- `512 KiB` restart-bundle hard ceiling.
- `8 KiB` manifest hard ceiling.
- Artifact order cannot change the bundle.
- Every committed component root changes the generation ID.
- Sequence one has no parent; later generations require a valid parent root.
- Tampered, truncated, duplicate and oversized bundles fail closed.
- The old generation remains byte-identical after a child is created.
- Raw request, response, teacher and episodic payloads are not persisted.
- Restored generations expose `execution_authority=false`.

The actor, renderer, verifier, capability and budget roots are committed but
not yet independently derived by their live owners. Their convergence is an
F7-E obligation, not an F7-A claim.

## Verification

```text
kernel generation tests       3 / 3 PASS
restart integration tests     2 / 2 PASS
kernel/runtime/proof baseline 77 PASS / 2 ignored
Clippy -D warnings            PASS
F7 changed-file rustfmt       PASS
git diff --check              PASS
NANDA owner-local routes      3 / 3 PASS
NANDA authority_ready         false
production callers            0
services restarted            NO
deployment changed            NO
```

Workspace-wide `cargo fmt --all --check` still reports the pre-existing
unrelated formatting debt in `crates/nando-core/src/wave.rs`; every F7-owned
Rust file passes `rustfmt --check` independently.

The live production cohort is separate from this controlled proof. During the
closeout snapshot it advanced from `23/32` to `24/32` future receipts without
an F7 deployment; admission correctly remained blocked.

Next boundary: F7-B generation-owned support/future ledger.
