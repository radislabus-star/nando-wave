use serde::Serialize;
use serde_json::Value;

const DASHBOARD_BUILD: &str = "2026.08.10-control-v5";

#[derive(Clone, Copy, Debug)]
pub(crate) struct InitialMetrics {
    pub(crate) server_total_tokens: u64,
    pub(crate) server_cpu_tokens: u64,
    pub(crate) epoch_total_tokens: u64,
    pub(crate) epoch_total_events: u64,
    pub(crate) epoch_cpu_tokens: u64,
    pub(crate) epoch_cpu_accepts: u64,
    pub(crate) miner_window_total_tokens: u64,
    pub(crate) miner_window_total_intents: u64,
    pub(crate) miner_window_cpu_tokens: u64,
    pub(crate) miner_window_cpu_intents: u64,
    pub(crate) cpu_allowed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct BridgeView {
    pub(crate) hot_available: bool,
    pub(crate) cold_available: bool,
    pub(crate) hot_accepted: u64,
    pub(crate) cold_accepted: u64,
    pub(crate) loss: u64,
    pub(crate) queue: u64,
    pub(crate) hot_instance: String,
    pub(crate) cold_instance: String,
    pub(crate) structural_epoch_match: bool,
    pub(crate) structural_produced_sequence: u64,
    pub(crate) structural_consumed_sequence: u64,
    pub(crate) structural_pending: u64,
    pub(crate) structural_sequence_gaps: u64,
    pub(crate) structures_applied: u64,
    pub(crate) join_attempts: u64,
    pub(crate) join_hits: u64,
    pub(crate) join_misses: u64,
    pub(crate) opportunity_produced_sequence: u64,
    pub(crate) opportunity_consumed_sequence: u64,
    pub(crate) opportunity_counter_epoch_match: bool,
    pub(crate) opportunity_counter_epoch_reason: String,
    pub(crate) producer_counter_started_after_sequence: u64,
    pub(crate) consumer_counter_started_after_sequence: u64,
    pub(crate) hot_started_at_unix_ms: u64,
    pub(crate) cold_started_at_unix_ms: u64,
    pub(crate) request_events: u64,
    pub(crate) request_tokens: u64,
    pub(crate) miner_request_events: u64,
    pub(crate) miner_request_tokens: u64,
    pub(crate) opportunity_pending: u64,
    pub(crate) opportunity_inflight: u64,
    pub(crate) raw_evaluated: u64,
    pub(crate) raw_verified: u64,
    pub(crate) raw_abstains: u64,
    pub(crate) failures: u64,
    pub(crate) false_accepts: u64,
    pub(crate) parity_mismatches: u64,
    pub(crate) execution_authority: bool,
    pub(crate) services_active: u64,
}

pub(crate) fn build_id() -> &'static str {
    DASHBOARD_BUILD
}

