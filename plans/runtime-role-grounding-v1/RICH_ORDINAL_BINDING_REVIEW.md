# Rich Ordinal Role Binding Reviewer Handoff
Status: implementation guidance only. It grants no admission or runtime authority.

## Immediate Objective

Close the current `AmbiguousRuntimeAction` for two or more values without
turning `RequestReferencedJsonFieldOrdinal` into a manually supplied answer.

```text
raw request + raw tool output
-> independently observed role candidates
-> circuit-constrained role mapping
-> winner-bound typed actor
-> independently reconstructed verifier expectation
-> PASS or ABSTAIN
```

`RequestReferencedJsonFieldOrdinal` is an admissible compact primitive. It is
not by itself a rich transferable operator.

## Root Cause

The current implementation searches a Cartesian product of generic selectors
and then groups successful results by rendered response. This creates three
problems:

1. search grows rapidly with role count;
2. role identity can be inferred from candidate order instead of observation;
3. a support-derived `actor_template` can bypass circuit-level induction.

The fix is to extract observed role nodes first and let the circuit bind those
nodes. Do not search all selector combinations and do not use the actor output
to decide what the roles meant.

## Required Ownership Split

```text
Observation owner
  raw request/output -> ObservedRoleCandidate[]

Induction owner
  support fragments -> competing complete blueprints -> sealed winner

Execution owner
  sealed winner + bound roles -> typed actor

Verification owner
  raw request/output -> independent expected response

Admission owner
  recomputed proof roots and counters -> ACTIVE or BLOCK
```

Only typed receipts cross these boundaries.

## Minimal Binding IR

The operator package should contain a compact binding contract, not field
names:

```rust
enum RoleBindingOp {
    RequestReferencedJsonFieldOrdinal {
        role: u8,
        ordinal: u8,
        value_type: AtomValueType,
        require_unique_path: bool,
    },
}

struct ObservedRoleCandidate {
    local_role: u8,
    selector: ResponseValueSelector,
    request_position: u16,
    json_path_sha256: [u8; 32],
    value_type: AtomValueType,
    source_class: SourceClass,
}
```

Concrete field names are allowed as ephemeral extractor input. They must not
be stored in the operator package. The runtime receipt stores only bounded
structural data and commitments.

For a compact page, one binding instruction can be encoded in eight bytes:

```text
opcode | role | ordinal | value_type | flags | reserved
```

Sixteen roles therefore cost at most 128 bytes.

## Runtime Algorithm

1. Read the latest bounded user request and latest bounded tool output.
2. Tokenize request identifiers using one frozen normalization version.
3. Traverse JSON while preserving the full structural path.
4. Produce only actually observed referenced-field candidates.
5. Assign request ordinal from mention position, never object iteration order.
6. Reject a role if one request mention resolves to multiple JSON paths.
7. Build `ObservedRuntimeSurface` from these candidates.
8. Run `RuntimeRoleBinder` against the crystallized `RoleGraph` and
   `RelationProgram`.
9. Accept only one complete action-equivalence class.
10. Compile the actor from the sealed blueprint and bound roles.

Do not emit all ordinals `0..15` and form a broad Cartesian selector product.
The extractor should normally emit exactly the role candidates present in the
current request and output.

## Ambiguity Policy

Search-order tie breaks may be deterministic, but they cannot grant authority.

```text
one mention -> one structural JSON path       bind
one mention -> several paths                  ABSTAIN
several mentions -> one indistinguishable role ABSTAIN
missing mention or missing value              ABSTAIN
search budget exhausted                       ABSTAIN
multiple executable action classes            ABSTAIN
```

Do not break semantic ties by field name, JSON map order, value text, hash, or
the actor's rendered response.

The path, not only the leaf field name, must participate in evidence. Two
nested objects may legally contain the same leaf key.

## Circuit And Actor Contract

The winning blueprint must commit to all executable parts:

```text
RoleGraph
RelationProgram
RoleBindingProgram
TransformProgram
CompositionDag
RendererProgram
VerifierContract
```

