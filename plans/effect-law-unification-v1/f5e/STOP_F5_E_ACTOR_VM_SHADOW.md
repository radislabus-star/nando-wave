# STOP-F5-E Winner-Owned Actor And VM Shadow

Status: `PASS / F5_F_UNLOCKED_NOT_STARTED`

Implementation commit:

```text
a785ba330f330a5dbf7b371a89c75c791ec285a3
```

Authority: `false`

## Result

```text
BoundProtocolActionV3
-> closed kernel compiler
-> BoundProtocolProgramV3
   program root commits mode + request + mapping + physical action
        |
        +-> typed actor AST execution
        |
        `-> versioned bytecode compiler
            -> existing operator_vm owner
            -> independent bytecode execution
        |
        v
byte-identical shadow result
-> OperatorShadowExecutionReceiptV3
```

`BoundProtocolProgramV3` has no public constructor and is not deserializable.
It can only be compiled from the opaque F5-D action. There is no manual actor
template, provider-name lookup table, or path from bytecode to authority.

The actor and VM share the immutable opcode/value contract but not an
execution function. The actor executes the typed AST. The VM decodes and
executes bytecode independently. The VM verifies that the action and program
roots in its header equal the externally owned opaque program; self-attested
bytecode roots are insufficient.

The legacy `OPERATOR_RENDERER_TYPED_ACTOR` route delegates execution back to
the actor and therefore cannot prove actor/VM parity. F5-E does not use it. The
new protocol-call decoder is a submodule of the existing `operator_vm` owner,
not a second VM or a production route.

## Opcode Boundary

```text
version                         3
BEGIN_CALL                      function capability + current physical symbol
ARGUMENT_STRING                 typed bound role value
ARGUMENT_INTEGER                typed bound role value
ARGUMENT_BOOLEAN                typed bound role value
ARGUMENT_IDENTIFIER             typed bound role value
EMIT                            canonical call JSON
unknown opcode                  ABSTAIN
```

Bytecode carries the current physical names because they belong to the
request-owned binding, while the semantic law remains name-neutral. Both the
action derivation root and the program root are checked before execution.

## STOP Matrix

```text
actor program root owned by mode+binding             PASS
manual actor template                                0
actor/VM parity mismatches                           0
string/integer/boolean/identifier parity             PASS
bytecode replaces external program root              REJECT
unknown opcode                                       ABSTAIN
truncated bytecode                                    ABSTAIN
output budget violation                              ABSTAIN
unsupported custom-tool program                      ABSTAIN
raw output in durable receipt                        0
production callers                                   0
execution authority                                  false
```

Function capability calls are the first executable V3 opcode family. Custom
tool calls remain explicitly unsupported because their inner-tool and result
projection contracts are not yet present in F5 evidence. F5-E does not invent
those fields from a physical tool name.

## Budgets

```text
arguments per action                                 <= 32
value bytes per argument                             <= 16,384
bytecode bytes                                       <= 32,768
rendered output                                      <= 16,384
new production module size                          <= 219 lines
```

## Verification

Local exact-commit owner suite:

```text
nando-operator-kernel          13 PASS / 0 FAIL
nando-operator-learning      198 PASS / 0 FAIL
nando-operator-runtime        33 PASS / 0 FAIL
total                        244 PASS / 0 FAIL
focused F5-E                   7 PASS / 0 FAIL
Clippy -D warnings             PASS
rustfmt / diff check           PASS
```

Remote clean detached worktree:

```text
host           e@192.168.3.94
worktree       /home/e/projects/nando-wave-f5e-a785ba3
HEAD           a785ba330f330a5dbf7b371a89c75c791ec285a3
target         /home/e/build/nando-wave-f5e-target
incremental    disabled
tests          244 PASS / 0 FAIL
Clippy         PASS
```

Graphify after the implementation:

```text
nodes / edges / communities      26,205 / 58,820 / 1,212
protocol decoder community       existing operator_vm owner
```

The live composite gate remained fail-closed:

```text
verdict                         PASS
eligible_for_local_accept       false
response ACTIVE packages        0
response M3                     WATCH
response false accepts          0
response parity mismatches      0
```

No deployment, service restart, registry write, or authority change occurred.

## Next Boundary

Only F5-F is unlocked:

```text
complete structurally valid attempts
-> phase/control ranking
-> one already valid action or ABSTAIN
```

Wave may reduce checks among structurally valid attempts. It may not create a
missing capability, repair an incomplete binding, or choose between distinct
physical action classes.