pub(crate) fn bridge_view(hot: &Value, cold: &Value) -> BridgeView {
    let hot_available = pointer_bool(hot, "/ok");
    let cold_available = pointer_bool(cold, "/ok");
    let hot_failures = pointer_u64(hot, "/durable_structure/producer/failures");
    let cold_failures = pointer_u64(cold, "/durable_structure/consumer/failures");
    let hot_epoch = pointer_string(hot, "/durable_structure/bridge_epoch_sha256");
    let cold_epoch = pointer_string(cold, "/durable_structure/bridge_epoch_sha256");
    let structural_epoch_match = !hot_epoch.is_empty() && hot_epoch == cold_epoch;
    let structural_produced_sequence =
        pointer_u64(hot, "/durable_structure/producer/last_sequence");
    let structural_consumed_sequence =
        pointer_u64(cold, "/durable_structure/consumer/last_sequence");
    let structural_pending = pointer_u64(hot, "/durable_structure/pending_records")
        .max(pointer_u64(cold, "/durable_structure/pending_records"));
    let opportunity_pending = cold
        .pointer("/opportunity/pending_events")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| pointer_u64(hot, "/opportunity/pending_events"));
    let opportunity_inflight = pointer_u64(cold, "/opportunity/consumer_inflight_events");
    let hot_started_at_unix_ms = pointer_u64(hot, "/process/started_at_unix_ms");
    let cold_started_at_unix_ms = pointer_u64(cold, "/process/started_at_unix_ms");
    let producer_counter_started_after_sequence =
        pointer_u64(hot, "/opportunity/producer_counter_started_after_sequence");
    let consumer_counter_started_after_sequence =
        pointer_u64(cold, "/opportunity/consumer_counter_started_after_sequence");
    let producer_watermark_present = hot
        .pointer("/opportunity/producer_counter_started_after_sequence")
        .and_then(Value::as_u64)
        .is_some();
    let consumer_watermark_present = cold
        .pointer("/opportunity/consumer_counter_started_after_sequence")
        .and_then(Value::as_u64)
        .is_some();
    let opportunity_produced_sequence = pointer_u64(hot, "/opportunity/producer_last_sequence");
    let opportunity_consumed_sequence = pointer_u64(cold, "/opportunity/consumer_last_sequence");
    let empty_spool_sequence_divergence = opportunity_pending == 0
        && opportunity_inflight == 0
        && opportunity_produced_sequence != opportunity_consumed_sequence;
    let (opportunity_counter_epoch_match, opportunity_counter_epoch_reason) =
        if !producer_watermark_present || !consumer_watermark_present {
            (false, "counter_watermark_missing")
        } else if producer_counter_started_after_sequence != consumer_counter_started_after_sequence
        {
            (false, "counter_watermark_mismatch")
        } else if empty_spool_sequence_divergence {
            (false, "empty_spool_sequence_divergence")
        } else {
            (true, "common_counter_epoch")
        };

    BridgeView {
        hot_available,
        cold_available,
        hot_accepted: structural_produced_sequence,
        cold_accepted: structural_consumed_sequence,
        loss: if structural_epoch_match {
            structural_pending
        } else {
            0
        },
        queue: structural_pending.saturating_add(opportunity_pending),
        hot_instance: pointer_string(hot, "/process/instance_id_sha256"),
        cold_instance: pointer_string(cold, "/process/instance_id_sha256"),
        structural_epoch_match,
        structural_produced_sequence,
        structural_consumed_sequence,
        structural_pending,
        structural_sequence_gaps: pointer_u64(cold, "/durable_structure/sequence_gaps"),
        structures_applied: pointer_u64(cold, "/request_learning/structures_applied"),
        join_attempts: pointer_u64(cold, "/request_learning/lookup_attempts"),
        join_hits: pointer_u64(cold, "/request_learning/lookup_hits"),
        join_misses: pointer_u64(cold, "/request_learning/lookup_misses"),
        opportunity_produced_sequence,
        opportunity_consumed_sequence,
        opportunity_counter_epoch_match,
        opportunity_counter_epoch_reason: opportunity_counter_epoch_reason.to_owned(),
        producer_counter_started_after_sequence,
        consumer_counter_started_after_sequence,
        hot_started_at_unix_ms,
        cold_started_at_unix_ms,
        request_events: pointer_u64(hot, "/opportunity/producer_request_events"),
        request_tokens: pointer_u64(hot, "/opportunity/producer_request_input_tokens"),
        miner_request_events: pointer_u64(cold, "/opportunity/consumer_request_events"),
        miner_request_tokens: pointer_u64(cold, "/opportunity/consumer_request_input_tokens"),
        opportunity_pending,
        opportunity_inflight,
        raw_evaluated: pointer_u64(cold, "/raw_replay/evaluated"),
        raw_verified: pointer_u64(cold, "/raw_replay/verified"),
        raw_abstains: pointer_u64(cold, "/raw_replay/runtime_abstains"),
        failures: hot_failures
            .saturating_add(cold_failures)
            .saturating_add(pointer_u64(hot, "/opportunity/failures"))
            .saturating_add(pointer_u64(cold, "/opportunity/failures")),
        false_accepts: pointer_u64(cold, "/raw_replay/false_accepts"),
        parity_mismatches: pointer_u64(cold, "/raw_replay/parity_mismatches"),
        execution_authority: cold
            .pointer("/raw_replay/execution_authority")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        services_active: u64::from(hot_available)
            .saturating_add(u64::from(cold_available))
            .saturating_add(1),
    }
}

pub(crate) fn render(initial: InitialMetrics) -> String {
    let miner_unrecognized = initial
        .miner_window_total_tokens
        .saturating_sub(initial.miner_window_cpu_tokens);
    let cpu_gate = if initial.cpu_allowed {
        "ОТКРЫТ"
    } else {
        "ЗАКРЫТ"
    };
    let cpu_gate_tone = if initial.cpu_allowed { "good" } else { "bad" };
    TEMPLATE
        .replace("__DASHBOARD_BUILD__", DASHBOARD_BUILD)
        .replace(
            "__EPOCH_TOTAL__",
            &format_number(initial.epoch_total_tokens),
        )
        .replace("__EPOCH_CPU__", &format_number(initial.epoch_cpu_tokens))
        .replace(
            "__EPOCH_SHARE__",
            &format_percent(initial.epoch_cpu_tokens, initial.epoch_total_tokens, 2),
        )
        .replace(
            "__EPOCH_REQUESTS__",
            &format_number(initial.epoch_total_events),
        )
        .replace(
            "__EPOCH_ACCEPTS__",
            &format_number(initial.epoch_cpu_accepts),
        )
        .replace(
            "__LIFETIME_TOTAL__",
            &format_number(initial.server_total_tokens),
        )
        .replace(
            "__LIFETIME_CPU__",
            &format_number(initial.server_cpu_tokens),
        )
        .replace(
            "__LIFETIME_SHARE__",
            &format_percent(initial.server_cpu_tokens, initial.server_total_tokens, 2),
        )
        .replace(
            "__MINER_SEEN__",
            &format_number(initial.miner_window_total_tokens),
        )
        .replace(
            "__MINER_SEEN_INTENTS__",
            &format_number(initial.miner_window_total_intents),
        )
        .replace(
            "__MINER_RECOGNIZED__",
            &format_number(initial.miner_window_cpu_tokens),
        )
        .replace(
            "__MINER_RECOGNIZED_INTENTS__",
            &format_number(initial.miner_window_cpu_intents),
        )
        .replace(
            "__MINER_RECOGNIZED_SHARE__",
            &format_percent(
                initial.miner_window_cpu_tokens,
                initial.miner_window_total_tokens,
                2,
            ),
        )
        .replace("__MINER_UNRECOGNIZED__", &format_number(miner_unrecognized))
        .replace("__CPU_GATE__", cpu_gate)
        .replace("__CPU_GATE_TONE__", cpu_gate_tone)
}

