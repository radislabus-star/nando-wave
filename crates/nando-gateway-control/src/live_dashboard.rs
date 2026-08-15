use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const DASHBOARD_BUILD: &str = "2026.08.15-control-v24";
const HIDDEN_EFFECT_EVIDENCE: &str = include_str!(
    "../../../plans/effect-law-unification-v1/K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_EXECUTION_EVIDENCE_2026-08-14.md"
);
const COMPOSITION_RECEIPT: &str = include_str!(
    "../../../plans/effect-law-unification-v1/evidence/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_V1/capability-receipt.json"
);
const REPRESENTATION_RECEIPT: &str = include_str!(
    "../../../plans/effect-law-unification-v1/evidence/K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_V1/capability-receipt.json"
);
const HIDDEN_EFFECT_EVIDENCE_SHA256: &str =
    "aef3dd0025ecdf5ca6b5df0873da842321b03a9240eab2978d2ce8c4521eb9cb";
const COMPOSITION_RECEIPT_SHA256: &str =
    "95baf02f6a20a5b6bf884f8a47a0c00b5830ce0f775770273285e266ecb4ebb0";
const REPRESENTATION_RECEIPT_SHA256: &str =
    "c5c07cd2990d5f71f935977a932416c7daf6c6ff3b747d9e75243631ddf95a35";
const COMPOSITION_CLAIM: &str = "K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS";
const REPRESENTATION_CLAIM: &str = "K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_PASS";

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct GeneratedCapabilityView {
    hidden_effect_pass: bool,
    composition_pass: bool,
    representation_pass: bool,
    confirm_exact_goals: u64,
    confirm_total: u64,
    action_evaluations: u64,
    action_evaluation_limit: u64,
    complete_programs_each: u64,
    controls_passed: u64,
    controls_total: u64,
    production_authority_false: bool,
    natural_k2_not_proved: bool,
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
    let generated = generated_capability_view();
    let generated_state = |passed| if passed { "PASS" } else { "UNVERIFIED" };
    let generated_tone = |passed| if passed { "good" } else { "warn" };
    let generated_confirm_goals = if generated.representation_pass {
        format!(
            "{} / {}",
            generated.confirm_exact_goals, generated.confirm_total
        )
    } else {
        "UNVERIFIED".to_owned()
    };
    let generated_search_evaluations = if generated.representation_pass {
        format!(
            "{} / {} each",
            generated.action_evaluations, generated.action_evaluation_limit
        )
    } else {
        "UNVERIFIED".to_owned()
    };
    let generated_search_denominator = if generated.representation_pass {
        format_number(generated.complete_programs_each)
    } else {
        "UNVERIFIED".to_owned()
    };
    let generated_controls = if generated.representation_pass {
        format!(
            "{} / {}",
            generated.controls_passed, generated.controls_total
        )
    } else {
        "UNVERIFIED".to_owned()
    };
    let generated_authority = if generated.production_authority_false {
        "FALSE"
    } else {
        "UNVERIFIED"
    };
    let generated_natural_k2 = if generated.natural_k2_not_proved {
        "NOT PROVED"
    } else {
        "UNVERIFIED"
    };
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
        .replace(
            "__GENERATED_HIDDEN_EFFECT_STATE__",
            generated_state(generated.hidden_effect_pass),
        )
        .replace(
            "__GENERATED_HIDDEN_EFFECT_TONE__",
            generated_tone(generated.hidden_effect_pass),
        )
        .replace(
            "__GENERATED_COMPOSITION_STATE__",
            generated_state(generated.composition_pass),
        )
        .replace(
            "__GENERATED_COMPOSITION_TONE__",
            generated_tone(generated.composition_pass),
        )
        .replace(
            "__GENERATED_REPRESENTATION_STATE__",
            generated_state(generated.representation_pass),
        )
        .replace(
            "__GENERATED_REPRESENTATION_TONE__",
            generated_tone(generated.representation_pass),
        )
        .replace("__GENERATED_CONFIRM_GOALS__", &generated_confirm_goals)
        .replace(
            "__GENERATED_SEARCH_EVALUATIONS__",
            &generated_search_evaluations,
        )
        .replace(
            "__GENERATED_SEARCH_DENOMINATOR__",
            &generated_search_denominator,
        )
        .replace("__GENERATED_CONTROLS__", &generated_controls)
        .replace("__GENERATED_AUTHORITY__", generated_authority)
        .replace(
            "__GENERATED_AUTHORITY_TONE__",
            if generated.production_authority_false {
                "good"
            } else {
                "warn"
            },
        )
        .replace("__GENERATED_NATURAL_K2__", generated_natural_k2)
        .replace(
            "__GENERATED_RECEIPT_ROOT__",
            &REPRESENTATION_RECEIPT_SHA256[..12],
        )
}

