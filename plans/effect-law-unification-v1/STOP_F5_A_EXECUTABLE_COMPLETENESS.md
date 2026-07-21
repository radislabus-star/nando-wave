# STOP-F5-A Executable Completeness

Date: 2026-07-21

Base HEAD: `5f678fd5f0bc87877c081c715b4cee75089bf201`

Verdict: `F5A_EXECUTABLE_DOMAIN_PASS_CONSTANT_MODES_FAIL_CLOSED`

Authority: `false`

## Route

```text
CanonicalEffectLawV3 bytes
       +
ProtocolModeSetV2 bytes
       +
canonical physical-facet evidence bytes
       |
       v
ExecutableProtocolModeArtifactV3
  - embedded byte-identical F4R2 mode set
  - embedded source-neutral effect-law payload
  - executable capability class and typed argument topology
  - physical symbol source = current advertised capability surface
  - externally pinned artifact root for restart
  - production_admissible = false
  - execution_authority = false
```

F5-A is a cold artifact boundary. It does not inspect a live request, bind a
physical capability, execute an actor, construct a package, run admission, or
change serving.

## Completeness Inventory

```text
source role schema               executable V2 bytes
selector program                 executable V2 bytes
observed/emitted value contract  executable V2 bytes
protocol facet root              canonical physical-evidence commitment;
                                 proof/provenance only at runtime
physical program IDs             proof/provenance only
capability class                 executable V3 bytes
physical capability symbol       live source: current advertised surface
argument role schema             executable V2 bytes
constant contract roots          hash-only; accepted only when empty
raw physical constants           rejected without ordinal-bound payload
structural guard                 V2 selector bytes + committed invariant;
                                 exact law bytes embedded separately
temporal/cardinality contract    executable V2 bytes
effect law                       embedded canonical V3 bytes
coverage rows and counts         proof-only
production/admission authority   always false
```

The legacy `protocol_facet_root_sha256` is not an opaque unverified string: the
compiler receives the canonical facet bytes, re-hashes them, and requires exact
root parity. Those bytes contain physical surface names, so they are not used
as runtime instructions. `ProtocolFacetPayloadV3::payload_root_sha256` is a
separate commitment to the source-neutral executable projection.

## Supported Domain

The emitted artifact currently supports F4R2 modes with:

```text
capability kind                  function or custom tool
typed source roles               present
argument ordinal mapping         present
implicit defaults                forbidden
semantic/protocol constants      empty
physical names                   excluded from artifact bytes
```

This is a deliberate bounded domain, not silent data loss. V2 stores constants
as hashes but does not prove their argument ordinals. Therefore:

```text
non-empty V2 constant roots      HashOnlyConstantCommitment
raw facet constant atoms         UncommittedPhysicalConstant
```

F5-B may proceed for emitted no-constant artifacts. Constant-bearing modes
remain unsupported until evidence carries typed value bytes plus an explicit
argument ordinal; names or hash lookup cannot fill that gap.

## F4 Parity

F4 search, scoring, thresholds, exact cover, and mode-set validation were not
changed. The source `ProtocolModeSetV2` is embedded unchanged and its canonical
bytes compare byte-for-byte after F5-A compilation and restart.

```text
F4 selector matrix drift         0
F4 exact-cover drift             0
source mode-set byte drift       0
physical names in semantic law   0
runtime callers                  0
```

The controlled fixture proves that one effect law may carry both a function
facet and a custom-tool facet while retaining one source-neutral law. The
result stores only the capability class, typed ordinal mapping, commitments,
and the future live symbol source.

## Fail-Closed Controls

```text
invalid or authority-bearing F4 mode set       REJECT
effect-law payload/root mismatch               REJECT
missing facet evidence                         REJECT
extra facet evidence                           REJECT
non-canonical or tampered facet bytes           REJECT
unsupported/mixed capability shape             REJECT
uncommitted physical constant                  REJECT
hash-only constant commitment                  REJECT
tampered payload or artifact root              REJECT
foreign externally expected artifact root      REJECT
canonical restart                              byte-identical
```

## Checks

Heavy Rust checks ran on:

```text
e@192.168.3.94:/home/e/projects/nando-wave-f5a-build
```

```text
physical_trial_v2_tests             21 / 21 PASS
binding_evidence suite              100 / 100 PASS
effect_law_v3                       40 / 40 PASS
historical effect_law               15 / 15 PASS
cargo check --all-targets           PASS
rustfmt --check                     PASS
git diff --check                    PASS
full lib baseline                   502 PASS / 26 known FAIL
```

The full baseline added three passing F5-A tests and preserved the same 26
historical failures. `clippy -D warnings` remains blocked by the existing 12
library and 8 test-only diagnostics; no diagnostic points at
`executable_protocol_mode/*` or the F5-A test slice.

## Structural And Live Gates

Owner-local NANDA routes:

```text
artifact owner                      PASS / authority_ready=false
payload/provenance owner            PASS / authority_ready=false
authority boundary owner            PASS / authority_ready=false
```

The final read-only composite gate reports:

```text
composite verdict                   PASS
eligible_for_local_accept           false
response ACTIVE packages            0
response M3                         WATCH
response false accepts              0
response runtime parity failures    0
transition false accepts            0
transition parity mismatches        0
```

Structural PASS is coherence only. It does not grant proof or runtime
authority.

## Process Boundary

```text
nando-transition-serving InvocationID  74ac3080f80b4fe387de2a94380e3657
nando-response-learning InvocationID    8e59505eb1b943778601c9b3bacbd607
service restarts                         0
deploy                                   no
registry or checkpoint change            no
background F5-A build/test processes     0
```

Final boundary:

```text
F5-A executable completeness       PASS in bounded no-constant domain
F5-B canonical runtime context     NEXT / NOT_STARTED
runtime grounding                  NOT_CONNECTED
actor / VM                         NOT_CONNECTED
independent verifier               F6 / NOT_STARTED
admission                          BLOCKED
production authority              false
```
