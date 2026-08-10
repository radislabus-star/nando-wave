use serde::Serialize;
use serde_json::Value;

const DASHBOARD_BUILD: &str = "2026.08.10-control-v1";

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
        "OPEN"
    } else {
        "LOCKED"
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
  --bg:#0b0e10;
  --panel:#101417;
  --line:#30373c;
  --line-strong:#4b555c;
  --text:#edf1f3;
  --muted:#8b969d;
  --green:#63d487;
  --cyan:#55bde9;
  --amber:#e0b454;
  --red:#f26c64;
  width:100%; max-width:none; min-height:100vh; margin:0; padding:0;
  background:var(--bg); color:var(--text); letter-spacing:0;
}
.nando-live * { box-sizing:border-box; letter-spacing:0; }
.nd-inner { width:min(1320px,100%); margin:0 auto; padding:20px 24px; }
.nd-head { border-bottom:1px solid var(--line-strong); background:#080a0c; }
.nd-head .nd-inner { display:flex; align-items:center; justify-content:space-between; gap:20px; min-height:64px; }
.nd-brand { display:flex; align-items:baseline; gap:12px; min-width:0; }
.nd-brand strong { color:#fff; font-size:22px; }
.nd-brand span { color:var(--muted); font-size:12px; font-weight:800; }
.nd-live { color:var(--muted); font-size:12px; font-weight:700; text-align:right; }
.nd-live b { color:var(--green); }
.nd-band { border-bottom:1px solid var(--line); }
.nd-section-head { display:flex; align-items:baseline; justify-content:space-between; gap:18px; margin-bottom:14px; }
.nd-section-head h1,.nd-section-head h2 { margin:0; color:#e5eaed; font-size:14px; font-weight:800; }
.nd-scope { color:var(--muted); font-size:11px; font-weight:700; text-align:right; }
.nd-status { color:var(--amber); font-weight:800; }
.nd-status.good { color:var(--green); }
.nd-status.bad { color:var(--red); }
.product-flow { display:grid; grid-template-columns:minmax(0,1fr) 34px minmax(0,1fr) minmax(170px,.7fr); border:1px solid var(--line-strong); background:var(--panel); }
.flow-cell { min-width:0; min-height:148px; padding:18px 20px; border-right:1px solid var(--line); }
.flow-cell:last-child { border-right:0; }
.flow-arrow { display:flex; align-items:center; justify-content:center; color:var(--muted); border-right:1px solid var(--line); font-size:20px; }
.metric-label { color:var(--muted); font-size:11px; font-weight:800; }
.metric-value { display:block; max-width:100%; margin-top:10px; color:#f4f6f7; font-size:29px; font-weight:800; overflow-wrap:anywhere; }
.flow-cell.cpu .metric-value,.flow-cell.result .metric-value { color:var(--green); }
.metric-note { margin-top:7px; color:#b9c1c6; font-size:12px; line-height:1.45; overflow-wrap:anywhere; }
.metric-sub { margin-top:5px; color:var(--muted); font-size:10px; line-height:1.4; overflow-wrap:anywhere; }
.ratio-rail { height:8px; margin-top:13px; background:#262c30; overflow:hidden; }
.ratio-fill { width:0; height:100%; background:var(--green); transition:width .2s ease; }
.history-line { display:grid; grid-template-columns:minmax(0,1.4fr) repeat(2,minmax(150px,.8fr)); border:1px solid var(--line); border-top:0; }
.history-cell { min-width:0; padding:12px 18px; border-right:1px solid var(--line); }
.history-cell:last-child { border-right:0; }
.history-label { color:var(--muted); font-size:10px; font-weight:800; }
.history-value { display:block; margin-top:4px; color:#dce2e5; font-size:16px; font-weight:800; overflow-wrap:anywhere; }
.scope-warning { margin-top:10px; color:var(--amber); font-size:11px; font-weight:700; line-height:1.45; }
.miner-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.miner-cell { min-width:0; padding:17px 20px; border-right:1px solid var(--line); }
.miner-cell:last-child { border-right:0; }
.miner-cell.recognized .metric-value { color:var(--cyan); }
.miner-cell.unresolved .metric-value { color:var(--amber); }
.miner-progress { display:grid; grid-template-columns:minmax(0,1fr) auto; align-items:center; gap:14px; margin-top:13px; }
.miner-progress .ratio-fill { background:var(--cyan); }
.miner-share { color:var(--cyan); font-size:18px; font-weight:800; }
.discovery-summary { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border:1px solid var(--line); }
.discovery-cell { min-width:0; padding:15px 18px; border-right:1px solid var(--line); }
.discovery-cell:last-child { border-right:0; }
.discovery-cell.ready .metric-value { color:var(--amber); }
.discovery-cell .metric-value { font-size:23px; }
.next-transition { display:grid; grid-template-columns:190px minmax(0,1fr); gap:16px; margin-top:12px; padding:12px 15px; border-left:3px solid var(--amber); background:#121619; }
.next-transition strong { color:var(--amber); font-size:12px; }
.next-transition span { min-width:0; color:#d1d7da; font-size:13px; overflow-wrap:anywhere; }
.blocker-list { display:flex; flex-wrap:wrap; gap:8px 18px; margin-top:10px; color:var(--muted); font-size:11px; }
.blocker-list b { color:#c7cfd3; }
.package-table { border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.package-row { display:grid; grid-template-columns:minmax(260px,2fr) repeat(4,minmax(105px,.7fr)); gap:12px; align-items:center; min-height:50px; border-bottom:1px solid var(--line); }
.package-row:last-child { border-bottom:0; }
.package-row.header { min-height:30px; color:var(--muted); font-size:10px; font-weight:800; }
.package-id { min-width:0; color:#dce2e5; font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:11px; overflow-wrap:anywhere; }
.certificate { color:var(--muted); font-size:11px; font-weight:800; }
.certificate.good { color:var(--green); }
.certificate.watch { color:var(--amber); }
.certificate.bad { color:var(--red); }
.k1-line { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border-bottom:1px solid var(--line); }
.k1-cell { min-width:0; padding:13px 18px; border-right:1px solid var(--line); }
.k1-cell:last-child { border-right:0; }
.k1-cell span { display:block; color:var(--muted); font-size:10px; font-weight:800; }
.k1-cell strong { display:block; margin-top:5px; color:#dce2e5; font-size:16px; }
.k1-cell.closed strong { color:var(--amber); }
.safety-grid { display:grid; grid-template-columns:repeat(5,minmax(0,1fr)); border:1px solid var(--line); }
.safety-cell { min-width:0; padding:14px 16px; border-right:1px solid var(--line); }
.safety-cell:last-child { border-right:0; }
.safety-cell span { display:block; color:var(--muted); font-size:10px; font-weight:800; }
.safety-cell strong { display:block; margin-top:6px; color:var(--green); font-size:15px; overflow-wrap:anywhere; }
.safety-cell strong.watch { color:var(--amber); }
.safety-cell strong.bad { color:var(--red); }
.quiet-details { margin-top:14px; border-top:1px solid var(--line); }
.quiet-details summary { padding:12px 0; color:#aeb7bc; cursor:pointer; font-size:12px; font-weight:800; list-style:none; }
.quiet-details summary::-webkit-details-marker { display:none; }
.quiet-details summary::before { content:"+"; display:inline-block; width:18px; color:var(--cyan); }
.quiet-details[open] summary::before { content:"-"; }
.window-table { padding-bottom:8px; }
.window-row { display:grid; grid-template-columns:minmax(170px,1.2fr) minmax(90px,.6fr) minmax(110px,.7fr) minmax(0,1.5fr); gap:14px; align-items:center; min-height:38px; border-top:1px solid #252b2f; font-size:11px; }
.window-row.header { color:var(--muted); font-size:10px; font-weight:800; }
.window-route { color:var(--green); font-weight:800; }
.window-route.mixed { color:var(--amber); }
.window-route.outside_nando { color:var(--red); }
.window-route.idle { color:var(--muted); }
.nd-foot .nd-inner { display:flex; justify-content:space-between; gap:20px; color:var(--muted); font-size:11px; }
@media (max-width:900px) {
  .product-flow { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .flow-arrow { display:none; }
  .flow-cell { min-height:125px; border-bottom:1px solid var(--line); }
  .flow-cell:nth-child(3) { border-right:0; }
  .flow-cell:nth-last-child(-n+2) { border-bottom:0; }
  .history-line { grid-template-columns:1fr; }
  .history-cell { border-right:0; border-bottom:1px solid var(--line); }
  .history-cell:last-child { border-bottom:0; }
  .discovery-summary,.k1-line { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .discovery-cell:nth-child(2),.k1-cell:nth-child(2) { border-right:0; }
  .discovery-cell:nth-child(-n+2),.k1-cell:nth-child(-n+2) { border-bottom:1px solid var(--line); }
  .safety-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .safety-cell { border-bottom:1px solid var(--line); }
  .safety-cell:nth-child(2n) { border-right:0; }
  .safety-cell:last-child { grid-column:1 / -1; border-bottom:0; }
}
@media (max-width:680px) {
  .nd-inner { padding:16px 12px; }
  .nd-head .nd-inner,.nd-section-head,.nd-foot .nd-inner { align-items:flex-start; flex-direction:column; gap:7px; }
  .nd-live,.nd-scope { text-align:left; }
  .product-flow,.miner-grid,.discovery-summary,.k1-line,.safety-grid { grid-template-columns:1fr; }
  .flow-cell,.miner-cell,.discovery-cell,.k1-cell,.safety-cell,.safety-cell:last-child { grid-column:auto; min-height:0; padding:14px 12px; border-right:0; border-bottom:1px solid var(--line); }
  .flow-cell:last-child,.miner-cell:last-child,.discovery-cell:last-child,.k1-cell:last-child,.safety-cell:last-child { border-bottom:0; }
  .metric-value { font-size:25px; }
  .next-transition { grid-template-columns:1fr; gap:4px; }
  .package-row { grid-template-columns:minmax(0,1fr); gap:3px; padding:11px 0; }
  .package-row.header { display:none; }
  .certificate::before { color:var(--muted); }
  .certificate.execution::before { content:"CPU "; }
  .certificate.law::before { content:"LAW "; }
  .certificate.mechanism::before { content:"MECHANISM "; }
  .certificate.k1::before { content:"K1 "; }
  .window-row { grid-template-columns:1fr; gap:2px; padding:8px 0; }
  .window-row.header { display:none; }
}
</style>
<main class="nando-live" data-dashboard-build="__DASHBOARD_BUILD__" aria-label="Nando live control">
  <header class="nd-head"><div class="nd-inner">
    <div class="nd-brand"><strong>NANDO</strong><span>LIVE CONTROL</span></div>
    <div class="nd-live"><b>LIVE</b> · SNAPSHOT <span id="snapshot-age">0</span> с · SOURCE <span id="source-age">—</span> с · CORE <span id="services">—/3</span></div>
  </div></header>

  <section class="nd-band"><div class="nd-inner">
    <div class="nd-section-head">
      <h1>ПРОДУКТ · ТЕКУЩАЯ СОПОСТАВИМАЯ ЭПОХА</h1>
      <div class="nd-scope">ОДИН ЗНАМЕНАТЕЛЬ · ORDINARY REQUEST_EVENT.V1</div>
    </div>
    <div class="product-flow">
      <div class="flow-cell">
        <div class="metric-label">ВЕСЬ ВХОДНОЙ ТРАФИК ЭПОХИ</div>
        <output id="epoch-total" class="metric-value">__EPOCH_TOTAL__</output>
        <div class="metric-note"><span id="epoch-requests">__EPOCH_REQUESTS__</span> завершённых запросов</div>
        <div id="epoch-start" class="metric-sub">ТЕКУЩАЯ ACCOUNTING EPOCH</div>
      </div>
      <div class="flow-arrow" aria-hidden="true">→</div>
      <div class="flow-cell cpu">
        <div class="metric-label">РЕАЛЬНО ИСПОЛНЕНО НА CPU</div>
        <output id="epoch-cpu" class="metric-value">__EPOCH_CPU__</output>
        <div class="metric-note"><span id="epoch-accepts">__EPOCH_ACCEPTS__</span> CPU accepts · <span id="avoided-calls">__EPOCH_ACCEPTS__</span> upstream calls avoided</div>
        <div class="metric-sub">ТОЛЬКО VERIFIED EXECUTION RECEIPTS</div>
      </div>
      <div class="flow-cell result">
        <div class="metric-label">ДОКАЗАННЫЙ CPU-ОХВАТ</div>
        <output id="epoch-share" class="metric-value">__EPOCH_SHARE__</output>
        <div class="ratio-rail"><div id="epoch-bar" class="ratio-fill"></div></div>
        <div class="metric-note">CPU authority <strong id="cpu-gate" class="nd-status __CPU_GATE_TONE__">__CPU_GATE__</strong></div>
      </div>
    </div>
    <div class="history-line">
      <div class="history-cell"><span class="history-label">ВСЯ ЗАПИСАННАЯ ИСТОРИЯ СЕРВЕРА</span><output id="lifetime-total" class="history-value">__LIFETIME_TOTAL__ токенов</output></div>
      <div class="history-cell"><span class="history-label">CPU ЗА ВСЮ ИСТОРИЮ</span><output id="lifetime-cpu" class="history-value">__LIFETIME_CPU__</output></div>
      <div class="history-cell"><span class="history-label">ДОЛЯ ЗА ВСЮ ИСТОРИЮ</span><output id="lifetime-share" class="history-value">__LIFETIME_SHARE__</output></div>
    </div>
  </div></section>

  <section class="nd-band"><div class="nd-inner">
    <div class="nd-section-head">
      <h2>МАЙНЕР · ОТДЕЛЬНОЕ ОКНО КЛАССИФИКАЦИИ</h2>
      <div id="miner-window-start" class="nd-scope">СВОЙ WATERMARK</div>
    </div>
    <div class="miner-grid">
      <div class="miner-cell">
        <div class="metric-label">МАЙНЕР ВИДЕЛ</div>
        <output id="miner-seen" class="metric-value">__MINER_SEEN__</output>
        <div class="metric-note"><span id="miner-seen-intents">__MINER_SEEN_INTENTS__</span> intents</div>
      </div>
      <div class="miner-cell recognized">
        <div class="metric-label">МАЙНЕР РАСПОЗНАЛ CPU-КЛАСС</div>
        <output id="miner-recognized" class="metric-value">__MINER_RECOGNIZED__</output>
        <div class="miner-progress"><div class="ratio-rail"><div id="miner-bar" class="ratio-fill"></div></div><output id="miner-share" class="miner-share">__MINER_RECOGNIZED_SHARE__</output></div>
        <div class="metric-note"><span id="miner-recognized-intents">__MINER_RECOGNIZED_INTENTS__</span> verified intents</div>
      </div>
      <div class="miner-cell unresolved">
        <div class="metric-label">ЕЩЁ НЕ РАСПОЗНАНО</div>
        <output id="miner-unrecognized" class="metric-value">__MINER_UNRECOGNIZED__</output>
        <div id="miner-unresolved" class="metric-note">исследуемый unresolved —</div>
      </div>
    </div>
    <div class="scope-warning">MINER WINDOW И SERVER ACCOUNTING ИМЕЮТ РАЗНЫЕ ПЕРИОДЫ. МЕЖДУ НИМИ НЕТ КОРРЕКТНОГО ПРОЦЕНТА.</div>
  </div></section>

  <section class="nd-band"><div class="nd-inner">
    <div class="nd-section-head">
      <h2>DISCOVERY · СЛЕДУЮЩИЙ ЕСТЕСТВЕННЫЙ ЗАКОН</h2>
      <div id="scheduler-state" class="nd-status">ЗАГРУЗКА</div>
    </div>
    <div class="discovery-summary">
      <div class="discovery-cell"><div class="metric-label">СТРУКТУРНЫХ КОГОРТ</div><output id="catalog-cohorts" class="metric-value">—</output><div class="metric-sub">НАЙДЕНО В NATURAL TRAFFIC</div></div>
      <div class="discovery-cell ready"><div class="metric-label">READY ПРЯМО СЕЙЧАС</div><output id="ready-now" class="metric-value">—</output><div class="metric-sub">МОЖНО ЗАМОРОЗИТЬ БЕЗ ПОВТОРА</div></div>
      <div class="discovery-cell"><div class="metric-label">READY УЖЕ ИСЧЕРПАНО</div><output id="completed-ready" class="metric-value">—</output><div class="metric-sub">НЕ ПЕРЕОБУЧАТЬ ПОВТОРНО</div></div>
      <div class="discovery-cell"><div class="metric-label">ОЧЕРЕДЬ ИССЛЕДОВАНИЯ</div><output id="retained-queue" class="metric-value">—</output><div class="metric-sub">OPERATOR-BLIND RANKING</div></div>
    </div>
    <div class="next-transition"><strong>СЛЕДУЮЩИЙ ПЕРЕХОД</strong><span id="next-transition">новое ordinary evidence → readiness PASS → frozen generation → Law #2</span></div>
    <div id="readiness-blockers" class="blocker-list"></div>
  </div></section>

  <section class="nd-band"><div class="nd-inner">
    <div class="nd-section-head">
      <h2>ДОКАЗАННЫЕ ПАКЕТЫ</h2>
      <div class="nd-scope"><span id="active-packages">—</span> ACTIVE · FALSE ACCEPTS <span id="false-accepts">—</span> · PARITY <span id="parity-failures">—</span></div>
    </div>
    <div class="package-table">
      <div class="package-row header"><span>PACKAGE</span><span>CPU</span><span>LAW</span><span>MECHANISM</span><span>K1</span></div>
      <div id="package-rows"></div>
    </div>
    <div class="k1-line">
      <div class="k1-cell"><span>LAWS</span><strong id="k1-laws">—</strong></div>
      <div class="k1-cell"><span>SEMANTICS</span><strong id="k1-semantics">—</strong></div>
      <div class="k1-cell"><span>TOPOLOGIES</span><strong id="k1-topologies">—</strong></div>
      <div class="k1-cell closed"><span>NATURAL L2</span><strong id="natural-l2">CLOSED</strong></div>
    </div>
  </div></section>

  <section class="nd-band"><div class="nd-inner">
    <div class="nd-section-head"><h2>СИСТЕМА И БЕЗОПАСНОСТЬ</h2><div class="nd-scope">FAIL-CLOSED</div></div>
    <div class="safety-grid">
      <div class="safety-cell"><span>CORE SIGNALS</span><strong id="safety-services">—/3</strong></div>
      <div class="safety-cell"><span>STRUCTURAL BACKLOG</span><strong id="structural-backlog">—</strong></div>
      <div class="safety-cell"><span>EVIDENCE BACKLOG</span><strong id="evidence-backlog">—</strong></div>
      <div class="safety-cell"><span>TRANSPORT FAILURES</span><strong id="transport-failures">—</strong></div>
      <div class="safety-cell"><span>COUNTER EPOCH</span><strong id="counter-epoch">—</strong></div>
    </div>
    <details class="quiet-details" id="route-details"><summary id="route-summary">МАРШРУТЫ ОКОН CODEX</summary><div class="window-table"><div class="window-row header"><span>PROJECT</span><span>SESSION</span><span>ROUTE</span><span>ENDPOINT</span></div><div id="window-rows"></div></div></details>
  </div></section>

  <footer class="nd-foot"><div class="nd-inner"><span>COMPACT SNAPSHOT · <span id="payload-bytes">—</span> BYTES</span><span id="dashboard-build">BUILD __DASHBOARD_BUILD__</span></div></footer>
</main>
<script>
(() => {
  const base = window.location.pathname.replace(/\/$/, "");
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
  const shortRoot = value => value ? `${value.slice(0, 8)}…${value.slice(-8)}` : "—";
  const readable = value => ({
    waiting_for_evidence:"ЖДЁТ НОВЫЕ ДАННЫЕ",
    no_readiness_pass_candidate:"НЕТ НОВОЙ READY-КОГОРТЫ",
    settled_evidence_below_freeze_minimum:"МАЛО ЗАВЕРШЁННЫХ НАБЛЮДЕНИЙ",
    independent_lineages_below_freeze_minimum:"МАЛО НЕЗАВИСИМЫХ LINEAGE",
    selected_role_witness_missing:"CAPTURE НЕ СОХРАНИЛ ТИПИЗИРОВАННУЮ РОЛЬ",
  }[value] || String(value || "—").replaceAll("_", " ").toUpperCase());

  const certificateTone = value => {
    if (["pass", "pass_legacy", "yes"].includes(value)) return "good";
    if (["partial", "collecting", "not_evaluated", "legacy", "unresolved"].includes(value)) return "watch";
    if (["fail", "revoked", "rejected", "no"].includes(value)) return "bad";
    return "";
  };

  const renderPackages = snapshot => {
    const rows = node("package-rows");
    if (!rows) return;
    rows.replaceChildren();
    for (const certificate of snapshot.packages?.certificates || []) {
      const row = document.createElement("div");
      row.className = "package-row";
      const packageId = document.createElement("span");
      packageId.className = "package-id";
      packageId.textContent = certificate.package_id?.length > 52 ? `${certificate.package_id.slice(0, 30)}…${certificate.package_id.slice(-14)}` : certificate.package_id || "—";
      packageId.title = certificate.package_id || "";
      row.appendChild(packageId);
      const states = [
        ["execution", certificate.execution_status || "pending"],
        ["law", certificate.law_status || "partial"],
        ["mechanism", certificate.mechanism_status || "not_evaluated"],
        ["k1", certificate.k1_unit_eligible === true ? "yes" : "no"],
      ];
      for (const [kind, value] of states) {
        const cell = document.createElement("span");
        cell.className = `certificate ${kind} ${certificateTone(value)}`;
        cell.textContent = readable(value);
        row.appendChild(cell);
      }
      rows.appendChild(row);
    }
    if (rows.childElementCount === 0) {
      const row = document.createElement("div");
      row.className = "package-row";
      const cell = document.createElement("span");
      cell.className = "package-id";
      cell.textContent = "НЕТ ACTIVE PACKAGES";
      row.appendChild(cell);
      rows.appendChild(row);
    }
  };

  const renderDashboard = (snapshot, bytes) => {
    if (!snapshot?.available) return;
    if (snapshot.dashboard_build && snapshot.dashboard_build !== expectedBuild) {
      window.location.reload();
      return;
    }
    const epoch = snapshot.product?.current_epoch || {};
    const lifetime = snapshot.product?.lifetime || {};
    const miner = snapshot.miner || {};
    const discovery = snapshot.discovery || {};
    const safety = snapshot.safety || {};
    const k1 = snapshot.k1 || {};
    sourceGeneratedAt = snapshot.generated_at_unix || 0;

    text("epoch-total", number.format(epoch.input_tokens || 0));
    text("epoch-cpu", number.format(epoch.cpu_tokens || 0));
    text("epoch-share", percent(epoch.cpu_tokens || 0, epoch.input_tokens || 0));
    text("epoch-requests", number.format(epoch.requests || 0));
    text("epoch-accepts", number.format(epoch.cpu_accepts || 0));
    text("avoided-calls", number.format(epoch.avoided_upstream_calls || 0));
    text("epoch-start", `EPOCH С ${localTime(epoch.started_at_unix || 0)}`);
    bar("epoch-bar", epoch.cpu_tokens || 0, epoch.input_tokens || 0);
    text("lifetime-total", `${number.format(lifetime.input_tokens || 0)} токенов`);
    text("lifetime-cpu", number.format(lifetime.cpu_tokens || 0));
    text("lifetime-share", percent(lifetime.cpu_tokens || 0, lifetime.input_tokens || 0));
    text("cpu-gate", safety.cpu_allowed ? "OPEN" : "LOCKED");
    className("cpu-gate", `nd-status ${safety.cpu_allowed ? "good" : "bad"}`);

    const unrecognized = Math.max(0, (miner.seen_tokens || 0) - (miner.recognized_tokens || 0));
    text("miner-seen", number.format(miner.seen_tokens || 0));
    text("miner-seen-intents", number.format(miner.seen_intents || 0));
    text("miner-recognized", number.format(miner.recognized_tokens || 0));
    text("miner-recognized-intents", number.format(miner.recognized_intents || 0));
    text("miner-share", percent(miner.recognized_tokens || 0, miner.seen_tokens || 0));
    text("miner-unrecognized", number.format(unrecognized));
    text("miner-unresolved", `из них unresolved ${number.format(miner.unresolved_tokens || 0)}`);
    text("miner-window-start", `ОКНО С ${localTime(miner.started_at_unix || 0)} · СВОЙ WATERMARK`);
    bar("miner-bar", miner.recognized_tokens || 0, miner.seen_tokens || 0);

    text("scheduler-state", readable(discovery.state));
    className("scheduler-state", `nd-status ${discovery.ready_now > 0 ? "good" : ""}`);
    text("catalog-cohorts", number.format(discovery.catalog_cohorts || 0));
    text("ready-now", number.format(discovery.ready_now || 0));
    text("completed-ready", `${number.format(discovery.completed_ready_excluded || 0)} / ${number.format(discovery.historical_readiness_pass || 0)}`);
    text("retained-queue", number.format(discovery.retained_queue || 0));
    const next = discovery.ready_now > 0
      ? `READY ${number.format(discovery.ready_now)} → freeze generation ${number.format(discovery.next_generation_sequence || 0)} → identifier → Law #2`
      : `${readable(discovery.blocker)} → fresh ordinary evidence → readiness PASS → generation ${number.format(discovery.next_generation_sequence || 0)} → Law #2`;
    text("next-transition", next);
    const blockers = node("readiness-blockers");
    if (blockers) {
      blockers.replaceChildren();
      const entries = Object.entries(discovery.readiness_blockers || {}).sort((a, b) => b[1] - a[1]);
      for (const [name, count] of entries) {
        const item = document.createElement("span");
        const total = document.createElement("b");
        total.textContent = number.format(count);
        item.append(total, ` ${readable(name)}`);
        blockers.appendChild(item);
      }
      const root = document.createElement("span");
      const label = document.createElement("b");
      label.textContent = "LEAD";
      root.append(label, ` ${shortRoot(discovery.leading_candidate_root_sha256)}`);
      blockers.appendChild(root);
    }

    text("active-packages", number.format(snapshot.packages?.active || 0));
    text("false-accepts", number.format(safety.false_accepts || 0));
    text("parity-failures", number.format(safety.parity_failures || 0));
    renderPackages(snapshot);
    text("k1-laws", `${number.format(k1.law_certificates || 0)} / ${number.format(k1.min_law_certificates || 3)}`);
    text("k1-semantics", `${number.format(k1.semantic_laws || 0)} / ${number.format(k1.min_semantic_laws || 3)}`);
    text("k1-topologies", `${number.format(k1.role_topologies || 0)} / ${number.format(k1.min_role_topologies || 2)}`);
    text("natural-l2", k1.open === true ? "OPEN" : "CLOSED");
    className("natural-l2", `nd-status ${k1.open === true ? "good" : ""}`);

    text("services", `${number.format(safety.services_active || 0)}/${number.format(safety.services_expected || 3)}`);
    text("safety-services", `${number.format(safety.services_active || 0)}/${number.format(safety.services_expected || 3)}`);
    text("structural-backlog", number.format(safety.structural_pending || 0));
    text("evidence-backlog", number.format(safety.opportunity_pending || 0));
    text("transport-failures", number.format(safety.transport_failures || 0));
    text("counter-epoch", safety.counter_epoch_match ? "MATCH" : "SPLIT");
    className("structural-backlog", safety.structural_pending > 0 ? "watch" : "");
    className("evidence-backlog", safety.opportunity_pending > 0 ? "watch" : "");
    className("transport-failures", safety.transport_failures > 0 ? "bad" : "");
    className("counter-epoch", safety.counter_epoch_match ? "" : "bad");
    text("payload-bytes", number.format(bytes));
    lastSuccess = Date.now();
  };

  const renderConnections = snapshot => {
    text("route-summary", `МАРШРУТЫ ОКОН CODEX · NANDO ${number.format(snapshot.active_nando || 0)} · MIXED ${number.format(snapshot.active_mixed || 0)} · DIRECT ${number.format(snapshot.active_outside_nando || 0)} · IDLE ${number.format(snapshot.idle || 0)}`);
    const rows = node("window-rows");
    if (!rows) return;
    rows.replaceChildren();
    for (const window of snapshot.windows || []) {
      const row = document.createElement("div");
      row.className = "window-row";
      const route = window.route || "idle";
      const endpoint = (window.remote_endpoints || []).join(", ") || "—";
      const values = [window.project?.toUpperCase() || "—", window.session?.startsWith("pid-") ? window.session : window.session?.slice(0, 8) || "—", readable(route), endpoint];
      values.forEach((value, index) => { const cell = document.createElement("span"); cell.textContent = value; if (index === 2) cell.className = `window-route ${route}`; row.appendChild(cell); });
      rows.appendChild(row);
    }
  };

  const refreshDashboard = async () => {
    if (refreshing) return;
    refreshing = true;
    try {
      const response = await fetch(`${base}/api/v1/dashboard`, {cache:"no-store"});
      if (!response.ok) return;
      const payload = await response.text();
      renderDashboard(JSON.parse(payload), new Blob([payload]).size);
    } catch (_) {
    } finally {
      refreshing = false;
    }
  };
  const refreshConnections = async () => {
    try {
      const response = await fetch(`${base}/connections`, {cache:"no-store"});
      if (response.ok) renderConnections(await response.json());
    } catch (_) {}
  };
  window.setInterval(() => {
    text("snapshot-age", Math.floor((Date.now() - lastSuccess) / 1000));
    text("source-age", sourceGeneratedAt > 0 ? Math.max(0, Math.floor(Date.now() / 1000) - sourceGeneratedAt) : "—");
  }, 1000);
  Promise.all([refreshDashboard(), refreshConnections()]);
  window.setInterval(refreshDashboard, 3000);
  window.setInterval(refreshConnections, 15000);
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
    fn dashboard_leads_with_comparable_cpu_coverage_and_separates_miner_scope() {
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
        assert!(html.contains("ПРОДУКТ · ТЕКУЩАЯ СОПОСТАВИМАЯ ЭПОХА"));
        assert!(html.contains("1 733 026 637"));
        assert!(html.contains("165 104 290"));
        assert!(html.contains("9,53%"));
        assert!(html.contains("7 694 807 361 токенов"));
        assert!(html.contains("10 882 437 482"));
        assert!(html.contains("1 613 584 240"));
        assert!(html.contains("РАЗНЫЕ ПЕРИОДЫ"));
        assert!(html.contains("/api/v1/dashboard"));
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
        assert!(html.contains("class=\"nd-status bad\">LOCKED"));
        assert!(!html.contains("__CPU_GATE__"));
    }
}
