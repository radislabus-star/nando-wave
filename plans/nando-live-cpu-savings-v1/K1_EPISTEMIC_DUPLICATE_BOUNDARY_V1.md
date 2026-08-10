# K1 Epistemic Duplicate Boundary V1

## Decision

K1 discovery must suppress protocol modes that already have a valid
`LawCertificate` in the Epistemic Registry. It must not suppress a mode merely
because an execution-only package is ACTIVE in the Product Registry.

The serialized V1 field name `active_protocol_mode_set_root_sha256` is retained
for historical compatibility, but new freezes bind a versioned epistemic-known
mode-set root and a new discovery-basis root.

## Affected Signal Path

```text
natural topology cohort
-> operator-blind readiness and queue
-> immutable candidate freeze
-> source-neutral candidate generation
-> exact identifier
-> known-law duplicate veto
   -> Epistemic Registry LawCertificate modes only
-> version space / semantic quotient
-> independent future
-> LawCertificate
```

The live blocker before this change is:

```text
37 readiness-PASS cohorts
-> identifier receives two ACTIVE Product Registry modes
   |- wait: LawCertificate PASS
   `- write_stdin: legacy execution package, no LawCertificate
-> both are treated as already known
-> all_supported_t1_protocol_modes_already_active
-> Law #2 cannot be evaluated
```

This violates the established two-registry boundary. Product execution proof
does not imply epistemic law proof.

## Live Baseline

Captured on the mini-PC before the change at dashboard source timestamp
`1786341585`:

| Measure | Value |
| --- | ---: |
| Lifetime input / CPU tokens | `8,295,601,162 / 264,569,607` |
| Current epoch input / CPU tokens | `2,333,820,438 / 222,054,310` |
| Miner seen / recognized tokens | `11,309,678,823 / 1,656,241,203` |
| Catalog / readiness-PASS cohorts | `746 / 37` |
| Ready after completed exclusions | `11` |
| Completed scheduler generations | `110` |
| ACTIVE Product packages | `2` |
| Epistemic LawCertificates | `1` |
| False accepts / parity failures | `0 / 0` |
| Structural / opportunity pending | `0 / 0` |
| Transition state disk | `34G` |

The repeated duplicate terminal loop was also consuming material CPU while no
new law could be evaluated. This is an authority error, not missing evidence.

## Versioning

- Freeze V1-V4 roots and replay semantics remain unchanged.
- Discovery basis V1-V2 remain unchanged.
- New freezes use Freeze V5 and Discovery Basis V3.
- The new basis binds the epistemic-known protocol-mode-set schema.
- Historical duplicate verdicts may reopen only under the new discovery basis;
  no historical verdict or root is rewritten.

## Authority Boundary

The certification authority reconstructs the known set from both durable
sources:

```text
anchored certification ledger
-> latest entries where epistemic_registry_member == true
-> package_id join against validated response registry
-> canonical protocol-mode roots
-> epistemic-known mode-set root
```

The freeze CAS and the learner must independently derive the same root. Missing,
invalid, or unbound registry data fails closed.

## Non-Goals

- Do not grant a LawCertificate to the legacy `write_stdin` package.
- Do not synthesize natural evidence or independent future.
- Do not change scheduler scoring, readiness thresholds, capture, Wave phase,
  package activation, or CPU admission.
- Do not restart Nginx or the connector.

## Expected Live Transition

```text
legacy write_stdin Product package
-> no longer classified as an epistemically known law
-> natural write_stdin cohort may enter exact identification
-> PASS / PROBE / ABSTAIN / ACQUISITION_FAIL
```

Only a genuine independent future and the existing certification path may
produce LawCertificate #2.
