# K2 Self-Formed Uncertainty V5 R8B Implementation Discrepancy

Status: `P0 / R8B V2 PREFLIGHT SUPERSEDED / BLOCKED_BEFORE_CODE`

Date: `2026-08-20`

Observed source commit: `bdcae5351c7de75f325b0ebe752804066823cc38`

Superseded preflight manifest root:
`5e48e334eee6d19ddd8b288c5322cf1fb924fdf501db729e2861c432af6972d8`

## 1. Required Route

The frozen V5 preregistration requires one DevelopmentRehearsal request to
traverse:

```text
confirm owner
-> generator pipe
-> public/private output split
-> public coordinator
-> private resolver
-> terminal
-> cleanup
```

The generator must be dispatched exactly once. Development generator request
and response bytes remain unchanged. No nonce, authorization slot or sealed
attempt may exist.

## 2. Observed Route

Current `execute_development_rehearsal_v1` in `confirm_owner.rs:127-186`:

1. dispatches the generator once;
2. decodes and validates `K2UncertaintyGeneratorResponseV1`;
3. records response, public-batch and private-batch roots;
4. appends `CasesGenerated` with the generator response root;
5. returns `K2UncertaintyConfirmOwnerReceiptV1` with
   `split_receipt_root_sha256 = None`;
6. persists no public/private split artifacts.

The receipt validator in `confirm_owner_model.rs:399-409` requires the split
root to be absent in DevelopmentRehearsal mode. Restart projection in
`confirm_owner.rs:107-115` loads a split only for Confirm mode.

The only split publisher, loader and validator in `confirm_artifacts.rs` accept
`K2UncertaintyConfirmGeneratorRequestV1` and
`K2UncertaintyConfirmGeneratorResponseV1`. They cannot consume the unchanged
Development generator wire types.

R7H's positive Development owner test asserts that the split root is absent.
R7I and R7J create a separate Confirm response from a fixed `vec![0xa5; 32]`
request. They do not continue the Development owner response.

## 3. Consequence

The previously recorded component PASS results are disconnected:

```text
Development owner transport PASS
+ separate Confirm split/downstream PASS
!= one linked DevelopmentRehearsal route PASS
```

R8B Contract V2 cannot be implemented while preserving its own frozen owner
map. A runner would have to dispatch the generator a second time, fabricate a
split, splice a Confirm fixture, or construct private artifacts outside their
owner. Every such route violates the preregistration.

Therefore:

```text
R8B Contract V2                         SUPERSEDED
code-route design V2                    SUPERSEDED
implementation preflight V2             SUPERSEDED
READY_TO_IMPLEMENT                      REVOKED
R8B implementation/execution            FORBIDDEN
```

No R8B suite, resource run, R9B freeze, nonce, authorization slot or sealed
attempt occurred under the invalid preflight.

## 4. Required Repair

The repair must:

```text
preserve Development generator request/response bytes
preserve Confirm request/response/owner/split bytes
persist a distinct typed DevelopmentRehearsal split inside confirm-owner
bind mode, owner request, attempt, generator request/response and batch roots
publish public artifacts before coordinator execution
retain private contents outside the public coordinator
append CasesGenerated with the typed Development split root
permit no second generator dispatch
reject Confirm/Development cross-mode substitution
classify every partial-write and restart state
give the linked runner metadata only, never private truth contents
```

Only a repaired contract, adversarial critique, owner-bounded structural gates,
code-route gate and fresh implementation preflight may restore
`READY_TO_IMPLEMENT`.

The repaired paper sequence is now:

```text
R8B Contract V3                         VETO
R8B Contract V4                         VETO
R8B Contract V5 structural routes       PASS / AUTHORITY FALSE
R8B Contract V5 design code-route       PASS / SOURCE UNVERIFIED
R8B Contract V5 implementation preflight PENDING
```

Contract V5:
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md`
