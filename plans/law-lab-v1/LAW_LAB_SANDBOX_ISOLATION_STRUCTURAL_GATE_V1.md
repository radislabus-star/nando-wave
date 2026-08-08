# Law Lab Sandbox Isolation Structural Gate V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t01 | executor manifest | binds before freeze | worker, bwrap, prlimit, limits, and mounts | LAW_LAB_SANDBOX_ADAPTER_V1.md section 4 | 1.0 | preregistration | executor | sandbox-isolation | sandbox-isolation |
| t02 | bwrap | isolates | PID, network, mount, IPC, UTS, cgroup, and user namespaces | LAW_LAB_SANDBOX_ADAPTER_V1.md section 5 | 1.0 | sandbox adapter | isolation boundary | sandbox-isolation | sandbox-isolation |
| t03 | sandbox adapter | applies prlimit inside isolated namespace to bound | worker CPU, memory, processes, and file size | LAW_LAB_SANDBOX_ADAPTER_V1.md section 5 | 1.0 | sandbox adapter | worker | sandbox-isolation | sandbox-isolation |
| t04 | exact source snapshot | mounts read-only as | /source | LAW_LAB_SANDBOX_ADAPTER_V1.md section 5 | 1.0 | immutable input | sandbox path | sandbox-isolation | sandbox-isolation |
| t05 | disposable workspace | mounts writable as | /work only | LAW_LAB_SANDBOX_ADAPTER_V1.md section 5 | 1.0 | disposable state | sandbox path | sandbox-isolation | sandbox-isolation |
| t06 | worker attestation | proves absent | non-loopback routes and forbidden host paths | LAW_LAB_SANDBOX_ADAPTER_V1.md section 6 | 1.0 | in-namespace witness | host capability | sandbox-isolation | sandbox-isolation |
| t07 | parent verifier | independently verifies | exact source and post-work roots | LAW_LAB_SANDBOX_ADAPTER_V1.md section 7 | 1.0 | external verifier | worker outcome | sandbox-isolation | sandbox-isolation |
| t08 | timeout or failure | still triggers | verified workspace cleanup | LAW_LAB_SANDBOX_ADAPTER_V1.md section 8 | 1.0 | terminal path | cleanup proof | sandbox-isolation | sandbox-isolation |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c01 | executor manifest | binds before freeze | worker, bwrap, prlimit, limits, and mounts | law_lab_sandbox/manifest.rs LawLabSandboxExecutorManifestV1 | 1.0 | preregistration | executor | sandbox-isolation | sandbox-isolation |
| c02 | bwrap | isolates | PID, network, mount, IPC, UTS, cgroup, and user namespaces | law_lab_sandbox/adapter.rs command_spec_v1 | 1.0 | sandbox adapter | isolation boundary | sandbox-isolation | sandbox-isolation |
| c03 | sandbox adapter | applies prlimit inside isolated namespace to bound | worker CPU, memory, processes, and file size | law_lab_sandbox/adapter.rs command_spec_v1 | 1.0 | sandbox adapter | worker | sandbox-isolation | sandbox-isolation |
| c04 | exact source snapshot | mounts read-only as | /source | law_lab_sandbox/adapter.rs command_spec_v1 | 1.0 | immutable input | sandbox path | sandbox-isolation | sandbox-isolation |
| c05 | disposable workspace | mounts writable as | /work only | law_lab_sandbox/adapter.rs WorkspaceGuardV1 | 1.0 | disposable state | sandbox path | sandbox-isolation | sandbox-isolation |
| c06 | worker attestation | proves absent | non-loopback routes and forbidden host paths | law_lab_sandbox/worker.rs collect_isolation_attestation_v1 | 1.0 | in-namespace witness | host capability | sandbox-isolation | sandbox-isolation |
| c07 | parent verifier | independently verifies | exact source and post-work roots | law_lab_sandbox/adapter.rs verify_operations_independently_v1 | 1.0 | external verifier | worker outcome | sandbox-isolation | sandbox-isolation |
| c08 | timeout or failure | still triggers | verified workspace cleanup | law_lab_sandbox/adapter.rs WorkspaceGuardV1 | 1.0 | terminal path | cleanup proof | sandbox-isolation | sandbox-isolation |
