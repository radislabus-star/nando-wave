# Nando Attractor-to-VM Implementation Journal

This is the resumable execution log for the canonical roadmap. Each entry
records the exact boundary, evidence, modification, checks, remaining blocker,
and measured command time. A code change is not complete until its entry has a
check result.

## 2026-07-19 15:37 EEST - Stage 8 Runtime Role Grounding, Pass 1

Starting state:

```text
HEAD ca80c15
production services untouched
graphify query wall time 2.98 s
focused source inspection commands < 0.2 s total
```

Confirmed defects:

```text
observed_multi_role_runtime_surface
  derived output role, relation planes, program atoms, and role order from the
  sealed operator TransformProgram instead of raw pre-action observation

independently_bind_verifier
  ignored RoleGraph and RelationProgram and verified a caller-bound selector
```

Implemented in the current worktree:

```text
raw observation bundle now contains only context + observed source roles
raw observation bundle contains no virtual output and no program atoms
RuntimeRoleBinder maps this partial observed graph into the sealed RoleGraph
verifier re-extracts ordinal roles from raw request/output
verifier independently reruns circuit-constrained CSP binding
```

Current check state:

```text
local stable rustfmt: ENVIRONMENT_UNAVAILABLE in 0.18 s (missing rustc driver)
Rust 1.97 fmt check: one formatting delta in 4.42 s
Rust 1.97 formatting applied: 2.95 s
remote compile: PASS in 40.54 s, linker peak RSS 1,823,016 KB
pre-action observation test: PASS 1/1 in 0.08 s
64-row rich role/admission proof: PASS 1/1 in 34.18 s
scalar crystallization rerun: FAIL in 33 s, MissingRuntimeAnchor
scalar crystallization after source-neutral signature repair: PASS 1/1 in 26.22 s
rich regression after scalar repair: PASS 1/1 in 25.86 s
rich reversed-request ordinal proof: PASS 1/1 in 33.67 s (60 s with relink)
remote cargo check --lib: PASS in 15.99 s
remote crate-wide Clippy -D warnings: ENVIRONMENT/BASELINE BLOCK in 36 s
  11 pre-existing warnings outside crystallized_operator.rs
  no warning reported in the changed module
```

Scalar failure diagnosis:

```text
historical scalar blueprint source role constraint mask = 2
runtime selector-specific role mask = 2 | selector class
single unique scalar has no observable selector-class law
repair: keep concrete selector only as ephemeral anchor and expose a
source-neutral scalar role signature to the circuit binder
```

Independent verifier audit:

```text
wrong: retain only mappings whose response already equals actor response
right: compute every independently grounded response class, require exactly
       one class, then compare that class with the actor response
multiple mappings with one action remain legal; multiple actions ABSTAIN
```

Stage 8 pass boundary:

```text
raw multi-role surface excludes output/program atoms                 PASS
raw scalar surface excludes output/program atoms                     PASS
circuit-constrained partial-role CSP                                 PASS
independent raw re-extraction and response-class verification         PASS
equal support values -> diverging future role proof                   PASS
renamed fields and reversed request ordinals                          PASS
restart + external laboratory admission                              PASS
production deployment                                                 NOT RUN
remote graphify update                                                 ENVIRONMENT_UNAVAILABLE
  wrapper shebang points to missing /home/ubu Python environment
  no replacement Python route installed or enabled
```

Next boundary: Stage 9 admission must recompute authority from sealed receipts
instead of trusting externally deserialized candidate booleans or counters.

Next action:

```text
remove the same operator-derived virtual output/program atom from scalar runtime
preserve independent scalar verification
rerun focused scalar and rich proofs
```
