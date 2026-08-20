# K2 Self-Formed Uncertainty V5 R8B Contract V6 Critique V1

Status: `CRITIQUE COMPLETE / V6 DRAFT VETO / REPAIR REQUIRED BEFORE GATES`

Date: `2026-08-21`

Critiqued commit: `dbd3bc3`

## Verdict

The V6 draft correctly rejects `af18cad` and restores the missing owner, oracle,
restart and cleanup routes, but it is not implementable as written. Two P0 and
seven P1 defects remain. No structural PASS or implementation authority may be
issued against the draft bytes.

## Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | The claimed 24-identity manifest still omits `nando-k2-self-formed-r7k-control-case`, which produces the twelve fresh K1-K12 outcomes, and `nando-k2-self-formed-result-publisher`, which alone emits `DEVELOPMENT_REHEARSAL_COMPLETE`. | A complete route needs 26 identities. The draft could again pass by listing consumers while skipping two required producers. | Freeze an exact 26-entry manifest and name both omitted process roles in the receipt chain. |
| P0 | The draft requires the resource receipt to cover aggregate publication while also requiring the aggregate authorizer to consume that resource receipt. A process cannot know its final cgroup peak and exit state before it and all measured descendants finish. | C18 -> C20 is circular if C20-C21 are inside the measured child. | Split one executable into parent-observer and child-route invocations. The child ends at `DEVELOPMENT_REHEARSAL_COMPLETE`; the parent finalizes child cgroup metrics, production survival and then launches aggregate authorization/publication. |
| P1 | The process-cardinality contract is said to freeze before C06, but closure-plan lengths do not exist until `ALL_CASES_PRECOMMITTED` is durable. | The denominator is temporally impossible or must be guessed. | Freeze public cardinalities before C06; after C06 and before any private process, derive and publish a downstream invocation contract from immutable public closure plans. |
| P1 | The oracle mount is described semantically, but the existing sandbox has no Oracle role, no oracle evidence-root mount and no oracle working-directory transition. | A coder could fall back to the R7J runner-side private read or create an unreviewed second sandbox. | Name `confirm_sandbox.rs` as the sixth modified predecessor file. Add one Oracle guest role, read-only evidence-root target, read-only private-truth target and fixed child working directory while preserving oracle evaluator bytes. |
| P1 | The draft does not close the path-validation-to-mount race. Passing a validated pathname to bubblewrap allows substitution between metadata inspection and mount. | The oracle could read bytes different from the split descriptor. | Open private truth with `O_PATH | O_NOFOLLOW`, verify inode/type/mode/length from that descriptor, keep the descriptor open, and bind from the inherited descriptor path. The oracle must still recompute content SHA-256 and semantic root. |
| P1 | Aggregate input permits either canonical bytes or descriptors without choosing one and without defining how the authorizer obtains bytes. | Synthetic roots can still satisfy a positive component constructor, repeating the `af18cad` weakness. | Mount one closed read-only evidence packet into the R8B authorizer. Its stdin names only route ID and manifest root. The authorizer reopens every entry, checks path set, bytes, schema, semantic root, producer and denominator, then emits authorization. |
| P1 | P07 requires two real owners but does not preregister how the first owner is held after acquiring the lock. The existing X20 test uses a foreign test-process lock holder, not a first owner. | A test can relabel the old weaker case as P07. | Trace the real first owner process, stop it immediately after successful lab-root lock acquisition, verify PID/inode ownership in `/proc/locks`, run the contender, then resume the first owner. Unavailable tracing is VETO, not skip. |
| P1 | The fresh and historical control denominators are ambiguous. R7K executes K1-K12 in a real control-case child but synthesizes old static outcomes before evaluator replay. | Static historical regression could be reported as fresh process execution. | Keep 32/4/16 as separately named frozen regression receipts; require 12/12 fresh control-case process executions at the V6 implementation commit. Never merge the two evidence classes. |
| P1 | Mandatory suite receipts have producer test binaries that are not members of the linked process manifest. | The aggregate packet can bind route processes but leave restart/mode/publication suite ownership unnamed. | Add a separate suite-producer manifest. It is not added to the 26 linked identities and is reported as a separate exact denominator. |
| P1 | Process-ledger durability is unspecified. The runner could accept a child receipt and crash before recording its identity and request binding. | Recovery can duplicate a child or lose cardinality evidence. | Journal `ChildStarted` before spawn and atomically append `ChildFinished` after canonical receipt validation, fsync both transitions, and fail closed on started-without-finished. No automatic replay after an indeterminate child. |
| P1 | Production survival has no exact observation owner and conflicts with the draft's unqualified zero-network statement. | Health HTTP reads can be hidden as experiment traffic or produced by an unbound shell. | Let the manifest-bound parent runner own the read-only observer event. Record its health GETs separately; retain zero network calls only for the sandboxed child route. |

## Correct Identity Derivation

```text
17 R7J process identities
+ 1 Development owner
+ 1 fresh R7K control-case producer
+ 3 cleanup identities
+ 1 Development result publisher
+ 1 linked runner executable
+ 1 R8B aggregate authorizer
+ 1 R8B evidence publisher
= 26 linked identities
```

The 17 R7J identities already include the control evaluator and oracle baseline,
but not the fresh control-case producer. The cleanup verifier does not publish
`DEVELOPMENT_REHEARSAL_COMPLETE`; the distinct result publisher does.

## Required Parent/Child Topology

The same manifest-bound linked-runner executable may appear twice in the process
ledger without appearing twice in the identity manifest:

```text
linked parent observer
  -> records pre-production snapshot
  -> launches linked child in fresh delegated cgroup

linked child route
  -> owner through DEVELOPMENT_REHEARSAL_COMPLETE
  -> exits with immutable candidate packet

linked parent observer
  -> finalizes child cgroup receipt
  -> records post-production snapshot
  -> launches R8B authorizer over closed packet
  -> launches R8B publisher over exact authorization bytes
```

This removes the resource cycle. The aggregate authorizer and publisher are not
included in the child-route cgroup denominator; each receives a separate small
process outcome and timeout denominator. The parent itself cannot issue a PASS,
authorize evidence or publish `R8B_FROZEN`.

## Oracle Feasibility

The existing oracle evaluator can remain byte-identical. It already validates a
closed current-directory evidence tree, recomputes each entry content hash,
decodes private truth and checks the descriptor semantic root. The missing
connection is sandbox ownership:

```text
public oracle evidence root -> read-only /oracle
O_PATH private descriptor   -> read-only /oracle/private-truth.json overlay
oracle child cwd            -> /oracle
```

The host evidence root contains a classified non-authoritative mountpoint file.
The child namespace overlays the real private truth before execution. Manifest
closure is validated inside the child namespace. The runner never opens the
private file for reading.

## Claim Boundary

This critique does not prove that the repaired route will work. It proves that
the draft still contained hidden process identities, a temporal cycle and
unclosed authority channels. The next legal action is paper repair, followed by
fresh structural, code-route and implementation-preflight gates. Rust edits,
R8B execution, push and deployment remain forbidden.

