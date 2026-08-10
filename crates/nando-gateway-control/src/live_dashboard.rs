use serde::Serialize;
use serde_json::Value;

const DASHBOARD_BUILD: &str = "2026.08.10-control-v7";

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
.nd-status { color:var(--amber); font-weight:700; }
.nd-status.good { color:var(--green); }
.nd-status.bad { color:var(--red); }
.nd-foot .nd-inner { display:flex; justify-content:space-between; gap:20px; padding-top:12px; padding-bottom:12px; color:#667075; font-size:9px; }
.safety-line { display:flex; flex-wrap:wrap; gap:6px 22px; color:#7f898e; font-size:10px; }
.safety-line b { color:#bfc6c9; font-weight:680; }
/* control-v7 is a single operator ledger: no staged flow or dashboard cards. */
.control-v6 .nd-inner { width:min(1080px,100%); margin:0 auto; padding-left:28px; padding-right:28px; }
.control-v6 .nd-head .nd-inner { min-height:58px; padding-top:0; padding-bottom:0; }
.control-v6 .nd-brand strong { font-size:19px; }
.control-v6 .nd-brand span,.control-v6 .nd-live { font-size:11px; }
.control-v6 .nd-main { padding-top:34px; padding-bottom:38px; }
.control-v6 .section-head { display:flex; align-items:end; justify-content:space-between; gap:24px; margin-bottom:14px; }
.control-v6 .section-head h1 { margin:0; color:#f4f6f7; font-size:20px; font-weight:730; line-height:1.25; }
.control-v6 .section-head p { margin:0; color:var(--muted); font-size:11px; line-height:1.45; text-align:right; }
.control-v6 .ledger { border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.control-v6 .ledger-row { display:grid; grid-template-columns:minmax(220px,1.25fr) repeat(3,minmax(150px,1fr)); gap:20px; align-items:center; min-height:92px; border-bottom:1px solid var(--line); }
.control-v6 .ledger-row:last-child { border-bottom:0; }
.control-v6 .ledger-head { min-height:32px; color:var(--muted); font-size:10px; font-weight:700; }
.control-v6 .ledger-title,.control-v6 .ledger-cell { min-width:0; }
.control-v6 .ledger-title strong { display:block; color:#e3e7e9; font-size:14px; font-weight:720; }
.control-v6 .ledger-title span { display:block; margin-top:5px; color:var(--muted); font-size:10px; line-height:1.4; }
.control-v6 .ledger-cell strong { display:block; color:#f2f4f5; font-size:22px; font-weight:720; line-height:1.15; overflow-wrap:anywhere; }
.control-v6 .ledger-cell strong.good,.control-v6 .ledger-share strong { color:var(--green); }
.control-v6 .ledger-cell span { display:block; margin-top:5px; color:var(--muted); font-size:10px; line-height:1.35; }
.control-v6 .law { margin-top:34px; border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.control-v6 .law-head { display:flex; align-items:center; justify-content:space-between; gap:20px; min-height:64px; border-bottom:1px solid var(--line); }
.control-v6 .law-title { display:flex; align-items:baseline; gap:12px; min-width:0; }
.control-v6 .law-title h2 { margin:0; color:#f4f6f7; font-size:18px; font-weight:730; }
.control-v6 .law-title span { color:var(--muted); font-size:11px; }
.control-v6 .law-verdict { color:var(--amber); font-size:13px; font-weight:760; line-height:1.25; text-align:right; }
.control-v6 .law-verdict.good { color:var(--green); }
.control-v6 .law-verdict.bad { color:var(--red); }
.control-v6 .law-body { padding:18px 0 20px; }
.control-v6 .law-counts { display:flex; flex-wrap:wrap; gap:7px 24px; margin:0; color:#c9cfd2; font-size:12px; line-height:1.5; }
.control-v6 .law-counts span { white-space:nowrap; }
.control-v6 .law-counts b { color:#f0f3f4; font-size:15px; font-weight:720; }
.control-v6 .law-blocker { margin:14px 0 0; color:#b7bec1; font-size:12px; line-height:1.55; }
.control-v6 .law-blocker span,.control-v6 .law-next { color:var(--muted); }
.control-v6 .law-blocker b { color:#e4e8ea; font-weight:680; }
.control-v6 .law-next { margin:7px 0 0; font-size:11px; line-height:1.5; }
.control-v6 .law-next b { color:#cbd1d4; font-weight:650; }
.control-v6 .nd-foot { border-top:1px solid var(--line); }
.control-v6 .nd-foot .nd-inner { padding-top:12px; padding-bottom:12px; }
@media (max-width:760px) {
  .control-v6 .nd-inner { padding-left:16px; padding-right:16px; }
  .control-v6 .nd-head .nd-inner { align-items:flex-start; flex-direction:column; justify-content:center; gap:4px; min-height:68px; }
  .control-v6 .nd-live { text-align:left; }
  .control-v6 .nd-main { padding-top:24px; padding-bottom:28px; }
  .control-v6 .section-head { align-items:flex-start; flex-direction:column; gap:5px; }
  .control-v6 .section-head p { text-align:left; }
  .control-v6 .ledger-head { display:none; }
  .control-v6 .ledger-row { grid-template-columns:1fr; gap:12px; min-height:0; padding:18px 0; }
  .control-v6 .ledger-cell { display:grid; grid-template-columns:112px minmax(0,1fr); align-items:baseline; gap:12px; }
  .control-v6 .ledger-cell::before { content:attr(data-label); color:var(--muted); font-size:10px; font-weight:700; }
  .control-v6 .ledger-cell strong { font-size:20px; }
  .control-v6 .ledger-cell span { grid-column:2; margin-top:-7px; }
  .control-v6 .law-head { align-items:flex-start; padding:15px 0; }
  .control-v6 .law-title { align-items:flex-start; flex-direction:column; gap:3px; }
  .control-v6 .law-verdict { max-width:44%; }
  .control-v6 .law-counts { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:8px 14px; }
  .control-v6 .law-counts span { white-space:normal; }
}
</style>
<main class="nando-live control-v6" data-dashboard-build="__DASHBOARD_BUILD__" aria-label="Nando live control">
  <header class="nd-head"><div class="nd-inner">
    <div class="nd-brand"><strong>NANDO LIVE</strong><span>факты, не прогнозы</span></div>
    <div class="nd-live"><b>LIVE</b> · снимок <span id="snapshot-age">0</span> с · источник <span id="source-age">—</span> с · сервисы <span id="services">—/3</span></div>
  </div></header>

  <div class="nd-inner nd-main">
    <section aria-labelledby="traffic-title">
      <div class="section-head">
        <h1 id="traffic-title">Трафик и CPU</h1>
        <p>Каждая доля считается только внутри своей строки.</p>
      </div>
      <div class="ledger" role="table" aria-label="Фактический трафик и результат">
        <div class="ledger-row ledger-head" role="row">
          <span role="columnheader">Контур</span><span role="columnheader">Вход</span><span role="columnheader">Результат</span><span role="columnheader">Доля</span>
        </div>
        <div class="ledger-row" role="row">
          <div class="ledger-title" role="rowheader"><strong>Вся история сервера</strong><span>весь записанный ordinary traffic</span></div>
          <div class="ledger-cell" data-label="Вход" role="cell"><strong id="lifetime-total">__LIFETIME_TOTAL__</strong><span>входных токенов</span></div>
          <div class="ledger-cell" data-label="CPU" role="cell"><strong id="lifetime-cpu" class="good">__LIFETIME_CPU__</strong><span>воспроизведено на CPU</span></div>
          <div class="ledger-cell ledger-share" data-label="Доля" role="cell"><strong id="lifetime-share">__LIFETIME_SHARE__</strong><span>CPU / весь сервер · допуск <b id="cpu-gate" class="nd-status __CPU_GATE_TONE__">__CPU_GATE__</b></span></div>
        </div>
        <div class="ledger-row" role="row">
          <div class="ledger-title" role="rowheader"><strong>Окно майнера</strong><span id="miner-window-start">отдельный watermark</span></div>
          <div class="ledger-cell" data-label="Увидел" role="cell"><strong id="miner-seen">__MINER_SEEN__</strong><span><b id="miner-seen-intents">__MINER_SEEN_INTENTS__</b> запросов</span></div>
          <div class="ledger-cell" data-label="Распознал" role="cell"><strong id="miner-recognized" class="good">__MINER_RECOGNIZED__</strong><span><b id="miner-recognized-intents">__MINER_RECOGNIZED_INTENTS__</b> запросов</span></div>
          <div class="ledger-cell ledger-share" data-label="Доля" role="cell"><strong id="miner-share">__MINER_RECOGNIZED_SHARE__</strong><span>распознано / увидено · не смешивать с историей сервера</span></div>
        </div>
      </div>
    </section>

    <section class="law" aria-labelledby="law2-title">
      <div class="law-head">
        <div class="law-title"><h2 id="law2-title">Law #2</h2><span>следующий естественный закон K1 · K1 <b id="k1-laws-main">—</b></span></div>
        <strong id="law2-state" class="law-verdict">ЗАГРУЗКА</strong>
      </div>
      <div class="law-body">
        <p class="law-counts">
          <span>live-когорт <b id="catalog-cohorts">—</b></span>
          <span>readiness-PASS сейчас <b id="ready-now">—</b></span>
          <span>terminal generations <b id="generations-checked">—</b></span>
          <span id="generation-label">следующая generation <b id="next-generation">—</b></span>
        </p>
        <p class="law-blocker"><span>Discovery сейчас:</span> <b id="discovery-state">—</b>. <span>Текущий blocker:</span> <b id="current-blocker">—</b>.</p>
        <p class="law-blocker"><span>Последний terminal:</span> <b id="latest-verdict">—</b> · <b id="latest-verdict-blocker">—</b>.</p>
        <p class="law-next"><b>Law #2 появится только после:</b> unique semantic class → independent post-freeze future → BundleV4 → verified ordinary CPU → exact economics → cleanup → LawCertificate.</p>
      </div>
    </section>
  </div>

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
  const localTime = unix => unix > 0 ? new Date(unix * 1000).toLocaleString("ru-RU", {dateStyle:"short", timeStyle:"medium"}) : "—";
  const readable = value => ({
    waiting_for_evidence:"WAITING FOR EVIDENCE",
    candidate_frozen:"CANDIDATE FROZEN",
    identifying:"IDENTIFYING",
    future_pending:"FUTURE PENDING",
    no_readiness_pass_candidate:"нет readiness-PASS кандидата",
    settled_evidence_below_freeze_minimum:"мало завершённых наблюдений",
    verified_evidence_below_freeze_minimum:"мало независимо проверенных наблюдений",
    independent_lineages_below_freeze_minimum:"мало независимых lineage",
    selected_role_witness_missing:"capture не сохранил типизированную роль",
    all_supported_t1_protocol_modes_already_active:"кандидат дублирует уже активный T1 protocol mode",
    durable_future_prediction_pending_outcome:"prediction записана, ожидается настоящий outcome",
    independent_post_identification_future_pending:"ожидается independent future после identification",
    independent_future_not_observed:"independent future не появился в bounded window",
    generation_deadline_exhausted:"bounded window завершён без доказательства",
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
    text("law2-state", law2Proved ? "ДОКАЗАН" : "НЕ ДОКАЗАН");
    className("law2-state", `law-verdict ${law2Proved ? "good" : ""}`);
    text("catalog-cohorts", number.format(discovery.catalog_cohorts || 0));
    text("generations-checked", number.format(discovery.completed_generations || 0));
    text("ready-now", number.format(discovery.ready_now || 0));
    text("discovery-state", readable(discovery.state));
    text("current-blocker", readable(discovery.blocker || "нет"));
    text("latest-verdict", readable(discovery.latest_verdict));
    text("latest-verdict-blocker", readable(discovery.latest_verdict_blocker || "нет"));
    text("generation-label", law2Active ? "active generation " : "следующая generation ");
    const generationLabel = node("generation-label");
    if (generationLabel) {
      const generation = document.createElement("b");
      generation.id = "next-generation";
      generation.textContent = number.format(discovery.next_generation_sequence || 0);
      generationLabel.appendChild(generation);
    }
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
        assert!(html.contains("Трафик и CPU"));
        assert!(html.contains("Вся история сервера"));
        assert!(html.contains("7 694 807 361"));
        assert!(html.contains("207 619 587"));
        assert!(html.contains("2,70%"));
        assert!(html.contains("10 882 437 482"));
        assert!(html.contains("1 613 584 240"));
        assert!(html.contains("не смешивать с историей сервера"));
        assert!(html.contains("Law #2"));
        assert!(html.contains("id=\"catalog-cohorts\""));
        assert!(html.contains("readiness-PASS сейчас"));
        assert!(html.contains("id=\"generations-checked\""));
        assert!(html.contains("id=\"discovery-state\""));
        assert!(html.contains("id=\"current-blocker\""));
        assert!(html.contains("id=\"latest-verdict\""));
        assert!(html.contains("verified ordinary CPU"));
        assert!(!html.contains("повторяемых <b"));
        assert!(!html.contains("все доступные epistemic modes уже доказаны"));
        assert_eq!(html.matches("class=\"ledger-row\"").count(), 2);
        assert!(!html.contains("class=\"flow-index\""));
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
