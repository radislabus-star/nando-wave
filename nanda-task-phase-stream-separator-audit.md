# NANDA Task: Phase Stream Separator Audit

## Query

Check that `phase-stream-real-traffic-separator-audit-v1` is only a request-side
atom mining audit. It may rank atoms that separate verifier-true and
verifier-false events on the current labelled trace set, but it must not treat
static-clean atoms as proof, promote `.nwpc`, enable local accept, claim market
money, or revive `.nwrb`.

## Triads

| subject | relation | object | evidence |
|---|---|---|---|
| separator audit command | entrypoint | phase-stream-real-traffic-separator-audit-v1 | crates/nando-cli/src/main.rs |
| separator audit command | implementation | run_phase_stream_real_traffic_separator_audit_v1 | crates/nando-cli/src/phase_streaming_cmd.rs |
| separator audit command | report path | target/nando-wave/streaming/real-traffic-phase-center-separator-audit-v1.report.json | command output |
| separator audit report | parsed labelled events | 374 | audit report |
| separator audit report | skipped no shadow request | 16595 | audit report |
| separator audit report | skipped no verifier label | 31 | audit report |
| separator audit report | exact cache hits | 185 | audit report |
| separator audit report | static clean candidates | 164 | audit report |
| agent route | best atom | tool_count_exact:4 | route summary |
| agent route | best true over exact | 14 | route summary |
| agent route | static clean candidates | 91 | route summary |
| metrics route | best atom | active_center_exact:13081 | route summary |
| metrics route | best true over exact | 6 | route summary |
| metrics route | static clean candidates | 72 | route summary |
| separator audit command | local accept enabled | false | audit report boundary |
| separator audit command | market claim allowed | false | audit report boundary |
| separator audit command | forbidden flags | all false | audit report forbidden_flags |
| executor notes | records | static clean atoms are not accept frontier | docs/EXECUTOR_REVIEW_NOTES.md |

## Candidate Triads

| subject | relation | object | evidence |
|---|---|---|---|
| separator audit report | treats | static-clean atom as proof | negative-contract:static_clean_not_proof |
| separator audit command | enables | product local accept | negative-contract:local_accept_false |
| separator audit command | promotes | `.nwpc` runtime package | negative-contract:audit_only_no_promotion |
| separator audit command | revives | `.nwrb` role-binding backend | negative-contract:legacy_backend_false |
| separator audit report | claims | market money proof | negative-contract:market_claim_false |