fn generated_capability_view() -> GeneratedCapabilityView {
    generated_capability_view_from_sources(
        HIDDEN_EFFECT_EVIDENCE,
        COMPOSITION_RECEIPT,
        REPRESENTATION_RECEIPT,
    )
}

fn generated_capability_view_from_sources(
    hidden_effect_evidence: &str,
    composition_receipt: &str,
    representation_receipt: &str,
) -> GeneratedCapabilityView {
    let hidden_effect_pass = sha256_hex(hidden_effect_evidence.as_bytes())
        == HIDDEN_EFFECT_EVIDENCE_SHA256
        && hidden_effect_evidence.contains("K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS")
        && hidden_effect_evidence
            .contains("authority                                              false")
        && hidden_effect_evidence
            .contains("natural K2 claim                                      not made");
    let composition = validated_capability_receipt(
        composition_receipt,
        COMPOSITION_RECEIPT_SHA256,
        COMPOSITION_CLAIM,
    );
    let representation = validated_capability_receipt(
        representation_receipt,
        REPRESENTATION_RECEIPT_SHA256,
        REPRESENTATION_CLAIM,
    );
    let composition_pass = composition.is_some();
    let representation_pass = representation.is_some();
    let representation = representation.unwrap_or(Value::Null);
    let evaluations = representation
        .pointer("/denominators/policy_action_evaluations_each")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .and_then(|values| {
            let first = values.first()?.as_u64()?;
            values
                .iter()
                .all(|value| value.as_u64() == Some(first))
                .then_some(first)
        })
        .unwrap_or(0);
    let production_authority_false = hidden_effect_pass
        && composition_pass
        && representation_pass
        && composition
            .as_ref()
            .and_then(|value| value.pointer("/authority/production_execution"))
            .and_then(Value::as_bool)
            == Some(false)
        && representation
            .pointer("/authority/production_execution")
            .and_then(Value::as_bool)
            == Some(false);
    let natural_k2_not_proved = hidden_effect_pass
        && composition_pass
        && representation_pass
        && composition
            .as_ref()
            .and_then(|value| value.pointer("/authority/natural_k2"))
            .and_then(Value::as_bool)
            == Some(false)
        && representation
            .pointer("/authority/natural_k2")
            .and_then(Value::as_bool)
            == Some(false);

    GeneratedCapabilityView {
        hidden_effect_pass,
        composition_pass,
        representation_pass,
        confirm_exact_goals: pointer_u64(&representation, "/denominators/confirm_exact_goals"),
        confirm_total: pointer_u64(&representation, "/denominators/confirm_tasks"),
        action_evaluations: evaluations,
        action_evaluation_limit: pointer_u64(
            &representation,
            "/denominators/policy_action_evaluation_limit_each",
        ),
        complete_programs_each: pointer_u64(
            &representation,
            "/denominators/confirm_complete_programs_each",
        ),
        controls_passed: pointer_u64(&representation, "/denominators/negative_controls_passed"),
        controls_total: pointer_u64(&representation, "/denominators/negative_controls_total"),
        production_authority_false,
        natural_k2_not_proved,
    }
}

