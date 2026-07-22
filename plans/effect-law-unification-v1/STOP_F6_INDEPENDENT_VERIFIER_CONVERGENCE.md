# STOP-F6 Independent Verifier Convergence

Date: 2026-07-22 Europe/Tallinn

Verdict: `COMPLETE / CONTROLLED PROOF PASS`

Authority: `false`

## Signal Boundary

```text
F5 BoundProtocolActionV3 + actor output
-> exact raw provider bytes
-> cold IndependentVerifierArtifactSetV3
-> independent request provenance
-> independent structural roles and capabilities
-> complete bounded candidate paths
-> one physical action-equivalence class
-> independent CALL/COPY reference execution
-> postcondition + output-only preserved frame
-> IndependentVerifierReceiptV3
```

The verifier does not accept actor-selected request text, role mappings,
values, capability names, or expected output as truth. A verified receipt is
opaque, restart-stable, stores no raw payload, and cannot grant execution
authority.

## Proof Results

```text
valid F5 actor/VM handoff                  VERIFIED
renamed physical surface                  VERIFIED
equivalent structural paths               VERIFIED
actor role/value/capability mutation      REJECT
actor output mutation                     REJECT
missing request provenance                REJECT
duplicate physical capability paths       ABSTAIN
multiple physical action classes          ABSTAIN
missing role                              ABSTAIN
unsupported projection/effect             ABSTAIN
budget exhaustion                         ABSTAIN
receipt tamper                            REJECT
receipt restart bytes                     PASS
normal proof -> runtime dependency         ABSENT
production callers                        0
execution authority                       false
```

## Remote Gates

Host: `e@192.168.3.94`, worktree
`/home/e/projects/nando-wave-f6`, `CARGO_INCREMENTAL=0`.

```text
nando-operator-proof unit                 5 PASS
F6 integration                            8 PASS / 1 perf ignored normally
kernel + runtime + proof                  72 PASS / 2 ignored
workspace cargo check --all-targets       PASS
kernel/runtime/proof Clippy -D warnings   PASS
gateway control receipt tests             17 PASS
gateway control Clippy -D warnings        PASS
owner-local NANDA routes                 4/4 PASS
single-owner composite NANDA             VETO (expected owner split)
release performance gate                  PASS
matched p99                               291773 ns
no-match p99                               34659 ns
hard maximum                              354921 ns
samples                                   4096 per route
```

The composite structural packet correctly vetoed treating evidence parsing,
artifact validation, action reconstruction, and authority as one owner. The
four owner-local packets under `f6/` passed with `authority_ready=false`.

Final live inspection remained fail-closed:

```text
composite gate                            PASS
eligible_for_local_accept                 false
response ACTIVE packages                  0
response M3                               WATCH
response false accepts                    0
response runtime parity failures          0
gateway health                            PASS
```

## Honest Boundary

F6 proves the controlled function `CALL` + `COPY`, output-only verifier path.
It does not prove live traffic receipt integration, generation persistence,
support/future ownership, external admission, ACTIVE status, token savings, or
M3. The control-page source is receipt-backed for F6, but its running service
was not restarted. F7 is unlocked but not started.
