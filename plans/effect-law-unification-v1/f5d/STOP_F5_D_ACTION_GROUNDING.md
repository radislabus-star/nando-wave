# STOP-F5-D Runtime Binding And Capability Grounding

Status: `PASS / F5_E_UNLOCKED_NOT_STARTED`

Implementation commit:

```text
759701564f0bd69c484617f7ea1efd246a602642
```

Authority: `false`

## Result

```text
CompleteRuntimeRoleBindingReportV3
+ current request-owned capability surface
+ request-local typed role values
        |
        v
one BoundProtocolActionV3 per complete structural mapping
        |
        v
semantic and physical action-class collapse
        |
        +-- exactly one physical action -> BoundProtocolActionSetV3
        `-- otherwise                  -> ABSTAIN
```

The semantic action identity contains the effect law, capability kind,
argument ordinals, source roles, and typed values. It deliberately excludes
the provider's physical capability and argument names. The physical identity
adds those current-surface names. This lets a renamed compatible capability
preserve the learned law while still producing a request-owned physical call.

No raw role value is serialized into the durable structural context. Values
exist only in the request-local binding environment used to derive the action.

F5-D does not render or execute the action, invoke a verifier, persist a
generation, create an admission candidate, or grant authority.

## Complete Mapping And Action Matrix

| Surface | Structural result | Semantic classes | Physical classes | Verdict |
|---|---:|---:|---:|---|
| Original compatible capability | complete | 1 | 1 | `COMPLETE` |
| Same schema, renamed capability | complete | 1, same as original | 1, new physical identity | `COMPLETE` |
| Two modes deriving the same action | at least 2 mappings | 1 | 1 | `COMPLETE`, one action |
| Two complete mappings with different values | at least 2 mappings | at least 2 | at least 2 | `ABSTAIN_AMBIGUOUS_ACTION` |
| Duplicate byte-identical capability declarations | complete | 1 | 1 | `COMPLETE`, duplicates collapse |
| Two compatible declarations with different symbols | complete | 1 | 2 | `ABSTAIN_AMBIGUOUS_CAPABILITY` |
| Capability absent | no mapping | 0 | 0 | `ABSTAIN_NO_STRUCTURAL_MAPPING` |
| Capability argument type incompatible | no mapping | 0 | 0 | `ABSTAIN_NO_STRUCTURAL_MAPPING` |
| Binding report reused on another request root | rejected | 0 | 0 | `REJECT_INDEX_MISMATCH` |

Each successful mapping records its mode ID, mapping root, runtime source
role, fixed-point phase fit, capability ID, semantic action root, physical
action root, and derivation verdict. The report remains available before
collapse; aggregate class counts cannot hide a conflicting mapping.

## Budgets And Failure Semantics

```text
structural candidates                           <= 32
mappings per mode                               <= 64
total action derivations                        <= 2048
advertised capabilities                         <= 64
incomplete structural search                    ABSTAIN
missing role value                              ABSTAIN
unsupported role/value type                     ABSTAIN
ambiguous semantic action                       ABSTAIN
ambiguous physical capability                   ABSTAIN
wrong bindings                                  0
negative accepts                                0
production callers                              0
execution authority                             false
```

The action IR supports multiple typed arguments and source roles. The current
F5-C graph compiler exposes only the source role it has proved; any unproved
higher role fails closed instead of being guessed. This is a proof boundary,
not a narrower action schema.

## Verification

Local exact-commit owner suite:

```text
nando-operator-kernel          13 PASS / 0 FAIL
nando-operator-learning      198 PASS / 0 FAIL
nando-operator-runtime        26 PASS / 0 FAIL
total                        237 PASS / 0 FAIL
Clippy -D warnings             PASS
git diff --check               PASS
```

Remote clean detached worktree on `e@192.168.3.94`:

```text
worktree       /home/e/projects/nando-wave-f5d-7597015
HEAD           759701564f0bd69c484617f7ea1efd246a602642
target         /home/e/build/nando-wave-f5d-target
incremental    disabled
tests          237 PASS / 0 FAIL
Clippy         PASS
```

The live composite gate after implementation remained fail-closed:

```text
composite verdict                   PASS
eligible_for_local_accept           false
response ACTIVE packages            0
response M3                         WATCH
response false accepts              0
response runtime parity failures    0
verified token saving share         0.7%
```

No service was started, restarted, deployed, or enabled. The inspected user
units were inactive with `NRestarts=0` at the STOP snapshot.

## Next Boundary

Only F5-E is unlocked:

```text
BoundProtocolActionV3
-> automatically compiled actor program
-> versioned program for the existing Operator VM owner
-> independent shadow executions
-> byte-identical result or ABSTAIN
```

F5-E must not use a manual actor template and must not implement a second VM.
F6 independent verification remains locked.
