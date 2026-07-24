# STOP Natural CPU Accept 2026-07-24

## Verdict

```text
adaptive identification                    PASS
immutable candidate freeze                 PASS
independent post-freeze transfer            PASS
crystallization                             PASS
runtime role grounding                      PASS
external admission                          PASS
ACTIVE registry                             PASS
gateway -> package -> CPU                    PASS
package-owned accounting                    PASS
false accepts                               0
runtime parity mismatches                   0
M3 mature windows                           0 / 3
```

This closes the first narrow natural operator route. It does not claim rich
multi-source coverage or M3.

## Live Receipt

```text
package_id
  crystallized-scalar-db6407c627296d8b

program
  project_selected_value
  selector=request_referenced_json_field_ordinal(0, string)

request route
  window-compatible request
  -> 127.0.0.1:8787/v2/responses
  -> nando-transition-serving
  -> admitted ResponseExecutor
  -> crystallized role binder
  -> actor
  -> independent verifier
  -> response projector
  -> post-verifier
  -> HTTP 200

verified output
  nando_cpu_probe_8ad6243

serving process counters after the receipt
  accepts=1
  ordinary_accepts=1
  ordinary_input_tokens=104
  response_cpu_by_package_valid=true
  response_cpu_by_package_overflow=0
```

The same registry and payload returned `EXECUTED`,
`crystallized_operator_verified`, one exact actor check, and the same package
from the read-only shadow inspector. Shadow inspection has no authority.

## Fixed-row Boundary

Natural readiness is:

```text
complete bounded candidate generation
-> one semantic class
-> immutable freeze
-> independent post-freeze transfer
-> runtime parity
```

It is not `32 support + 32 future`. Historical fixed-row paths remain
explicitly named `LEGACY_CONTROL_*`, retain their old baseline behavior, and
cannot become the proof basis of an adaptive package.

## Code

```text
a0dc3f3  adaptive transfer-basis crystallization
8ad6243  adaptive runtime role grounding
f1597eb  package-owned CPU diagnostics and shadow execution inspector
```
