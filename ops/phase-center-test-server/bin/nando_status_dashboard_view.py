"""Business-first Russian HTML renderer for Nando CPU server statistics."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
import html
import math
import urllib.parse
from typing import Any, Iterable
from zoneinfo import ZoneInfo


DASHBOARD_TIMEZONE = ZoneInfo("Europe/Tallinn")


@dataclass(frozen=True)
class Issue:
    severity: str
    title: str
    detail: str


@dataclass(frozen=True)
class DashboardModel:
    generated: str
    age_seconds: int | None
    numbers: dict[str, int]
    flags: dict[str, bool]
    texts: dict[str, str]
    issues: tuple[Issue, ...]
    class_rows: tuple[dict[str, Any], ...]
    profile_rows: tuple[dict[str, Any], ...]
    quarantined_rows: tuple[dict[str, Any], ...]
    history: tuple[dict[str, Any], ...]


def _number(value: Any) -> int:
    if isinstance(value, bool):
        return 0
    if isinstance(value, int):
        return value
    if isinstance(value, float) and math.isfinite(value):
        return int(value)
    return 0


def _first_number(sources: Iterable[dict[str, Any]], *keys: str) -> int:
    for source in sources:
        for key in keys:
            if key in source and source.get(key) is not None:
                return _number(source.get(key))
    return 0


def _object(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def _rows(value: Any) -> tuple[dict[str, Any], ...]:
    if not isinstance(value, list):
        return ()
    return tuple(row for row in value if isinstance(row, dict))


def _flag(value: Any) -> bool:
    return value is True or value == 1 or value == "1"


def _age_seconds(value: Any) -> int | None:
    if not isinstance(value, str) or not value:
        return None
    try:
        stamp = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if stamp.tzinfo is None:
        stamp = stamp.replace(tzinfo=timezone.utc)
    return max(0, int((datetime.now(timezone.utc) - stamp.astimezone(timezone.utc)).total_seconds()))


def _age_text(seconds: int | None) -> str:
    if seconds is None:
        return "время неизвестно"
    if seconds < 60:
        return f"{seconds} с назад"
    if seconds < 3600:
        return f"{seconds // 60} мин назад"
    return f"{seconds // 3600} ч назад"


def _format_number(value: int) -> str:
    return f"{value:,}".replace(",", " ")


def _format_bytes(value: int) -> str:
    if value < 1024:
        return f"{value} Б"
    if value < 1024 * 1024:
        return f"{value / 1024:.1f} КиБ"
    return f"{value / (1024 * 1024):.1f} МиБ"


def _format_duration_ms(value: int) -> str:
    if value < 1000:
        return f"{value} мс"
    return f"{value / 1000:.1f} с"


def _percentage(part: int, total: int) -> float:
    if total <= 0:
        return 0.0
    return max(0.0, part * 100.0 / total)


def _format_percentage(part: int, total: int) -> str:
    return f"{_percentage(part, total):.1f}%"


def _human_blocker(value: str) -> str:
    labels = {
        "none": "нет",
        "append_window_below_min_rows": "окно трафика меньше минимального",
        "market_money_claim_blocked": "нет подтверждённых данных о деньгах",
        "local_accept_promotion_blocked": "широкое CPU-выполнение не разрешено",
        "shadow_metrics_not_ready": "теневые метрики ещё не готовы",
        "external_provider_export_missing": "нет выгрузки реального биллинга провайдера",
        "stable_clean_token_false_accepts_nonzero": "в накопленном теневом окне есть ошибки",
        "append_false_accepts_nonzero": "в накопленном append-окне есть ошибки",
        "serving_local_accepts_zero": "нет фактических широких CPU-выполнений",
        "source_report_missing": "нет отчёта майнера",
        "append_live_tail_running": "майнер работает",
    }
    return labels.get(value, value.replace("_", " ") if value else "нет")


def _human_profile_kind(value: Any) -> str:
    text = str(value or "неизвестно")
    replacements = {
        "hidden_state": "межслойный признак",
        "observable_subcenter": "наблюдаемый подцентр",
        "observable_primary": "наблюдаемый основной",
        "final_hot": "горячий",
        "quarantined": "карантин",
        "exportable": "готов локально",
        "candidate": "кандидат",
        "mixed": "смешанный",
        "unknown": "неизвестно",
    }
    for source, target in replacements.items():
        text = text.replace(source, target)
    return text.replace("_", " ")


def build_dashboard_model(
    *,
    metrics: dict[str, Any],
    live_tail: dict[str, Any],
    status: dict[str, Any],
    appender: dict[str, Any],
    history: list[dict[str, Any]],
    transition_metrics: dict[str, Any] | None = None,
    transition_registry: dict[str, Any] | None = None,
    economics: dict[str, Any] | None = None,
    upstream_base_url_configured: bool = False,
) -> DashboardModel:
    verify = _object(status.get("verify"))
    summary = _object(status.get("summary"))
    bridge = _object(status.get("bridge"))
    upstream = _object(status.get("upstream"))
    readiness = _object(status.get("readiness"))
    provider_evidence = _object(status.get("provider_evidence"))
    latency = _object(status.get("latency"))
    resources = _object(status.get("resources"))
    runtime_budget = _object(live_tail.get("runtime_budget"))
    sources = (live_tail, metrics)
    transition_metrics = _object(transition_metrics)
    transition_registry = _object(transition_registry)
    economics = _object(economics)
    cpu_decidability = _object(economics.get("cpu_decidability"))
    cpu_decidability_classes = _object(cpu_decidability.get("classes"))
    transition_packages = _object(transition_registry.get("packages"))
    transition_profile_rows = tuple(
        profile
        for package in transition_packages.values()
        if isinstance(package, dict)
        for profile in _rows(package.get("profiles"))
    )

    generated = str(status.get("generated_utc") or datetime.now(timezone.utc).isoformat())
    age = _age_seconds(status.get("generated_utc"))

    n: dict[str, int] = {
        "capture_json_rows": _number(appender.get("json_rows_seen")),
        "captured_rows": _number(appender.get("rows_written")),
        "capture_skipped": _number(appender.get("skipped_unhandled_payload")),
        "capture_files": _number(appender.get("active_session_files")),
        "parsed_rows": _first_number(sources, "append_parsed_rows"),
        "incoming_tokens": _first_number(sources, "append_total_tokens"),
        "exact_cache_hits": _first_number(sources, "append_exact_cache_hits"),
        "non_exact_rows": _first_number(sources, "append_non_exact_rows"),
        "score_signals": _first_number(sources, "append_score_candidate_events"),
        "shadow_candidates": _first_number(
            sources, "product_hot_score_only_unique_cpu_accepts_over_exact_cache"
        ),
        "shadow_candidate_events": _first_number(
            sources, "product_hot_score_only_post_quarantine_score_candidate_events"
        ),
        "shadow_tokens": _first_number(sources, "product_hot_score_only_tokens_saved"),
        "shadow_false": _first_number(
            sources, "product_hot_score_only_post_quarantine_false_accepts"
        ),
        "quarantine_false": _first_number(
            sources, "product_hot_score_only_quarantine_false_accepts"
        ),
        "actual_rows": _first_number(
            (economics, metrics),
            "dedupe_eligible_client_intents",
            "stable_serving_cpu_clean_suffix_rows",
            "stable_serving_cpu_rows",
        ),
        "actual_tokens": _first_number(
            (economics, metrics),
            "global_input_tokens",
            "stable_serving_cpu_clean_suffix_total_tokens",
            "stable_serving_cpu_total_tokens",
        ),
        "actual_accepts": _first_number(
            (economics, metrics),
            "avoided_calls",
            "stable_serving_cpu_clean_suffix_local_accept_events",
            "stable_serving_cpu_local_accept_events",
        ),
        "actual_saved_tokens": _first_number(
            (economics, metrics),
            "avoided_input_tokens",
            "stable_serving_cpu_clean_suffix_tokens_saved",
            "stable_serving_cpu_tokens_saved",
        ),
        "actual_false": _first_number(
            (economics, metrics),
            "false_accepts",
            "stable_serving_cpu_clean_suffix_false_accepts",
            "stable_serving_cpu_false_accepts",
        ),
        "decidability_classified": _number(
            cpu_decidability.get("classified_fallback_requests")
        ),
        "decidability_miner_gap": _number(
            cpu_decidability_classes.get("potentially_cpu_executable")
        ),
        "decidability_not_executable": _number(
            cpu_decidability_classes.get("not_executable_current_evidence")
        ),
        "decidability_dsl_gap": _number(
            cpu_decidability_classes.get("unsupported_by_current_dsl")
        ),
        "canary_rows": _number(metrics.get("provider_bridge_decision_window_rows")),
        "canary_accepts": _number(metrics.get("provider_bridge_local_accept_events")),
        "canary_saved_tokens": _number(metrics.get("provider_bridge_tokens_saved_estimated")),
        "canary_false": _number(metrics.get("provider_bridge_false_accepts")),
        "typed_actor_accepts": max(
            _number(metrics.get("provider_bridge_typed_actor_local_accept_events")),
            _number(transition_metrics.get("local_accepts")),
        ),
        "typed_actor_saved_tokens": max(
            _number(metrics.get("provider_bridge_typed_actor_tokens_saved_estimated")),
            _number(transition_metrics.get("tokens_saved")),
        ),
        "typed_actor_false": max(
            _number(metrics.get("provider_bridge_typed_actor_false_accepts")),
            _number(transition_metrics.get("false_accepts")),
        ),
        "typed_transition_requests": _number(transition_metrics.get("total_bridge_requests")),
        "typed_transition_p99_ns": _number(transition_metrics.get("execution_p99_ns")),
        "raw_phase_families": _number(transition_metrics.get("raw_phase_family_count")),
        "raw_phase_memorization": _number(
            transition_metrics.get("raw_phase_memorization_families")
        ),
        "raw_phase_circuits": _number(
            transition_metrics.get("raw_phase_circuit_families")
        ),
        "raw_phase_cleanup": _number(
            transition_metrics.get("raw_phase_cleanup_families")
        ),
        "raw_phase_surfaces": _number(
            transition_metrics.get("raw_phase_max_observed_surfaces")
        ),
        "raw_phase_predicates": _number(
            transition_metrics.get("raw_phase_max_discovered_predicates")
        ),
        "raw_phase_packages": _number(
            transition_metrics.get("raw_phase_packages_induced")
        ),
        "gateway_rows": _number(metrics.get("gateway_decision_window_rows")),
        "gateway_accepts": _number(metrics.get("gateway_local_accept_events")),
        "gateway_saved_tokens": _number(metrics.get("gateway_tokens_saved_estimated")),
        "provider_boundary_rows": _number(metrics.get("provider_bridge_boundary_window_rows")),
        "provider_boundary_tokens": _number(metrics.get("provider_bridge_boundary_total_tokens")),
        "verifier_blocked_rows": _first_number(
            sources, "append_verifier_blocked_non_exact_rows"
        ),
        "verifier_blocked_tokens": _first_number(
            sources, "append_verifier_blocked_non_exact_tokens"
        ),
        "missing_safe_atoms": _first_number(sources, "append_skipped_no_safe_atoms"),
        "missing_verifier": _first_number(sources, "append_skipped_no_verifier_label"),
        "trust_filtered": _first_number(sources, "product_hot_phase_trust_filtered_events"),
        "online_profiles": _first_number(sources, "online_bucket_count")
        + _number(transition_metrics.get("profile_count")),
        "active_profiles": _first_number(sources, "active_bucket_count")
        + _number(transition_metrics.get("active_profile_count")),
        "candidate_profiles": _first_number(sources, "candidate_bucket_count"),
        "shadow_ready_profiles": _first_number(sources, "shadow_ready_bucket_count"),
        "hot_profiles": _first_number(
            sources, "product_hot_score_only_active_profile_count", "final_hot_profile_count"
        ),
        "quarantined_profiles": _first_number(
            sources, "product_hot_score_only_quarantined_profile_count"
        )
        + _number(transition_metrics.get("quarantined_profile_count")),
        "rejected_profiles": _first_number(sources, "rejected_bucket_count")
        + _number(transition_metrics.get("revoked_profile_count")),
        "promoted_profiles": _first_number(sources, "auto_recovery_promoted_profile_count")
        + _number(transition_metrics.get("profiles_promoted")),
        "pending_profiles": _first_number(sources, "auto_recovery_pending_profile_count"),
        "released_profiles": _first_number(
            sources, "auto_recovery_quarantine_released_profiles"
        ),
        "package_bytes": _first_number(sources, "product_hot_score_only_package_bytes"),
        "hot_bytes": _number(runtime_budget.get("hot_bytes_estimate")),
        "warm_bytes": _number(runtime_budget.get("warm_bytes_estimate")),
        "max_hot_bytes": _number(runtime_budget.get("max_hot_bytes_per_worker")),
        "stable_rows": _number(metrics.get("stable_decision_log_rows")),
        "stable_false": _number(metrics.get("stable_decision_log_false_accepts")),
        "proof_rows": _number(metrics.get("stable_decision_log_clean_suffix_rows")),
        "proof_candidates": _number(
            metrics.get("stable_decision_log_clean_suffix_score_candidate_events")
        ),
        "proof_false": _number(
            metrics.get("stable_clean_token_compression_false_accepts")
        ),
        "target_share_milli": _first_number(sources, "cpu_call_share_target_milli"),
        "estimated_cost_saved": _first_number(
            sources, "product_hot_score_only_cost_saved_microusd"
        ),
        "profiles_exported": _number(metrics.get("profile_exchange_exported_count")),
        "profiles_imported": _number(metrics.get("profile_exchange_imported_count")),
        "profiles_rejected_remote": _number(
            metrics.get("profile_exchange_rejected_remote_count")
        ),
        "bridge_latency_count": _number(latency.get("bridge_egress_count")),
        "bridge_p99_ms": _number(latency.get("bridge_egress_p99_ms")),
        "cpu_latency_count": _number(latency.get("cpu_local_accept_count")),
        "cpu_p99_ms": _number(latency.get("cpu_local_accept_p99_ms")),
        "bridge_rss": _number(resources.get("provider_bridge_rss_bytes")),
        "live_tail_rss": _number(resources.get("live_tail_rss_bytes")),
        "appender_rss": _number(resources.get("appender_rss_bytes")),
        "serving_rss": _number(resources.get("serving_rss_bytes")),
    }

    flags = {
        "health_ok": _flag(bridge.get("health_ok")),
        "source_report_present": _flag(metrics.get("source_report_present")),
        "canary_enabled": _flag(bridge.get("local_accept_enabled")),
        "broad_local_accept_enabled": _flag(live_tail.get("local_accept_enabled")),
        "product_promotion_enabled": _flag(live_tail.get("product_promotion_enabled")),
        "market_money_claim_allowed": _flag(metrics.get("market_money_claim_allowed")),
        "provider_evidence_ready": _flag(
            metrics.get("provider_bridge_boundary_cost_evidence_ready")
        )
        or _flag(provider_evidence.get("provider_billing_evidence_present")),
        "hot_budget_passed": _flag(runtime_budget.get("hot_budget_passed")),
        "warm_budget_passed": _flag(runtime_budget.get("warm_budget_passed")),
        "runtime_budget_passed": _flag(runtime_budget.get("product_runtime_budget_passed")),
        "profile_exchange_enabled": _flag(metrics.get("profile_exchange_enabled")),
        "upstream_base_configured": upstream_base_url_configured
        or _flag(bridge.get("upstream_base_url_configured")),
        "upstream_server_key_configured": _flag(
            bridge.get("upstream_server_api_key_configured")
        ),
        "client_auth_forwarding_supported": _flag(
            bridge.get("client_auth_forwarding_supported")
        ),
        "broad_provider_ready": _flag(summary.get("broad_provider_traffic_ready")),
        "exact_cache_overlap_excluded": _flag(live_tail.get("exact_cache_overlap_excluded")),
    }

    blockers = verify.get("blockers") if isinstance(verify.get("blockers"), list) else []
    blocker_codes = [str(item) for item in blockers]
    readiness_blocker = str(readiness.get("blocker") or "")
    if readiness_blocker and readiness_blocker not in blocker_codes:
        blocker_codes.append(readiness_blocker)
    money_blocker = str(
        metrics.get("provider_money_claim_blocker")
        or provider_evidence.get("blocker")
        or ""
    )

    issues: list[Issue] = []
    if not flags["source_report_present"]:
        issues.append(Issue("critical", "Нет отчёта майнера", "Поток профилей не наблюдается."))
    if age is None or age > 120:
        issues.append(
            Issue(
                "critical" if age is None or age > 600 else "warning",
                "Метрики устарели",
                _age_text(age),
            )
        )
    if n["actual_false"] > 0 or n["canary_false"] > 0 or n["typed_actor_false"] > 0:
        issues.append(
            Issue(
                "critical",
                "Есть ложные принятия",
                f"факт {n['actual_false']}, typed actors {n['typed_actor_false']}, проверочный контур {n['canary_false']}",
            )
        )
    if n["shadow_false"] > 0:
        issues.append(
            Issue(
                "warning",
                "Теневой эксперимент нашёл ошибки",
                f"{n['shadow_false']} отклонений профилей; пользователю эти ответы не выдавались",
            )
        )
    if n["proof_false"] > 0:
        issues.append(
            Issue(
                "warning",
                "Накопленный proof-контур не прошёл",
                f"{n['proof_false']} ошибок на {n['proof_rows']} строках и "
                f"{n['proof_candidates']} проверках кандидатов",
            )
        )
    if n["non_exact_rows"] > 0 and n["shadow_candidates"] > n["non_exact_rows"]:
        issues.append(
            Issue(
                "critical",
                "Смешаны окна знаменателей",
                f"кандидатов поверх кэша {n['shadow_candidates']}, а строк вне кэша {n['non_exact_rows']}",
            )
        )
    if n["parsed_rows"] > 0 and n["exact_cache_hits"] + n["shadow_candidates"] > n["parsed_rows"]:
        issues.append(
            Issue(
                "warning",
                "Кэш и кандидаты пересекаются",
                f"{n['exact_cache_hits']} + {n['shadow_candidates']} > {n['parsed_rows']}",
            )
        )
    if n["stable_rows"] > 0 and n["stable_false"] > n["stable_rows"]:
        issues.append(
            Issue(
                "info",
                "На одну строку проверяется несколько кандидатов",
                f"кандидатных отклонений {n['stable_false']}, строк {n['stable_rows']}; это не пользовательские ответы",
            )
        )
    if n["shadow_candidates"] > 0 and not flags["exact_cache_overlap_excluded"]:
        issues.append(
            Issue(
                "warning",
                "Пересечение с точным кэшем не доказано",
                "Теневой процент нельзя считать фактической экономией.",
            )
        )
    if n["actual_saved_tokens"] > n["actual_tokens"] > 0:
        issues.append(
            Issue(
                "critical",
                "Сохранённые токены больше входящих",
                f"{n['actual_saved_tokens']} > {n['actual_tokens']}",
            )
        )
    if not flags["warm_budget_passed"] and n["warm_bytes"] > 0:
        issues.append(
            Issue(
                "warning",
                "Тёплый пул превысил бюджет",
                f"используется {_format_bytes(n['warm_bytes'])}",
            )
        )
    if not flags["broad_local_accept_enabled"] and n["typed_actor_accepts"] == 0:
        issues.append(
            Issue(
                "info",
                "Широкое CPU-выполнение выключено",
                "Фактическая продуктовая экономия токенов остаётся нулевой.",
            )
        )
    if not flags["profile_exchange_enabled"]:
        issues.append(
            Issue(
                "info",
                "Обмен профилями между серверами не подключён",
                "Реестр, подпись, импорт и локальная перепроверка ещё не измеряются.",
            )
        )

    texts = {
        "age": _age_text(age),
        "appender_verdict": str(appender.get("verdict") or "неизвестно"),
        "miner_verdict": str(live_tail.get("verdict") or "неизвестно"),
        "next_action": _human_blocker(str(summary.get("next_action") or "")),
        "money_blocker": _human_blocker(money_blocker),
        "readiness_blocker": (
            f"{n['proof_false']} ошибок в накопленном proof-окне"
            if readiness_blocker == "stable_clean_token_false_accepts_nonzero"
            and n["proof_false"] > 0
            else _human_blocker(readiness_blocker)
        ),
        "blockers": ", ".join(_human_blocker(value) for value in blocker_codes) or "нет",
        "safety_policy": str(bridge.get("safety_policy") or "неизвестно"),
        "architecture_version": str(metrics.get("architecture_version_key") or "неизвестно"),
        "upstream_readiness": (
            f"{_number(upstream.get('observed_live_success_count'))} живых 2xx; "
            f"последний {_number(upstream.get('observed_live_latest_status'))} "
            f"{str(upstream.get('observed_live_latest_path') or '')}".rstrip()
            if _flag(upstream.get("observed_live_upstream_success"))
            else _human_blocker(str(summary.get("next_action") or ""))
        ),
    }

    return DashboardModel(
        generated=generated,
        age_seconds=age,
        numbers=n,
        flags=flags,
        texts=texts,
        issues=tuple(issues),
        class_rows=_rows(metrics.get("operator_class_token_ranking")),
        profile_rows=_rows(metrics.get("operator_profile_token_ranking"))
        + transition_profile_rows,
        quarantined_rows=_rows(metrics.get("quarantined_profile_token_ranking")),
        history=tuple(history),
    )


def _tone_for_zero_errors(value: int) -> str:
    return "good" if value == 0 else "bad"


def _badge(text: str, tone: str = "neutral") -> str:
    return f'<span class="badge {html.escape(tone)}">{html.escape(text)}</span>'


def _metric_card(
    title: str,
    value: str,
    unit: str,
    detail: str,
    scope: str,
    tone: str,
) -> str:
    return (
        f'<article class="metric-card {html.escape(tone)}">'
        f'<div class="metric-head"><span>{html.escape(title)}</span>{_badge(scope, tone)}</div>'
        f'<div class="metric-value">{html.escape(value)}<small>{html.escape(unit)}</small></div>'
        f'<div class="metric-detail">{html.escape(detail)}</div>'
        "</article>"
    )


def _progress_row(label: str, value: int, total: int, detail: str, tone: str) -> str:
    width = min(100.0, _percentage(value, total))
    return (
        '<div class="progress-row">'
        '<div class="progress-copy">'
        f'<span>{html.escape(label)}</span><b>{_format_number(value)} <small>из {_format_number(total)}</small></b>'
        "</div>"
        f'<div class="progress-track"><i class="{html.escape(tone)}" style="width:{width:.2f}%"></i></div>'
        f'<div class="progress-detail"><span>{width:.1f}%</span><small>{html.escape(detail)}</small></div>'
        "</div>"
    )


def _table(rows: list[list[str]], headers: list[str], classes: list[str] | None = None) -> str:
    head = "".join(f"<th>{html.escape(value)}</th>" for value in headers)
    body: list[str] = []
    for row_index, row in enumerate(rows):
        cells: list[str] = []
        for index, value in enumerate(row):
            css = ""
            if classes and index < len(classes) and classes[index]:
                css = f' class="{html.escape(classes[index])}"'
            cells.append(f"<td{css}>{html.escape(value)}</td>")
        body.append(f"<tr data-row=\"{row_index}\">{''.join(cells)}</tr>")
    if not body:
        body.append(f'<tr><td colspan="{len(headers)}">Нет данных</td></tr>')
    return f'<div class="table-wrap"><table><thead><tr>{head}</tr></thead><tbody>{"".join(body)}</tbody></table></div>'


def _history_value(row: dict[str, Any], *keys: str) -> float:
    for key in keys:
        value = row.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, int | float) and math.isfinite(float(value)):
            return max(0.0, float(value))
    return 0.0


def _history_clock(value: Any) -> str:
    raw = str(value or "")
    try:
        parsed = datetime.fromisoformat(raw.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            parsed = parsed.replace(tzinfo=timezone.utc)
        return parsed.astimezone(DASHBOARD_TIMEZONE).strftime("%H:%M:%S")
    except ValueError:
        return raw[11:19]


def _normalized_history(rows: tuple[dict[str, Any], ...]) -> list[dict[str, Any]]:
    version_two = [
        row
        for row in rows
        if str(row.get("schema_version") or "") == "nando_status_dashboard_history_v2"
    ]
    source_rows = version_two if version_two else list(rows)
    normalized: list[dict[str, Any]] = []
    for row in source_rows:
        incoming_rows = _history_value(row, "incoming_rows")
        actual_rows = _history_value(row, "actual_broad_rows")
        actual_accepts = _history_value(
            row, "actual_broad_cpu_accepts", "clean_cpu_accepts"
        )
        shadow_candidates = _history_value(row, "shadow_cpu_candidates")
        exact_hits = _history_value(row, "exact_cache_hits")
        canary_rows = _history_value(row, "canary_rows")
        canary_accepts = _history_value(
            row, "canary_cpu_accepts", "edge_serving_cpu_accepts"
        )
        normalized.append(
            {
                "timestamp": str(row.get("timestamp") or ""),
                "incoming_tokens": _history_value(
                    row, "incoming_tokens", "clean_total_tokens"
                ),
                "actual_saved_tokens": _history_value(
                    row, "actual_broad_saved_tokens", "clean_saved_tokens"
                ),
                "shadow_tokens": _history_value(row, "shadow_potential_tokens"),
                "canary_tokens": _history_value(
                    row, "canary_saved_tokens", "edge_saved_tokens", "edge_serving_cpu_tokens"
                ),
                "actual_share": _percentage(int(actual_accepts), int(actual_rows)),
                "shadow_share": _percentage(int(shadow_candidates), int(incoming_rows)),
                "exact_share": _percentage(int(exact_hits), int(incoming_rows)),
                "canary_share": _percentage(int(canary_accepts), int(canary_rows)),
                "profiles_total": _history_value(row, "profiles_mined"),
                "profiles_hot": _history_value(row, "profiles_hot"),
                "profiles_quarantine": _history_value(row, "profiles_quarantined"),
                "profiles_promoted": _history_value(row, "profiles_promoted"),
                "actual_false": _history_value(
                    row, "actual_broad_false_accepts", "clean_false_accepts"
                ),
                "shadow_false": _history_value(
                    row, "shadow_false_accepts", "active_false_accepts"
                ),
                "canary_false": _history_value(
                    row, "canary_false_accepts", "edge_serving_cpu_false_accepts"
                ),
            }
        )
    previous: dict[str, float] | None = None
    for row in normalized:
        current = {
            "actual_false": float(row["actual_false"]),
            "shadow_false": float(row["shadow_false"]),
            "canary_false": float(row["canary_false"]),
        }
        for key, value in current.items():
            prior = previous.get(key, value) if previous is not None else value
            row[f"{key}_new"] = value - prior if value >= prior else value
        previous = current
    return normalized


def _line_chart(
    title: str,
    rows: list[dict[str, Any]],
    series: list[tuple[str, str, str]],
    unit: str,
    fixed_max: float | None = None,
) -> str:
    usable = rows[-240:]
    if len(usable) < 2:
        return (
            '<figure class="chart"><figcaption><b>'
            + html.escape(title)
            + "</b><span>Недостаточно точек</span></figcaption>"
            + '<div class="chart-empty">График появится после новых снимков метрик</div></figure>'
        )

    width = 960
    height = 250
    left = 64
    right = 18
    top = 22
    bottom = 36
    plot_width = width - left - right
    plot_height = height - top - bottom
    maximum = fixed_max or max(
        1.0,
        max(float(row.get(key, 0.0)) for row in usable for _, key, _ in series),
    )

    grid: list[str] = []
    for step in range(5):
        ratio = step / 4
        y = top + plot_height - plot_height * ratio
        label = maximum * ratio
        label_text = f"{label:.0f}" if maximum < 10_000 else f"{label / 1000:.0f}k"
        grid.append(
            f'<line x1="{left}" y1="{y:.1f}" x2="{width - right}" y2="{y:.1f}" class="chart-grid"/>'
            f'<text x="4" y="{y + 4:.1f}" class="axis-label">{html.escape(label_text)}</text>'
        )

    lines: list[str] = []
    legends: list[str] = []
    for label, key, color in series:
        points: list[str] = []
        for index, row in enumerate(usable):
            value = min(maximum, max(0.0, float(row.get(key, 0.0))))
            x = left + plot_width * index / max(1, len(usable) - 1)
            y = top + plot_height - plot_height * value / maximum
            points.append(f"{x:.1f},{y:.1f}")
        latest = int(float(usable[-1].get(key, 0.0)))
        lines.append(
            f'<polyline points="{" ".join(points)}" class="chart-line {html.escape(color)}"/>'
        )
        legends.append(
            f'<span><i class="legend-mark {html.escape(color)}"></i>{html.escape(label)} <b>{_format_number(latest)}{html.escape(unit)}</b></span>'
        )

    first_stamp = html.escape(_history_clock(usable[0].get("timestamp")))
    last_stamp = html.escape(_history_clock(usable[-1].get("timestamp")))
    return (
        '<figure class="chart">'
        f'<figcaption><b>{html.escape(title)}</b><span>{len(usable)} точек</span></figcaption>'
        f'<div class="chart-legend">{"".join(legends)}</div>'
        f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(title)}">'
        f'{"".join(grid)}{"".join(lines)}'
        f'<text x="{left}" y="{height - 8}" class="axis-label">{first_stamp}</text>'
        f'<text x="{width - right - 48}" y="{height - 8}" class="axis-label">{last_stamp}</text>'
        "</svg></figure>"
    )


def _issue_list(model: DashboardModel) -> str:
    return "".join(
        '<li class="issue '
        + html.escape(issue.severity)
        + '"><span>'
        + _badge(
            {"critical": "КРИТИЧНО", "warning": "ВНИМАНИЕ", "info": "СТАТУС"}.get(
                issue.severity, "СТАТУС"
            ),
            {"critical": "bad", "warning": "warn", "info": "neutral"}.get(
                issue.severity, "neutral"
            ),
        )
        + "</span><div><b>"
        + html.escape(issue.title)
        + "</b><small>"
        + html.escape(issue.detail)
        + "</small></div></li>"
        for issue in model.issues
    )


def _module_rows(model: DashboardModel) -> list[list[str]]:
    n = model.numbers
    capture_share = _format_percentage(n["captured_rows"], n["capture_json_rows"])
    parse_share = _format_percentage(n["parsed_rows"], n["captured_rows"])
    signal_share = _format_percentage(n["score_signals"], n["parsed_rows"])
    shadow_share = _format_percentage(n["shadow_candidates"], n["parsed_rows"])
    actual_share = _format_percentage(n["actual_accepts"], n["actual_rows"])
    canary_share = _format_percentage(n["canary_accepts"], n["canary_rows"])
    compile_share = _format_percentage(n["hot_profiles"], n["candidate_profiles"])
    return [
        [
            "Сборщик событий",
            _format_number(n["capture_json_rows"]),
            _format_number(n["captured_rows"]),
            capture_share,
            _format_number(n["capture_skipped"]),
            "работает" if n["captured_rows"] > 0 else "нет потока",
        ],
        [
            "Разбор потока",
            _format_number(n["captured_rows"]),
            _format_number(n["parsed_rows"]),
            parse_share,
            _format_number(n["missing_safe_atoms"]),
            "работает" if n["parsed_rows"] > 0 else "нет данных",
        ],
        [
            "Майнер классов",
            _format_number(n["parsed_rows"]),
            _format_number(n["score_signals"]),
            signal_share,
            _format_number(n["rejected_profiles"]),
            "теневой режим",
        ],
        [
            "Горячий маршрутизатор",
            _format_number(n["parsed_rows"]),
            _format_number(n["shadow_candidates"]),
            shadow_share,
            _format_number(n["shadow_false"]),
            "кандидаты, не факт",
        ],
        [
            "Проверяющий контур",
            _format_number(n["non_exact_rows"]),
            _format_number(n["verifier_blocked_rows"]),
            _format_percentage(n["verifier_blocked_rows"], n["non_exact_rows"]),
            _format_number(n["actual_false"]),
            "блокирует риск",
        ],
        [
            "Широкое CPU-выполнение",
            _format_number(n["actual_rows"]),
            _format_number(n["actual_accepts"]),
            actual_share,
            _format_number(n["actual_false"]),
            "включено" if model.flags["broad_local_accept_enabled"] else "выключено",
        ],
        [
            "Проверочный серверный контур",
            _format_number(n["canary_rows"]),
            _format_number(n["canary_accepts"]),
            canary_share,
            _format_number(n["canary_false"]),
            "ограниченные маршруты",
        ],
        [
            "Компилятор профилей",
            _format_number(n["candidate_profiles"]),
            _format_number(n["hot_profiles"]),
            compile_share,
            _format_number(n["quarantined_profiles"]),
            "локальные профили",
        ],
    ]


def _profile_rows(model: DashboardModel) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in model.profile_rows[:12]:
        profile_id = str(row.get("profile_id") or "неизвестно")
        kind = _human_profile_kind(
            row.get("class") or row.get("kind") or row.get("operator_kind") or row.get("status")
        )
        accepts = _number(
            row.get("unique_cpu_accepts_over_exact_cache") or row.get("future_clean_rows")
        )
        tokens = _number(row.get("tokens_saved"))
        false_accepts = _number(row.get("false_accepts"))
        status = _human_profile_kind(
            row.get("status")
            or row.get("state")
            or ("final_hot" if row.get("final_hot") else "unknown")
        )
        rows.append(
            [
                profile_id,
                kind,
                _format_number(accepts),
                _format_number(tokens),
                _format_number(false_accepts),
                status,
            ]
        )
    return rows


def _class_rows(model: DashboardModel) -> list[list[str]]:
    rows: list[list[str]] = []
    for row in model.class_rows[:10]:
        rows.append(
            [
                _human_profile_kind(row.get("class") or row.get("kind")),
                _format_number(_number(row.get("profiles"))),
                _format_number(_number(row.get("unique_cpu_accepts_over_exact_cache"))),
                _format_number(_number(row.get("tokens_saved"))),
                _format_number(_number(row.get("false_accepts"))),
            ]
        )
    return rows


def render_status_dashboard(
    *,
    metrics: dict[str, Any],
    live_tail: dict[str, Any],
    status: dict[str, Any],
    appender: dict[str, Any],
    history: list[dict[str, Any]],
    transition_metrics: dict[str, Any] | None = None,
    transition_registry: dict[str, Any] | None = None,
    economics: dict[str, Any] | None = None,
    request_path: str = "",
    upstream_base_url_configured: bool = False,
) -> str:
    model = build_dashboard_model(
        metrics=metrics,
        live_tail=live_tail,
        status=status,
        appender=appender,
        transition_metrics=transition_metrics,
        transition_registry=transition_registry,
        economics=economics,
        history=history,
        upstream_base_url_configured=upstream_base_url_configured,
    )
    n = model.numbers
    f = model.flags
    safe_path = html.escape(urllib.parse.urlsplit(request_path).path or "/v2/status/")
    broad_fallback = max(0, n["actual_rows"] - n["actual_accepts"])
    chart_rows = _normalized_history(model.history)

    if n["actual_saved_tokens"] > 0 and n["actual_false"] == 0:
        verdict_tone = "good"
        verdict_title = "Экономия токенов работает"
        verdict_detail = (
            f"CPU фактически сохранил {_format_number(n['actual_saved_tokens'])} токенов "
            f"в окне из {_format_number(n['actual_tokens'])}."
        )
    elif n["typed_actor_accepts"] > 0 and n["typed_actor_false"] == 0:
        verdict_tone = "good"
        verdict_title = "Typed actors работают на CPU"
        verdict_detail = (
            f"Предотвращено {_format_number(n['typed_actor_accepts'])} обращений к LLM; "
            f"оценка экономии {_format_number(n['typed_actor_saved_tokens'])} токенов. "
            "Широкий обычный трафик пока передаётся модели."
        )
    else:
        verdict_tone = "warn"
        verdict_title = "Широкая экономия токенов пока не включена"
        verdict_detail = (
            f"Фактически сохранено {_format_number(n['actual_saved_tokens'])}; "
            f"теневой потенциал {_format_number(n['shadow_tokens'])} токенов."
        )

    metrics_html = "".join(
        [
            _metric_card(
                "LLM-вызовов предотвращено",
                _format_number(n["typed_actor_accepts"]),
                "вызовов",
                "верифицированные typed transitions",
                "ФАКТ",
                "good" if n["typed_actor_accepts"] > 0 and n["typed_actor_false"] == 0 else "neutral",
            ),
            _metric_card(
                "Экономия typed actors",
                _format_number(n["typed_actor_saved_tokens"]),
                "токенов, оценка",
                "LLM не вызывалась для этих переходов",
                "ОЦЕНКА",
                "good" if n["typed_actor_accepts"] > 0 and n["typed_actor_false"] == 0 else "neutral",
            ),
            _metric_card(
                "Широкая экономия",
                _format_number(n["actual_saved_tokens"]),
                "токенов",
                f"{_format_number(n['actual_saved_tokens'])} из {_format_number(n['actual_tokens'])}",
                "ФАКТ" if n["actual_saved_tokens"] > 0 else "НЕ ВКЛЮЧЕНА",
                "good" if n["actual_saved_tokens"] > 0 else "neutral",
            ),
            _metric_card(
                "Обычный трафик на CPU",
                _format_percentage(n["actual_accepts"], n["actual_rows"]),
                "",
                f"{_format_number(n['actual_accepts'])} из {_format_number(n['actual_rows'])} событий",
                "ФАКТ",
                "good" if n["actual_accepts"] > 0 else "neutral",
            ),
            _metric_card(
                "Потенциал экономии",
                _format_number(n["shadow_tokens"]),
                "токенов",
                f"{_format_percentage(n['shadow_tokens'], n['incoming_tokens'])} входящих токенов",
                "ТЕНЕВОЙ",
                "blue",
            ),
            _metric_card(
                "Потенциальное покрытие CPU",
                _format_percentage(n["shadow_candidates"], n["parsed_rows"]),
                "",
                f"{_format_number(n['shadow_candidates'])} из {_format_number(n['parsed_rows'])} событий",
                "ТЕНЕВОЙ",
                "blue",
            ),
            _metric_card(
                "Проверочный CPU-контур",
                _format_number(n["canary_saved_tokens"]),
                "токенов",
                f"оценка, {_format_number(n['canary_accepts'])} локальных ответов",
                "ОГРАНИЧЕННЫЙ",
                "purple",
            ),
            _metric_card(
                "Входящий объём",
                _format_number(n["incoming_tokens"]),
                "токенов",
                f"{_format_number(n['parsed_rows'])} разобранных событий",
                "ПОТОК",
                "neutral",
            ),
        ]
    )

    traffic_html = "".join(
        [
            _progress_row(
                "Разобрано майнером",
                n["parsed_rows"],
                max(n["captured_rows"], n["parsed_rows"]),
                f"пропущено сборщиком: {_format_number(n['capture_skipped'])}",
                "green",
            ),
            _progress_row(
                "Найдено точным кэшем",
                n["exact_cache_hits"],
                n["parsed_rows"],
                "показывается отдельно от переносимых профилей",
                "amber",
            ),
            _progress_row(
                "Сигналы для майнера",
                n["score_signals"],
                n["parsed_rows"],
                "события, дошедшие до фазовой оценки",
                "blue",
            ),
            _progress_row(
                "Теневые CPU-кандидаты",
                n["shadow_candidates"],
                n["parsed_rows"],
                f"ложных в текущем окне: {_format_number(n['shadow_false'])}",
                "blue",
            ),
            _progress_row(
                "Фактически выполнено CPU",
                n["actual_accepts"],
                n["actual_rows"],
                f"ложных принятий: {_format_number(n['actual_false'])}",
                "green" if n["actual_accepts"] > 0 else "gray",
            ),
            _progress_row(
                "Передано LLM",
                broad_fallback,
                n["actual_rows"],
                "фактический serving-контур",
                "red" if broad_fallback > 0 else "green",
            ),
            _progress_row(
                "CPU-кандидат, пропущенный майнером",
                n["decidability_miner_gap"],
                n["decidability_classified"],
                "есть уникальное grounded evidence, но нет ACTIVE-программы",
                "amber",
            ),
            _progress_row(
                "Недоопределено текущими данными",
                n["decidability_not_executable"],
                n["decidability_classified"],
                "без дополнительного отношения CPU обязан отказаться",
                "red",
            ),
            _progress_row(
                "Форма не поддерживается DSL",
                n["decidability_dsl_gap"],
                n["decidability_classified"],
                "кандидат для расширения typed DSL",
                "blue",
            ),
        ]
    )

    profile_metrics = "".join(
        [
            _metric_card("Создано профилей", _format_number(n["online_profiles"]), "", "в локальном пуле", "МАЙНЕР", "neutral"),
            _metric_card("Активные", _format_number(n["active_profiles"]), "", "получают новые события", "ЖИВЫЕ", "blue"),
            _metric_card("Горячие", _format_number(n["hot_profiles"]), "", _format_bytes(n["package_bytes"]), "CPU", "good"),
            _metric_card("Карантин", _format_number(n["quarantined_profiles"]), "", f"отклонено {_format_number(n['rejected_profiles'])}", "РИСК", "warn"),
            _metric_card("Продвинуты", _format_number(n["promoted_profiles"]), "", f"ожидают {_format_number(n['pending_profiles'])}", "ЛОКАЛЬНО", "purple"),
            _metric_card(
                "Фазовые семьи",
                _format_number(n["raw_phase_families"]),
                "",
                f"memorization {_format_number(n['raw_phase_memorization'])}; circuit {_format_number(n['raw_phase_circuits'])}; cleanup {_format_number(n['raw_phase_cleanup'])}",
                "RAW PHASE",
                "blue",
            ),
            _metric_card(
                "Открыто отношений",
                _format_number(n["raw_phase_predicates"]),
                "",
                f"поверхностей {_format_number(n['raw_phase_surfaces'])}",
                "GROKKING",
                "good" if n["raw_phase_cleanup"] > 0 else "neutral",
            ),
            _metric_card(
                "Компактные packages",
                _format_number(n["raw_phase_packages"]),
                "",
                "автоматически созданы майнером",
                "CPU",
                "good" if n["raw_phase_packages"] > 0 else "neutral",
            ),
            _metric_card(
                "Обмен между серверами",
                _format_number(n["profiles_exported"] + n["profiles_imported"]),
                "",
                "не подключён" if not f["profile_exchange_enabled"] else "работает",
                "СЕТЬ",
                "neutral" if not f["profile_exchange_enabled"] else "good",
            ),
        ]
    )

    token_chart = _line_chart(
        "Токены во времени",
        chart_rows,
        [
            ("входящие", "incoming_tokens", "line-ink"),
            ("фактически сохранено", "actual_saved_tokens", "line-green"),
            ("теневой потенциал", "shadow_tokens", "line-blue"),
            ("проверочный контур", "canary_tokens", "line-purple"),
        ],
        "",
    )
    share_chart = _line_chart(
        "Доля событий во времени",
        chart_rows,
        [
            ("фактический CPU", "actual_share", "line-green"),
            ("теневой потенциал", "shadow_share", "line-blue"),
            ("точный кэш", "exact_share", "line-amber"),
            ("проверочный контур", "canary_share", "line-purple"),
        ],
        "%",
        100.0,
    )
    profile_chart = _line_chart(
        "Профили во времени",
        chart_rows,
        [
            ("всего", "profiles_total", "line-ink"),
            ("горячие", "profiles_hot", "line-green"),
            ("карантин", "profiles_quarantine", "line-amber"),
            ("продвинуты", "profiles_promoted", "line-blue"),
        ],
        "",
    )
    error_chart = _line_chart(
        "Новые ошибки за интервал",
        chart_rows,
        [
            ("фактические", "actual_false_new", "line-red"),
            ("теневой эксперимент", "shadow_false_new", "line-amber"),
            ("проверочный контур", "canary_false_new", "line-purple"),
        ],
        "",
    )

    module_table = _table(
        _module_rows(model),
        ["Модуль", "Вход", "Выход", "Доля", "Ошибки/отказы", "Состояние"],
        ["", "numeric", "numeric", "numeric", "numeric", ""],
    )
    profile_table = _table(
        _profile_rows(model),
        ["Профиль", "Класс", "Кандидаты", "Токены", "Ошибки", "Состояние"],
        ["mono", "", "numeric", "numeric", "numeric", ""],
    )
    class_table = _table(
        _class_rows(model),
        ["Класс", "Профили", "Кандидаты", "Токены", "Ошибки"],
        ["", "numeric", "numeric", "numeric", "numeric"],
    )

    server_rows = [
        ["Шлюз", "работает" if f["health_ok"] else "ошибка", model.texts["age"]],
        [
            "Upstream-провайдер",
            "настроен" if f["upstream_base_configured"] else "не настроен",
            (
                "авторизация клиента передаётся"
                if f["client_auth_forwarding_supported"]
                and not f["upstream_server_key_configured"]
                else "серверный API-ключ настроен"
                if f["upstream_server_key_configured"]
                else "нет способа авторизации"
            ),
        ],
        ["Широкое CPU-выполнение", "включено" if f["broad_local_accept_enabled"] else "выключено", model.texts["readiness_blocker"]],
        ["Проверочный CPU-контур", "включён" if f["canary_enabled"] else "выключен", model.texts["safety_policy"]],
        ["Широкий трафик провайдера", "готов" if f["broad_provider_ready"] else "не готов", model.texts["upstream_readiness"]],
        ["Доказательство денег", "готово" if f["provider_evidence_ready"] else "нет", model.texts["money_blocker"]],
        ["Горячая память", _format_bytes(n["hot_bytes"]), "бюджет пройден" if f["hot_budget_passed"] else "бюджет не пройден"],
        ["Тёплая память", _format_bytes(n["warm_bytes"]), "бюджет пройден" if f["warm_budget_passed"] else "бюджет превышен"],
        [
            "p99 CPU local accept",
            _format_duration_ms(n["cpu_p99_ms"]),
            f"{_format_number(n['cpu_latency_count'])} наблюдений",
        ],
        [
            "p99 шлюза end-to-end",
            _format_duration_ms(n["bridge_p99_ms"]),
            f"{_format_number(n['bridge_latency_count'])} наблюдений, включая время LLM",
        ],
        [
            "RSS рабочего контура",
            _format_bytes(n["serving_rss"]),
            f"bridge {_format_bytes(n['bridge_rss'])}; miner {_format_bytes(n['live_tail_rss'])}; appender {_format_bytes(n['appender_rss'])}",
        ],
    ]
    server_table = _table(server_rows, ["Контур", "Значение", "Граница"])

    html_template = """<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="refresh" content="15">
  <title>Nando CPU - статистика экономии токенов</title>
  <style>
    :root {
      color-scheme: light;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      --canvas: #f3f6f7;
      --surface: #ffffff;
      --ink: #1c2529;
      --muted: #66747b;
      --line: #d9e1e4;
      --green: #147a52;
      --green-soft: #e4f3ec;
      --blue: #2864c7;
      --blue-soft: #e8f0ff;
      --amber: #a76100;
      --amber-soft: #fff1d6;
      --red: #b93636;
      --red-soft: #fde9e8;
      --purple: #74509b;
      --purple-soft: #f0e9f7;
    }
    * { box-sizing: border-box; }
    html { scroll-behavior: smooth; }
    body { margin: 0; overflow-x: hidden; background: var(--canvas); color: var(--ink); }
    a { color: inherit; }
    .shell { width: min(1440px, calc(100% - 32px)); margin: 0 auto; }
    .topbar { background: var(--surface); border-bottom: 1px solid var(--line); }
    .topbar-inner { min-height: 72px; display: flex; align-items: center; justify-content: space-between; gap: 24px; }
    .brand { display: flex; align-items: center; gap: 12px; min-width: 0; }
    .brand-mark { width: 9px; height: 36px; background: var(--green); border-radius: 2px; flex: 0 0 auto; }
    .brand h1 { margin: 0; font-size: 21px; line-height: 1.15; letter-spacing: 0; }
    .brand p { margin: 3px 0 0; color: var(--muted); font-size: 13px; }
    .top-actions { display: flex; align-items: center; gap: 12px; color: var(--muted); font-size: 12px; text-align: right; }
    .refresh { display: inline-flex; align-items: center; min-height: 34px; padding: 0 12px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface); text-decoration: none; color: var(--ink); font-weight: 700; }
    .tabs { position: sticky; top: 0; z-index: 10; background: rgba(255, 255, 255, .96); border-bottom: 1px solid var(--line); overflow-x: auto; scrollbar-width: none; }
    .tabs::-webkit-scrollbar { display: none; }
    .tabs .shell { display: flex; min-height: 44px; gap: 24px; align-items: stretch; }
    .tabs a { display: inline-flex; align-items: center; color: var(--muted); font-size: 13px; font-weight: 700; text-decoration: none; white-space: nowrap; border-bottom: 2px solid transparent; }
    .tabs a:hover { color: var(--ink); border-bottom-color: var(--green); }
    main { min-width: 0; overflow-x: hidden; padding-bottom: 36px; }
    .band { min-width: 0; overflow-x: hidden; padding: 24px 0; border-bottom: 1px solid var(--line); scroll-margin-top: 48px; }
    .band.white { background: var(--surface); }
    .section-head { display: flex; justify-content: space-between; gap: 20px; align-items: end; margin-bottom: 16px; }
    .section-head h2 { margin: 0; font-size: 18px; letter-spacing: 0; }
    .section-head p { margin: 0; max-width: 760px; color: var(--muted); font-size: 13px; line-height: 1.45; text-align: right; }
    .verdict { display: flex; align-items: center; justify-content: space-between; gap: 20px; min-height: 66px; padding: 14px 16px; border-left: 5px solid var(--amber); background: var(--surface); }
    .verdict.good { border-left-color: var(--green); }
    .verdict b { display: block; font-size: 16px; }
    .verdict span { color: var(--muted); font-size: 13px; }
    .metric-grid { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 10px; margin-top: 12px; }
    .metric-card { min-width: 0; min-height: 142px; display: grid; align-content: space-between; gap: 14px; padding: 14px; background: var(--surface); border: 1px solid var(--line); border-top: 3px solid #aab5ba; border-radius: 6px; }
    .metric-card.good { border-top-color: var(--green); }
    .metric-card.blue { border-top-color: var(--blue); }
    .metric-card.warn { border-top-color: var(--amber); }
    .metric-card.purple { border-top-color: var(--purple); }
    .metric-head { display: flex; align-items: start; justify-content: space-between; gap: 8px; color: var(--muted); font-size: 12px; font-weight: 700; line-height: 1.3; }
    .metric-value { font-variant-numeric: tabular-nums; font-size: 27px; line-height: 1; font-weight: 780; overflow-wrap: anywhere; }
    .metric-value small { display: block; margin-top: 6px; color: var(--muted); font-size: 12px; font-weight: 600; }
    .metric-detail { color: var(--muted); font-size: 12px; line-height: 1.35; }
    .badge { display: inline-flex; align-items: center; min-height: 20px; padding: 2px 6px; border-radius: 4px; background: #edf1f2; color: #546168; font-size: 9px; font-weight: 800; letter-spacing: 0; white-space: nowrap; }
    .badge.good { background: var(--green-soft); color: var(--green); }
    .badge.blue { background: var(--blue-soft); color: var(--blue); }
    .badge.warn { background: var(--amber-soft); color: var(--amber); }
    .badge.bad { background: var(--red-soft); color: var(--red); }
    .badge.purple { background: var(--purple-soft); color: var(--purple); }
    .two-col { display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(320px, .75fr); gap: 28px; align-items: start; }
    .progress-list { display: grid; gap: 16px; }
    .progress-row { min-width: 0; }
    .progress-copy, .progress-detail { display: flex; justify-content: space-between; gap: 12px; align-items: baseline; }
    .progress-copy span { color: var(--muted); font-size: 12px; font-weight: 700; }
    .progress-copy b { font-size: 14px; font-variant-numeric: tabular-nums; }
    .progress-copy b small, .progress-detail small { color: var(--muted); font-weight: 500; }
    .progress-track { height: 9px; margin: 7px 0 5px; background: #e5eaec; overflow: hidden; border-radius: 3px; }
    .progress-track i { display: block; height: 100%; min-width: 0; background: #aab5ba; }
    .progress-track i.green { background: var(--green); }
    .progress-track i.blue { background: var(--blue); }
    .progress-track i.amber { background: var(--amber); }
    .progress-track i.red { background: var(--red); }
    .progress-detail { color: var(--muted); font-size: 11px; }
    .progress-detail span { color: var(--ink); font-weight: 800; }
    .scope-list { margin: 0; padding: 0; list-style: none; border-top: 1px solid var(--line); }
    .scope-list li { display: grid; grid-template-columns: minmax(150px, 1fr) auto; gap: 16px; padding: 11px 0; border-bottom: 1px solid var(--line); }
    .scope-list span { color: var(--muted); font-size: 12px; }
    .scope-list b { font-size: 13px; text-align: right; font-variant-numeric: tabular-nums; }
    .chart-grid-layout { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
    .chart { min-width: 0; margin: 0; padding: 14px; background: var(--surface); border: 1px solid var(--line); border-radius: 6px; }
    .chart figcaption { display: flex; justify-content: space-between; gap: 12px; color: var(--muted); font-size: 12px; }
    .chart figcaption b { color: var(--ink); font-size: 14px; }
    .chart svg { display: block; width: 100%; height: auto; margin-top: 8px; }
    .chart-grid { stroke: #e3e8ea; stroke-width: 1; }
    .chart-line { fill: none; stroke-width: 3; stroke-linecap: round; stroke-linejoin: round; }
    .line-ink { stroke: #47545a; color: #47545a; background: #47545a; }
    .line-green { stroke: var(--green); color: var(--green); background: var(--green); }
    .line-blue { stroke: var(--blue); color: var(--blue); background: var(--blue); }
    .line-amber { stroke: var(--amber); color: var(--amber); background: var(--amber); }
    .line-red { stroke: var(--red); color: var(--red); background: var(--red); }
    .line-purple { stroke: var(--purple); color: var(--purple); background: var(--purple); }
    .axis-label { fill: var(--muted); font-size: 11px; }
    .chart-legend { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-top: 10px; color: var(--muted); font-size: 11px; }
    .chart-legend span { display: inline-flex; align-items: center; gap: 5px; }
    .chart-legend b { color: var(--ink); font-variant-numeric: tabular-nums; }
    .legend-mark { display: inline-block; width: 12px; height: 3px; border-radius: 1px; }
    .chart-empty { min-height: 190px; display: grid; place-items: center; color: var(--muted); font-size: 12px; }
    .table-wrap { width: 100%; overflow-x: auto; border-top: 1px solid var(--line); }
    table { width: 100%; border-collapse: collapse; font-size: 12px; }
    th, td { min-width: 105px; padding: 10px 8px; text-align: left; border-bottom: 1px solid var(--line); vertical-align: top; }
    th { color: var(--muted); font-weight: 750; white-space: nowrap; }
    td.numeric { text-align: right; font-variant-numeric: tabular-nums; }
    td.mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
    .table-grid { min-width: 0; display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 26px; }
    .table-grid > div { min-width: 0; overflow: hidden; }
    .table-grid h3 { margin: 0 0 10px; font-size: 14px; }
    .issue-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin: 0; padding: 0; list-style: none; }
    .issue { min-width: 0; display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 10px; align-items: start; padding: 11px 0; border-bottom: 1px solid var(--line); }
    .issue b, .issue small { display: block; }
    .issue b { font-size: 13px; }
    .issue small { margin-top: 3px; color: var(--muted); font-size: 11px; line-height: 1.4; }
    .error-counters { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 10px; margin-bottom: 18px; }
    .counter { padding: 12px; border-left: 3px solid var(--green); background: var(--surface); }
    .counter.bad { border-left-color: var(--red); }
    .counter.warn { border-left-color: var(--amber); }
    .counter span { display: block; color: var(--muted); font-size: 11px; line-height: 1.3; }
    .counter b { display: block; margin-top: 6px; font-size: 22px; font-variant-numeric: tabular-nums; }
    .footer { padding: 18px 0; color: var(--muted); font-size: 11px; line-height: 1.5; overflow-wrap: anywhere; }
    @media (max-width: 1180px) {
      .metric-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
      .error-counters { grid-template-columns: repeat(3, minmax(0, 1fr)); }
    }
    @media (max-width: 820px) {
      .shell { width: min(100% - 20px, 1440px); }
      .topbar-inner { align-items: flex-start; flex-direction: column; padding: 14px 0; gap: 10px; }
      .top-actions { width: 100%; justify-content: space-between; text-align: left; }
      .tabs .shell { gap: 18px; }
      .section-head { align-items: flex-start; flex-direction: column; gap: 6px; }
      .section-head p { text-align: left; }
      .two-col, .chart-grid-layout, .table-grid { grid-template-columns: 1fr; }
      .issue-list { grid-template-columns: 1fr; }
    }
    @media (max-width: 620px) {
      .metric-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .metric-card { min-height: 132px; padding: 12px; }
      .metric-head { flex-wrap: wrap; }
      .metric-head .badge { max-width: 100%; white-space: normal; overflow-wrap: anywhere; text-align: center; }
      .metric-value { font-size: 23px; }
      .error-counters { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .verdict { align-items: flex-start; flex-direction: column; gap: 6px; }
      .band { padding: 19px 0; }
    }
    @media (max-width: 390px) {
      .metric-grid { grid-template-columns: 1fr; }
      .metric-card { min-height: 118px; }
      .error-counters { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <header class="topbar">
    <div class="shell topbar-inner">
      <div class="brand">
        <i class="brand-mark" aria-hidden="true"></i>
        <div><h1>Nando CPU</h1><p>Сервер статистики экономии токенов</p></div>
      </div>
      <div class="top-actions">
        <span>Метрики: __AGE__<br>Автообновление: 15 с</span>
        <a class="refresh" href="__REFRESH_PATH__">Обновить</a>
      </div>
    </div>
  </header>
  <nav class="tabs" aria-label="Разделы статистики">
    <div class="shell">
      <a href="#economy">Экономия</a>
      <a href="#traffic">Трафик</a>
      <a href="#charts">Графики</a>
      <a href="#modules">Модули</a>
      <a href="#profiles">Профили</a>
      <a href="#errors">Ошибки</a>
      <a href="#server">Сервер</a>
    </div>
  </nav>
  <main>
    <section class="band" id="economy">
      <div class="shell">
        <div class="verdict __VERDICT_TONE__"><div><b>__VERDICT_TITLE__</b><span>__VERDICT_DETAIL__</span></div>__HEALTH_BADGE__</div>
        <div class="metric-grid">__METRICS__</div>
      </div>
    </section>
    <section class="band white" id="traffic">
      <div class="shell">
        <div class="section-head"><h2>Движение трафика</h2><p>Теневые кандидаты, точный кэш и проверочные маршруты не смешиваются с фактическим широким CPU-выполнением.</p></div>
        <div class="two-col">
          <div class="progress-list">__TRAFFIC__</div>
          <ul class="scope-list">
            <li><span>Живой поток майнера</span><b>__PARSED_ROWS__ событий / __INCOMING_TOKENS__ токенов</b></li>
            <li><span>Фактический serving-контур</span><b>__ACTUAL_ROWS__ событий</b></li>
            <li><span>Граница провайдера</span><b>__BOUNDARY_ROWS__ событий / __BOUNDARY_TOKENS__ токенов</b></li>
            <li><span>Проверочный серверный контур</span><b>__CANARY_ROWS__ событий</b></li>
            <li><span>Цель CPU-покрытия</span><b>__TARGET_SHARE__</b></li>
          </ul>
        </div>
      </div>
    </section>
    <section class="band" id="charts">
      <div class="shell">
        <div class="section-head"><h2>Графики</h2><p>Накопительные значения из периодических снимков сервера.</p></div>
        <div class="chart-grid-layout">__TOKEN_CHART____SHARE_CHART____PROFILE_CHART____ERROR_CHART__</div>
      </div>
    </section>
    <section class="band white" id="modules">
      <div class="shell">
        <div class="section-head"><h2>Эффективность модулей</h2><p>Каждая строка использует собственный явно указанный вход и выход.</p></div>
        __MODULE_TABLE__
      </div>
    </section>
    <section class="band" id="profiles">
      <div class="shell">
        <div class="section-head"><h2>Профили</h2><p>Создание, продвижение, карантин и готовность к будущему обмену между серверами.</p></div>
        <div class="metric-grid">__PROFILE_METRICS__</div>
        <div class="table-grid" style="margin-top:24px">
          <div><h3>Лучшие профили</h3>__PROFILE_TABLE__</div>
          <div><h3>Классы профилей</h3>__CLASS_TABLE__</div>
        </div>
      </div>
    </section>
    <section class="band white" id="errors">
      <div class="shell">
        <div class="section-head"><h2>Ошибки и блокировки</h2><p>Нулевое значение имеет смысл только внутри указанного контура.</p></div>
        <div class="error-counters">
          __ERROR_COUNTERS__
        </div>
        <ul class="issue-list">__ISSUES__</ul>
      </div>
    </section>
    <section class="band" id="server">
      <div class="shell">
        <div class="section-head"><h2>Сервер и ресурсы</h2><p>Состояние serving, provider boundary и бюджетов памяти.</p></div>
        __SERVER_TABLE__
      </div>
    </section>
  </main>
  <footer class="shell footer">Архитектура: __ARCHITECTURE__<br>Обновлено: __GENERATED__</footer>
</body>
</html>
"""

    error_counters = "".join(
        [
            f'<div class="counter {_tone_for_zero_errors(n["actual_false"])}"><span>Фактические CPU-ошибки</span><b>{_format_number(n["actual_false"])}</b></div>',
            f'<div class="counter {_tone_for_zero_errors(n["canary_false"])}"><span>Ошибки проверочного контура</span><b>{_format_number(n["canary_false"])}</b></div>',
            f'<div class="counter {_tone_for_zero_errors(n["shadow_false"])}"><span>Ошибки текущего теневого окна</span><b>{_format_number(n["shadow_false"])}</b></div>',
            f'<div class="counter {_tone_for_zero_errors(n["proof_false"])}"><span>Ошибки накопленного proof-окна</span><b>{_format_number(n["proof_false"])}</b></div>',
            f'<div class="counter warn"><span>Заблокировано проверкой</span><b>{_format_number(n["verifier_blocked_rows"])}</b></div>',
            f'<div class="counter warn"><span>Профили в карантине</span><b>{_format_number(n["quarantined_profiles"])}</b></div>',
        ]
    )

    replacements = {
        "__AGE__": html.escape(model.texts["age"]),
        "__REFRESH_PATH__": safe_path,
        "__VERDICT_TONE__": verdict_tone,
        "__VERDICT_TITLE__": html.escape(verdict_title),
        "__VERDICT_DETAIL__": html.escape(verdict_detail),
        "__HEALTH_BADGE__": _badge("СЕРВЕР РАБОТАЕТ" if f["health_ok"] else "СЕРВЕР НЕДОСТУПЕН", "good" if f["health_ok"] else "bad"),
        "__METRICS__": metrics_html,
        "__TRAFFIC__": traffic_html,
        "__PARSED_ROWS__": _format_number(n["parsed_rows"]),
        "__INCOMING_TOKENS__": _format_number(n["incoming_tokens"]),
        "__ACTUAL_ROWS__": _format_number(n["actual_rows"]),
        "__BOUNDARY_ROWS__": _format_number(n["provider_boundary_rows"]),
        "__BOUNDARY_TOKENS__": _format_number(n["provider_boundary_tokens"]),
        "__CANARY_ROWS__": _format_number(n["canary_rows"]),
        "__TARGET_SHARE__": f"{n['target_share_milli'] / 10:.1f}%",
        "__TOKEN_CHART__": token_chart,
        "__SHARE_CHART__": share_chart,
        "__PROFILE_CHART__": profile_chart,
        "__ERROR_CHART__": error_chart,
        "__MODULE_TABLE__": module_table,
        "__PROFILE_METRICS__": profile_metrics,
        "__PROFILE_TABLE__": profile_table,
        "__CLASS_TABLE__": class_table,
        "__ERROR_COUNTERS__": error_counters,
        "__ISSUES__": _issue_list(model),
        "__SERVER_TABLE__": server_table,
        "__ARCHITECTURE__": html.escape(model.texts["architecture_version"]),
        "__GENERATED__": html.escape(model.generated),
    }
    rendered = html_template
    for placeholder, value in replacements.items():
        rendered = rendered.replace(placeholder, value)
    return rendered