The current route

```text
build_support_actor_template
-> crystallize_with_actor_template
```

is acceptable only as development plumbing while `apply_authority=false`.
Before admission, the actor template must be inside the frozen candidate set,
its digest must be inside the winner receipt, and `crystallize` must obtain it
from the selected blueprint. A caller must not pass a ready actor after phase
selection.

A bounded symbolic synthesizer may select the exact renderer and program.
Phase coherence does not need to replace symbolic exactness, but it must select
the complete blueprint that owns the executable program.

## Independent Verifier

The verifier receives:

```text
raw request
raw tool output
sealed RoleBindingProgram
actor response
```

It must independently:

1. tokenize the request;
2. recover request ordinals;
3. traverse JSON paths;
4. require unique path bindings;
5. compute the expected rendered response;
6. compare it with the actor response.

It must not receive actor-selected selectors, actor-selected values, or a
caller-provided `BoundRoleEnvironment` as truth. Separate runtime and verifier
implementations are desirable; shared immutable schemas are acceptable.

## Phase Proof

The scored proof should contain competing complete role-order blueprints, for
example:

```text
X: ordinal 0 -> total role,  ordinal 1 -> failed role
Y: ordinal 0 -> failed role, ordinal 1 -> total role
Z: one role duplicated or one role omitted
```

Required result:

```text
full phase             selects one complete blueprint
no phase               ABSTAIN
shuffled phase         ABSTAIN
magnitude only         ABSTAIN
matched random center  ABSTAIN
```

The selected actor must be unavailable when no blueprint wins. Running the
same attached actor after phase ablation does not prove circuit causality.

## Admission Repair

`LiveScalarAdmissionCandidate` is externally deserializable. Admission must
not trust its booleans or counters.

Recompute or validate through one sealed receipt:

```text
operator page SHA-256
candidate-set SHA-256
binding-program SHA-256
actor/renderer SHA-256
support root
future root
phase-control report root
binding receipt root
execution receipt root
distinct sessions and surfaces
wrong accepts
parity mismatches
```

`wave_causal_pass=true` supplied by a candidate is not evidence.

## Adversarial Gate

The rich operator is not complete until all cases are covered:

```text
renamed fields                         PASS
request field order reversed           roles reverse correctly
equal scalar values                     PASS without role confusion
extra unrelated scalar                 ignored or ABSTAIN by contract
nested duplicate leaf key              ABSTAIN unless path is unique
repeated request mention               ABSTAIN if ordinal is ambiguous
missing referenced field               ABSTAIN
same fields in older tool output        latest-output policy remains exact
content-part array request              parity with string request
actor selector mutation                 verifier REJECT
verifier path mutation                  verifier REJECT
restart                                 byte-identical decision
full phase                              PASS
all causal phase controls               ABSTAIN
false accepts                           0
parity mismatches                       0
```

Include at least one future case where support values were equal but future
values differ. This prevents action-equivalence on support from hiding a role
swap.

## Resource Gate

Keep the hot path bounded:

```text
payload bytes        <= 64 KiB
JSON depth           <= 8
observed paths       <= 64
operator roles       <= 16
role mappings        existing fixed binder budget
selector product     removed from the normal ordinal path
```

Report extraction, binding, actor, verifier, and total p99 separately. JSON
parsing will not be nanosecond-scale; the product claim is CPU-local verified
execution, not a misleading 34 ns end-to-end claim.

## Implementation Order

```text
O1 path-aware observed ordinal candidates
O2 direct bounded role binding without selector Cartesian product
O3 binding program included in blueprint fingerprint
O4 renderer and actor included in sealed winner
O5 independently reconstructed verifier result
O6 adversarial and phase-ablation proof
O7 restart parity
O8 admission proof recomputation
O9 live shadow denominator
O10 ACTIVE only after external gate PASS
```

Do not expand to status, count, filter, or compose before O1-O8 pass for the
two-role operator.
