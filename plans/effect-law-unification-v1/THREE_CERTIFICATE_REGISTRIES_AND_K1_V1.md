# Three Certificates, Two Registries, One K1 Gate

Status: implemented as a fail-closed cold-path contract. The current MS4
operator remains executable, but is not yet eligible for the epistemic K1
vocabulary because its bundle-bound exact-memory cleanup receipt is missing.

## Immutable Boundary

```text
CrystallizedOperatorBundleV4
  bundle_id remains immutable
          |
          v
signed append-only OperatorCertificationJournalV1
  |- ExecutionCertificate
  |- LawCertificate
  `- MechanismCertificate
          |
          `-> external monotonic signed anchor
          |
          +-> Product Registry projection
          `-> Epistemic Registry projection
```

Certificates are not serialized back into BundleV4. Exact Wave changes from
`COLLECTING` to `PASS` or `FAIL` after the bundle is sealed; embedding that
state would change `bundle_id` and destroy content-addressed identity.

## Independent Meanings

```text
ExecutionCertificate PASS
  external admission + immutable ordinary CPU completion
  + verifier/runtime parity clean
  -> Product Registry

LawCertificate PASS
  frozen unique law + independent future transfer
  + bundle-bound exact-memory cleanup receipt
  -> Epistemic Registry

MechanismCertificate
  WAVE_CAUSAL | STRUCTURAL | UNRESOLVED
  with NOT_EVALUATED | COLLECTING | PASS | FAIL assessment
  -> provenance of discovery, never execution authority
```

`WAVE_CAUSAL_FAIL` does not revoke an otherwise valid execution certificate
and does not erase a separately proven law. It means only that the exact
package holdout did not establish causal necessity of Wave. Operational false
accepts and runtime parity failures remain independent revocation evidence.

## Trust Split

The serving process cannot issue a cleanup receipt or advance certification
history by itself.

```text
unprivileged serving
  -> proposes certificate entry over Unix socket

root-owned certification authority
  -> validates signed cleanup evidence and durable runtime revocations
  -> appends one create-new signed journal event
  -> fsyncs the event and directory
  -> advances a signed anchor outside the transition rollback root

independent cleanup verifier
  -> receives only BundleV4 plus an independent-future challenge
  -> restores the canonical bundle
  -> rebuilds entry Page32 exactly from canonical IR
  -> binds pre-action roles
  -> executes actor and independent verifier
  -> matches the durable actor-response commitment
  -> signs ExactMemoryCleanupReceiptV1 with a separate private key
```

The serving process holds only public keys. Restoring an old journal together
with an old projection cannot pass against the newer external anchor. Missing,
tampered, truncated, reordered, or unsigned history fails closed.

`PASS -> REVOKED` is driven by the package-specific durable runtime revocation
ledger. A late false apply sets `false_bad_apply > 0`, removes Product Registry
and K1 eligibility, and cannot be hidden by the frozen package proof.

Role-topology diversity is the digest of the canonical source-neutral
`RoleGraph`. Tool names, capability names, routing atom IDs, verifier labels,
and package IDs are excluded. A one-time authority-checked migration converts
the pre-anchor capability-derived identity without changing any certificate.

## K1 Vocabulary Gate

An individual K1 unit requires:

```text
ExecutionCertificate PASS
+ LawCertificate PASS
+ false_bad_apply = 0
```

Opening the first K2 grounded-meaning experiment requires the minimum K1 basis:

```text
law certificates       >= 3
distinct semantic laws >= 3
distinct role topology >= 2
cleanup receipts       == law certificates
false_bad_apply        == 0
```

Mechanism classification is recorded but does not block K1. This preserves
the distinction between discovering a real transferable law and proving that
Wave was the necessary discovery mechanism.

This `3 / 3` gate is a minimum experimental seed, not a completed alphabet and
not a cap on K1. Every additional independently certified K1 law can increase
CPU coverage and provide more diverse verified transitions for K2.

K2 receives frozen source-neutral K1 action contracts plus independently
verified decision episodes. Every meaning-eligible episode binds a pre-action
typed goal, constraints, observation mask, applicable K1 action alternatives,
frozen outcome horizon, transition sequence, and independently verified goal
satisfaction. A transition without an honest goal or alternative remains
`DYNAMICS_ONLY`. K2 does not receive a prepared composition DAG, family mapping,
meta-skill name, correct ordering, exact episode identity, tool name, field
name, or latent vector as law identity.

A hidden representation may rank an explicit equivalence or composition
candidate only under
`GROUNDED_MEANING_ARCHITECTURE_V1.md`. It has no execution, certification,
phase-mutation, or admission authority. The resulting K2 candidate must return
to an explicit bounded meta-program, independent future, existing certificate
chain, and external admission.

## Current Honest Projection

```text
MS4 natural package
|- CPU SAFE        PASS
|- LAW PROVED      PARTIAL
|  `- blocker      exact_memory_cleanup_receipt_missing
|- WAVE CAUSAL     COLLECTING / UNRESOLVED
`- K1 ELIGIBLE     NO

legacy ACTIVE packages
|- CPU SAFE        PASS / LEGACY ADMISSION
|- LAW PROVED      LEGACY
|- WAVE CAUSAL     NOT_EVALUATED
`- K1 ELIGIBLE     NO
```

MS5-MS8 remain free to improve product breadth in shadow. They enter K1 only
through the same certificates and diversity gate; generated capability tests
cannot manufacture natural LawCertificate status.
