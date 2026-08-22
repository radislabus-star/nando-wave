# K2 Self-Formed Uncertainty V5 R8B Contract V8 Critique V2

Status: `SUPERSEDED / REOPENED BY POST-CRITIQUE LIVE PROBE`

Date: `2026-08-21`

Post-critique note: the current user receives `EACCES` for
`/proc/<user-manager-pid>/exe`. The statement below that the running image was
bound was therefore not implementable as written. Contract V8 was repaired and
Critique V3 re-evaluates the replacement; this document retains only the V2
historical review state and grants no gate authority.

## Verdict

All V1 P0 findings are repaired. The revised architecture now separates
expected schedule authority, observed provenance, evidence kinds and resource
measurement. Five P1 ambiguities remain before structural gates.

## Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P1 | The repaired text says M25 requires C08-to-ledger equality without limiting that equality to the C09-C20 downstream projection. C08 is frozen only after M10 and cannot authorize P01, M01/M02 or M03-M10 history. | Literal whole-ledger equality is impossible and could cause C08 to absorb authority it never had. | Define three independent expected-vs-observed projections: producer requests for suites/M24-child, schedule grammar plus validated M04 facts for M10 descendants, and C08 only for C09-C20 downstream invocations. |
| P1 | The producer expected-output row and authority-output descriptor still require an evidence kind, but C08 is now a retained non-evidence child output. | C08 cannot be represented without fake evidence kind 20 or a special untyped escape. | Bind an `object_role` enum and an optional evidence kind. Evidence objects require one exact kind; C08 requires `downstream_invocation_contract` and no evidence kind. |
| P1 | The open ledger lock and journal staging location are not separated from the exact 23-file P06 directory. | A lock file or open-name artifact could become an unexplained packet member, or a cross-filesystem rename could break atomic freeze. | Place open journal and lock in a request-bound sibling staging directory on the same device as P06. Only the sealed ledger is renamed into P06. Lock/staging are retained diagnostics outside the packet and never packet members. |
| P1 | The exact command contract freezes unit creation, but the stop operation lacks an outcome contract and the environment capability assumptions are not mapped to a non-attempt test. | P06 could freeze while the test unit is still active, or implementation readiness could assume unsupported user-service properties. | Require exact stop exit, terminal inactive/not-found state after a previously observed exact unit, no descendant residue, and a separately authorized non-R8B capability/parity test before any R8B execution. Paper preflight may verify property availability but must not launch a capability unit. |
| P1 | Event line size is checked after serialization, but path/schema/validator/fact cardinalities are not frozen at request validation. | An oversized request can durably start and then become impossible to complete within the pair budget. | Freeze field limits and reject the producer request before any ledger event: path <= 240 bytes, schema/validator <= 128 bytes, fixed role/stage enums, roots exactly 64 hex, facts from the closed enum, authority outputs <= 4. |

## Closed V1 Findings

- Packet census is now `19 evidence + C08 + resource + ledger + manifest = 23`.
- C08 is restored as expected downstream contract; ledger is observed provenance.
- Ledger append now has exclusive locking, tail validation and fail-stop.
- M24 child emits four outputs and descriptors are bounded at four.
- S02 is split into ten direct M01, three direct setup M02 and three nested M02.
- The running user-manager process image and exact systemd command are bound.
- M04 facts are closed, typed and explicitly writer-attested.
- Byte parity no longer claims a nonexistent V7 aggregate.
- Exact new paths and dirty index/worktree provenance are required.

## Repair Verification

| Finding | Closed by |
|---|---|
| C08 was compared to too much history. | Three disjoint projections now bind producer plans, dynamic M10 schedule and C09-C20 C08 authority separately. |
| C08 could not fit an evidence-only descriptor. | Closed object roles now carry an optional evidence kind; C08 has a typed non-evidence role. |
| Ledger staging could contaminate P06. | Open journal and lock are same-device sibling diagnostics; only the sealed ledger enters the exact 23-file packet. |
| Unit stop and capability assumptions were open. | Exact stop outcome/residue rules and a non-R8B capability/parity test are mandatory before execution. |
| Request fields could make completion exceed the ledger budget. | Field lengths, enums, roots, facts and descriptor cardinality are rejected before the first event. |

The deterministic contract-consistency check passed with a 14,741,504-byte
ledger headroom. `git diff --check` is clean.

## Historical Next Action

This recommendation was revoked by the live permission probe. Critique V3 is
required before structural worksheets.
