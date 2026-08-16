# K2 Self-Formed Uncertainty R10 Pre-Attempt Discrepancy

Status: `P0 BLOCKED_BEFORE_NONCE / ATTEMPT NOT CONSUMED`

Date: `2026-08-16`

## Authorization Event

The user explicitly authorized one R10 sealed scientific attempt under the
frozen V4 contract. The mandatory pre-attempt audit ran before nonce creation
or sealed generator execution.

```text
confirm nonce created              false
NONCE_COMMITTED published          false
generator confirm interaction      false
sealed scientific attempts         0 / 1
production effects                 0
```

## P0 Finding

The R9 freeze proved a development-only generator and a declarative
confirm-read capability. It did not freeze an executable confirm route.

Exact contradictions:

```text
generator_model.rs:49-70
  exposes K2UncertaintyGeneratorRequestV1::development only

generator_model.rs:76-83
  rejects every split except Development
  requires the fixed development seed commitment

generator_model.rs:152-156
  rejects every public case whose vocabulary split is not Development

generator_model.rs:273-276
  requires every public batch commitment to equal the development commitment

generator.rs:27-29
  exported generator entry point is development-only

EXECUTABLE_MANIFEST_V1.json
  freezes thirteen owner executables but no nonce owner or confirm coordinator

k2_self_formed_uncertainty_process_v1.rs:693-706
  the only full coordinator is a Cargo test executable whose hash is absent
  from the R9 executable manifest

development_freeze/model.rs:165-213
  confirm-read capability is derived from booleans and the development
  generator hash; it never proves a valid Confirm request or end-to-end route
```

The process test also stops after sixteen per-case verification receipts. It
does not publish the sealed attempt's complete top-level control, terminal,
result, and cleanup sequence required by the durable state machine.

A second P0 exists in the combined paper contract:

```text
V2:459-469
  defines a one-probe oracle and requires oracle residual-class equality

V4:332-334
  permits one-or-two-probe confirm plans and requires every case to close
```

For a case that requires two probes, the model-guided plan has one residual
class while the complete one-probe oracle has more than one. Equality is then
impossible. V4 did not explicitly replace the oracle definition or baseline
accounting, and the development process test did not evaluate those conjuncts.
The successor paper must define an independent bounded one-or-two-probe oracle
and exact baseline aggregation before implementation.

## Why The Attempt Was Not Consumed

The V2 contract consumes the sole attempt at post-freeze nonce creation. The
audit stopped before that transition. Running the frozen generator with random
nonce bytes would be a known-invalid infrastructure action, not a scientific
test, and would falsely burn the only attempt.

No retry, seed shopping, or hidden probe occurred. The nonce preimage does not
exist.

## R9 Interpretation

The following R9 claim is superseded:

```text
confirm-read capability  READY
```

It is replaced by:

```text
development transport capability  PASS
confirm execution capability       NOT IMPLEMENTED
```

The original R9 files and roots remain immutable evidence of the missed
boundary. They are not rewritten from PASS to FAIL after the fact.

## Required Repair Before A New R10 Boundary

```text
Confirm-aware generator request and batch validation
-> repaired bounded one-or-two-probe oracle and baseline PASS clauses
-> post-freeze CSPRNG nonce owner
-> durable NONCE_COMMITTED before generator dispatch
-> one-shot nonce-to-generator file descriptor
-> frozen public coordinator executable
-> all-case precommit before private material becomes readable by coordinator
-> frozen private safety and dispatch route
-> independent worker and observer execution
-> independent final verification
-> exact controls and conjunctive terminal result
-> complete cleanup census
-> executable manifest containing every process root
-> new R8 verification and new R9 freeze
-> fresh exact-root R10 authorization
```

Changing any frozen source or executable invalidates the old R9 freeze for a
sealed attempt. Therefore the existing R10 authorization cannot silently carry
across the repair. It authorized an attempt against the old frozen roots; a new
freeze requires a new explicit authorization.
