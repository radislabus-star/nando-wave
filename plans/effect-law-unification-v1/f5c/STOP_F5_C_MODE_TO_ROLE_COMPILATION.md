# STOP-F5-C Mode-To-Role Compilation

Status: `PASS / F5_D_UNLOCKED_NOT_STARTED`

Implementation commits:

```text
e558e40273f51c270f263cc6837f135e43c3a9cc  graph compiler + complete binder report
e54518832fdc19dbc0ed69710cca92d273421de4  exact immutable bitset dispatch
ba0824702f8fedf93a2a2f05c88dad2c17e88a6c  complete-only F5-D handoff
```

Authority: `false`

## Result

```text
ExecutableProtocolModeArtifactV3
  -> canonical-byte and root validation
  -> selector predicates
  -> existing StructuralRoleSignature + RoleGraph
  -> existing OperatorCircuit relation cells
  -> immutable capability/selector bitset index
                              |
CanonicalRuntimeRequestV3     |
  -> exact observable dispatch
  -> <= 32 mode references or ABSTAIN
  -> existing RuntimeRoleBinder
  -> complete structural mapping set
  -> separate phase-winner view + runner-up margin
  -> CompleteRuntimeRoleBindingReportV3
```

F5-C does not bind a physical capability symbol, derive an action, render a
response, execute the VM, verify a result, persist a generation, or grant
authority. Those owners begin at F5-D.

## Architecture

Selector semantics have one source. Each `BindingPredicateV1` is compiled to
typed constraint roles and relation cells in the existing operator graph. The
dispatch index is a derived immutable acceleration structure over the same
compiled dimensions; it cannot authorize a mapping. The binder independently
replays the graph against the current canonical request surface.

The index uses fixed bounded bitsets over:

```text
current capability kind + required arity + argument type multiset
+ source event class
+ call lineage
+ capability class
+ temporal distance
+ completion state
+ candidate cardinality
+ value type
+ request relation
+ all eight words of the 256-bit topology commitment
```

Wildcard selector dimensions remain wildcard masks. Runtime intersects masks
and returns every matching mode in canonical index order. If more than 32
modes survive, it returns the exact count and no mode references. It never
keeps the first 32 by package, fingerprint, phase, or arrival order.

The binder now preserves two explicit views:

```text
structural_mappings()     every exact relation-satisfying mapping
phase_winner_mappings()   only the best fixed-point phase equivalence class
phase_runner_up           the next distinct phase score
```

The historical `mappings()` method remains a compatibility view of phase
winners, so existing callers do not silently change behavior.

## Exact-Cap Repair

The old DFS marked a search incomplete as soon as it produced the 64th
mapping. That conflated two different states:

```text
exactly 64 mappings and no remaining branch       COMPLETE
64 retained mappings and a live 65th branch       EXHAUSTED
```

The search now keeps one bounded look-ahead mapping solely to prove whether a
frontier remains, then truncates the report back to the contractual cap.

## STOP Matrix

```text
new graph or predicate language                         0
V3 calls bind_raw_pre_action_components                 0
V3 provider payload rescans                             0
hidden pre-report phase pruning                         0
exactly-at-cap hidden frontier                          0
capability shape mismatch                               no dispatch
selector mismatch                                       no dispatch
wildcard selector false negative                        0
full 256-bit topology mismatch                          no dispatch
70-mode overfull bucket                                 ABSTAIN, count=70
runtime modes after dispatch                            <=32 or ABSTAIN
mappings per mode                                       <=64 or ABSTAIN
source-candidate evaluations                            <=2048 or ABSTAIN
mapping evaluations                                     <=2048 or ABSTAIN
package/fingerprint/phase-order truncation               0
missing/tampered/contradictory artifact                  REJECT
production callers                                      0
execution authority                                     false
```

## Verification

All broad builds ran on the remote 20-thread machine. The exact final runtime
STOP used clean commit `ba082470`, no overlay, and no untracked input.

```text
nando-operator-runtime       20 PASS / 0 FAIL / Clippy PASS
nando-operator-kernel        13 PASS / 0 FAIL / Clippy PASS
nando-core                  176 PASS / 0 FAIL / 5 ignored / Clippy PASS
owner total                 209 PASS / 0 FAIL / 5 ignored

focused final core            3 PASS / 0 FAIL
focused final F5-C            7 PASS / 0 FAIL
```

The full core baseline ran on `e558e40`; the following two commits changed
only `nando-operator-runtime`. A clean exact-final fast receipt reran all three
new core binder tests at `ba082470`. Runtime received a full STOP receipt on
the same exact final HEAD.

Remote timings:

```text
core compile / test          3.264 s / 169.703 s
kernel compile / test        4.538 s /   0.007 s
runtime compile / test       5.933 s /   0.393 s
```

The long core time belongs to the pre-existing hard semantic-grokking test;
the exact-final binder subset took `0.004 s`.

Static size budget:

```text
largest new production module       206 lines
largest new test module             269 lines
mode_to_role_v3 production files      9
legacy/raw forbidden-route matches    0
```

No F5-C latency or RSS claim is made because production callers are zero.
The `250 us / 1 ms / 2 ms / 16 MiB` product gates remain F5-G obligations.

## Structural Proof

Owner-local NANDA routes:

```text
graph compiler          PASS / authority_ready=false
binder core             PASS / authority_ready=false
runtime bridge          PASS / authority_ready=false
```

Remote Graphify on exact final HEAD:

```text
25,885 nodes / 58,197 edges / 1,197 communities
update wall time       16.05 s
compiler -> binder      4 hops through RoleGraph
runtime bridge -> context 1 hop
```

The live composite gate remains fail-closed:

```text
verdict                         PASS
eligible_for_local_accept       false
response ACTIVE packages        0
M3                              WATCH / false
false accepts                   0
runtime parity failures         0
```

Both service `InvocationID` values remain byte-identical to STOP-F5-B and
`NRestarts=0`. No deployment or restart occurred. Remote STOP runners left no
new Cargo or rustc process.

Canonical artifacts are the machine receipt, three remote gate receipts, the
exact-final core receipt, the live gate, the systemd snapshot, and the three
owner-local NANDA traces in this directory.

## Next Boundary

Only F5-D is unlocked:

```text
CompleteRuntimeRoleBindingReportV3
+ current advertised capability bindings
-> one canonical action per phase-winning mapping
-> collapse by pre-render action-equivalence digest
-> BoundProtocolActionSetV3 | ABSTAIN
```

F5-D must prove renamed capability transfer, same-action collapse, ambiguous
action abstention, missing capability abstention, and zero wrong/negative
bindings. It may not execute the actor, verify itself, persist a package, or
grant authority.
