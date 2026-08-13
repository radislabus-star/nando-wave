use serde::Serialize;
use serde_json::Value;

const DASHBOARD_BUILD: &str = "2026.08.13-control-v21";

#[derive(Clone, Copy, Debug)]
pub(crate) struct InitialMetrics {
    pub(crate) server_total_tokens: u64,
    pub(crate) server_cpu_tokens: u64,
    pub(crate) epoch_total_tokens: u64,
    pub(crate) epoch_total_events: u64,
    pub(crate) epoch_cpu_tokens: u64,
    pub(crate) epoch_cpu_accepts: u64,
    pub(crate) epoch_avoided_calls: u64,
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
            "__EPOCH_AVOIDED__",
            &format_number(initial.epoch_avoided_calls),
        )
        .replace(
            "__EPOCH_UPSTREAM__",
            &format_number(
                initial
                    .epoch_total_events
                    .saturating_sub(initial.epoch_avoided_calls),
            ),
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

const TEMPLATE: &str = include_str!("live_dashboard_v21.html");

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
    fn dashboard_leads_with_one_llm_denominator_and_its_s1c_subset() {
        let html = render(InitialMetrics {
            server_total_tokens: 7_694_807_361,
            server_cpu_tokens: 207_619_587,
            epoch_total_tokens: 1_733_026_637,
            epoch_total_events: 5_704,
            epoch_cpu_tokens: 165_104_290,
            epoch_cpu_accepts: 677,
            epoch_avoided_calls: 677,
            miner_window_total_tokens: 10_882_437_482,
            miner_window_total_intents: 49_122,
            miner_window_cpu_tokens: 1_613_584_240,
            miner_window_cpu_intents: 9_832,
            cpu_allowed: true,
        });
        assert!(html.contains("Маршрут запросов к модели"));
        assert!(html.contains("Служебный HTTP"));
        assert!(html.contains("НЕ ВХОДИТ"));
        assert!(html.contains("Реальные LLM-запросы"));
        assert!(html.contains("/v1|v2/responses · /v1|v2/chat/completions"));
        assert!(html.contains("S1C post-freeze выборка"));
        assert!(html.contains("конечное подмножество LLM ingress, не вся история"));
        assert!(html.contains("10 882 437 482"));
        assert!(html.contains("49 122"));
        assert!(html.contains("Результат текущей accounting epoch"));
        assert!(html.contains("1 733 026 637"));
        assert!(html.contains("165 104 290"));
        assert!(html.contains("9,53%"));
        assert!(html.contains("id=\"epoch-upstream\""));
        assert!(html.contains("upstream-вызовов предотвращено"));
        assert!(html.contains("С открытия страницы"));
        assert!(html.contains("Исследовательский статус"));
        assert!(html.contains("id=\"k1-progress\""));
        assert!(html.contains("id=\"law2-state\""));
        assert!(html.contains("id=\"k1-blocker\""));
        assert!(html.contains("id=\"s1c4-state\""));
        assert!(html.contains("id=\"s1c4-verdict\""));
        assert!(html.contains("id=\"s1c4-window\""));
        assert!(html.contains("id=\"s1c4-goals\""));
        assert!(html.contains("id=\"k2-next\""));
        assert!(html.contains("Ожидание не изменит это закрытое окно"));
        assert!(html.contains("отдельная preregistered среда"));
        assert_eq!(html.matches("class=\"status-line\"").count(), 3);
        assert_eq!(html.matches("class=\"route-row").count(), 3);
        assert!(!html.contains("Распознавание майнера"));
        assert!(!html.contains("CPU economics · вся история"));
        assert!(!html.contains("Transition censors"));
        assert!(!html.contains("CANDIDATE INPUT"));
        assert!(html.contains("/api/v1/dashboard"));
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
            epoch_avoided_calls: 0,
            miner_window_total_tokens: 40,
            miner_window_total_intents: 2,
            miner_window_cpu_tokens: 0,
            miner_window_cpu_intents: 0,
            cpu_allowed: false,
        });
        assert!(html.contains("id=\"cpu-gate\" class=\"bad\">ЗАКРЫТ"));
        assert!(!html.contains("__CPU_GATE__"));
    }
}
