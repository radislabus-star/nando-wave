# NANDA Task: phase-stream-serving-append-shadow-replay-v3-latest-extractor

## query

Check that the latest-window Codex-session tool-status extractor scans session
files, selects the newest events by event timestamp, writes them
chronologically, and still writes no raw request, response, or tool output text.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_format_extractor | selects | latest_events_by_event_timestamp | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#selection_policy |
| live_format_extractor | scans | codex_session_files | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#session_files_scanned |
| live_format_trace_report | writes | append_latest_trace_rows | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#rows_written |
| live_format_trace_report | counts | response_item_tool_output_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#response_item_tool_output_events_seen |
| live_format_trace_report | keeps_false | raw_tool_output_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_tool_output_written |
| live_format_trace_report | keeps_false | raw_request_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_request_text_written |
| live_format_trace_report | keeps_false | raw_response_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_response_text_written |
| live_format_trace_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#local_accept_enabled |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_format_extractor | selects | latest_events_by_event_timestamp | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#selection_policy |
| live_format_extractor | scans | codex_session_files | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#session_files_scanned |
| live_format_trace_report | writes | append_latest_trace_rows | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#rows_written |
| live_format_trace_report | counts | response_item_tool_output_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#response_item_tool_output_events_seen |
| live_format_trace_report | keeps_false | raw_tool_output_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_tool_output_written |
| live_format_trace_report | keeps_false | raw_request_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_request_text_written |
| live_format_trace_report | keeps_false | raw_response_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#raw_response_text_written |
| live_format_trace_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v3-latest.report.json#local_accept_enabled |
