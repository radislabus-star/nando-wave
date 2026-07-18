from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import sys
import unittest


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
sys.path.insert(0, str(BIN_DIR))

from nando_status_dashboard_view import (  # noqa: E402
    _history_clock,
    _normalized_history,
    build_dashboard_model,
    render_status_dashboard,
)
import nando_status_dashboard as dashboard_entrypoint  # noqa: E402


def fixture() -> tuple[dict, dict, dict, dict, list[dict]]:
    metrics = {
        "source_report_present": True,
        "stable_serving_cpu_clean_suffix_rows": 100,
        "stable_serving_cpu_clean_suffix_total_tokens": 10_000,
        "stable_serving_cpu_clean_suffix_local_accept_events": 0,
        "stable_serving_cpu_clean_suffix_tokens_saved": 0,
        "stable_serving_cpu_clean_suffix_false_accepts": 0,
        "provider_bridge_decision_window_rows": 10,
        "provider_bridge_local_accept_events": 10,
        "provider_bridge_tokens_saved_estimated": 200,
        "provider_bridge_false_accepts": 0,
        "provider_bridge_typed_actor_local_accept_events": 2,
        "provider_bridge_typed_actor_tokens_saved_estimated": 159,
        "provider_bridge_typed_actor_false_accepts": 0,
        "stable_decision_log_clean_suffix_rows": 403,
        "stable_decision_log_clean_suffix_score_candidate_events": 1773,
        "stable_clean_token_compression_false_accepts": 955,
        "operator_class_token_ranking": [],
        "operator_profile_token_ranking": [],
        "quarantined_profile_token_ranking": [],
        "market_money_claim_allowed": False,
    }
    live_tail = {
        "append_parsed_rows": 80,
        "append_total_tokens": 8_000,
        "append_exact_cache_hits": 30,
        "append_non_exact_rows": 50,
        "append_score_candidate_events": 45,
        "product_hot_score_only_unique_cpu_accepts_over_exact_cache": 40,
        "product_hot_score_only_post_quarantine_score_candidate_events": 40,
        "product_hot_score_only_tokens_saved": 4_000,
        "product_hot_score_only_post_quarantine_false_accepts": 0,
        "product_hot_score_only_active_profile_count": 4,
        "product_hot_score_only_quarantined_profile_count": 2,
        "online_bucket_count": 12,
        "active_bucket_count": 6,
        "candidate_bucket_count": 5,
        "shadow_ready_bucket_count": 4,
        "rejected_bucket_count": 1,
        "auto_recovery_promoted_profile_count": 4,
        "auto_recovery_pending_profile_count": 1,
        "local_accept_enabled": False,
        "exact_cache_overlap_excluded": True,
        "runtime_budget": {
            "hot_bytes_estimate": 1024,
            "warm_bytes_estimate": 2048,
            "hot_budget_passed": True,
            "warm_budget_passed": True,
            "product_runtime_budget_passed": True,
        },
    }
    status = {
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "bridge": {
            "health_ok": True,
            "local_accept_enabled": True,
            "safety_policy": "guarded_verified_routes",
            "upstream_base_url_configured": True,
            "upstream_server_api_key_configured": False,
            "client_auth_forwarding_supported": True,
        },
        "summary": {
            "broad_provider_traffic_ready": False,
            "next_action": "shadow_metrics_not_ready",
        },
        "upstream": {
            "observed_live_upstream_success": True,
            "observed_live_success_count": 48,
            "observed_live_latest_status": 200,
            "observed_live_latest_path": "/responses",
        },
        "latency": {
            "bridge_egress_count": 273,
            "bridge_egress_p99_ms": 50_310,
            "cpu_local_accept_count": 478,
            "cpu_local_accept_p99_ms": 133,
        },
        "resources": {
            "provider_bridge_rss_bytes": 32 * 1024 * 1024,
            "live_tail_rss_bytes": 289 * 1024 * 1024,
            "appender_rss_bytes": 4 * 1024 * 1024,
            "serving_rss_bytes": 325 * 1024 * 1024,
        },
        "verify": {"blockers": ["local_accept_promotion_blocked"]},
    }
    appender = {
        "json_rows_seen": 120,
        "rows_written": 80,
        "skipped_unhandled_payload": 20,
        "active_session_files": 3,
    }
    history = [
        {
            "schema_version": "nando_status_dashboard_history_v2",
            "timestamp": f"2026-07-10T01:00:0{index}Z",
            "incoming_rows": 70 + index,
            "incoming_tokens": 7_000 + index * 100,
            "actual_broad_rows": 100,
            "actual_broad_cpu_accepts": 0,
            "actual_broad_saved_tokens": 0,
            "shadow_cpu_candidates": 30 + index,
            "shadow_potential_tokens": 3_000 + index * 100,
            "canary_rows": 10,
            "canary_cpu_accepts": 10,
            "canary_saved_tokens": 200,
            "profiles_mined": 10 + index,
            "profiles_hot": 4,
            "profiles_quarantined": 2,
            "profiles_promoted": 4,
            "actual_broad_false_accepts": 0,
            "shadow_false_accepts": 0,
            "canary_false_accepts": 0,
        }
        for index in range(3)
    ]
    return metrics, live_tail, status, appender, history


