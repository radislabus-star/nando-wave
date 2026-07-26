# V4 Live Rollout Receipt

Status: `PASS`

## Source

- Source commit: `f90771d`
- Registry schema: `nando.response-registry.v6`
- Registry revision: `1156666783133460436`
- Canonical registry digest: `3e5874df2a7836f3816583925c23cc209546168361f34a4191f2307a8f32c0b0`
- Rollback root: `/var/lib/nando-wave/transition/deploy-backups/v4-20260726-042416-8795071`

## Deployed Binaries

| Artifact | SHA-256 |
|---|---|
| `nando-transition-serving` | `4f28f084ae4821840a7c8870829f8bda60681fc920bdb0d97cf7bf00e08b99bb` |
| `nando-response-admission` | `777b3133f316ecdb349d812aa966af3bfa07a9aba841708f2751f656d4a1de02` |
| `nando-response-authority-inspect` | `e2741b158edea12a60f3eff56612f76d81ebbb29d4dfc76f3069e6482b81cb29` |
| `nando-live-transition-gate` | `480ba8a8dd5f8dd7ec6ae460b81f03a3fc5979a7af840e9cc8472ba0ed4dc538` |

## Immutable V4 Generation

| Package | Canonical bundle ID | V4 CBOR bytes |
|---|---|---:|
| `crystallized-scalar-a40ca964f20c4b67` | `ff24b6894a14ca2532f377ef01fdf0b4431512ee9c151f13f4e43d7c3a0d24cd` | 14498 |
| `crystallized-scalar-db6407c627296d8b` | `46e9981d423c8ce863d8fa17c766e1ed7156691020b137ad29a8e650eb5ff773` | 14437 |
| `crystallized-scalar-e35441ca7863327d` | `bee8fa2de7c7721369d9af364d46b606bf38438011c1b57c087369a83cebed12` | 14438 |

All three registry entries contain `canonical_bundle_id`,
`crystallized_bundle_v4_cbor`, the compatibility entry page, and the
compatibility runtime registry.

## Live Proof

The final composite gate reported:

- verdict: `PASS`
- local accept eligible: `true`
- ACTIVE packages: `3`
- false accepts: `0`
- runtime parity failures: `0`
- M3: `WATCH`

Hot serving reported:

- mode: `CPU`
- admission fresh: `true`
- response executor cache ready: `true`
- loaded response registry revision: `1156666783133460436`

During one 20-second ordinary-traffic interval:

- CPU accepts: `7 -> 16`
- accepted input tokens: `640953 -> 1345112`
- interval delta: `9` accepts and `704159` input tokens
- ordinary package exercised:
  `crystallized-scalar-a40ca964f20c4b67`
- false accepts: `0`
- runtime parity failures: `0`

This is live CPU execution from the newly published V4 generation, not a
fixture or shadow-only replay.

## Deployment Repair

The first rollout stayed fail-closed because the installed
`nando-response-authority-inspect` predated Bundle V4. It decoded the new
registry through the old schema and produced a digest that omitted V4 fields.
The new serving binary rejected that authority with
`response_authority_registry_digest_mismatch`.

The inspector was rebuilt from the V4 source commit and installed as part of
the same deployment set. The gate script was also made independent of a
systemd `HOME` environment. After reissuing the composite admission lease,
the inspector, controller, gate, and serving runtime agreed on the canonical
registry digest.

## Boundary

- Bundle V4 production generation: `DEPLOYED`
- Live route, grounding, execution, verifier, and CPU accept: `PASS`
- Independent transfer to a second physical server: `NOT_EVALUATED`
- M3: `WATCH`
