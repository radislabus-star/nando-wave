use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub(crate) struct InitialMetrics {
    pub(crate) total_tokens: u64,
    pub(crate) miner_tokens: u64,
    pub(crate) cpu_tokens: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct BridgeView {
    pub(crate) hot_available: bool,
    pub(crate) cold_available: bool,
    // Compatibility aliases for dashboard pages opened before bridge-health V2.
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
    pub(crate) request_events: u64,
    pub(crate) request_tokens: u64,
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
    let opportunity_pending = pointer_u64(hot, "/opportunity/pending_events");
    let opportunity_inflight = pointer_u64(cold, "/opportunity/consumer_inflight_events");
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
        queue: structural_pending
            .saturating_add(opportunity_pending)
            .saturating_add(opportunity_inflight),
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
        opportunity_produced_sequence: pointer_u64(hot, "/opportunity/producer_last_sequence"),
        opportunity_consumed_sequence: pointer_u64(cold, "/opportunity/consumer_last_sequence"),
        request_events: pointer_u64(hot, "/opportunity/producer_request_events"),
        request_tokens: pointer_u64(hot, "/opportunity/producer_request_input_tokens"),
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
    TEMPLATE
        .replace("__TOTAL__", &format_number(initial.total_tokens))
        .replace("__MINER__", &format_number(initial.miner_tokens))
        .replace("__CPU__", &format_number(initial.cpu_tokens))
        .replace(
            "__MINER_SHARE__",
            &format_percent(initial.miner_tokens, initial.total_tokens, 2),
        )
        .replace(
            "__CPU_SHARE__",
            &format_percent(initial.cpu_tokens, initial.total_tokens, 3),
        )
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
.nando-live { --bg:#0d1012; --line:#384047; --muted:#879199; --text:#eef1f3; --cyan:#4db8ec; --green:#65d487; --amber:#e6b84f; --red:#ff6861; width:100%; max-width:none; min-height:100vh; margin:0; padding:0; background:var(--bg); color:var(--text); }
.nando-live * { box-sizing:border-box; }
.live-head,.live-band,.live-foot { width:100%; border-bottom:1px solid var(--line); }
.live-inner { width:min(1440px,100%); margin:0 auto; padding:20px 24px; }
.live-head .live-inner { display:flex; justify-content:space-between; align-items:center; min-height:66px; }
.live-title { margin:0; font-size:19px; font-weight:800; letter-spacing:0; }
.live-clock { color:var(--muted); font-size:13px; font-weight:700; }
.live-clock b { color:var(--green); }
.band-title { margin:0 0 15px; color:#d9dfe3; font-size:15px; }
.token-tracks { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); }
.token-track { min-width:0; padding:0 30px; border-right:1px solid var(--line); }
.token-track:first-child { padding-left:0; }
.token-track:last-child { padding-right:0; border-right:0; }
.track-label { color:#cbd2d7; font-size:13px; font-weight:800; }
.track-value-row { display:flex; justify-content:space-between; align-items:baseline; gap:18px; margin-top:8px; }
.track-value { min-width:0; font-size:30px; font-weight:800; white-space:nowrap; }
.track-share { flex:0 0 auto; color:var(--muted); font-size:16px; font-weight:800; }
.track-miner .track-value,.track-miner .track-share { color:var(--cyan); }
.track-cpu .track-value,.track-cpu .track-share { color:var(--green); }
.track-rail { height:12px; margin-top:20px; background:#242a2e; overflow:hidden; }
.track-fill { width:0; height:100%; min-width:2px; background:#dce2e6; transition:width .25s ease; }
.track-miner .track-fill { background:var(--cyan); }
.track-cpu .track-fill { background:var(--green); }
.track-note { min-height:22px; margin-top:15px; color:var(--muted); font-size:13px; font-weight:700; }
.track-note.good { color:var(--green); }
.track-note.watch { color:var(--amber); }
.epoch-strip { display:flex; justify-content:space-between; gap:24px; align-items:center; padding:15px 24px; border-bottom:1px solid var(--line); color:#d8dde1; font-size:14px; font-weight:700; }
.epoch-strip b,.epoch-visibility { color:var(--green); }
.window-head { display:flex; justify-content:space-between; gap:20px; align-items:baseline; margin-bottom:12px; }
.window-summary { color:var(--muted); font-size:13px; font-weight:700; }
.window-scroll { overflow-x:auto; }
.window-table { min-width:820px; }
.window-row { display:grid; grid-template-columns:1.1fr 1fr 1fr 1fr 1.2fr; gap:18px; align-items:center; min-height:50px; border-bottom:1px solid var(--line); font-size:14px; }
.window-row.header { min-height:34px; color:var(--muted); font-size:12px; font-weight:800; }
.window-status { font-weight:800; }
.window-status.nando { color:var(--green); }
.window-status.outside_nando { color:var(--red); }
.window-status.mixed { color:var(--amber); }
.window-status.idle { color:var(--muted); }
.pipeline-scroll { overflow-x:auto; padding-bottom:5px; }
.pipeline { position:relative; display:grid; grid-template-columns:repeat(8,minmax(140px,1fr)); min-width:1200px; border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.pipe-step { position:relative; min-height:100px; padding:17px 18px; border-right:1px solid var(--line); }
.pipe-step:last-child { border-right:0; }
.pipe-step::after { content:"→"; position:absolute; right:-8px; top:37px; z-index:1; color:#c7cdd1; background:var(--bg); }
.pipe-step:last-child::after { content:""; }
.pipe-name { color:#d6dce0; font-size:12px; font-weight:800; }
.pipe-state { margin-top:12px; color:var(--green); font-size:14px; font-weight:800; }
.pipe-step.watch .pipe-state { color:var(--amber); }
.pipe-step.block .pipe-state,.pipe-step.locked .pipe-state { color:var(--red); }
.pipe-step.muted .pipe-state { color:var(--muted); }
.break-line { position:absolute; left:50%; top:-29px; bottom:-34px; border-left:2px dashed var(--red); pointer-events:none; }
.break-label { position:absolute; left:calc(50% - 112px); top:-28px; width:224px; color:var(--red); background:var(--bg); text-align:center; font-size:12px; font-weight:800; }
.blocker { margin-top:14px; color:var(--red); text-align:center; font-size:13px; }
.activity { display:grid; grid-template-columns:150px minmax(0,1fr); gap:18px; align-items:end; margin-top:18px; }
.activity-label { color:var(--muted); font-size:12px; font-weight:800; }
.activity-bars { display:flex; align-items:end; gap:3px; height:38px; border-bottom:1px solid var(--line); }
.activity-bar { flex:1 1 0; min-width:2px; height:2px; background:var(--cyan); }
.live-foot { border-bottom:0; }
.live-foot .live-inner { display:flex; justify-content:space-between; gap:20px; color:var(--muted); font-size:12px; }
.next-route { color:var(--cyan); }
@media (max-width:900px) {
  .token-tracks { grid-template-columns:1fr; }
  .token-track,.token-track:first-child,.token-track:last-child { padding:20px 0; border-right:0; border-bottom:1px solid var(--line); }
  .token-track:last-child { border-bottom:0; }
  .epoch-strip,.window-head,.live-foot .live-inner { align-items:flex-start; flex-direction:column; gap:8px; }
}
@media (max-width:560px) {
  .live-inner { padding:16px 12px; }
  .live-head .live-inner { align-items:flex-start; flex-direction:column; gap:7px; }
  .track-value { font-size:24px; }
  .track-share { font-size:14px; }
  .epoch-strip { padding:13px 12px; font-size:12px; }
  .activity { grid-template-columns:1fr; gap:6px; }
}
</style>
<main class="nando-live" aria-label="Nando live traffic control">
  <header class="live-head"><div class="live-inner">
    <h1 class="live-title">NANDO / LIVE TRAFFIC CONTROL</h1>
    <div class="live-clock"><b>LIVE</b> · обновлено <span id="live-age">0</span> с назад · SERVICES <span id="services-count">—/3</span></div>
  </div></header>
  <section class="live-band"><div class="live-inner">
    <h2 class="band-title">КУДА УШЛИ ТОКЕНЫ</h2>
    <div class="token-tracks">
      <article class="token-track"><div class="track-label">ВХОД NANDO</div><div class="track-value-row"><output id="total-token-count" class="track-value">__TOTAL__</output><span class="track-share">100%</span></div><div class="track-rail"><div id="total-bar" class="track-fill" style="width:100%"></div></div><div class="track-note">ПОЛНЫЙ ЗНАМЕНАТЕЛЬ</div></article>
      <article class="token-track track-miner"><div class="track-label">ВИДИТ МАЙНЕР</div><div class="track-value-row"><output id="miner-token-count" class="track-value">__MINER__</output><output id="miner-token-share" class="track-share">__MINER_SHARE__</output></div><div class="track-rail"><div id="miner-bar" class="track-fill"></div></div><div id="miner-epoch" class="track-note good">НОВЫЙ EPOCH: ЗАГРУЗКА</div></article>
      <article class="token-track track-cpu"><div class="track-label">CPU</div><div class="track-value-row"><output id="cpu-token-count" class="track-value">__CPU__</output><output id="cpu-token-share" class="track-share">__CPU_SHARE__</output></div><div class="track-rail"><div id="cpu-bar" class="track-fill"></div></div><div id="cpu-note" class="track-note watch">НЕ РАСТЁТ: AUTHORITY LOCKED</div></article>
    </div>
  </div></section>
  <div class="epoch-strip"><span>МОСТ: <b>opportunity seq <span id="bridge-pair">— / —</span></b> · токены <span id="bridge-tokens">—</span> · pending <span id="bridge-queue">—</span></span><span id="epoch-visibility" class="epoch-visibility">STRUCTURE —</span></div>
  <section class="live-band"><div class="live-inner">
    <div class="window-head"><h2 class="band-title">ЖИВЫЕ ОКНА</h2><div id="window-summary" class="window-summary">NANDO — / MIXED — / OUTSIDE — / IDLE —</div></div>
    <div class="window-scroll"><div class="window-table"><div class="window-row header"><span>ОКНО</span><span>СЕССИЯ</span><span>КОНФИГ</span><span>СТАТУС</span><span>КОНЕЧНАЯ ТОЧКА</span></div><div id="window-rows"></div></div></div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <h2 class="band-title">ПОЧЕМУ CPU НЕ РАСТЁТ</h2>
    <div class="pipeline-scroll"><div class="pipeline">
      <div class="pipe-step"><div class="pipe-name">INGRESS</div><div class="pipe-state">PASS</div></div>
      <div class="pipe-step"><div class="pipe-name">LEARNING BRIDGE</div><div id="pipe-bridge" class="pipe-state">—</div></div>
      <div class="pipe-step"><div class="pipe-name">RELATION FRAMES</div><div id="pipe-relation" class="pipe-state">—</div></div>
      <div class="pipe-step watch"><div class="pipe-name">OPERATOR DISCOVERY</div><div id="pipe-discovery" class="pipe-state">WATCH</div></div>
      <div class="pipe-step block"><div class="pipe-name">CANDIDATE BUNDLE</div><div id="pipe-controller" class="pipe-state">INPUT 0</div></div>
      <div class="pipe-step block"><div class="pipe-name">NATURAL PACKAGE</div><div id="pipe-package" class="pipe-state">MISSING</div></div>
      <div class="pipe-step locked"><div class="pipe-name">ADMISSION</div><div id="pipe-admission" class="pipe-state">LOCKED</div></div>
      <div class="pipe-step muted"><div class="pipe-name">CPU ACCEPT</div><div id="pipe-cpu" class="pipe-state">0 NEW</div></div>
      <div class="break-line"></div><div class="break-label">ТЕКУЩИЙ РАЗРЫВ</div>
    </div></div>
    <div id="blocker-text" class="blocker">нет доказанного ACTIVE OperatorPackage</div>
    <div class="activity"><span class="activity-label">ЗАПРОСЫ / 60 С</span><div id="activity-bars" class="activity-bars"></div></div>
  </div></section>
  <footer class="live-foot"><div class="live-inner"><span>СЛЕДУЮЩИЙ РУБЕЖ: <span class="next-route">relation evidence → circuit → future proof → admission</span></span><span>false accepts <b id="false-accepts">0</b> · parity <b id="parity-mismatches">0</b> · bridge failures <b id="bridge-failures">0</b></span></div></footer>
</main>
<script>
(() => {
  const base = window.location.pathname.replace(/\/$/, "");
  const number = new Intl.NumberFormat("ru-RU");
  const samples = [];
  let previousRequests = null;
  let lastSuccess = Date.now();
  const node = (id) => document.getElementById(id);
  const text = (id, value) => { const target = node(id); if (target) target.textContent = value; };
  const ratio = (part, total, digits) => total > 0 ? `${(part * 100 / total).toFixed(digits).replace(".", ",")}%` : `0,${"0".repeat(digits)}%`;
  const width = (id, part, total) => { const target = node(id); if (target) target.style.width = total > 0 ? `${Math.max(0.25, part * 100 / total)}%` : "0"; };
  const routeLabel = (window) => window.route === "nando" ? "NANDO" : window.route === "mixed" ? "СМЕШАННО" : window.route === "outside_nando" ? "ВНЕ NANDO" : "ОЖИДАНИЕ";
  const renderWindows = (snapshot) => {
    text("window-summary", `NANDO ${snapshot.active_nando} / MIXED ${snapshot.active_mixed} / OUTSIDE ${snapshot.active_outside_nando} / IDLE ${snapshot.idle}`);
    const rows = node("window-rows");
    if (!rows) return;
    rows.replaceChildren();
    for (const window of snapshot.windows) {
      const row = document.createElement("div"); row.className = "window-row";
      const values = [window.project.toUpperCase(), window.session.startsWith("pid-") ? window.session : window.session.slice(0, 8), window.configured_for_nando ? "nando_nginx" : "default", routeLabel(window), window.remote_endpoints.join(", ") || "—"];
      values.forEach((value, index) => { const cell = document.createElement("span"); cell.textContent = value; if (index === 3) cell.className = `window-status ${window.route}`; row.appendChild(cell); });
      rows.appendChild(row);
    }
  };
  const renderActivity = (requestEvents) => {
    if (previousRequests !== null) samples.push(Math.max(0, requestEvents - previousRequests));
    previousRequests = requestEvents;
    while (samples.length > 30) samples.shift();
    const bars = node("activity-bars"); if (!bars) return;
    bars.replaceChildren(); const max = Math.max(1, ...samples);
    for (let index = 0; index < 30; index += 1) { const bar = document.createElement("span"); bar.className = "activity-bar"; const value = samples[index] || 0; bar.style.height = `${Math.max(2, value * 36 / max)}px`; bars.appendChild(bar); }
  };
  const renderTokens = (snapshot) => {
    text("total-token-count", number.format(snapshot.total_input_tokens)); text("miner-token-count", number.format(snapshot.miner_input_tokens)); text("cpu-token-count", number.format(snapshot.cpu_input_tokens));
    text("miner-token-share", ratio(snapshot.miner_input_tokens, snapshot.total_input_tokens, 2)); text("cpu-token-share", ratio(snapshot.cpu_input_tokens, snapshot.total_input_tokens, 3));
    width("miner-bar", snapshot.miner_input_tokens, snapshot.total_input_tokens); width("cpu-bar", snapshot.cpu_input_tokens, snapshot.total_input_tokens);
    const bridge = snapshot.bridge; const bridgeAvailable = bridge.hot_available && bridge.cold_available; const queue = bridge.opportunity_pending + bridge.opportunity_inflight; const structureComparable = bridgeAvailable && bridge.structural_epoch_match;
    text("miner-epoch", structureComparable ? `STRUCTURE SEQ ${bridge.structural_produced_sequence}/${bridge.structural_consumed_sequence} · PENDING ${bridge.structural_pending}` : bridgeAvailable ? "STRUCTURE: EPOCH НЕ СОВПАДАЕТ" : "STRUCTURE: HEALTH НЕДОСТУПЕН"); text("bridge-pair", `${bridge.hot_available ? bridge.opportunity_produced_sequence : "—"} / ${bridge.cold_available ? bridge.opportunity_consumed_sequence : "—"}`); text("bridge-tokens", number.format(bridge.request_tokens)); text("bridge-queue", queue); text("epoch-visibility", structureComparable ? `JOIN ${bridge.join_hits}/${bridge.join_attempts} · MISS ${bridge.join_misses}` : "STRUCTURE: НЕТ ОБЩЕГО EPOCH");
    text("services-count", `${bridge.services_active}/3`); text("false-accepts", bridge.false_accepts); text("parity-mismatches", bridge.parity_mismatches); text("bridge-failures", bridge.failures);
    const controllerInput = snapshot.controller_relation_candidates + snapshot.controller_collection_candidates;
    text("pipe-bridge", structureComparable ? `STRUCT ${bridge.structural_produced_sequence}/${bridge.structural_consumed_sequence} · PENDING ${bridge.structural_pending}` : "EPOCH/HEALTH BLOCK"); text("pipe-relation", structureComparable && bridge.structural_pending === 0 && bridge.structural_sequence_gaps === 0 && bridge.failures === 0 ? `JOIN ${bridge.join_hits}/${bridge.join_attempts} · RAW ${bridge.raw_evaluated}/${bridge.raw_verified}/${bridge.raw_abstains}` : "WATCH"); text("pipe-discovery", snapshot.admission_ready_cohorts > 0 ? `COHORTS ${snapshot.admission_ready_cohorts}` : "WATCH"); text("pipe-controller", `INPUT ${controllerInput} · CRYST ${snapshot.controller_crystallized_candidates}`);
    text("pipe-package", snapshot.response_package_count > 0 ? `PRESENT ${snapshot.response_package_count}` : "MISSING"); text("pipe-admission", snapshot.cpu_allowed ? "OPEN" : "LOCKED"); text("pipe-cpu", snapshot.cpu_allowed ? "ENABLED" : "0 NEW"); text("cpu-note", snapshot.cpu_allowed ? "AUTHORITY OPEN" : "НЕ РАСТЁТ: AUTHORITY LOCKED");
    text("blocker-text", controllerInput === 0 ? `cohort export → controller: ${snapshot.controller_blocker}` : snapshot.response_package_count === 0 ? "controller получил candidates, но Natural OperatorPackage ещё не выпущен" : snapshot.cpu_allowed ? "маршрут до CPU открыт" : "OperatorPackage существует, но authority остаётся закрыта");
    renderActivity(bridge.request_events); lastSuccess = Date.now();
  };
  const refresh = async () => {
    try { const [tokensResponse, connectionsResponse] = await Promise.all([fetch(`${base}/tokens`, {cache:"no-store"}), fetch(`${base}/connections`, {cache:"no-store"})]); if (!tokensResponse.ok || !connectionsResponse.ok) return; renderTokens(await tokensResponse.json()); renderWindows(await connectionsResponse.json()); } catch (_) {}
  };
  window.setInterval(() => text("live-age", Math.floor((Date.now() - lastSuccess) / 1000)), 1000);
  refresh(); window.setInterval(refresh, 2000);
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn bridge_view_keeps_hot_and_cold_owners_separate() {
        let hot = json!({"ok":true,"process":{"instance_id_sha256":"hot"},"structural":{"producer_failures":0},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"producer":{"last_sequence":45}},"opportunity":{"producer_last_sequence":45,"pending_events":2,"producer_request_events":20,"producer_request_input_tokens":6_876_562,"failures":0}});
        let cold = json!({"ok":true,"process":{"instance_id_sha256":"cold"},"structural":{"consumer_failures":0},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"sequence_gaps":0,"consumer":{"last_sequence":43}},"request_learning":{"structures_applied":43,"lookup_attempts":20,"lookup_hits":17,"lookup_misses":3},"opportunity":{"consumer_last_sequence":44,"consumer_inflight_events":1,"failures":0},"raw_replay":{"evaluated":12,"verified":3,"runtime_abstains":9,"execution_authority":false,"false_accepts":0,"parity_mismatches":0}});
        assert_eq!(
            bridge_view(&hot, &cold),
            BridgeView {
                hot_available: true,
                cold_available: true,
                hot_accepted: 45,
                cold_accepted: 43,
                loss: 2,
                queue: 5,
                hot_instance: "hot".to_owned(),
                cold_instance: "cold".to_owned(),
                structural_epoch_match: true,
                structural_produced_sequence: 45,
                structural_consumed_sequence: 43,
                structural_pending: 2,
                structures_applied: 43,
                join_attempts: 20,
                join_hits: 17,
                join_misses: 3,
                opportunity_produced_sequence: 45,
                opportunity_consumed_sequence: 44,
                request_events: 20,
                request_tokens: 6_876_562,
                opportunity_pending: 2,
                opportunity_inflight: 1,
                raw_evaluated: 12,
                raw_verified: 3,
                raw_abstains: 9,
                services_active: 3,
                ..BridgeView::default()
            }
        );
    }

    #[test]
    fn unavailable_health_does_not_invent_cross_epoch_loss() {
        let hot = json!({"ok":true,"durable_structure":{"producer":{"last_sequence":39}}});
        let view = bridge_view(&hot, &Value::Null);
        assert!(view.hot_available);
        assert!(!view.cold_available);
        assert_eq!(view.structural_produced_sequence, 39);
        assert_eq!(view.structural_consumed_sequence, 0);
        assert!(!view.structural_epoch_match);
    }

    #[test]
    fn render_contains_the_selected_signal_first_layout() {
        let html = render(InitialMetrics {
            total_tokens: 5_948_645_890,
            miner_tokens: 548_423_296,
            cpu_tokens: 42_515_297,
        });
        assert!(html.contains("КУДА УШЛИ ТОКЕНЫ"));
        assert!(html.contains("ПОЧЕМУ CPU НЕ РАСТЁТ"));
        assert!(html.contains("CANDIDATE BUNDLE"));
        assert!(html.contains("5 948 645 890"));
        assert!(html.contains("9,22%"));
    }
}