fn validated_capability_receipt(
    source: &str,
    expected_sha256: &str,
    expected_claim: &str,
) -> Option<Value> {
    if sha256_hex(source.as_bytes()) != expected_sha256 {
        return None;
    }
    let receipt: Value = serde_json::from_str(source).ok()?;
    let authority = receipt.pointer("/authority")?.as_object()?;
    (receipt.pointer("/claim")?.as_str()? == expected_claim
        && receipt.pointer("/verdict")?.as_str()? == "PASS"
        && !authority.is_empty()
        && authority
            .values()
            .all(|value| value.as_bool() == Some(false)))
    .then_some(receipt)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
        assert!(html.contains("id=\"exact-writer-state\""));
        assert!(html.contains("id=\"exact-writer-detail\""));
        assert!(html.contains("id=\"k1-transport-state\""));
        assert!(html.contains("exactWake.exact_unseen_opportunities"));
        assert!(html.contains("k1.law_2_status"));
        assert!(!html.contains("laws !== null && laws >= 2"));
        assert!(html.contains("id=\"k1-progress\""));
        assert!(html.contains("id=\"law2-state\""));
        assert!(html.contains("id=\"k1-blocker\""));
        assert!(html.contains("id=\"s1c4-state\""));
        assert!(html.contains("id=\"s1c4-verdict\""));
        assert!(html.contains("id=\"s1c4-window\""));
        assert!(html.contains("id=\"s1c4-goals\""));
        assert!(html.contains("id=\"k2-next\""));
        assert!(html.contains("Ожидание не изменит это закрытое окно"));
        assert!(html.contains("Generated-эксперименты ниже не входят в эту строку"));
        assert_eq!(html.matches("class=\"status-line\"").count(), 4);
        assert_eq!(html.matches("class=\"route-row").count(), 3);
        assert!(!html.contains("Распознавание майнера"));
        assert!(!html.contains("CPU economics · вся история"));
        assert!(!html.contains("Transition censors"));
        assert!(!html.contains("CANDIDATE INPUT"));
        assert!(html.contains("/api/v1/dashboard"));
        assert!(html.contains(&format!("data-dashboard-build=\"{DASHBOARD_BUILD}\"")));
    }

    #[test]
    fn generated_capability_is_receipt_bound_and_separate_from_natural_k2() {
        let generated = generated_capability_view();
        assert!(generated.hidden_effect_pass);
        assert!(generated.composition_pass);
        assert!(generated.representation_pass);
        assert_eq!(
            (generated.confirm_exact_goals, generated.confirm_total),
            (2, 2)
        );
        assert_eq!(
            (
                generated.action_evaluations,
                generated.action_evaluation_limit
            ),
            (61, 67)
        );
        assert_eq!(generated.complete_programs_each, 8_659);
        assert_eq!(
            (generated.controls_passed, generated.controls_total),
            (18, 18)
        );
        assert!(generated.production_authority_false);
        assert!(generated.natural_k2_not_proved);

        let html = render(InitialMetrics {
            server_total_tokens: 0,
            server_cpu_tokens: 0,
            epoch_total_tokens: 0,
            epoch_total_events: 0,
            epoch_cpu_tokens: 0,
            epoch_cpu_accepts: 0,
            epoch_avoided_calls: 0,
            miner_window_total_tokens: 0,
            miner_window_total_intents: 0,
            miner_window_cpu_tokens: 0,
            miner_window_cpu_intents: 0,
            cpu_allowed: false,
        });
        assert!(html.contains("Generated causal AI"));
        assert!(html.contains("Hidden effects learned"));
        assert!(html.contains("Explicit composition"));
        assert!(html.contains("Hidden representation"));
        assert!(html.contains("61 / 67 each"));
        assert!(html.contains("complete denominator 8 659 each"));
        assert!(html.contains("Production authority</span><strong class=\"good\">FALSE"));
        assert_eq!(html.matches("Natural K2").count(), 2);
        assert!(html.contains("NOT PROVED"));
        assert!(!html.contains("__GENERATED_"));
    }

    #[test]
    fn generated_capability_fails_closed_on_unbound_evidence() {
        let generated = generated_capability_view_from_sources(
            "tampered",
            COMPOSITION_RECEIPT,
            REPRESENTATION_RECEIPT,
        );
        assert!(!generated.hidden_effect_pass);
        assert!(generated.composition_pass);
        assert!(generated.representation_pass);
        assert!(!generated.production_authority_false);
        assert!(!generated.natural_k2_not_proved);
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
