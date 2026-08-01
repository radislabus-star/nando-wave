use serde::Serialize;
use serde_json::Value;

const DASHBOARD_BUILD: &str = "2026.07.31-b046";

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
    pub(crate) miner_window_unresolved_tokens: u64,
    pub(crate) optimistic_upper_bound_tokens: u64,
    pub(crate) legacy_total_tokens: u64,
    pub(crate) legacy_cpu_tokens: u64,
    pub(crate) cpu_allowed: bool,
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
    // The consumer owns acknowledgement and unlinks pending spool files. The
    // producer's process-local counter only catches up on periodic reconcile.
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
    let producer_counter_watermark_present = hot
        .pointer("/opportunity/producer_counter_started_after_sequence")
        .and_then(Value::as_u64)
        .is_some();
    let consumer_counter_watermark_present = cold
        .pointer("/opportunity/consumer_counter_started_after_sequence")
        .and_then(Value::as_u64)
        .is_some();
    let opportunity_produced_sequence = pointer_u64(hot, "/opportunity/producer_last_sequence");
    let opportunity_consumed_sequence = pointer_u64(cold, "/opportunity/consumer_last_sequence");
    let empty_spool_sequence_divergence = opportunity_pending == 0
        && opportunity_inflight == 0
        && opportunity_produced_sequence != opportunity_consumed_sequence;
    let (opportunity_counter_epoch_match, opportunity_counter_epoch_reason) =
        if !producer_counter_watermark_present || !consumer_counter_watermark_present {
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
        // In-flight events still own their pending spool files. Adding them
        // again would double-count the same durable backlog.
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
    let miner_window_unrecognized_tokens = initial
        .miner_window_total_tokens
        .saturating_sub(initial.miner_window_cpu_tokens);
    let miner_window_llm_only_tokens =
        miner_window_unrecognized_tokens.saturating_sub(initial.miner_window_unresolved_tokens);
    let (
        pipeline_title,
        admission_step_class,
        admission_state,
        cpu_step_class,
        cpu_state,
        blocker_class,
        blocker,
    ) = if initial.cpu_allowed {
        (
            "МАРШРУТ ДО CPU",
            "good",
            "OPEN",
            "good",
            "ENABLED",
            "coverage",
            "маршрут до CPU открыт",
        )
    } else {
        (
            "ПОЧЕМУ CPU НЕ РАСТЁТ",
            "locked",
            "LOCKED",
            "muted",
            "0 NEW",
            "critical",
            "нет доказанного ACTIVE OperatorPackage",
        )
    };
    TEMPLATE
        .replace("__DASHBOARD_BUILD__", DASHBOARD_BUILD)
        .replace(
            "__SERVER_TOTAL__",
            &format_number(initial.server_total_tokens),
        )
        .replace("__SERVER_CPU__", &format_number(initial.server_cpu_tokens))
        .replace(
            "__SERVER_CPU_SHARE__",
            &format_percent(initial.server_cpu_tokens, initial.server_total_tokens, 1),
        )
        .replace(
            "__EPOCH_TOTAL__",
            &format_number(initial.epoch_total_tokens),
        )
        .replace(
            "__EPOCH_EVENTS__",
            &format_number(initial.epoch_total_events),
        )
        .replace("__EPOCH_CPU__", &format_number(initial.epoch_cpu_tokens))
        .replace(
            "__EPOCH_CPU_ACCEPTS__",
            &format_number(initial.epoch_cpu_accepts),
        )
        .replace(
            "__EPOCH_CPU_SHARE__",
            &format_percent(initial.epoch_cpu_tokens, initial.epoch_total_tokens, 1),
        )
        .replace(
            "__MINER_TOTAL__",
            &format_number(initial.miner_window_total_tokens),
        )
        .replace(
            "__MINER_INTENTS__",
            &format_number(initial.miner_window_total_intents),
        )
        .replace(
            "__MINER_CPU__",
            &format_number(initial.miner_window_cpu_tokens),
        )
        .replace(
            "__MINER_CPU_INTENTS__",
            &format_number(initial.miner_window_cpu_intents),
        )
        .replace(
            "__LEGACY_TOTAL__",
            &format_number(initial.legacy_total_tokens),
        )
        .replace("__LEGACY_CPU__", &format_number(initial.legacy_cpu_tokens))
        .replace(
            "__MINER_CPU_SHARE__",
            &format_percent(
                initial.miner_window_cpu_tokens,
                initial.miner_window_total_tokens,
                1,
            ),
        )
        .replace(
            "__MINER_UNRESOLVED__",
            &format_number(miner_window_unrecognized_tokens),
        )
        .replace(
            "__MINER_UNRESOLVED_SHARE__",
            &format_percent(
                miner_window_unrecognized_tokens,
                initial.miner_window_total_tokens,
                1,
            ),
        )
        .replace(
            "__MINER_RESEARCHABLE__",
            &format_number(initial.miner_window_unresolved_tokens),
        )
        .replace(
            "__MINER_LLM_ONLY__",
            &format_number(miner_window_llm_only_tokens),
        )
        .replace(
            "__CEILING_VALUES__",
            &format!(
                "{} / {}",
                format_number(initial.optimistic_upper_bound_tokens),
                format_number(initial.miner_window_total_tokens)
            ),
        )
        .replace(
            "__CEILING_SHARE__",
            &format_percent(
                initial.optimistic_upper_bound_tokens,
                initial.miner_window_total_tokens,
                1,
            ),
        )
        .replace(
            "__LEGACY_VALUES__",
            &format!(
                "{} вход / {} CPU",
                format_number(initial.legacy_total_tokens),
                format_number(initial.legacy_cpu_tokens)
            ),
        )
        .replace("__PIPELINE_TITLE__", pipeline_title)
        .replace("__ADMISSION_STEP_CLASS__", admission_step_class)
        .replace("__ADMISSION_STATE__", admission_state)
        .replace("__CPU_STEP_CLASS__", cpu_step_class)
        .replace("__CPU_STATE__", cpu_state)
        .replace("__BLOCKER_CLASS__", blocker_class)
        .replace("__BLOCKER__", blocker)
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
.overview-head { display:flex; justify-content:space-between; align-items:baseline; gap:18px; margin-bottom:15px; }
.overview-head .band-title { margin:0; }
.overview-rule { color:var(--amber); font-size:12px; font-weight:800; text-align:right; }
.overview-rule.good { color:var(--green); }
.overview-rule.warning { color:var(--red); }
.traffic-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border:1px solid var(--line); }
.traffic-stage { min-width:0; min-height:230px; padding:18px 20px; border-right:1px solid var(--line); border-top:3px solid #59636a; background:#111518; }
.traffic-stage:last-child { border-right:0; }
.traffic-stage.miner { border-top-color:var(--cyan); }
.traffic-stage.recognized,.traffic-stage.cpu { border-top-color:var(--green); }
.stage-index { color:var(--muted); font-size:11px; font-weight:800; }
.stage-label { min-height:38px; margin-top:9px; color:#dce2e6; font-size:13px; font-weight:800; line-height:1.45; }
.stage-value-row { display:flex; flex-wrap:wrap; align-items:baseline; justify-content:space-between; gap:6px 10px; margin-top:14px; }
.stage-value { min-width:0; max-width:100%; color:#eef1f3; font-size:27px; font-weight:800; white-space:nowrap; }
.traffic-stage.miner .stage-value,.traffic-stage.miner .stage-share { color:var(--cyan); }
.traffic-stage.recognized .stage-value,.traffic-stage.recognized .stage-share,.traffic-stage.cpu .stage-value,.traffic-stage.cpu .stage-share { color:var(--green); }
.stage-share { flex:0 0 auto; color:var(--muted); font-size:15px; font-weight:800; }
.stage-unit { margin-top:5px; color:var(--muted); font-size:11px; font-weight:800; }
.stage-meta { margin-top:15px; color:#c6cdd1; font-size:12px; font-weight:700; line-height:1.45; }
.stage-scope { margin-top:9px; color:var(--muted); font-size:10px; font-weight:700; line-height:1.4; }
.stage-rail { height:8px; margin-top:13px; background:#282e32; overflow:hidden; }
.stage-fill { width:0; height:100%; min-width:2px; background:var(--green); transition:width .25s ease; }
.scope-alert { margin-top:14px; padding:11px 14px; color:#ced5d9; background:#171b1e; border-left:3px solid var(--amber); font-size:12px; font-weight:700; line-height:1.45; }
.scope-alert strong { color:var(--amber); }
.compression-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.compression-cell { min-width:0; min-height:188px; padding:17px 20px; border-right:1px solid var(--line); background:#111518; }
.compression-cell:last-child { border-right:0; }
.compression-cell.natural { border-top:3px solid var(--green); padding-top:14px; }
.compression-label { color:var(--muted); font-size:11px; font-weight:800; }
.compression-main { display:flex; flex-wrap:wrap; justify-content:space-between; align-items:baseline; gap:7px 12px; margin-top:11px; }
.compression-value { color:#e7ecef; font-size:22px; font-weight:800; overflow-wrap:anywhere; }
.compression-ratio { color:var(--green); font-size:20px; font-weight:800; }
.compression-unit { margin-top:5px; color:var(--muted); font-size:10px; font-weight:800; }
.compression-rail { height:10px; margin-top:14px; background:#282e32; overflow:hidden; }
.compression-fill { width:0; height:100%; min-width:2px; background:var(--green); transition:width .25s ease; }
.compression-meta { margin-top:11px; color:#c9d0d4; font-size:12px; font-weight:700; line-height:1.45; }
.compression-scope { margin-top:7px; color:var(--muted); font-size:10px; line-height:1.4; overflow-wrap:anywhere; }
.compression-proof { display:grid; grid-template-columns:auto minmax(0,1fr); gap:8px 14px; margin-top:13px; color:var(--muted); font-size:11px; line-height:1.45; }
.compression-proof strong { color:var(--green); }
.compression-proof strong.warning { color:var(--amber); }
.compression-root { font-family:ui-monospace,SFMono-Regular,Menlo,monospace; color:#cbd2d6; overflow-wrap:anywhere; }
.scope-grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); border-top:1px solid var(--line); }
.scope-metric { min-width:0; padding:16px 22px; border-right:1px solid var(--line); }
.scope-metric:first-child { padding-left:0; }
.scope-metric:last-child { padding-right:0; border-right:0; }
.scope-label { color:var(--muted); font-size:11px; font-weight:800; }
.scope-value { margin-top:7px; color:#dce2e6; font-size:15px; font-weight:800; overflow-wrap:anywhere; }
.scope-share { display:block; margin-top:5px; color:var(--green); font-size:22px; font-weight:800; }
.scope-note { margin-top:6px; color:var(--muted); font-size:11px; line-height:1.35; }
.scope-metric.ceiling .scope-share { color:var(--amber); }
.scope-metric.unresolved .scope-share { color:var(--amber); }
.ingestion-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border:1px solid var(--line); }
.ingestion-cell { min-width:0; padding:16px 18px; border-right:1px solid var(--line); background:#111518; }
.ingestion-cell:last-child { border-right:0; }
.ingestion-label { color:var(--muted); font-size:11px; font-weight:800; }
.ingestion-value { margin-top:8px; color:var(--cyan); font-size:20px; font-weight:800; overflow-wrap:anywhere; }
.ingestion-cell.applied .ingestion-value { color:var(--green); }
.ingestion-cell.backlog .ingestion-value { color:var(--amber); }
.ingestion-cell.backlog.clear .ingestion-value { color:var(--green); }
.ingestion-cell.backlog.invalid .ingestion-value { color:var(--red); }
.ingestion-note { margin-top:6px; color:var(--muted); font-size:10px; line-height:1.35; }
.legacy-strip { display:flex; justify-content:center; gap:14px; align-items:center; flex-wrap:wrap; padding:13px 24px; border-top:1px solid var(--line); color:var(--muted); text-align:center; font-size:12px; font-weight:700; }
.legacy-strip b { color:#dce2e6; }
.legacy-scope { color:var(--amber); }
.ms3-grid { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.ms3-cell { min-width:0; padding:14px 18px; border-right:1px solid var(--line); }
.ms3-cell:nth-child(4n) { border-right:0; }
.ms3-cell:nth-child(-n+4) { border-bottom:1px solid var(--line); }
.ms3-label { color:var(--muted); font-size:11px; font-weight:800; }
.ms3-value { margin-top:7px; color:var(--cyan); font-size:15px; font-weight:800; overflow-wrap:anywhere; }
.ms3-value.good { color:var(--green); }
.ms3-value.watch { color:var(--amber); }
.ms3-value.locked { color:var(--red); }
.ms3-value.muted { color:var(--muted); }
.ms3-note { margin-top:11px; color:var(--muted); font-size:12px; line-height:1.45; overflow-wrap:anywhere; }
.epoch-strip { display:flex; justify-content:center; gap:24px; align-items:center; flex-wrap:wrap; padding:15px 24px; border-bottom:1px solid var(--line); color:#d8dde1; text-align:center; font-size:14px; font-weight:700; }
.epoch-strip b,.epoch-visibility { color:var(--green); }
.window-head { display:flex; justify-content:space-between; gap:20px; align-items:baseline; margin-bottom:12px; }
.window-summary { color:var(--muted); font-size:13px; font-weight:700; }
.window-scroll { max-height:430px; overflow:auto; scrollbar-gutter:stable; }
.window-table { min-width:820px; }
.window-row { display:grid; grid-template-columns:1.1fr 1fr 1fr 1fr 1.2fr; gap:18px; align-items:center; min-height:50px; border-bottom:1px solid var(--line); font-size:14px; }
.window-row.header { position:sticky; top:0; z-index:2; min-height:34px; color:var(--muted); background:var(--bg); font-size:12px; font-weight:800; }
.window-status { font-weight:800; }
.window-status.nando { color:var(--green); }
.window-status.outside_nando { color:var(--red); }
.window-status.mixed { color:var(--amber); }
.window-status.idle { color:var(--muted); }
.pipeline-scroll { overflow-x:auto; padding-bottom:5px; }
.pipeline { position:relative; display:grid; grid-template-columns:repeat(9,minmax(120px,1fr)); min-width:1215px; border-top:1px solid var(--line); border-bottom:1px solid var(--line); }
.pipe-step { position:relative; min-height:100px; padding:17px 18px; border-right:1px solid var(--line); }
.pipe-step:last-child { border-right:0; }
.pipe-step::after { content:"→"; position:absolute; right:-8px; top:37px; z-index:1; color:#c7cdd1; background:var(--bg); }
.pipe-step:last-child::after { content:""; }
.pipe-name { color:#d6dce0; font-size:12px; font-weight:800; }
.pipe-state { margin-top:12px; color:var(--green); font-size:14px; font-weight:800; }
.pipe-step.watch .pipe-state { color:var(--amber); }
.pipe-step.block .pipe-state,.pipe-step.locked .pipe-state { color:var(--red); }
.pipe-step.muted .pipe-state { color:var(--muted); }
.blocker { display:flex; justify-content:center; gap:9px; margin-top:14px; color:var(--red); text-align:center; font-size:13px; }
.blocker.coverage { color:var(--amber); }
.blocker-label { flex:0 0 auto; font-weight:800; }
.blocker-copy { min-width:0; overflow-wrap:anywhere; }
.activity { display:grid; grid-template-columns:150px minmax(0,1fr); gap:18px; align-items:end; margin-top:18px; }
.activity-label { color:var(--muted); font-size:12px; font-weight:800; }
.activity-bars { display:flex; align-items:end; gap:3px; height:38px; border-bottom:1px solid var(--line); }
.activity-bar { flex:1 1 0; min-width:2px; height:2px; background:var(--cyan); }
.live-foot { border-bottom:0; }
.live-foot .live-inner { display:flex; justify-content:space-between; gap:20px; color:var(--muted); font-size:12px; }
.next-route { color:var(--cyan); }
@media (max-width:1200px) {
  .stage-value { font-size:23px; }
  .stage-share { font-size:13px; }
  .pipeline-scroll { overflow-x:visible; }
  .pipeline { grid-template-columns:repeat(3,minmax(0,1fr)); min-width:0; }
  .pipe-step { min-height:88px; border-bottom:1px solid var(--line); }
  .pipe-step:nth-of-type(3) { grid-column:3; grid-row:1; }
  .pipe-step:nth-of-type(4) { grid-column:3; grid-row:2; }
  .pipe-step:nth-of-type(5) { grid-column:2; grid-row:2; }
  .pipe-step:nth-of-type(6) { grid-column:1; grid-row:2; }
  .pipe-step:nth-of-type(7) { grid-column:1; grid-row:3; }
  .pipe-step:nth-of-type(8) { grid-column:2; grid-row:3; }
  .pipe-step:nth-of-type(9) { grid-column:3; grid-row:3; }
  .pipe-step:nth-of-type(3)::after,.pipe-step:nth-of-type(6)::after { content:"↓"; right:14px; top:auto; bottom:-10px; }
  .pipe-step:nth-of-type(4)::after,.pipe-step:nth-of-type(5)::after { content:"←"; right:auto; left:-8px; }
  .pipe-step:nth-of-type(7)::after,.pipe-step:nth-of-type(8)::after { content:"→"; }
  .pipe-step:nth-of-type(9)::after { content:""; }
}
@media (max-width:900px) {
  .overview-head { align-items:flex-start; flex-direction:column; gap:7px; }
  .overview-rule { text-align:left; }
  .traffic-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .traffic-stage { border-bottom:1px solid var(--line); }
  .traffic-stage:nth-child(2n) { border-right:0; }
  .traffic-stage:nth-last-child(-n+2) { border-bottom:0; }
  .scope-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .compression-grid { grid-template-columns:1fr; }
  .compression-cell { min-height:0; border-right:0; border-bottom:1px solid var(--line); }
  .compression-cell:last-child { border-bottom:0; }
  .ingestion-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .ingestion-cell:nth-child(2n) { border-right:0; }
  .ingestion-cell:nth-child(-n+2) { border-bottom:1px solid var(--line); }
  .scope-metric:nth-child(2) { border-right:0; }
  .scope-metric:nth-child(-n+2) { border-bottom:1px solid var(--line); }
  .ms3-grid { grid-template-columns:repeat(2,minmax(0,1fr)); }
  .ms3-cell { border-bottom:1px solid var(--line); }
  .ms3-cell:nth-child(2n) { border-right:0; }
  .ms3-cell:nth-last-child(-n+2) { border-bottom:0; }
  .window-head,.live-foot .live-inner { align-items:flex-start; flex-direction:column; gap:8px; }
}
@media (max-width:560px) {
  .nando-live { overflow-x:hidden; }
  .live-inner { padding:16px 12px; }
  .live-head .live-inner { align-items:flex-start; flex-direction:column; gap:7px; }
  .traffic-grid { grid-template-columns:1fr; }
  .traffic-stage { min-height:0; padding:15px 12px; border-right:0; border-bottom:1px solid var(--line); }
  .traffic-stage:nth-last-child(-n+2) { border-bottom:1px solid var(--line); }
  .traffic-stage:last-child { border-bottom:0; }
  .stage-label { min-height:0; }
  .stage-value { font-size:24px; }
  .stage-share { font-size:14px; }
  .scope-grid { grid-template-columns:1fr; }
  .compression-cell { padding:14px 12px; }
  .compression-proof { grid-template-columns:1fr; gap:4px; }
  .ingestion-grid { grid-template-columns:1fr; }
  .ingestion-cell:nth-child(n) { border-right:0; border-bottom:1px solid var(--line); }
  .ingestion-cell:last-child { border-bottom:0; }
  .scope-metric,.scope-metric:first-child,.scope-metric:last-child { padding:13px 0; border-right:0; border-bottom:1px solid var(--line); }
  .scope-metric:last-child { border-bottom:0; }
  .ms3-grid { grid-template-columns:1fr; }
  .ms3-cell,.ms3-cell:last-child { grid-column:auto; padding:12px 0; border-right:0; border-bottom:1px solid var(--line); }
  .ms3-cell:last-child { border-bottom:0; }
  .epoch-strip { padding:13px 12px; font-size:12px; }
  .window-scroll { max-height:430px; overflow-y:auto; overflow-x:visible; scrollbar-gutter:auto; }
  .pipeline-scroll { overflow-x:visible; }
  .window-table,.pipeline { min-width:0; }
  .window-row.header { display:none; }
  .window-row { grid-template-columns:minmax(0,1fr); gap:3px; align-items:baseline; padding:10px 0; font-size:12px; }
  .window-row span { min-width:0; overflow-wrap:anywhere; }
  .window-row span:first-child { font-size:14px; font-weight:800; }
  .window-row span:nth-child(2)::before { content:"SESSION "; color:var(--muted); }
  .window-row span:nth-child(3)::before { content:"CONFIG "; color:var(--muted); }
  .window-row span:nth-child(4)::before { content:"ROUTE "; color:var(--muted); }
  .window-row span:nth-child(5) { color:var(--muted); }
  .window-row span:nth-child(5)::before { content:"ENDPOINT "; }
  .pipeline { grid-template-columns:1fr; }
  .pipe-step:nth-of-type(n) { grid-column:auto; grid-row:auto; }
  .pipe-step { min-height:0; padding:11px 12px; border-right:0; border-bottom:1px solid var(--line); }
  .pipe-step:last-of-type { border-bottom:0; }
  .pipe-step:nth-of-type(n)::after { content:"↓"; right:12px; left:auto; top:auto; bottom:-10px; }
  .pipe-step:nth-of-type(9)::after { content:""; }
  .pipe-name,.pipe-state { display:inline; }
  .pipe-state { margin:0 0 0 10px; }
  .blocker { align-items:flex-start; flex-direction:column; gap:4px; text-align:left; }
  .activity { grid-template-columns:1fr; gap:6px; }
}
</style>
<main class="nando-live" data-dashboard-build="__DASHBOARD_BUILD__" aria-label="Nando live traffic control">
  <header class="live-head"><div class="live-inner">
    <h1 class="live-title">NANDO / LIVE TRAFFIC CONTROL</h1>
    <div class="live-clock"><b>LIVE</b> · API <span id="live-age">0</span> с · SOURCE <span id="source-age">—</span> с · SERVICES <span id="services-count">—/3</span></div>
  </div></header>
  <section class="live-band"><div class="live-inner">
    <div class="overview-head">
      <h2 class="band-title">ЧЕТЫРЕ ГЛАВНЫЕ ЦИФРЫ</h2>
      <div class="overview-rule">РАЗДЕЛЬНЫЕ SCOPE · SERVER HISTORY / MINER WINDOW / EXECUTION RECEIPTS</div>
    </div>
    <div class="traffic-grid">
      <article class="traffic-stage">
        <div class="stage-index">1 · SERVER ACCOUNTING</div>
        <div class="stage-label">ВСЕ ЗАПИСАННЫЕ ACCOUNTING PARTITIONS</div>
        <div class="stage-value-row"><output id="server-total-token-count" class="stage-value">__SERVER_TOTAL__</output></div>
        <div class="stage-unit">УЧТЁННЫХ ВХОДНЫХ ТОКЕНОВ · REQUEST_EVENT.V1</div>
        <div id="server-total-breakdown" class="stage-meta">V3 __LEGACY_TOTAL__ + V4 __EPOCH_TOTAL__</div>
        <div id="server-total-scope" class="stage-scope">ВСЯ СОХРАНЁННАЯ ИСТОРИЯ УЧЁТА · ТЕКУЩАЯ V4: __EPOCH_EVENTS__ ЗАПРОСОВ</div>
      </article>
      <article class="traffic-stage miner">
        <div class="stage-index">2 · MINER CLASSIFICATION WINDOW</div>
        <div class="stage-label">ОПУБЛИКОВАННЫЙ КОРПУС КЛАССИФИКАЦИИ</div>
        <div class="stage-value-row"><output id="miner-window-total" class="stage-value">__MINER_TOTAL__</output></div>
        <div class="stage-unit">ВХОДНЫХ ТОКЕНОВ В OPPORTUNITY-BOARD.V3</div>
        <div id="miner-window-intents" class="stage-meta">__MINER_INTENTS__ intents</div>
        <div id="miner-window-start" class="stage-scope">СВОЙ WATERMARK И ПЕРИОД</div>
      </article>
      <article class="traffic-stage recognized">
        <div class="stage-index">3 · CPU_VERIFIED CLASS</div>
        <div class="stage-label">МАЙНЕР РАСПОЗНАЛ</div>
        <div class="stage-value-row"><output id="miner-window-cpu-token-count" class="stage-value">__MINER_CPU__</output><output id="miner-window-cpu-share" class="stage-share">__MINER_CPU_SHARE__</output></div>
        <div class="stage-unit">ТОКЕНОВ С ДОКАЗАННЫМ CPU-КЛАССОМ</div>
        <div id="miner-window-cpu-intents" class="stage-meta">__MINER_CPU_INTENTS__ verified intents</div>
        <div class="stage-rail"><div id="miner-recognized-bar" class="stage-fill"></div></div>
      </article>
      <article class="traffic-stage cpu">
        <div class="stage-index">4 · EXECUTION RECEIPTS</div>
        <div class="stage-label">РЕАЛЬНО ВОСПРОИЗВЕДЕНО НА CPU</div>
        <div class="stage-value-row"><output id="server-cpu-token-count" class="stage-value">__SERVER_CPU__</output><output id="server-cpu-share" class="stage-share">__SERVER_CPU_SHARE__</output></div>
        <div class="stage-unit">ТОКЕНОВ С VERIFIED EXECUTION RECEIPT · ДОЛЯ ОТ RECORDED SERVER ACCOUNTING</div>
        <div id="server-cpu-breakdown" class="stage-meta">V3 __LEGACY_CPU__ + V4 __EPOCH_CPU__</div>
        <div id="current-v4-execution" class="stage-scope">V4 __EPOCH_CPU__ / __EPOCH_TOTAL__ · __EPOCH_CPU_SHARE__ · __EPOCH_CPU_ACCEPTS__ accepts</div>
      </article>
    </div>
    <div class="scope-alert"><strong>ЭТО НЕ ОДНА ПОСЛЕДОВАТЕЛЬНАЯ ВОРОНКА.</strong> Recorded server accounting, miner classification window и live process epoch имеют разные watermark и периоды: между ними нельзя считать остаток или процент. CPU share в четвёртом блоке относится только к записанным request_event.v1 partitions; recognition share — только к опубликованному корпусу майнера.</div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <div class="overview-head">
      <h2 class="band-title">СЖАТИЕ ВХОДНОГО ТРАФИКА НА CPU</h2>
      <div class="overview-rule good">AVOIDED INPUT / ELIGIBLE INPUT · EXACT O200K TOKENS</div>
    </div>
    <div class="compression-grid">
      <article class="compression-cell">
        <div class="compression-label">ВСЯ ЗАПИСАННАЯ ИСТОРИЯ</div>
        <div class="compression-main"><output id="compression-lifetime-tokens" class="compression-value">— / —</output><output id="compression-lifetime-ratio" class="compression-ratio">—</output></div>
        <div class="compression-unit">CPU-ПРЕДОТВРАЩЕНО / УЧТЕНО ВХОДНЫХ ТОКЕНОВ</div>
        <div class="compression-rail"><div id="compression-lifetime-bar" class="compression-fill"></div></div>
        <div class="compression-meta">ВСЕ ACCOUNTING PARTITIONS</div>
        <div class="compression-scope">TOKEN TOTALS COMPOSABLE · CALL COUNTS МЕЖДУ LEGACY PARTITIONS НЕ СКЛЕИВАЮТСЯ</div>
      </article>
      <article class="compression-cell">
        <div class="compression-label">ТЕКУЩАЯ V4 ACCOUNTING EPOCH</div>
        <div class="compression-main"><output id="compression-epoch-tokens" class="compression-value">— / —</output><output id="compression-epoch-ratio" class="compression-ratio">—</output></div>
        <div class="compression-unit">CPU-ПРЕДОТВРАЩЕНО / ORDINARY INPUT</div>
        <div class="compression-rail"><div id="compression-epoch-bar" class="compression-fill"></div></div>
        <div id="compression-epoch-calls" class="compression-meta">— REQUESTS · — CPU ACCEPTS · — UPSTREAM CALLS AVOIDED</div>
        <div id="compression-epoch-since" class="compression-scope">EPOCH —</div>
      </article>
      <article class="compression-cell natural">
        <div class="compression-label">НОВЫЙ NATURAL MS4 PACKAGE</div>
        <div class="compression-main"><output id="compression-ms4-tokens" class="compression-value">— / —</output><output id="compression-ms4-ratio" class="compression-ratio">—</output></div>
        <div class="compression-unit">CPU-ПРЕДОТВРАЩЕНО / PACKAGE-MATCHED INPUT</div>
        <div class="compression-rail"><div id="compression-ms4-bar" class="compression-fill"></div></div>
        <div id="compression-ms4-calls" class="compression-meta">ORDINARY PROOF PENDING</div>
        <div id="compression-ms4-package" class="compression-scope">PACKAGE —</div>
      </article>
    </div>
    <div class="compression-proof"><strong id="compression-ms4-status">MS4 PENDING</strong><span id="compression-ms4-time">LAST ACCEPT —</span><span>RECEIPT ROOT</span><span id="compression-ms4-root" class="compression-root">—</span></div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <div class="overview-head">
      <h2 class="band-title">ЖИВОЙ ВХОД МАЙНЕРА</h2>
      <div id="ingestion-epoch" class="overview-rule">PROCESS EPOCH</div>
    </div>
    <div class="ingestion-grid">
      <div class="ingestion-cell"><div class="ingestion-label">ПОЛУЧЕНО DURABLE</div><div id="ingestion-received" class="ingestion-value">—</div><div id="ingestion-received-events" class="ingestion-note">hot producer</div></div>
      <div class="ingestion-cell applied"><div class="ingestion-label">ПРИМЕНЕНО LEARNER</div><div id="ingestion-applied" class="ingestion-value">—</div><div id="ingestion-applied-events" class="ingestion-note">cold consumer</div></div>
      <div id="ingestion-backlog-cell" class="ingestion-cell backlog"><div class="ingestion-label">DURABLE BACKLOG</div><div id="ingestion-backlog" class="ingestion-value">—</div><div id="ingestion-inflight" class="ingestion-note">inflight является частью backlog</div></div>
      <div class="ingestion-cell"><div class="ingestion-label">TOKEN COUNTER DELTA</div><div id="ingestion-token-lag" class="ingestion-value">—</div><div id="ingestion-token-scope" class="ingestion-note">только при общем process epoch</div></div>
    </div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <h2 class="band-title">ЧТО МАЙНЕР ЕЩЁ НЕ РАСПОЗНАЛ</h2>
    <div class="scope-grid">
      <div class="scope-metric unresolved"><div class="scope-label">ВСЕГО БЕЗ CPU-КЛАССА</div><div id="miner-unresolved-values" class="scope-value">__MINER_UNRESOLVED__</div><output id="miner-unresolved-share" class="scope-share">__MINER_UNRESOLVED_SHARE__</output><div id="miner-unresolved-note" class="scope-note">из них исследуемо __MINER_RESEARCHABLE__ · доказанно LLM-only __MINER_LLM_ONLY__</div></div>
      <div class="scope-metric ceiling"><div class="scope-label">ТЕОРЕТИЧЕСКИЙ ПОТОЛОК</div><div id="scope-ceiling-values" class="scope-value">__CEILING_VALUES__</div><output id="scope-ceiling-share" class="scope-share">__CEILING_SHARE__</output><div class="scope-note">ordinary минус доказанно irreducible; не CPU и не authority</div></div>
    </div>
    <div id="miner-class-ledger" class="ms3-note">Загрузка классов opportunity…</div>
    <div class="legacy-strip"><span>АРХИВ V3: <b id="legacy-values">__LEGACY_VALUES__</b></span><span class="legacy-scope">АРХИВНАЯ PARTITION · УЖЕ ВКЛЮЧЕНА В SERVER TOTAL · ТЕКУЩАЯ ДОЛЯ ПОКАЗАНА ОТДЕЛЬНО ДЛЯ V4</span></div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <h2 class="band-title">АВТОНОМНЫЙ ЦИКЛ ЕСТЕСТВЕННОГО ОПЕРАТОРА · MS3 → MS4</h2>
    <div class="ms3-grid">
      <div class="ms3-cell"><div class="ms3-label">ACTIVE GENERATION</div><div id="ms3-generation" class="ms3-value watch">—</div></div>
      <div class="ms3-cell"><div class="ms3-label">PREDECESSOR</div><div id="ms3-predecessor" class="ms3-value locked">—</div></div>
      <div class="ms3-cell"><div class="ms3-label">SCIENTIFIC / LINKED LIMIT</div><div id="ms3-acquisition" class="ms3-value watch">— / 256</div><div id="ms3-acquisition-raw" class="scope-note">CANDIDATE — · RAW — / 4096</div></div>
      <div class="ms3-cell"><div class="ms3-label">TERMINAL / RELEVANT / LINKED</div><div id="ms3-evidence" class="ms3-value watch">0 / 0 / 0</div></div>
      <div class="ms3-cell"><div class="ms3-label">ROUTE SETTLEMENT</div><div id="ms3-settlement" class="ms3-value watch">0 / 0</div><div id="ms3-settlement-note" class="scope-note">SETTLED / PENDING / STALLED / STRUCTURAL</div></div>
      <div class="ms3-cell"><div class="ms3-label">TRANSPORT BINDING FAILURES</div><div id="ms3-binding-failures" class="ms3-value watch">0</div><div id="ms3-binding-failure-note" class="scope-note">NONE · REUSE 0</div></div>
      <div class="ms3-cell"><div class="ms3-label">LAW</div><div id="ms3-law" class="ms3-value locked">НЕ ЗАМОРОЖЕН</div></div>
      <div class="ms3-cell"><div class="ms3-label">FUTURE APPLICABILITY</div><div id="ms3-future-applicability" class="ms3-value watch">0 / 256</div></div>
      <div class="ms3-cell"><div class="ms3-label">DURABLE / ACTIVE PREDICTIONS</div><div id="ms3-predictions" class="ms3-value watch">0 / 0</div></div>
      <div class="ms3-cell"><div class="ms3-label">INDEPENDENT FUTURE</div><div id="ms3-future" class="ms3-value watch">НЕ ОЦЕНЕН</div></div>
      <div class="ms3-cell"><div class="ms3-label">AUTHORITY</div><div id="ms3-authority" class="ms3-value locked">FALSE</div></div>
      <div class="ms3-cell"><div class="ms3-label">CAPTURE / RECEIPT HEALTH</div><div id="ms3-operational-health" class="ms3-value watch">ЗАГРУЗКА</div><div id="ms3-operational-note" class="scope-note">operational guard</div></div>
      <div class="ms3-cell"><div class="ms3-label">AUTONOMOUS STAGE</div><div id="ms4-stage" class="ms3-value watch">WAITING FOR MS3</div></div>
      <div class="ms3-cell"><div class="ms3-label">BUNDLE / PACKAGE</div><div id="ms4-package" class="ms3-value muted">НЕ ЗАПЕЧАТАН</div></div>
      <div class="ms3-cell"><div class="ms3-label">EXTERNAL ADMISSION</div><div id="ms4-admission" class="ms3-value locked">FALSE</div></div>
      <div class="ms3-cell"><div class="ms3-label">ORDINARY CPU PROOF</div><div id="ms4-ordinary-proof" class="ms3-value locked">PENDING</div></div>
      <div class="ms3-cell"><div class="ms3-label">INDEPENDENT EXACT WAVE</div><div id="ms4-exact-wave" class="ms3-value watch">COLLECTING</div><div id="ms4-exact-wave-note" class="scope-note">POST-CENTER HOLDOUT</div></div>
      <div class="ms3-cell"><div class="ms3-label">CALIBRATION CONTROLS / ATOMS</div><div id="ms4-calibration-controls" class="ms3-value watch">0 / 0</div><div id="ms4-calibration-note" class="scope-note">IN-SAMPLE DIAGNOSTIC ONLY</div></div>
    </div>
    <div id="ms3-note" class="ms3-note">Загрузка generation lifecycle и frozen acquisition…</div>
  </div></section>
  <div class="epoch-strip"><span>МОСТ: <b>opportunity seq <span id="bridge-pair">— / —</span></b> · hot process tokens <span id="bridge-tokens">—</span> · pending <span id="bridge-queue">—</span></span><span id="epoch-visibility" class="epoch-visibility">STRUCTURE —</span></div>
  <section class="live-band"><div class="live-inner">
    <h2 id="pipeline-title" class="band-title">__PIPELINE_TITLE__</h2>
    <div class="pipeline-scroll"><div class="pipeline">
      <div class="pipe-step"><div class="pipe-name">INGRESS</div><div class="pipe-state">PASS</div></div>
      <div class="pipe-step"><div class="pipe-name">LEARNING BRIDGE</div><div id="pipe-bridge" class="pipe-state">—</div></div>
      <div class="pipe-step"><div class="pipe-name">RELATION FRAMES</div><div id="pipe-relation" class="pipe-state">—</div></div>
      <div id="pipe-discovery-step" class="pipe-step watch"><div class="pipe-name">OPERATOR DISCOVERY</div><div id="pipe-discovery" class="pipe-state">WATCH</div></div>
      <div id="pipe-candidate-step" class="pipe-step watch"><div class="pipe-name">CANDIDATE INPUT</div><div id="pipe-candidate" class="pipe-state">0</div></div>
      <div id="pipe-crystallizer-step" class="pipe-step block"><div class="pipe-name">CRYSTALLIZER</div><div id="pipe-crystallizer" class="pipe-state">ВХОД 0 · ДОПУЩЕНО 0 · HELD 0</div></div>
      <div id="pipe-package-step" class="pipe-step block"><div class="pipe-name">OPERATOR PACKAGES</div><div id="pipe-package" class="pipe-state">DELTA 0 · ACTIVE 0</div></div>
      <div id="pipe-admission-step" class="pipe-step __ADMISSION_STEP_CLASS__"><div class="pipe-name">ADMISSION</div><div id="pipe-admission" class="pipe-state">__ADMISSION_STATE__</div></div>
      <div id="pipe-cpu-step" class="pipe-step __CPU_STEP_CLASS__"><div class="pipe-name">CPU ACCEPT</div><div id="pipe-cpu" class="pipe-state">__CPU_STATE__</div></div>
    </div></div>
    <div id="pipeline-note" class="blocker __BLOCKER_CLASS__"><span id="pipeline-note-label" class="blocker-label">ТЕКУЩИЙ РАЗРЫВ</span><span id="blocker-text" class="blocker-copy">__BLOCKER__</span></div>
    <div class="activity"><span class="activity-label">NANDO INGRESS REQUESTS · ПОСЛЕДНИЕ 60 С</span><div id="activity-bars" class="activity-bars"></div></div>
  </div></section>
  <section class="live-band"><div class="live-inner">
    <div class="window-head"><h2 class="band-title">ПРОЦЕССЫ CODEX НА ХОСТЕ CONTROL</h2><div id="window-summary" class="window-summary">SCOPE CONTROL HOST · NANDO — / MIXED — / DIRECT — / NO SOCKET —</div></div>
    <div class="window-scroll"><div class="window-table"><div class="window-row header"><span>ОКНО</span><span>СЕССИЯ</span><span>КОНФИГ</span><span>СТАТУС</span><span>КОНЕЧНАЯ ТОЧКА</span></div><div id="window-rows"></div></div></div>
  </div></section>
  <footer class="live-foot"><div class="live-inner"><span>СЛЕДУЮЩИЙ РУБЕЖ: <span id="next-route" class="next-route">relation evidence → circuit → future proof → admission</span></span><span>bridge current: false accepts <b id="false-accepts">0</b> · parity <b id="parity-mismatches">0</b> · failures <b id="bridge-failures">0</b> · miner completed history: false accepts <b id="historical-false-accepts">0</b> · parity <b id="historical-parity-mismatches">0</b></span></div></footer>
</main>
<script>
(() => {
  const base = window.location.pathname.replace(/\/$/, "");
  const dashboardBuild = document.querySelector(".nando-live")?.dataset.dashboardBuild || "";
  const number = new Intl.NumberFormat("ru-RU");
  const samples = [];
  let previousRequests = null;
  let lastSuccess = Date.now();
  let sourceGeneratedAt = 0;
  const node = (id) => document.getElementById(id);
  const text = (id, value) => { const target = node(id); if (target) target.textContent = value; };
  const stateClass = (id, value) => { const target = node(id); if (target) target.className = value; };
  const ratio = (part, total, digits) => total > 0 ? `${(part * 100 / total).toFixed(digits).replace(".", ",")}%` : `0,${"0".repeat(digits)}%`;
  const width = (id, part, total) => { const target = node(id); if (target) target.style.width = total > 0 ? `${Math.max(0.25, part * 100 / total)}%` : "0"; };
  const localTime = (unix) => unix > 0 ? new Date(unix * 1000).toLocaleString("ru-RU", {dateStyle:"short", timeStyle:"medium"}) : "—";
  const duration = (seconds) => { const value = Math.max(0, seconds); const hours = Math.floor(value / 3600); const minutes = Math.floor((value % 3600) / 60); return hours > 0 ? `${hours} ч ${minutes} мин` : minutes > 0 ? `${minutes} мин` : `${Math.floor(value)} с`; };
  const routeLabel = (window) => window.route === "nando" ? "NANDO" : window.route === "mixed" ? "СМЕШАННО" : window.route === "outside_nando" ? "ВНЕ NANDO" : "ОЖИДАНИЕ";
  const routeEndpoints = (window) => {
    if (!window.configured_for_nando || window.route !== "nando") return window.remote_endpoints.join(", ") || "—";
    const api = window.remote_endpoints.filter(value => value === "127.0.0.1:8787" || value === "[::1]:8787");
    const authOpen = window.remote_endpoints.some(value => value.endsWith(":443"));
    return `${api.join(", ") || "nando_nginx contract"}${authOpen ? " · HTTPS auth (не API)" : ""}`;
  };
  const renderWindows = (snapshot) => {
    text("window-summary", `SCOPE CONTROL HOST · NANDO ${snapshot.active_nando} / MIXED ${snapshot.active_mixed} / DIRECT ${snapshot.active_outside_nando} / NO SOCKET ${snapshot.idle}`);
    const rows = node("window-rows");
    if (!rows) return;
    rows.replaceChildren();
    for (const window of snapshot.windows) {
      const row = document.createElement("div"); row.className = "window-row";
      const values = [window.project.toUpperCase(), window.session.startsWith("pid-") ? window.session : window.session.slice(0, 8), window.configured_for_nando ? "nando_nginx" : "default", routeLabel(window), routeEndpoints(window)];
      values.forEach((value, index) => { const cell = document.createElement("span"); cell.textContent = value; if (index === 3) cell.className = `window-status ${window.route}`; row.appendChild(cell); });
      rows.appendChild(row);
    }
    if (snapshot.total_windows === 0) {
      const row = document.createElement("div"); row.className = "window-row";
      const values = ["CONTROL HOST", "—", "—", "НЕ НАБЛЮДАЕТСЯ", "Окна клиентов на других хостах не входят в этот process observer"];
      values.forEach((value, index) => { const cell = document.createElement("span"); cell.textContent = value; if (index === 3) cell.className = "window-status idle"; row.appendChild(cell); });
      rows.appendChild(row);
    }
    if (snapshot.active_nando === 0 && snapshot.active_mixed === 0 && snapshot.active_outside_nando > 0) {
      text("pipeline-title", "ОКНА ИДУТ МИМО NANDO");
      text("blocker-text", `${snapshot.active_outside_nando} активных окон подключены напрямую к upstream; CPU-маршрут готов, но не получает их запросы`);
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
    const accounting = snapshot.accounting || {};
    const overview = snapshot.overview || {};
    const server = overview.server || {};
    const serverPrior = server.prior_epoch || {};
    const legacy = snapshot.legacy_v3 || {};
    const execution = overview.execution || {};
    const executionTotal = execution.total || {};
    const executionCpu = execution.cpu || {};
    const epochTotal = executionTotal.input_tokens ?? accounting.input_tokens ?? snapshot.current_epoch_total_input_tokens ?? 0;
    const epochEvents = executionTotal.requests ?? accounting.terminal_request_events ?? 0;
    const epochCpu = executionCpu.input_tokens ?? accounting.cpu_verified_input_tokens ?? snapshot.current_epoch_cpu_input_tokens ?? 0;
    const epochAccepts = executionCpu.verified_accepts ?? accounting.actual_local_accepts ?? 0;
    const serverTotal = server.input_tokens ?? snapshot.server_recorded_total_input_tokens ?? ((legacy.input_tokens || 0) + epochTotal);
    const serverCpu = server.cpu_verified_input_tokens ?? snapshot.server_recorded_cpu_input_tokens ?? ((legacy.cpu_tokens || 0) + epochCpu);
    const priorTotal = serverPrior.input_tokens ?? legacy.input_tokens ?? Math.max(0, serverTotal - epochTotal);
    const priorCpu = serverPrior.cpu_verified_input_tokens ?? legacy.cpu_tokens ?? Math.max(0, serverCpu - epochCpu);
    sourceGeneratedAt = accounting.generated_at_unix || 0;
    text("source-age", sourceGeneratedAt > 0 ? Math.max(0, Math.floor(Date.now() / 1000) - sourceGeneratedAt) : "—");
    text("server-total-token-count", number.format(serverTotal));
    text("server-total-breakdown", `V3 ${number.format(priorTotal)} + V4 ${number.format(epochTotal)}`);
    text("server-total-scope", `ВСЯ СОХРАНЁННАЯ ИСТОРИЯ УЧЁТА · ТЕКУЩАЯ V4: ${number.format(epochEvents)} ЗАПРОСОВ`);
    text("server-cpu-token-count", number.format(serverCpu));
    text("server-cpu-share", ratio(serverCpu, serverTotal, 1));
    text("server-cpu-breakdown", `V3 ${number.format(priorCpu)} + V4 ${number.format(epochCpu)}`);
    const completedWindows = Array.isArray(accounting.completed_m3_windows) ? accounting.completed_m3_windows : [];
    let m3Streak = 0;
    for (let index = completedWindows.length - 1; index >= 0 && completedWindows[index]?.pass === true; index -= 1) m3Streak += 1;
    const productM3 = accounting.product_m3_pass === true ? "PASS" : "WATCH";
    const currentM3 = accounting.m3_current_window_pass === true ? "PASS" : "WATCH";
    text("current-v4-execution", `ТЕКУЩАЯ V4: ${number.format(epochCpu)} / ${number.format(epochTotal)} · ${ratio(epochCpu, epochTotal, 1)} · ${number.format(epochAccepts)} ACCEPTS · PRODUCT M3 ${productM3} · PASS STREAK ${m3Streak}/${accounting.m3_required_consecutive_windows || 3} · CURRENT WINDOW ${currentM3}`);

    const compression = snapshot.cpu_compression || {};
    const lifetimeCompression = compression.lifetime || {};
    const epochCompression = compression.current_epoch || {};
    const ms4Compression = compression.natural_ms4_package || {};
    const lifetimeEligible = lifetimeCompression.eligible_input_tokens ?? serverTotal;
    const lifetimeAvoided = lifetimeCompression.avoided_input_tokens ?? serverCpu;
    const epochEligible = epochCompression.eligible_input_tokens ?? epochTotal;
    const epochAvoided = epochCompression.avoided_input_tokens ?? epochCpu;
    const ms4Eligible = ms4Compression.eligible_input_tokens || 0;
    const ms4Avoided = ms4Compression.avoided_input_tokens || 0;
    const ms4Accepts = ms4Compression.cpu_accepts || 0;
    const ms4Receipt = ms4Compression.receipt_root_sha256 || "";
    const ms4LifecycleReceipt = ms4Compression.lifecycle_receipt_root_sha256 || "";
    const ms4CompletionReceipt = ms4Compression.completion_root_sha256 || "";
    const ms4ReceiptMatched = Boolean(ms4Receipt) && ms4Receipt === ms4LifecycleReceipt;
    const ms4CompressionComplete = ms4Compression.stage === "complete" && ms4Accepts > 0 && ms4ReceiptMatched && Boolean(ms4CompletionReceipt);
    text("compression-lifetime-tokens", `${number.format(lifetimeAvoided)} / ${number.format(lifetimeEligible)}`);
    text("compression-lifetime-ratio", ratio(lifetimeAvoided, lifetimeEligible, 2));
    width("compression-lifetime-bar", lifetimeAvoided, lifetimeEligible);
    text("compression-epoch-tokens", `${number.format(epochAvoided)} / ${number.format(epochEligible)}`);
    text("compression-epoch-ratio", ratio(epochAvoided, epochEligible, 2));
    width("compression-epoch-bar", epochAvoided, epochEligible);
    text("compression-epoch-calls", `${number.format(epochCompression.ordinary_requests_seen ?? epochEvents)} REQUESTS · ${number.format(epochCompression.cpu_accepts ?? epochAccepts)} CPU ACCEPTS · ${number.format(epochCompression.avoided_upstream_calls ?? epochAccepts)} UPSTREAM CALLS AVOIDED`);
    text("compression-epoch-since", `EPOCH С ${localTime(epochCompression.started_at_unix ?? accounting.epoch_started_at_unix ?? 0)}`);
    text("compression-ms4-tokens", `${number.format(ms4Avoided)} / ${number.format(ms4Eligible)}`);
    text("compression-ms4-ratio", ratio(ms4Avoided, ms4Eligible, 2));
    width("compression-ms4-bar", ms4Avoided, ms4Eligible);
    text("compression-ms4-calls", `${number.format(ms4Compression.ordinary_requests_seen || 0)} ORDINARY REQUEST · ${number.format(ms4Accepts)} CPU ACCEPT · ${number.format(ms4Compression.avoided_upstream_calls || 0)} UPSTREAM CALL AVOIDED`);
    const compactPackage = ms4Compression.package_id ? `${ms4Compression.package_id.slice(0, 28)}…${ms4Compression.package_id.slice(-12)}` : "—";
    text("compression-ms4-package", `PACKAGE ${compactPackage}`);
    text("compression-ms4-status", ms4CompressionComplete && ms4CompletionReceipt ? "MS4 COMPLETE · IMMUTABLE CPU ROOT" : "MS4 OPERATIONAL PROOF PENDING");
    text("compression-ms4-time", `LAST ACCEPT ${localTime(ms4Compression.last_accept_timestamp_unix || 0)} · FIRST RECEIPT LATCHED · FALSE ACCEPTS ${number.format(ms4Compression.false_accepts || 0)} · PARITY ${number.format(ms4Compression.runtime_parity_mismatches || 0)}`);
    text("compression-ms4-root", ms4Receipt || "—");
    stateClass("compression-ms4-status", ms4CompressionComplete ? "good" : "warning");

    const miner = snapshot.miner_window || {};
    const minerOverview = overview.miner || {};
    const minerSeen = minerOverview.seen || {};
    const minerRecognized = minerOverview.recognized || {};
    const minerUnresolvedOverview = minerOverview.unresolved || {};
    const minerTotal = minerSeen.input_tokens ?? miner.ordinary_tokens ?? snapshot.verified_window_total_input_tokens ?? 0;
    const minerIntents = minerSeen.intents ?? miner.ordinary_intents ?? 0;
    const minerCpu = minerRecognized.input_tokens ?? miner.cpu_verified_tokens ?? snapshot.verified_window_cpu_input_tokens ?? 0;
    const minerCpuIntents = minerRecognized.intents ?? miner.cpu_verified_intents ?? 0;
    const minerUnresolved = minerUnresolvedOverview.input_tokens ?? miner.unresolved_tokens ?? 0;
    const minerUnrecognized = Math.max(0, minerTotal - minerCpu);
    const minerLlmOnly = Math.max(0, minerUnrecognized - minerUnresolved);
    const optimisticTokens = miner.optimistic_upper_bound_tokens ?? snapshot.optimistic_upper_bound_tokens ?? 0;
    text("miner-window-total", number.format(minerTotal));
    text("miner-window-intents", `${number.format(minerIntents)} intents`);
    text("miner-window-start", `MINER WINDOW С ${localTime(minerOverview.started_at_unix ?? miner.started_at_unix ?? 0)} · ОТДЕЛЬНЫЙ WATERMARK · НЕ ДЕЛИТЬ НА V4`);
    text("miner-window-cpu-token-count", number.format(minerCpu));
    text("miner-window-cpu-intents", `${number.format(minerCpuIntents)} verified intents`);
    text("miner-window-cpu-share", ratio(minerCpu, minerTotal, 1));
    width("miner-recognized-bar", minerCpu, minerTotal);
    text("miner-unresolved-values", number.format(minerUnrecognized));
    text("miner-unresolved-share", ratio(minerUnrecognized, minerTotal, 1));
    text("miner-unresolved-note", `из них исследуемо ${number.format(minerUnresolved)} · доказанно LLM-only ${number.format(minerLlmOnly)}`);
    text("scope-ceiling-values", `${number.format(optimisticTokens)} / ${number.format(minerTotal)}`);
    text("scope-ceiling-share", ratio(optimisticTokens, minerTotal, 1));
    const classRows = Object.entries(miner.classes || {})
      .filter(([name]) => name !== "CPU_VERIFIED")
      .sort((left, right) => (right[1]?.input_tokens || 0) - (left[1]?.input_tokens || 0))
      .map(([name, row]) => `${name} ${number.format(row?.input_tokens || 0)}`);
    text("miner-class-ledger", classRows.length > 0 ? `КЛАССЫ БЕЗ CPU: ${classRows.join(" · ")}` : "КЛАССЫ БЕЗ CPU: НЕТ ДАННЫХ");

    text("legacy-values", `${number.format(legacy.input_tokens || 0)} вход / ${number.format(legacy.cpu_tokens || 0)} CPU`);

    const ms3 = snapshot.ms3 || {};
    const ms4 = snapshot.ms4_closed_loop || {};
    const lifecycleStatus = snapshot.ms3_lifecycle || {};
    const lifecycle = lifecycleStatus.lifecycle || {};
    const acquisition = snapshot.ms3_acquisition || {};
    const captureHealth = snapshot.ms3_capture_health || {};
    const receiptHealth = captureHealth.receipt || {};
    const acquisitionContract = acquisition.acquisition_contract || {};
    const generations = lifecycleStatus.registry?.generations || [];
    const linkedAcquisitionFailures = lifecycleStatus.registry?.linked_acquisition_failures || [];
    const activeGeneration = lifecycleStatus.active_generation_sequence || lifecycle.active_generation_sequence || 0;
    const predecessor = generations.find(row => row.generation_sequence + 1 === activeGeneration)
      || linkedAcquisitionFailures
        .filter(row => row.generation_sequence + 1 === activeGeneration)
        .map(row => ({generation_sequence: row.generation_sequence, linked_acquisition_failure: row}))[0];
    const predecessorBlocker = predecessor?.terminal?.blocker || predecessor?.acquisition_failure?.blocker || predecessor?.linked_acquisition_failure?.blocker || "";
    const predecessorVerdict = predecessor?.terminal?.verdict || (predecessor?.acquisition_failure ? "acquisition_fail" : "none");
    const linkedClosureVerdict = blocker => blocker === "MS3_LINKED_FRAME_ACQUISITION_FAIL"
      ? "linked_acquisition_fail"
      : blocker === "ms3_capture_gap_repair_required"
        ? "capture_gap_repair"
        : blocker === "MS3_LINKED_EVIDENCE_REUSE"
          ? "linked_evidence_reuse"
          : blocker === "CENSORED_PRE_ROUTE_RECEIPT_EPOCH"
            ? "pre-route epoch censored"
          : "linked_generation_close";
    const effectivePredecessorVerdict = predecessor?.linked_acquisition_failure
      ? linkedClosureVerdict(predecessorBlocker)
      : predecessorVerdict;
    const predecessorPreRouteRows = predecessor?.linked_acquisition_failure?.censored_pre_route_receipt_rows || 0;
    const acquisitionVerdict = acquisition.verdict || "unavailable";
    const eligibleTopologyRows = acquisition.eligible_topology_rows ?? acquisition.evaluated_topology_rows ?? 0;
    const candidateTopologyRows = acquisition.candidate_topology_rows ?? eligibleTopologyRows;
    const topologyLimit = acquisitionContract.max_new_topology_rows || 256;
    const rawTopologyRows = acquisition.raw_scanned_topology_rows ?? acquisition.new_topology_rows_seen ?? 0;
    const rawTopologyLimit = acquisitionContract.max_raw_topology_rows || 4096;
    const censoredTopologyRows = acquisition.censored_topology_rows || 0;
    const terminalRows = acquisition.terminal_receipt_rows || 0;
    const relevantRows = acquisition.relevant_verified_frame_rows || 0;
    const linkedRows = acquisition.linked_frame_rows || 0;
    const settlementCounts = acquisition.candidate_settlement_counts || {};
    const settledRows = settlementCounts.settled_eligible || 0;
    const terminalPendingRows = settlementCounts.terminal_pending || 0;
    const routeFramePendingRows = settlementCounts.route_frame_pending || 0;
    const receiptStalledRows = settlementCounts.receipt_stalled || 0;
    const structurallyIneligibleRows = settlementCounts.structurally_ineligible || 0;
    const routeSettlementPendingRows = acquisition.route_settlement_pending_rows
      ?? terminalPendingRows + routeFramePendingRows + receiptStalledRows;
    const bindingFailureCounts = acquisition.transport_binding_failure_counts || {};
    const bindingFailureRows = Object.values(bindingFailureCounts)
      .reduce((total, rows) => total + Number(rows || 0), 0);
    const bindingFailureSummary = Object.entries(bindingFailureCounts)
      .filter(([, rows]) => Number(rows || 0) > 0)
      .sort((left, right) => Number(right[1]) - Number(left[1]))
      .map(([name, rows]) => `${name.replaceAll("_", " ").toUpperCase()} ${number.format(rows)}`)
      .join(" · ");
    const evidenceReuseExcludedRows = acquisition.evidence_reuse_excluded_rows || 0;
    const futureVerdict = ms3.effective_verdict || ms3.verdict || "not_evaluated";
    const activeFreezeBlocker = lifecycleStatus.active_freeze_blocker || "";
    const futureBlocker = ms3.effective_blocker || ms3.blocker || activeFreezeBlocker || "";
    const acquisitionBlocker = acquisition.blocker || "";
    const lawFrozen = Boolean(lifecycleStatus.active_frozen_envelope_root_sha256);
    const futureFrozen = Boolean(lifecycleStatus.active_future_envelope_root_sha256);
    const authorityReady = lifecycleStatus.authority_ready === true || ms3.authority_ready === true || ms4.authority_ready === true;
    const phaseMutation = lifecycleStatus.phase_mutation_allowed === true || acquisition.phase_update_allowed === true || ms3.phase_mutation_allowed === true;
    const predictionsCommitted = ms3.predictions_committed || 0;
    const activePredictions = ms3.effective_active_predictions ?? ms3.active_predictions ?? 0;
    const futureTopologies = ms3.independent_topologies || 0;
    const futureTopologyLimit = ms3.contract?.max_independent_topologies || 256;
    const futureAcquisitionFailed = futureVerdict === "acquisition_fail";
    const activePhase = authorityReady
      ? "ADMITTED"
      : futureVerdict === "future_pass"
        ? "FUTURE_PASS"
        : futureVerdict === "contradiction"
          ? "CONTRADICTION"
          : futureAcquisitionFailed
            ? "ACQUISITION_FAIL"
          : activePredictions > 0 || futureVerdict === "applicable_prediction_pending"
            ? "FUTURE_PENDING"
            : lawFrozen
              ? "UNIQUE_LAW_FROZEN"
              : acquisitionVerdict.toUpperCase();
    text("ms3-generation", activeGeneration > 0 ? `G${activeGeneration} · ${activePhase}` : "НЕТ ДАННЫХ");
    text("ms3-predecessor", predecessor ? `G${predecessor.generation_sequence} · ${effectivePredecessorVerdict.toUpperCase()}${predecessorPreRouteRows > 0 ? ` · ${number.format(predecessorPreRouteRows)}` : ""}` : "НЕТ");
    text("ms3-acquisition", `${number.format(eligibleTopologyRows)} / ${number.format(topologyLimit)}`);
    text("ms3-acquisition-raw", `CANDIDATE ${number.format(candidateTopologyRows)} · RAW ${number.format(rawTopologyRows)} / ${number.format(rawTopologyLimit)} · CENSORED ${number.format(censoredTopologyRows)}`);
    text("ms3-evidence", `${number.format(terminalRows)} / ${number.format(relevantRows)} / ${number.format(linkedRows)}`);
    text("ms3-settlement", `${number.format(settledRows)} / ${number.format(routeSettlementPendingRows)}`);
    text("ms3-settlement-note", `SETTLED ${number.format(settledRows)} · TERMINAL PENDING ${number.format(terminalPendingRows)} · FRAME PENDING ${number.format(routeFramePendingRows)} · STALLED ${number.format(receiptStalledRows)} · STRUCTURAL ${number.format(structurallyIneligibleRows)}`);
    text("ms3-binding-failures", number.format(bindingFailureRows));
    text("ms3-binding-failure-note", `${bindingFailureSummary || "NONE"} · REUSE ${number.format(evidenceReuseExcludedRows)}`);
    text("ms3-law", lawFrozen ? "UNIQUE LAW FROZEN" : "LAW NOT FROZEN");
    text("ms3-future-applicability", lawFrozen ? `${number.format(futureTopologies)} / ${number.format(futureTopologyLimit)}` : "NOT OPEN");
    text("ms3-predictions", lawFrozen ? `${number.format(predictionsCommitted)} / ${number.format(activePredictions)}` : "NOT OPEN");
    text("ms3-future", futureVerdict === "future_pass" ? "PASS" : futureVerdict === "contradiction" ? "CONTRADICTION" : futureAcquisitionFailed ? "ACQUISITION FAIL" : activePredictions > 0 ? "OUTCOME PENDING" : futureFrozen ? futureVerdict.toUpperCase() : "НЕ ОЦЕНЕН");
    text("ms3-authority", authorityReady ? "TRUE" : "FALSE");
    const captureStatus = captureHealth.status || "NOT_EVALUATED";
    const receiptStatus = receiptHealth.status || "NOT_EVALUATED";
    const receiptInflight = receiptHealth.in_flight_rows || 0;
    const receiptStalled = receiptHealth.stalled_rows || 0;
    const receiptLag = receiptHealth.oldest_uncovered_lag_seconds || 0;
    const receiptSlo = receiptHealth.receipt_lag_slo_seconds || 0;
    text("ms3-operational-health", `${captureStatus} · ${receiptStatus}${receiptInflight > 0 ? ` ${number.format(receiptInflight)}` : ""}`);
    text("ms3-operational-note", `oldest ${duration(receiptLag)} · SLO ${duration(receiptSlo)} · stalled ${number.format(receiptStalled)}`);
    const ms4Stage = (ms4.stage || "waiting_for_ms3").toUpperCase();
    const ms4Package = ms4.package_id || "";
    const ms4Complete = ms4.stage === "complete" && Boolean(ms4.ordinary_cpu_receipt_root_sha256) && Boolean(ms4.ordinary_cpu_completion_root_sha256);
    const ms4ExactWaveStatus = (ms4.exact_wave_status || "collecting").toUpperCase();
    const ms4ExactWave = ms4.exact_wave_status === "pass" && Boolean(ms4.exact_package_wave_proof_root_sha256);
    text("ms4-stage", ms4Stage);
    text("ms4-package", ms4Package ? ms4Package.slice(0, 20) : "НЕ ЗАПЕЧАТАН");
    text("ms4-admission", ms4.external_admission_pass === true ? "PASS" : "FALSE");
    text("ms4-ordinary-proof", ms4Complete ? "PASS · OPERATIONAL" : "PENDING");
    text("ms4-exact-wave", ms4ExactWave ? "PASS · INDEPENDENT HOLDOUT" : ms4ExactWaveStatus);
    text("ms4-exact-wave-note", `POST-CENTER +${number.format(ms4.exact_wave_positive_holdout_rows || 0)} / -${number.format(ms4.exact_wave_phase_challenging_negative_rows || 0)} · LINEAGES ${number.format(ms4.exact_wave_independent_lineages || 0)} · PRECOMMITTED ${number.format(ms4.exact_wave_precommitted_rows || 0)} · SETTLED ${number.format(ms4.exact_wave_settled_rows || 0)} · LATE ${number.format(ms4.exact_wave_precommit_disqualified_rows || 0)}`);
    text("ms4-calibration-controls", `${number.format(ms4.negative_controls || 0)} / ${number.format(ms4.anti_center_atoms || 0)}`);
    text("ms4-calibration-note", `TOPOLOGY NEGATIVES / ANTI-CENTER ATOMS · IN-SAMPLE ${ms4.in_sample_phase_ablation_root_sha256 ? "PASS" : "PENDING"}`);
    stateClass("ms3-generation", `ms3-value ${authorityReady || futureVerdict === "future_pass" ? "good" : futureVerdict === "contradiction" || futureAcquisitionFailed ? "locked" : "watch"}`);
    stateClass("ms3-predecessor", `ms3-value ${effectivePredecessorVerdict === "contradiction" || effectivePredecessorVerdict.endsWith("acquisition_fail") ? "locked" : "watch"}`);
    stateClass("ms3-future-applicability", `ms3-value ${!lawFrozen ? "muted" : futureAcquisitionFailed ? "locked" : "watch"}`);
    stateClass("ms3-predictions", `ms3-value ${lawFrozen ? "watch" : "muted"}`);
    stateClass("ms3-law", `ms3-value ${futureVerdict === "future_pass" ? "good" : lawFrozen ? "watch" : "locked"}`);
    stateClass("ms3-future", `ms3-value ${futureVerdict === "future_pass" ? "good" : futureVerdict === "contradiction" || futureAcquisitionFailed ? "locked" : "watch"}`);
    stateClass("ms3-authority", `ms3-value ${authorityReady ? "good" : "locked"}`);
    stateClass("ms3-operational-health", `ms3-value ${captureStatus === "CAPTURE_STALLED" || receiptStatus === "RECEIPT_STALLED" || captureStatus === "NOT_EVALUATED" ? "locked" : receiptStatus === "IN_FLIGHT" ? "watch" : "good"}`);
    stateClass("ms3-settlement", `ms3-value ${receiptStalledRows > 0 ? "locked" : routeSettlementPendingRows > 0 ? "watch" : "good"}`);
    stateClass("ms3-binding-failures", `ms3-value ${bindingFailureRows > 0 ? "watch" : "good"}`);
    stateClass("ms4-stage", `ms3-value ${ms4Complete ? "good" : ms4.stage === "blocked" ? "locked" : "watch"}`);
    stateClass("ms4-package", `ms3-value ${ms4Package ? "good" : "muted"}`);
    stateClass("ms4-admission", `ms3-value ${ms4.external_admission_pass === true ? "good" : "locked"}`);
    stateClass("ms4-ordinary-proof", `ms3-value ${ms4Complete ? "good" : "locked"}`);
    stateClass("ms4-exact-wave", `ms3-value ${ms4ExactWave ? "good" : ms4.exact_wave_status === "fail" || ms4.exact_wave_status === "acquisition_fail" ? "locked" : "watch"}`);
    stateClass("ms4-calibration-controls", "ms3-value watch");
    const predecessorText = predecessor
      ? `G${predecessor.generation_sequence} ${effectivePredecessorVerdict.toUpperCase()}${predecessorBlocker ? ` (${predecessorBlocker})` : ""} → immutable close → `
      : "";
    const currentBlocker = lawFrozen ? futureBlocker : acquisitionBlocker || futureBlocker;
    text("ms3-note", `${predecessorText}G${activeGeneration || "?"} ${activePhase} · linked acquisition ${acquisitionVerdict.toUpperCase()} · future topology ${lawFrozen ? `${number.format(futureTopologies)} / ${number.format(futureTopologyLimit)}` : "NOT OPEN"} · MS4 ${ms4Stage} (${ms4.blocker || "none"}) · authority ${authorityReady ? "TRUE" : "FALSE"} · phase mutation ${phaseMutation ? "TRUE" : "FALSE"}`);
    text("next-route", ms4Complete && ms4ExactWave
      ? "operational loop complete · independent exact-package Wave proof complete"
      : ms4Complete
      ? "operational loop complete · post-center holdout → independent exact-package Wave proof"
      : authorityReady
      ? "ordinary CPU receipt → avoided upstream call"
      : futureVerdict === "future_pass"
        ? "BundleV4 → external admission → bounded lease"
        : futureAcquisitionFailed
          ? "new preregistered generation → fresh independent lineage → support acquisition"
        : activePredictions > 0
          ? "terminal outcome → independent verifier → FUTURE_PASS"
          : lawFrozen
            ? "applicable topology → durable prediction → terminal outcome"
            : "linked evidence → version space → unique law freeze");

    const bridge = snapshot.bridge; const bridgeAvailable = bridge.hot_available && bridge.cold_available; const queue = bridge.opportunity_pending; const structureComparable = bridgeAvailable && bridge.structural_epoch_match;
    const liveIngestion = minerOverview.live_ingestion || {};
    const received = liveIngestion.durably_received || {};
    const applied = liveIngestion.learner_applied || {};
    const backlog = liveIngestion.backlog || {};
    const counterDelta = liveIngestion.counter_delta || {};
    const joinOpen = Math.max(0, bridge.join_attempts - bridge.join_hits - bridge.join_misses);
    const minerCurrentComplete = structureComparable && bridge.structural_pending === 0 && bridge.structural_sequence_gaps === 0 && bridge.failures === 0 && bridge.opportunity_produced_sequence === bridge.opportunity_consumed_sequence && queue === 0;
    const opportunityLag = backlog.events ?? bridge.opportunity_pending;
    const tokenLagComparable = liveIngestion.counter_epoch_match ?? bridge.opportunity_counter_epoch_match === true;
    const counterEpochReason = liveIngestion.counter_epoch_reason ?? bridge.opportunity_counter_epoch_reason ?? "";
    const receivedTokens = received.input_tokens ?? bridge.request_tokens;
    const receivedRequests = received.requests ?? bridge.request_events;
    const receivedSequence = received.sequence ?? bridge.opportunity_produced_sequence;
    const appliedTokens = applied.input_tokens ?? bridge.miner_request_tokens;
    const appliedRequests = applied.requests ?? bridge.miner_request_events;
    const appliedSequence = applied.sequence ?? bridge.opportunity_consumed_sequence;
    const countersReconciled = liveIngestion.closed_prefix_reconciled === true;
    const tokenLag = tokenLagComparable ? counterDelta.input_tokens ?? Math.max(0, receivedTokens - appliedTokens) : 0;
    const requestCounterLag = tokenLagComparable ? counterDelta.requests ?? Math.max(0, receivedRequests - appliedRequests) : 0;
    text("ingestion-received", number.format(receivedTokens));
    text("ingestion-received-events", `${number.format(receivedRequests)} requests · seq ${number.format(receivedSequence)}`);
    text("ingestion-applied", tokenLagComparable ? number.format(appliedTokens) : "НЕСОПОСТАВИМО");
    text("ingestion-applied-events", tokenLagComparable ? `${number.format(appliedRequests)} requests · seq ${number.format(appliedSequence)}` : `consumer seq ${number.format(appliedSequence)} · отдельная эпоха`);
    text("ingestion-backlog", `${number.format(opportunityLag)} events`);
    text("ingestion-inflight", `${number.format(backlog.inflight_events ?? bridge.opportunity_inflight)} inflight входят в durable backlog`);
    stateClass("ingestion-backlog-cell", `ingestion-cell backlog ${!tokenLagComparable ? "invalid" : opportunityLag === 0 ? "clear" : ""}`);
    text("ingestion-token-lag", tokenLagComparable ? number.format(tokenLag) : "НЕСОПОСТАВИМО");
    text("ingestion-token-scope", tokenLagComparable
      ? tokenLag > 0
        ? `${number.format(requestCounterLag)} request ещё не отражён в cold consumer counter`
        : "producer и consumer counters совпадают"
      : counterEpochReason === "empty_spool_sequence_divergence"
        ? "durable backlog пуст, cold counter потерял предыдущую эпоху при restart"
        : "hot/cold counters принадлежат разным эпохам");
    text("ingestion-epoch", tokenLagComparable
      ? countersReconciled
        ? `COMMON COUNTER EPOCH · AFTER SEQ ${number.format(bridge.producer_counter_started_after_sequence)}`
        : `COMMON COUNTER EPOCH · COUNTERS RECONCILING ${number.format(requestCounterLag)} REQUEST`
      : counterEpochReason === "empty_spool_sequence_divergence"
        ? `COUNTER EPOCH SPLIT · EMPTY-SPOOL RESTART · PRODUCER SEQ ${number.format(receivedSequence)} / CONSUMER SEQ ${number.format(appliedSequence)}`
        : `COUNTER EPOCH SPLIT · HOT AFTER ${number.format(bridge.producer_counter_started_after_sequence)} / COLD AFTER ${number.format(bridge.consumer_counter_started_after_sequence)}`);
    stateClass("ingestion-epoch", `overview-rule ${tokenLagComparable ? countersReconciled ? "good" : "" : "warning"}`);
    text("bridge-pair", `${bridge.hot_available ? bridge.opportunity_produced_sequence : "—"} / ${bridge.cold_available ? bridge.opportunity_consumed_sequence : "—"}`); text("bridge-tokens", number.format(bridge.request_tokens)); text("bridge-queue", queue); text("epoch-visibility", structureComparable ? `JOIN ${bridge.join_hits}/${bridge.join_attempts} · MISS ${bridge.join_misses} · OPEN ${joinOpen}` : "STRUCTURE: НЕТ ОБЩЕГО EPOCH");
    text("services-count", `${bridge.services_active}/3`); text("false-accepts", bridge.false_accepts); text("parity-mismatches", bridge.parity_mismatches); text("bridge-failures", bridge.failures); text("historical-false-accepts", miner.historical_completed_false_accepts || 0); text("historical-parity-mismatches", miner.historical_completed_parity_failures || 0);
    const controllerInput = snapshot.controller_relation_candidates + snapshot.controller_collection_candidates;
    const crystallizedInput = snapshot.controller_crystallized_candidates || 0; const crystallizedAdmissible = snapshot.controller_crystallized_admissible_candidates || 0; const crystallizedHeld = snapshot.controller_crystallized_held_candidates || 0; const semanticGuardHeld = snapshot.controller_crystallized_held_semantic_guard_candidates || 0; const generationDelta = snapshot.controller_generation_delta_packages || 0;
    text("pipe-bridge", structureComparable ? `STRUCT ${bridge.structural_produced_sequence}/${bridge.structural_consumed_sequence} · PENDING ${bridge.structural_pending}` : "EPOCH/HEALTH BLOCK"); text("pipe-relation", structureComparable && bridge.structural_pending === 0 && bridge.structural_sequence_gaps === 0 && bridge.failures === 0 ? `JOIN ${bridge.join_hits}/${bridge.join_attempts} · OPEN ${joinOpen} · RAW ${bridge.raw_evaluated}/${bridge.raw_verified}/${bridge.raw_abstains}` : "WATCH"); text("pipe-discovery", snapshot.admission_ready_cohorts > 0 ? `COHORTS ${snapshot.admission_ready_cohorts}` : "WATCH"); text("pipe-candidate", controllerInput); text("pipe-crystallizer", `ВХОД ${crystallizedInput} · ДОПУЩЕНО ${crystallizedAdmissible} · HELD ${crystallizedHeld}`);
    text("pipe-package", `DELTA ${generationDelta} · ACTIVE ${snapshot.response_package_count}`); text("pipe-admission", snapshot.cpu_allowed ? "OPEN" : "LOCKED"); text("pipe-cpu", snapshot.cpu_allowed ? "ENABLED" : "0 NEW"); text("pipeline-title", snapshot.cpu_allowed ? "МАРШРУТ ДО CPU" : "ПОЧЕМУ CPU НЕ РАСТЁТ"); text("pipeline-note-label", snapshot.cpu_allowed ? "РАЗВИТИЕ ПОКРЫТИЯ" : "ТЕКУЩИЙ РАЗРЫВ");
    stateClass("pipeline-note", `blocker ${snapshot.cpu_allowed ? "coverage" : "critical"}`); stateClass("pipe-discovery-step", `pipe-step ${snapshot.admission_ready_cohorts > 0 ? "good" : "watch"}`); stateClass("pipe-candidate-step", `pipe-step ${controllerInput > 0 ? "good" : "watch"}`); stateClass("pipe-crystallizer-step", `pipe-step ${crystallizedAdmissible > 0 ? "good" : crystallizedHeld > 0 ? "watch" : "block"}`); stateClass("pipe-package-step", `pipe-step ${snapshot.response_package_count > 0 ? "good" : "block"}`); stateClass("pipe-admission-step", `pipe-step ${snapshot.cpu_allowed ? "good" : "locked"}`); stateClass("pipe-cpu-step", `pipe-step ${snapshot.cpu_allowed ? "good" : "muted"}`);
    text("blocker-text", semanticGuardHeld > 0 ? `CPU-маршрут открыт · ACTIVE packages ${snapshot.response_package_count}; HELD candidates ${semanticGuardHeld}: semantic_applicability_guard_missing; generation delta ${generationDelta}` : controllerInput > 0 && crystallizedInput === 0 ? `ТЕКУЩИЙ РАЗРЫВ: INPUT ${controllerInput} → CRYST 0. Legacy candidate: ${snapshot.controller_blocker}` : controllerInput === 0 ? `discovery → candidate export: ${snapshot.controller_blocker}` : crystallizedInput > 0 && !snapshot.cpu_allowed ? `crystallized operator готов, admission закрыт: ${snapshot.controller_blocker}` : snapshot.cpu_allowed ? "маршрут до CPU открыт" : snapshot.controller_blocker);
    renderActivity(bridge.request_events); lastSuccess = Date.now();
  };
  const refresh = async () => {
    try { const [tokensResponse, connectionsResponse] = await Promise.all([fetch(`${base}/tokens`, {cache:"no-store"}), fetch(`${base}/connections`, {cache:"no-store"})]); if (!tokensResponse.ok || !connectionsResponse.ok) return; renderTokens(await tokensResponse.json()); renderWindows(await connectionsResponse.json()); } catch (_) {}
  };
  const refreshDocumentVersion = async () => {
    try {
      const response = await fetch(`${base}?dashboard-build=${encodeURIComponent(dashboardBuild)}`, {cache:"no-store"});
      if (!response.ok) return;
      const html = await response.text();
      const current = html.match(/data-dashboard-build="([^"]+)"/)?.[1];
      if (current && current !== dashboardBuild) window.location.reload();
    } catch (_) {}
  };
  window.setInterval(() => {
    text("live-age", Math.floor((Date.now() - lastSuccess) / 1000));
    text("source-age", sourceGeneratedAt > 0 ? Math.max(0, Math.floor(Date.now() / 1000) - sourceGeneratedAt) : "—");
  }, 1000);
  refresh(); window.setInterval(refresh, 2000); window.setInterval(refreshDocumentVersion, 15000);
})();
</script>
"#;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn bridge_view_keeps_hot_and_cold_owners_separate() {
        let hot = json!({"ok":true,"process":{"instance_id_sha256":"hot","started_at_unix_ms":1_000},"structural":{"producer_failures":0},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"producer":{"last_sequence":45}},"opportunity":{"producer_last_sequence":45,"producer_counter_started_after_sequence":21,"pending_events":2,"producer_request_events":20,"producer_request_input_tokens":6_876_562,"failures":0}});
        let cold = json!({"ok":true,"process":{"instance_id_sha256":"cold","started_at_unix_ms":3_000},"structural":{"consumer_failures":0},"durable_structure":{"bridge_epoch_sha256":"epoch","pending_records":2,"sequence_gaps":0,"consumer":{"last_sequence":43}},"request_learning":{"structures_applied":43,"lookup_attempts":20,"lookup_hits":17,"lookup_misses":3},"opportunity":{"consumer_last_sequence":44,"consumer_counter_started_after_sequence":21,"consumer_inflight_events":1,"failures":0},"raw_replay":{"evaluated":12,"verified":3,"runtime_abstains":9,"execution_authority":false,"false_accepts":0,"parity_mismatches":0}});
        assert_eq!(
            bridge_view(&hot, &cold),
            BridgeView {
                hot_available: true,
                cold_available: true,
                hot_accepted: 45,
                cold_accepted: 43,
                loss: 2,
                queue: 4,
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
                opportunity_counter_epoch_match: true,
                opportunity_counter_epoch_reason: "common_counter_epoch".to_owned(),
                producer_counter_started_after_sequence: 21,
                consumer_counter_started_after_sequence: 21,
                hot_started_at_unix_ms: 1_000,
                cold_started_at_unix_ms: 3_000,
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
    fn opportunity_backlog_uses_consumer_owned_counter() {
        let hot = json!({
            "ok": true,
            "opportunity": {
                "pending_events": 2,
                "producer_last_sequence": 44
            }
        });
        let cold = json!({
            "ok": true,
            "opportunity": {
                "pending_events": 0,
                "consumer_last_sequence": 44
            }
        });
        let view = bridge_view(&hot, &cold);
        assert_eq!(view.opportunity_pending, 0);
    }

    #[test]
    fn empty_spool_sequence_divergence_invalidates_counter_epoch() {
        let hot = json!({
            "ok": true,
            "opportunity": {
                "producer_last_sequence": 8_294,
                "producer_counter_started_after_sequence": 0,
                "producer_request_events": 4_147,
                "producer_request_input_tokens": 775_498_232,
                "pending_events": 0
            }
        });
        let cold = json!({
            "ok": true,
            "opportunity": {
                "consumer_last_sequence": 0,
                "consumer_counter_started_after_sequence": 0,
                "consumer_request_events": 0,
                "consumer_request_input_tokens": 0,
                "consumer_inflight_events": 0,
                "pending_events": 0
            }
        });

        let view = bridge_view(&hot, &cold);

        assert!(!view.opportunity_counter_epoch_match);
        assert_eq!(
            view.opportunity_counter_epoch_reason,
            "empty_spool_sequence_divergence"
        );
        assert_eq!(view.opportunity_pending, 0);
        assert_eq!(view.opportunity_inflight, 0);
    }

    #[test]
    fn render_leads_with_total_server_miner_recognition_and_cpu() {
        let html = render(InitialMetrics {
            server_total_tokens: 5_948_645_890,
            server_cpu_tokens: 90_515_297,
            epoch_total_tokens: 200_000_000,
            epoch_total_events: 2_400,
            epoch_cpu_tokens: 48_000_000,
            epoch_cpu_accepts: 720,
            miner_window_total_tokens: 135_000_000,
            miner_window_total_intents: 1_300,
            miner_window_cpu_tokens: 98_000_000,
            miner_window_cpu_intents: 940,
            miner_window_unresolved_tokens: 10_000_000,
            optimistic_upper_bound_tokens: 121_000_000,
            legacy_total_tokens: 5_748_645_890,
            legacy_cpu_tokens: 42_515_297,
            cpu_allowed: false,
        });
        assert!(html.contains("ЧЕТЫРЕ ГЛАВНЫЕ ЦИФРЫ"));
        assert!(html.contains("ВСЕ ЗАПИСАННЫЕ ACCOUNTING PARTITIONS"));
        assert!(html.contains("УЧТЁННЫХ ВХОДНЫХ ТОКЕНОВ · REQUEST_EVENT.V1"));
        assert!(html.contains("РЕАЛЬНО ВОСПРОИЗВЕДЕНО НА CPU"));
        assert!(html.contains("id=\"server-cpu-share\" class=\"stage-share\">1,5%"));
        assert!(html.contains("ДОЛЯ ОТ RECORDED SERVER ACCOUNTING"));
        assert!(html.contains("PRODUCT M3 ${productM3}"));
        assert!(html.contains("PASS STREAK ${m3Streak}"));
        assert!(html.contains("CURRENT WINDOW ${currentM3}"));
        assert!(html.contains("ОПУБЛИКОВАННЫЙ КОРПУС КЛАССИФИКАЦИИ"));
        assert!(html.contains("МАЙНЕР РАСПОЗНАЛ"));
        assert!(html.contains("ЖИВОЙ ВХОД МАЙНЕРА"));
        assert!(html.contains("NANDO INGRESS REQUESTS · ПОСЛЕДНИЕ 60 С"));
        assert!(html.contains("miner completed history: false accepts"));
        assert!(html.contains("ПОЛУЧЕНО DURABLE"));
        assert!(html.contains("ПРИМЕНЕНО LEARNER"));
        assert!(html.contains("TOKEN COUNTER DELTA"));
        assert!(html.contains("COUNTERS RECONCILING"));
        assert!(html.contains("COMMON COUNTER EPOCH · AFTER SEQ"));
        assert!(html.contains("COUNTER EPOCH SPLIT · HOT AFTER"));
        assert!(html.contains("id=\"ingestion-backlog-cell\""));
        assert!(html.contains("opportunityLag === 0 ? \"clear\""));
        assert!(html.contains("const applied = liveIngestion.learner_applied || {}"));
        assert!(html.contains("CPU-маршрут открыт · ACTIVE packages"));
        assert!(!html.contains("CPU работает на ${snapshot.response_package_count}"));
        assert!(html.contains("ВСЕГО БЕЗ CPU-КЛАССА"));
        assert!(html.contains("АРХИВНАЯ PARTITION · УЖЕ ВКЛЮЧЕНА В SERVER TOTAL"));
        assert!(html.contains("37 000 000"));
        assert!(html.contains("из них исследуемо 10 000 000 · доказанно LLM-only 27 000 000"));
        assert!(
            html.contains("РАЗДЕЛЬНЫЕ SCOPE · SERVER HISTORY / MINER WINDOW / EXECUTION RECEIPTS")
        );
        assert!(html.contains("ЭТО НЕ ОДНА ПОСЛЕДОВАТЕЛЬНАЯ ВОРОНКА"));
        assert!(html.contains("СЖАТИЕ ВХОДНОГО ТРАФИКА НА CPU"));
        assert!(html.contains("AVOIDED INPUT / ELIGIBLE INPUT · EXACT O200K TOKENS"));
        assert!(html.contains("id=\"compression-lifetime-bar\""));
        assert!(html.contains("id=\"compression-epoch-calls\""));
        assert!(html.contains("id=\"compression-ms4-root\""));
        assert!(html.contains("snapshot.cpu_compression"));
        assert!(html.contains("MS4 COMPLETE · IMMUTABLE CPU ROOT"));
        assert!(html.contains("INDEPENDENT EXACT WAVE"));
        assert!(html.contains("АВТОНОМНЫЙ ЦИКЛ ЕСТЕСТВЕННОГО ОПЕРАТОРА · MS3 → MS4"));
        assert!(html.contains("ACTIVE GENERATION"));
        assert!(html.contains("PREDECESSOR"));
        assert!(html.contains("SCIENTIFIC / LINKED LIMIT"));
        assert!(html.contains("CANDIDATE"));
        assert!(html.contains("acquisition.eligible_topology_rows"));
        assert!(html.contains("acquisition.candidate_topology_rows"));
        assert!(html.contains("acquisitionContract.max_raw_topology_rows"));
        assert!(html.contains("ROUTE SETTLEMENT"));
        assert!(html.contains("TRANSPORT BINDING FAILURES"));
        assert!(html.contains("acquisition.candidate_settlement_counts"));
        assert!(html.contains("acquisition.route_settlement_pending_rows"));
        assert!(html.contains("acquisition.transport_binding_failure_counts"));
        assert!(html.contains("acquisition.evidence_reuse_excluded_rows"));
        assert!(html.contains("CAPTURE / RECEIPT HEALTH"));
        assert!(html.contains("AUTONOMOUS STAGE"));
        assert!(html.contains("ORDINARY CPU PROOF"));
        assert!(html.contains("snapshot.ms4_closed_loop"));
        assert!(html.contains("RECEIPT_STALLED"));
        assert!(html.contains("· OPEN ${joinOpen}"));
        assert!(html.contains("linked_acquisition_failures"));
        assert!(html.contains("linked_acquisition_fail"));
        assert!(html.contains("capture_gap_repair"));
        assert!(html.contains("linked_evidence_reuse"));
        assert!(html.contains("pre-route epoch censored"));
        assert!(html.contains("censored_pre_route_receipt_rows"));
        assert!(html.contains("active_freeze_blocker"));
        assert!(
            html.contains("const currentBlocker = lawFrozen ? futureBlocker : acquisitionBlocker")
        );
        assert!(html.contains("TERMINAL / RELEVANT / LINKED"));
        assert!(html.contains("FUTURE APPLICABILITY"));
        assert!(html.contains("DURABLE / ACTIVE PREDICTIONS"));
        assert!(html.contains("lawFrozen ? `${number.format(futureTopologies)}"));
        assert!(html.contains("future topology ${lawFrozen ?"));
        assert!(html.contains("INDEPENDENT FUTURE"));
        assert!(html.contains("ПОЧЕМУ CPU НЕ РАСТЁТ"));
        assert!(html.contains("CANDIDATE INPUT"));
        assert!(html.contains("CRYSTALLIZER"));
        assert!(html.contains("ВХОД 0 · ДОПУЩЕНО 0 · HELD 0"));
        assert!(html.contains("DELTA 0 · ACTIVE 0"));
        assert!(html.contains(&format!("data-dashboard-build=\"{DASHBOARD_BUILD}\"")));
        assert!(html.contains("NANDO INGRESS REQUESTS · ПОСЛЕДНИЕ 60 С"));
        assert!(html.contains("SCOPE CONTROL HOST · NANDO"));
        assert!(html.contains("HTTPS auth (не API)"));
        assert!(html.contains("5 948 645 890"));
        assert!(html.contains("V3 5 748 645 890 + V4 200 000 000"));
        assert!(html.contains("90 515 297"));
        assert!(html.contains("V3 42 515 297 + V4 48 000 000"));
        assert!(html.contains("200 000 000"));
        assert!(html.contains("2 400 ЗАПРОСОВ"));
        assert!(html.contains("720 accepts"));
        assert!(html.contains("1 300 intents"));
        assert!(html.contains("940 verified intents"));
        assert!(html.contains("24,0%"));
        assert!(html.contains("5 748 645 890 вход / 42 515 297 CPU"));
        assert!(html.contains("УЖЕ ВКЛЮЧЕНА В SERVER TOTAL"));
        assert!(
            html.find("ПОЧЕМУ CPU НЕ РАСТЁТ").unwrap_or(usize::MAX)
                < html
                    .find("ПРОЦЕССЫ CODEX НА ХОСТЕ CONTROL")
                    .unwrap_or(usize::MAX)
        );
        assert!(html.contains("72,6%"));
        assert!(html.contains("89,6%"));
        assert!(!html.contains("ВХОД NANDO · ЗА ВСЁ ВРЕМЯ"));
    }

    #[test]
    fn admitted_cpu_route_is_rendered_open_before_the_first_refresh() {
        let html = render(InitialMetrics {
            server_total_tokens: 10_000,
            server_cpu_tokens: 340,
            epoch_total_tokens: 1_000,
            epoch_total_events: 10,
            epoch_cpu_tokens: 240,
            epoch_cpu_accepts: 3,
            miner_window_total_tokens: 800,
            miner_window_total_intents: 8,
            miner_window_cpu_tokens: 600,
            miner_window_cpu_intents: 6,
            miner_window_unresolved_tokens: 80,
            optimistic_upper_bound_tokens: 720,
            legacy_total_tokens: 9_000,
            legacy_cpu_tokens: 100,
            cpu_allowed: true,
        });
        assert!(html.contains("МАРШРУТ ДО CPU"));
        assert!(html.contains("id=\"server-total-token-count\" class=\"stage-value\">10 000"));
        assert!(html.contains("id=\"server-cpu-token-count\" class=\"stage-value\">340"));
        assert!(html.contains("id=\"miner-window-cpu-share\" class=\"stage-share\">75,0%"));
        assert!(html.contains("V4 240 / 1 000 · 24,0% · 3 accepts"));
        assert!(html.contains("id=\"scope-ceiling-share\" class=\"scope-share\">90,0%"));
        assert!(html.contains("class=\"pipe-step good\"><div class=\"pipe-name\">ADMISSION"));
        assert!(html.contains("class=\"pipe-state\">OPEN"));
        assert!(html.contains("маршрут до CPU открыт"));
        assert!(!html.contains("__SERVER_TOTAL__"));
    }
}