fn pointer_u64(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap_or(0)
}

fn pointer_bool(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn pointer_string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push(' ');
        }
        result.push(character);
    }
    result
}

fn format_percent(numerator: u64, denominator: u64, decimals: usize) -> String {
    if denominator == 0 {
        return format!("0,{}%", "0".repeat(decimals));
    }
    let percent = numerator as f64 * 100.0 / denominator as f64;
    format!("{percent:.decimals$}%").replace('.', ",")
}

const TEMPLATE: &str = r#"
<style>
.nando-live {
  --bg:#0d1012;
  --line:#30363a;
  --text:#edf0f1;
  --muted:#90999e;
  --green:#72c98a;
  --amber:#c9a75c;
  --red:#df746e;
  width:100%; max-width:none; min-height:100vh; margin:0; padding:0;
  background:var(--bg); color:var(--text); font-family:Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif; letter-spacing:0;
}
.nando-live * { box-sizing:border-box; letter-spacing:0; }
.nd-inner { width:min(1040px,100%); margin:0 auto; padding:24px 28px; }
.nd-head { border-bottom:1px solid var(--line); background:#090b0c; }
.nd-head .nd-inner { display:flex; align-items:center; justify-content:space-between; gap:20px; min-height:64px; padding-top:14px; padding-bottom:14px; }
.nd-brand { display:flex; align-items:baseline; gap:10px; min-width:0; }
.nd-brand strong { color:#fff; font-size:21px; font-weight:760; }
.nd-brand span { color:var(--muted); font-size:12px; font-weight:600; }
.nd-live { color:var(--muted); font-size:12px; font-weight:600; text-align:right; }
.nd-live b { color:var(--green); }
.nd-band { border-bottom:1px solid var(--line); }
.nd-status { color:var(--amber); font-weight:700; }
.nd-status.good { color:var(--green); }
.nd-status.bad { color:var(--red); }
.result-kicker { color:var(--muted); font-size:12px; font-weight:650; }
.coverage-title { max-width:820px; margin:12px 0 0; color:#f4f6f7; font-size:42px; font-weight:720; line-height:1.12; }
.coverage-title output { color:var(--green); font:inherit; }
.coverage-fraction { margin:18px 0 0; color:#b7bec2; font-size:15px; line-height:1.5; }
.coverage-fraction strong { color:#f0f3f4; font-weight:680; }
.ratio-rail { height:7px; margin-top:18px; background:#282d30; overflow:hidden; }
.ratio-fill { width:0; height:100%; background:var(--green); transition:width .2s ease; }
.result-meta { display:flex; flex-wrap:wrap; gap:8px 24px; margin-top:14px; color:var(--muted); font-size:12px; line-height:1.5; }
.result-meta strong { color:#dfe3e5; font-weight:680; }
.result-meta .epoch-result { color:var(--green); }
.status-row { display:grid; grid-template-columns:180px minmax(0,1fr); gap:34px; padding:25px 0; border-bottom:1px solid var(--line); }
.status-row:last-child { border-bottom:0; }
.status-name h2 { margin:0; color:#e1e5e7; font-size:15px; font-weight:700; }
.status-scope { margin-top:6px; color:var(--muted); font-size:11px; line-height:1.45; }
.status-content { min-width:0; }
.miner-sentence { display:flex; flex-wrap:wrap; gap:10px 34px; color:#bdc4c7; font-size:15px; line-height:1.45; }
.miner-sentence strong { display:block; margin-top:4px; color:#f1f3f4; font-size:24px; font-weight:700; overflow-wrap:anywhere; }
.miner-sentence .recognized strong,.miner-share { color:var(--green); }
.miner-progress { display:grid; grid-template-columns:minmax(0,1fr) auto; align-items:center; gap:14px; margin-top:14px; }
.miner-progress .ratio-rail { margin-top:0; }
.miner-share { font-size:16px; font-weight:700; }
.scope-note { margin-top:10px; color:var(--muted); font-size:11px; line-height:1.5; }
.law-state strong { display:block; color:var(--amber); font-size:24px; font-weight:700; line-height:1.25; }
.law-summary { margin-top:9px; color:#c7cdd0; font-size:14px; line-height:1.5; }
.law-summary b { color:#edf0f1; font-weight:700; }
.next-transition { margin-top:10px; color:#aeb7bb; font-size:12px; line-height:1.5; }
.next-transition strong { color:var(--muted); font-weight:650; }
.blocker-list { display:flex; flex-wrap:wrap; gap:5px 15px; margin-top:7px; color:var(--muted); font-size:11px; }
.blocker-list b { color:#bdc4c7; }
.technical-details > summary { display:flex; justify-content:space-between; gap:20px; padding:16px 0; color:#aeb7bb; cursor:pointer; font-size:12px; font-weight:650; list-style:none; }
.technical-details > summary::-webkit-details-marker { display:none; }
.technical-details > summary::before { content:"+"; flex:0 0 18px; color:var(--muted); }
.technical-details[open] > summary::before { content:"−"; }
.technical-details > summary .summary-meta { margin-left:auto; color:var(--muted); font-size:10px; font-weight:550; text-align:right; }
.technical-body { padding:4px 0 22px; }
.diagnostic-summary { display:flex; flex-wrap:wrap; gap:6px 18px; color:var(--muted); font-size:11px; }
.diagnostic-summary b { color:#d7dcde; font-weight:680; }
.technical-section { margin-top:20px; }
.technical-title { margin:0 0 10px; color:var(--muted); font-size:11px; font-weight:650; }
.package-table { border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.package-row { display:grid; grid-template-columns:minmax(260px,2fr) repeat(4,minmax(105px,.7fr)); gap:12px; align-items:center; min-height:50px; border-bottom:1px solid var(--line); }
.package-row:last-child { border-bottom:0; }
.package-row.header { min-height:30px; color:var(--muted); font-size:10px; font-weight:650; }
.package-id { min-width:0; color:#dce2e5; font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:11px; overflow-wrap:anywhere; }
.certificate { color:var(--muted); font-size:11px; font-weight:650; }
.certificate.good { color:var(--green); }
.certificate.watch { color:var(--amber); }
.certificate.bad { color:var(--red); }
.k1-line { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border-bottom:1px solid var(--line); }
.k1-cell { min-width:0; padding:13px 18px; border-right:1px solid var(--line); }
.k1-cell:last-child { border-right:0; }
.k1-cell span { display:block; color:var(--muted); font-size:10px; font-weight:650; }
.k1-cell strong { display:block; margin-top:5px; color:#dce2e5; font-size:16px; font-weight:680; }
.k1-cell.closed strong { color:var(--amber); }
.discovery-detail { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.discovery-detail > div { padding:12px 16px; border-right:1px solid var(--line); }
.discovery-detail > div:last-child { border-right:0; }
.discovery-detail span { display:block; color:var(--muted); font-size:10px; }
.discovery-detail strong { display:block; margin-top:5px; color:#dce2e5; font-size:15px; }
.safety-grid { display:grid; grid-template-columns:repeat(5,minmax(0,1fr)); border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.safety-cell { min-width:0; padding:14px 16px; border-right:1px solid var(--line); }
.safety-cell:last-child { border-right:0; }
.safety-cell span { display:block; color:var(--muted); font-size:10px; font-weight:650; }
.safety-cell strong { display:block; margin-top:6px; color:var(--green); font-size:15px; font-weight:680; overflow-wrap:anywhere; }
.safety-cell strong.watch { color:var(--amber); }
.safety-cell strong.bad { color:var(--red); }
.quiet-details { margin-top:12px; border-top:1px solid var(--line); }
.quiet-details summary { padding:12px 0; color:#aeb7bc; cursor:pointer; font-size:12px; font-weight:800; list-style:none; }
.quiet-details summary::-webkit-details-marker { display:none; }
.quiet-details summary::before { content:"+"; display:inline-block; width:18px; color:var(--muted); }
.quiet-details[open] summary::before { content:"-"; }
.window-table { padding-bottom:8px; }
.window-row { display:grid; grid-template-columns:minmax(170px,1.2fr) minmax(90px,.6fr) minmax(110px,.7fr) minmax(0,1.5fr); gap:14px; align-items:center; min-height:38px; border-top:1px solid #252b2f; font-size:11px; }
.window-row.header { color:var(--muted); font-size:10px; font-weight:800; }
.window-route { color:var(--green); font-weight:800; }
.window-route.mixed { color:var(--amber); }
.window-route.outside_nando { color:var(--red); }
.window-route.idle { color:var(--muted); }
.nd-foot .nd-inner { display:flex; justify-content:space-between; gap:20px; padding-top:12px; padding-bottom:12px; color:#667075; font-size:9px; }
.flow-intro { padding-top:34px; padding-bottom:18px; }
.flow-eyebrow { margin:0; color:var(--muted); font-size:11px; font-weight:700; }
.flow-title { margin:7px 0 0; color:#f4f6f7; font-size:30px; font-weight:730; line-height:1.2; }
.flow-subtitle { margin:8px 0 0; color:#9ea7ab; font-size:13px; line-height:1.5; }
.traffic-flow { margin:0; padding:0; list-style:none; border-top:1px solid var(--line); }
.flow-step { display:grid; grid-template-columns:34px minmax(190px,.7fr) minmax(0,1.3fr); gap:22px; align-items:center; min-height:126px; padding:24px 0; border-bottom:1px solid var(--line); }
.flow-index { align-self:start; padding-top:3px; color:#687277; font:650 11px/1 ui-monospace,SFMono-Regular,Menlo,monospace; }
.flow-label { min-width:0; }
.flow-label h2 { margin:0; color:#dfe4e6; font-size:15px; font-weight:720; line-height:1.3; }
.flow-scope { margin:8px 0 0; color:var(--muted); font-size:11px; line-height:1.45; }
.flow-value { min-width:0; }
.flow-value strong { display:block; color:#f5f7f8; font-size:34px; font-weight:730; line-height:1.05; overflow-wrap:anywhere; }
.flow-value strong.good { color:var(--green); }
.flow-value > span { display:block; margin-top:9px; color:#aeb6ba; font-size:12px; line-height:1.45; }
.flow-value output { color:var(--green); font:700 15px/1.2 inherit; }
.flow-value .scope-warning { color:var(--amber); }
.law-panel { display:grid; grid-template-columns:34px minmax(190px,.7fr) minmax(0,1.3fr); gap:22px; padding-top:30px; padding-bottom:32px; }
.law-label h2 { margin:0; color:#e4e8ea; font-size:15px; font-weight:720; }
.law-label p { margin:8px 0 0; color:var(--muted); font-size:11px; line-height:1.45; }
.law-verdict { display:block; color:var(--amber); font-size:28px; font-weight:740; line-height:1.15; }
.law-verdict.good { color:var(--green); }
.law-verdict.bad { color:var(--red); }
.law-facts { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); margin-top:18px; border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.law-fact { min-width:0; padding:13px 14px; border-right:1px solid var(--line); }
.law-fact:first-child { padding-left:0; }
.law-fact:last-child { padding-right:0; border-right:0; }
.law-fact span { display:block; color:var(--muted); font-size:10px; line-height:1.35; }
.law-fact strong { display:block; margin-top:5px; color:#e2e6e8; font-size:17px; font-weight:700; overflow-wrap:anywhere; }
.law-exit { margin:14px 0 0; color:#b9c0c3; font-size:12px; line-height:1.55; }
.law-exit b { color:#e2e6e8; font-weight:680; }
.safety-line { display:flex; flex-wrap:wrap; gap:6px 22px; color:#7f898e; font-size:10px; }
.safety-line b { color:#bfc6c9; font-weight:680; }
@media (max-width:900px) {
  .safety-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .safety-cell { border-bottom:1px solid var(--line); }
  .safety-cell:nth-child(2n) { border-right:0; }
  .safety-cell:last-child { grid-column:1 / -1; border-bottom:0; }
}
@media (max-width:680px) {
  .nd-inner { padding:20px 16px; }
  .nd-head .nd-inner,.nd-foot .nd-inner { align-items:flex-start; flex-direction:column; gap:7px; }
  .nd-live { text-align:left; }
  .coverage-title { font-size:32px; }
  .coverage-fraction { font-size:14px; }
  .result-meta { flex-direction:column; gap:4px; }
  .status-row { grid-template-columns:1fr; gap:14px; padding:22px 0; }
  .miner-sentence { display:grid; grid-template-columns:1fr; gap:13px; }
  .miner-sentence strong { font-size:22px; }
  .technical-details > summary { align-items:flex-start; }
  .technical-details > summary .summary-meta { display:none; }
  .package-row { grid-template-columns:minmax(0,1fr); gap:3px; padding:11px 0; }
  .package-row.header { display:none; }
  .certificate::before { color:var(--muted); }
  .certificate.execution::before { content:"CPU "; }
  .certificate.law::before { content:"LAW "; }
  .certificate.mechanism::before { content:"MECHANISM "; }
  .certificate.k1::before { content:"K1 "; }
  .k1-line,.safety-grid,.discovery-detail { grid-template-columns:1fr; }
  .k1-cell,.safety-cell,.safety-cell:last-child,.discovery-detail > div { grid-column:auto; padding:12px 0; border-right:0; border-bottom:1px solid var(--line); }
  .k1-cell:last-child,.safety-cell:last-child,.discovery-detail > div:last-child { border-bottom:0; }
  .window-row { grid-template-columns:1fr; gap:2px; padding:8px 0; }
  .window-row.header { display:none; }
  .flow-intro { padding-top:26px; padding-bottom:14px; }
  .flow-title { font-size:26px; }
  .flow-step,.law-panel { grid-template-columns:26px minmax(0,1fr); gap:10px 12px; min-height:0; padding:21px 0; }
  .flow-value,.law-content { grid-column:2; }
  .flow-value strong { font-size:28px; }
  .law-verdict { font-size:24px; }
  .law-facts { grid-template-columns:1fr; }
  .law-fact,.law-fact:first-child,.law-fact:last-child { padding:11px 0; border-right:0; border-bottom:1px solid var(--line); }
  .law-fact:last-child { border-bottom:0; }
}
</style>
<main class="nando-live" data-dashboard-build="__DASHBOARD_BUILD__" aria-label="Nando live control">
  <header class="nd-head"><div class="nd-inner">
    <div class="nd-brand"><strong>Nando</strong><span>live control</span></div>
    <div class="nd-live"><b>Live</b> · <span id="snapshot-age">0</span> с · источник <span id="source-age">—</span> с · сервисы <span id="services">—/3</span></div>
  </div></header>

  <section class="nd-band"><div class="nd-inner flow-intro">
    <p class="flow-eyebrow">Фактические токены · без прогнозов</p>
    <h1 class="flow-title">Трафик → CPU</h1>
    <p class="flow-subtitle">Сервер и майнер имеют разные начальные точки. Доли считаются только внутри своего окна.</p>
  </div></section>

  <section class="nd-band"><div class="nd-inner">
    <ol class="traffic-flow">
      <li class="flow-step">
        <span class="flow-index">01</span>
        <div class="flow-label"><h2>Весь трафик сервера</h2><p class="flow-scope">вся записанная accounting history</p></div>
        <div class="flow-value"><strong id="lifetime-total">__LIFETIME_TOTAL__</strong><span>входных токенов</span></div>
      </li>
      <li class="flow-step">
        <span class="flow-index">02</span>
        <div class="flow-label"><h2>Майнер увидел</h2><p id="miner-window-start" class="flow-scope">отдельное окно майнера</p></div>
        <div class="flow-value"><strong id="miner-seen">__MINER_SEEN__</strong><span><b id="miner-seen-intents">__MINER_SEEN_INTENTS__</b> запросов · <span class="scope-warning">не делить на строку 01</span></span></div>
      </li>
      <li class="flow-step">
        <span class="flow-index">03</span>
        <div class="flow-label"><h2>Майнер распознал</h2><p class="flow-scope">внутри того же окна майнера</p></div>
        <div class="flow-value"><strong id="miner-recognized" class="good">__MINER_RECOGNIZED__</strong><span><output id="miner-share">__MINER_RECOGNIZED_SHARE__</output> от увиденного · <b id="miner-recognized-intents">__MINER_RECOGNIZED_INTENTS__</b> запросов</span></div>
      </li>
      <li class="flow-step">
        <span class="flow-index">04</span>
        <div class="flow-label"><h2>Воспроизведено на CPU</h2><p class="flow-scope">доказанный результат по всей истории сервера</p></div>
        <div class="flow-value"><strong id="lifetime-cpu" class="good">__LIFETIME_CPU__</strong><span><output id="lifetime-share">__LIFETIME_SHARE__</output> от строки 01 · допуск CPU <b id="cpu-gate" class="nd-status __CPU_GATE_TONE__">__CPU_GATE__</b></span></div>
      </li>
    </ol>
  </div></section>

  <section class="nd-band"><div class="nd-inner law-panel">
    <span class="flow-index">05</span>
    <div class="law-label"><h2>Закон №2</h2><p>следующий естественный закон K1</p></div>
    <div class="law-content">
      <strong id="law2-state" class="law-verdict">ЗАГРУЗКА</strong>
      <div class="law-facts">
        <div class="law-fact"><span>Readiness достигли</span><strong id="historical-ready">—</strong></div>
        <div class="law-fact"><span>Доступно сейчас</span><strong id="ready-now">—</strong></div>
        <div class="law-fact"><span>K1 laws</span><strong id="k1-laws-main">—</strong></div>
      </div>
      <p class="law-exit">Последний исход: <b id="latest-verdict">—</b>. Причина: <b id="law2-blocker">—</b>. Следующая generation: <b id="next-generation">—</b>.</p>
    </div>
  </div></section>

  <footer class="nd-foot"><div class="nd-inner"><div class="safety-line"><span>false accepts <b id="false-accepts">—</b></span><span>parity failures <b id="parity-failures">—</b></span></div><span>build __DASHBOARD_BUILD__</span></div></footer>
</main>
<script>
(() => {
  const base = window.location.pathname.replace(/\/legacy$/, "").replace(/\/$/, "");
  const expectedBuild = document.querySelector(".nando-live")?.dataset.dashboardBuild || "";
  const number = new Intl.NumberFormat("ru-RU");
  let lastSuccess = Date.now();
  let sourceGeneratedAt = 0;
  let refreshing = false;
  const node = id => document.getElementById(id);
  const text = (id, value) => { const target = node(id); if (target) target.textContent = value; };
  const className = (id, value) => { const target = node(id); if (target) target.className = value; };
  const percent = (part, total, digits = 2) => total > 0 ? `${(part * 100 / total).toFixed(digits).replace(".", ",")}%` : `0,${"0".repeat(digits)}%`;
  const bar = (id, part, total) => { const target = node(id); if (target) target.style.width = total > 0 ? `${Math.min(100, part * 100 / total)}%` : "0"; };
  const localTime = unix => unix > 0 ? new Date(unix * 1000).toLocaleString("ru-RU", {dateStyle:"short", timeStyle:"medium"}) : "—";
  const readable = value => ({
    waiting_for_evidence:"Ждёт повторяемую когорту",
    no_readiness_pass_candidate:"нет готовой повторяемой когорты",
    settled_evidence_below_freeze_minimum:"мало завершённых наблюдений",
    independent_lineages_below_freeze_minimum:"мало независимых lineage",
    selected_role_witness_missing:"capture не сохранил типизированную роль",
    acquisition_fail:"ACQUISITION FAIL",
    probe_pending:"PROBE PENDING",
    probe_exhausted:"PROBE EXHAUSTED",
    abstain:"ABSTAIN",
    pass:"PASS",
  }[value] || String(value || "—").replaceAll("_", " "));

  const renderDashboard = snapshot => {
    if (!snapshot?.available) return;
    if (snapshot.dashboard_build && snapshot.dashboard_build !== expectedBuild) {
      window.location.reload();
      return;
    }
    const lifetime = snapshot.product?.lifetime || {};
    const miner = snapshot.miner || {};
    const discovery = snapshot.discovery || {};
    const safety = snapshot.safety || {};
    const k1 = snapshot.k1 || {};
    sourceGeneratedAt = snapshot.generated_at_unix || 0;

    text("lifetime-total", number.format(lifetime.input_tokens || 0));
    text("lifetime-cpu", number.format(lifetime.cpu_tokens || 0));
    text("lifetime-share", percent(lifetime.cpu_tokens || 0, lifetime.input_tokens || 0));
    text("cpu-gate", safety.cpu_allowed ? "ОТКРЫТ" : "ЗАКРЫТ");
    className("cpu-gate", `nd-status ${safety.cpu_allowed ? "good" : "bad"}`);

    text("miner-seen", number.format(miner.seen_tokens || 0));
    text("miner-seen-intents", number.format(miner.seen_intents || 0));
    text("miner-recognized", number.format(miner.recognized_tokens || 0));
    text("miner-recognized-intents", number.format(miner.recognized_intents || 0));
    text("miner-share", percent(miner.recognized_tokens || 0, miner.seen_tokens || 0));
    text("miner-window-start", `окно с ${localTime(miner.started_at_unix || 0)} · отдельный watermark`);

    const lawCount = k1.law_certificates || 0;
    const law2Proved = lawCount >= 2;
    const law2Active = discovery.active_candidate === true;
    text("law2-state", law2Proved ? "ДОКАЗАН" : law2Active ? "ПРОВЕРЯЕТСЯ" : "НЕ ДОКАЗАН");
    className("law2-state", `law-verdict ${law2Proved ? "good" : ""}`);
    text("historical-ready", number.format(discovery.historical_readiness_pass || 0));
    text("ready-now", number.format(discovery.ready_now || 0));
    text("latest-verdict", readable(discovery.latest_verdict));
    text("law2-blocker", readable(discovery.latest_verdict_blocker || discovery.blocker));
    text("next-generation", number.format(discovery.next_generation_sequence || 0));

    text("false-accepts", number.format(safety.false_accepts || 0));
    text("parity-failures", number.format(safety.parity_failures || 0));
    text("k1-laws-main", `${number.format(k1.law_certificates || 0)} / ${number.format(k1.min_law_certificates || 3)}`);

    text("services", `${number.format(safety.services_active || 0)}/${number.format(safety.services_expected || 3)}`);
    lastSuccess = Date.now();
  };

  const refreshDashboard = async () => {
    if (refreshing) return;
    refreshing = true;
    try {
      const response = await fetch(`${base}/api/v1/dashboard`, {cache:"no-store"});
      if (!response.ok) return;
      const payload = await response.text();
      renderDashboard(JSON.parse(payload));
    } catch (_) {
    } finally {
      refreshing = false;
    }
  };
  window.setInterval(() => {
    text("snapshot-age", Math.floor((Date.now() - lastSuccess) / 1000));
    text("source-age", sourceGeneratedAt > 0 ? Math.max(0, Math.floor(Date.now() / 1000) - sourceGeneratedAt) : "—");
  }, 1000);
  refreshDashboard();
  window.setInterval(refreshDashboard, 3000);
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn bridge_view_keeps_hot_and_cold_owners_separate() {
        let hot = json!({"ok":true,"process":{"instance_id_sha256":"hot","started_at_unix_ms":1_000},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"producer":{"last_sequence":45,"failures":0}},"opportunity":{"producer_last_sequence":45,"producer_counter_started_after_sequence":21,"pending_events":2,"producer_request_events":20,"producer_request_input_tokens":6_876_562,"failures":0}});
        let cold = json!({"ok":true,"process":{"instance_id_sha256":"cold","started_at_unix_ms":3_000},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"sequence_gaps":0,"consumer":{"last_sequence":43,"failures":0}},"request_learning":{"structures_applied":43,"lookup_attempts":20,"lookup_hits":17,"lookup_misses":3},"opportunity":{"consumer_last_sequence":44,"consumer_counter_started_after_sequence":21,"consumer_request_events":18,"consumer_request_input_tokens":6_000_000,"consumer_inflight_events":1,"failures":0},"raw_replay":{"evaluated":12,"verified":3,"runtime_abstains":9,"execution_authority":false,"false_accepts":0,"parity_mismatches":0}});
        let view = bridge_view(&hot, &cold);
        assert!(view.hot_available);
        assert!(view.cold_available);
        assert!(view.structural_epoch_match);
        assert_eq!(view.structural_produced_sequence, 45);
        assert_eq!(view.structural_consumed_sequence, 43);
        assert_eq!(view.opportunity_pending, 2);
        assert_eq!(view.opportunity_inflight, 1);
        assert_eq!(view.request_tokens, 6_876_562);
        assert_eq!(view.miner_request_tokens, 6_000_000);
        assert_eq!(view.queue, 4);
    }

    #[test]
    fn dashboard_shows_only_the_two_comparable_traffic_pairs_and_law_two() {
        let html = render(InitialMetrics {
            server_total_tokens: 7_694_807_361,
            server_cpu_tokens: 207_619_587,
            epoch_total_tokens: 1_733_026_637,
            epoch_total_events: 5_704,
            epoch_cpu_tokens: 165_104_290,
            epoch_cpu_accepts: 677,
            miner_window_total_tokens: 10_882_437_482,
            miner_window_total_intents: 49_122,
            miner_window_cpu_tokens: 1_613_584_240,
            miner_window_cpu_intents: 9_832,
            cpu_allowed: true,
        });
        assert!(html.contains("Трафик → CPU"));
        assert!(html.contains("Весь трафик сервера"));
        assert!(html.contains("7 694 807 361"));
        assert!(html.contains("207 619 587"));
        assert!(html.contains("2,70%"));
        assert!(html.contains("10 882 437 482"));
        assert!(html.contains("1 613 584 240"));
        assert!(html.contains("не делить на строку 01"));
        assert!(html.contains("Закон №2"));
        assert!(html.contains("id=\"latest-verdict\""));
        assert_eq!(html.matches("class=\"flow-step\"").count(), 4);
        assert!(html.contains("/api/v1/dashboard"));
        assert!(!html.contains("Диагностика"));
        assert!(!html.contains("Доказанные пакеты"));
        assert!(!html.contains("/connections"));
        assert!(!html.contains("CANDIDATE INPUT"));
        assert!(!html.contains("CRYSTALLIZER"));
        assert!(!html.contains("/tokens"));
        assert!(html.contains(&format!("data-dashboard-build=\"{DASHBOARD_BUILD}\"")));
    }

    #[test]
    fn locked_cpu_authority_is_visible_before_refresh() {
        let html = render(InitialMetrics {
            server_total_tokens: 100,
            server_cpu_tokens: 0,
            epoch_total_tokens: 50,
            epoch_total_events: 2,
            epoch_cpu_tokens: 0,
            epoch_cpu_accepts: 0,
            miner_window_total_tokens: 40,
            miner_window_total_intents: 2,
            miner_window_cpu_tokens: 0,
            miner_window_cpu_intents: 0,
            cpu_allowed: false,
        });
        assert!(html.contains("class=\"nd-status bad\">ЗАКРЫТ"));
        assert!(!html.contains("__CPU_GATE__"));
    }
}
