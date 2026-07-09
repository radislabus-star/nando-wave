# NANDA Task: phase-stream-serving-append-shadow-replay-v2-extractor

## query

Check that the v2 Codex-session tool-status extractor reads current
response_item tool call/output metadata and writes only privacy-safe phase atom
trace records, not raw request, response, or tool output text.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_format_extractor | reads | response_item_tool_call_metadata | crates/nando-cli/src/phase_streaming_cmd.rs#parse_session_tool_call_meta |
| live_format_extractor | reads | response_item_tool_output_status | crates/nando-cli/src/phase_streaming_cmd.rs#parse_session_tool_status_event_from_tool_output |
| live_format_trace_report | counts | response_item_tool_call_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#response_item_tool_call_events_seen |
| live_format_trace_report | counts | response_item_tool_output_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#response_item_tool_output_events_seen |
| live_format_trace_report | keeps_false | raw_tool_output_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_tool_output_written |
| live_format_trace_report | keeps_false | raw_request_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_request_text_written |
| live_format_trace_report | keeps_false | raw_response_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_response_text_written |
| live_format_trace_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#local_accept_enabled |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| live_format_extractor | reads | response_item_tool_call_metadata | crates/nando-cli/src/phase_streaming_cmd.rs#parse_session_tool_call_meta |
| live_format_extractor | reads | response_item_tool_output_status | crates/nando-cli/src/phase_streaming_cmd.rs#parse_session_tool_status_event_from_tool_output |
| live_format_trace_report | counts | response_item_tool_call_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#response_item_tool_call_events_seen |
| live_format_trace_report | counts | response_item_tool_output_events | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#response_item_tool_output_events_seen |
| live_format_trace_report | keeps_false | raw_tool_output_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_tool_output_written |
| live_format_trace_report | keeps_false | raw_request_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_request_text_written |
| live_format_trace_report | keeps_false | raw_response_text_written | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#raw_response_text_written |
| live_format_trace_report | keeps_disabled | product_local_accept | target/nando-wave/streaming/codex-session-tool-status-verifier-trace-append-v2.report.json#local_accept_enabled |