class DashboardModelTests(unittest.TestCase):
    def test_history_clock_uses_server_timezone(self) -> None:
        self.assertEqual(_history_clock("2026-07-10T08:31:46Z"), "11:31:46")

    def test_error_history_uses_interval_deltas(self) -> None:
        rows = tuple(
            {
                "schema_version": "nando_status_dashboard_history_v2",
                "timestamp": f"2026-07-10T01:00:0{index}Z",
                "actual_broad_false_accepts": actual,
                "shadow_false_accepts": shadow,
                "canary_false_accepts": canary,
            }
            for index, (actual, shadow, canary) in enumerate(
                [(0, 10, 0), (0, 10, 0), (1, 13, 0)]
            )
        )
        normalized = _normalized_history(rows)
        self.assertEqual([row["shadow_false_new"] for row in normalized], [0, 0, 3])
        self.assertEqual([row["actual_false_new"] for row in normalized], [0, 0, 1])

    def test_actual_and_shadow_savings_are_not_mixed(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        model = build_dashboard_model(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
        )

        self.assertEqual(model.numbers["actual_saved_tokens"], 0)
        self.assertEqual(model.numbers["actual_accepts"], 0)
        self.assertEqual(model.numbers["shadow_tokens"], 4_000)
        self.assertEqual(model.numbers["shadow_candidates"], 40)
        self.assertEqual(model.numbers["typed_actor_accepts"], 2)
        self.assertEqual(model.numbers["typed_actor_saved_tokens"], 159)

    def test_live_transition_metrics_and_profiles_are_visible(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        transition_metrics = {
            "profile_count": 4,
            "active_profile_count": 1,
            "quarantined_profile_count": 2,
            "revoked_profile_count": 1,
            "profiles_promoted": 1,
            "local_accepts": 7,
            "tokens_saved": 700,
            "false_accepts": 0,
            "execution_p99_ns": 80_000,
            "total_bridge_requests": 20,
            "raw_phase_family_count": 3,
            "raw_phase_memorization_families": 1,
            "raw_phase_circuit_families": 1,
            "raw_phase_cleanup_families": 1,
            "raw_phase_max_observed_surfaces": 4,
            "raw_phase_max_discovered_predicates": 669,
            "raw_phase_packages_induced": 4,
        }
        transition_registry = {
            "packages": {
                "package": {
                    "profiles": [
                        {
                            "profile_id": "package:0",
                            "operator_kind": "set_field",
                            "state": "active",
                            "future_clean_rows": 16,
                            "false_accepts": 0,
                        }
                    ]
                }
            }
        }
        model = build_dashboard_model(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
            transition_metrics=transition_metrics,
            transition_registry=transition_registry,
        )
        self.assertEqual(model.numbers["typed_actor_accepts"], 7)
        self.assertEqual(model.numbers["typed_actor_saved_tokens"], 700)
        self.assertEqual(model.numbers["active_profiles"], 7)
        self.assertEqual(model.numbers["raw_phase_families"], 3)
        self.assertEqual(model.numbers["raw_phase_predicates"], 669)
        self.assertEqual(model.numbers["raw_phase_packages"], 4)
        self.assertEqual(model.profile_rows[-1]["operator_kind"], "set_field")

    def test_denominator_scope_error_is_visible(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        live_tail["append_non_exact_rows"] = 20
        live_tail["product_hot_score_only_unique_cpu_accepts_over_exact_cache"] = 40
        model = build_dashboard_model(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
        )

        self.assertTrue(
            any(issue.title == "Смешаны окна знаменателей" for issue in model.issues)
        )

    def test_multi_candidate_counter_is_not_a_critical_error(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        metrics["stable_decision_log_rows"] = 10
        metrics["stable_decision_log_false_accepts"] = 20
        model = build_dashboard_model(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
        )
        issue = next(
            item
            for item in model.issues
            if item.title == "На одну строку проверяется несколько кандидатов"
        )
        self.assertEqual(issue.severity, "info")

    def test_accumulated_proof_errors_are_visible(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        model = build_dashboard_model(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
        )
        issue = next(
            item
            for item in model.issues
            if item.title == "Накопленный proof-контур не прошёл"
        )
        self.assertEqual(issue.severity, "warning")
        self.assertIn("955 ошибок", issue.detail)

    def test_renderer_is_russian_and_complete(self) -> None:
        metrics, live_tail, status, appender, history = fixture()
        rendered = render_status_dashboard(
            metrics=metrics,
            live_tail=live_tail,
            status=status,
            appender=appender,
            history=history,
            request_path="/v2/status/example",
        )

        for expected in (
            "LLM-вызовов предотвращено",
            "Экономия typed actors",
            "Широкая экономия",
            "Обычный трафик на CPU",
            "Потенциал экономии",
            "Эффективность модулей",
            "Создано профилей",
            "Фазовые семьи",
            "Открыто отношений",
            "Компактные packages",
            "Ошибки и блокировки",
            "авторизация клиента передаётся",
            "48 живых 2xx; последний 200 /responses",
            "133 мс",
            "50.3 с",
            "325.0 МиБ",
            "Ошибки накопленного proof-окна",
        ):
            self.assertIn(expected, rendered)
        self.assertNotIn("__TOKEN_CHART__", rendered)
        self.assertNotIn("Live Compression Charts", rendered)
        self.assertEqual(rendered.count("<figure"), 4)

    def test_protected_dashboard_path_contract_is_preserved(self) -> None:
        original = dashboard_entrypoint.STATUS_DASHBOARD_KEY
        try:
            dashboard_entrypoint.STATUS_DASHBOARD_KEY = "secret-key"
            self.assertEqual(
                dashboard_entrypoint.dashboard_key_for_path(
                    "/v2/status/secret-key.html?source=test"
                ),
                "secret-key",
            )
            self.assertTrue(
                dashboard_entrypoint.dashboard_path_authorized(
                    "/v2/status/secret-key?source=test"
                )
            )
            self.assertFalse(
                dashboard_entrypoint.dashboard_path_authorized("/v2/status/wrong")
            )
        finally:
            dashboard_entrypoint.STATUS_DASHBOARD_KEY = original


if __name__ == "__main__":
    unittest.main()
