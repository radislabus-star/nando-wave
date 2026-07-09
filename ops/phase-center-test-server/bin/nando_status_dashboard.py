"""Protected NANDA CPU v2 status dashboard.

This module is observability only. It reads already-produced server reports,
reads a tiny JSONL history produced by the metrics snapshot timer, and renders
static HTML. It must not score requests, proxy provider traffic, compile
profiles, or enable claims.
"""

from __future__ import annotations

from collections import deque
import hmac
import html
import json
import os
import time
import urllib.parse
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


STATUS_DASHBOARD_KEY = os.environ.get("NANDO_STATUS_DASHBOARD_KEY", "")
STATUS_JSON = Path(
    os.environ.get(
        "NANDO_STATUS_REPORT",
        "/var/lib/nando-wave/streaming/metrics/nando-phase-center.status.json",
    )
)
METRICS_JSON = Path(
    os.environ.get(
        "NANDO_METRICS_SNAPSHOT_JSON",
        "/var/lib/nando-wave/streaming/metrics/nando-phase-center.metrics.json",
    )
)
DASHBOARD_HISTORY_JSONL = Path(
    os.environ.get(
        "NANDO_STATUS_DASHBOARD_HISTORY_JSONL",
        "/var/lib/nando-wave/streaming/metrics/nando-phase-center.dashboard-history.jsonl",
    )
)
DASHBOARD_HISTORY_MAX_POINTS = int(
    os.environ.get("NANDO_STATUS_DASHBOARD_HISTORY_MAX_POINTS", "360")
)
DASHBOARD_HISTORY_COMPACT_MAX_BYTES = int(
    os.environ.get("NANDO_STATUS_DASHBOARD_COMPACT_MAX_BYTES", str(4 * 1024 * 1024))
)
CLEAN_PROMOTION_MANIFEST_JSON = Path(
    os.environ.get(
        "NANDO_CLEAN_PROMOTION_MANIFEST_JSON",
        "/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report-clean-promotion-manifest.json",
    )
)
CALL_TOKEN_PROMOTION_ACTIVE_MANIFEST_JSON = Path(
    os.environ.get(
        "NANDO_CALL_TOKEN_PROMOTION_ACTIVE_MANIFEST_JSON",
        "/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report-call-token-promotion-active-manifest.json",
    )
)
PROVIDER_EXPORT_WATCH_REPORT = Path(
    os.environ.get(
        "NANDO_PROVIDER_EXPORT_WATCH_REPORT",
        "/var/lib/nando-wave/streaming/provider-export-watch.report.json",
    )
)


def now_iso() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat()


def read_json_file(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def dashboard_key_for_path(path: str) -> str:
    prefixes = ("/status/", "/v1/status/", "/v2/status/")
    for prefix in prefixes:
        if path.startswith(prefix):
            key = path[len(prefix) :].split("?", 1)[0].split("#", 1)[0]
            return key.removesuffix(".html")
    return ""


def dashboard_lang_for_path(path: str) -> str:
    query = urllib.parse.urlsplit(path).query
    values = urllib.parse.parse_qs(query).get("lang", ["ru"])
    lang = values[0].strip().lower() if values else "ru"
    return "en" if lang in {"en", "eng"} else "ru"


def dashboard_enabled() -> bool:
    return bool(STATUS_DASHBOARD_KEY)


def dashboard_path_authorized(path: str) -> bool:
    if not STATUS_DASHBOARD_KEY:
        return False
    key = dashboard_key_for_path(path)
    return bool(key) and hmac.compare_digest(key, STATUS_DASHBOARD_KEY)


def dashboard_int(value: Any) -> int:
    return value if isinstance(value, int) else 0


def dashboard_float(value: Any) -> float:
    if isinstance(value, int | float):
        return float(value)
    return 0.0


def dashboard_bool(value: Any, lang: str) -> str:
    if lang == "en":
        return "true" if bool(value) else "false"
    return "да" if bool(value) else "нет"


def dashboard_pct(saved: int, total: int) -> str:
    if total <= 0:
        return "0.0%"
    return f"{saved * 100 / total:.1f}%"


def dashboard_metric_text(value: Any, lang: str, *, default: str | None = None) -> str:
    if value is None:
        return default if default is not None else ("unknown" if lang == "en" else "неизвестно")
    if isinstance(value, bool):
        return dashboard_bool(value, lang)
    if isinstance(value, int):
        return f"{value:,}".replace(",", " ")
    if isinstance(value, float):
        return f"{value:.2f}"
    text = str(value)
    return text if text else (default if default is not None else ("unknown" if lang == "en" else "неизвестно"))


def dashboard_age_text(value: Any, lang: str) -> str:
    if not isinstance(value, str) or not value:
        return "unknown" if lang == "en" else "неизвестно"
    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)
    except ValueError:
        return value
    seconds = max(0, int((datetime.now(timezone.utc) - dt).total_seconds()))
    if seconds < 60:
        return f"{seconds}s ago" if lang == "en" else f"{seconds}с назад"
    minutes = seconds // 60
    if minutes < 60:
        return f"{minutes}m ago" if lang == "en" else f"{minutes}м назад"
    hours = minutes // 60
    return f"{hours}h ago" if lang == "en" else f"{hours}ч назад"


def dashboard_metric_first(metrics: dict[str, Any], keys: tuple[str, ...], lang: str, *, default: str | None = None) -> str:
    for key in keys:
        value = metrics.get(key)
        if value is not None:
            return dashboard_metric_text(value, lang, default=default)
    return default if default is not None else ("unknown" if lang == "en" else "неизвестно")


def dashboard_card(title: str, value: str, hint: str = "") -> str:
    return (
        "<section class='card'>"
        f"<h2>{html.escape(title)}</h2>"
        f"<strong>{html.escape(value)}</strong>"
        f"<p>{html.escape(hint)}</p>"
        "</section>"
    )


def class_tokens(rows: list[dict[str, Any]], class_name: str) -> int:
    return sum(
        dashboard_int(row.get("tokens_saved"))
        for row in rows
        if str(row.get("class") or "").startswith(class_name)
    )


def read_dashboard_history(max_points: int) -> list[dict[str, Any]]:
    rows: deque[dict[str, Any]] = deque(maxlen=max(1, max_points))
    try:
        with DASHBOARD_HISTORY_JSONL.open("r", encoding="utf-8") as handle:
            for line in handle:
                try:
                    value = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(value, dict):
                    rows.append(value)
    except OSError:
        pass
    return list(rows)


def normalized_rows(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    return [row for row in value if isinstance(row, dict)]


def line_points(
    rows: list[dict[str, Any]],
    field: str,
    *,
    width: int,
    height: int,
    left: int,
    right: int,
    top: int,
    bottom: int,
    max_value: float,
) -> str:
    chart_w = width - left - right
    chart_h = height - top - bottom
    count = len(rows)
    points: list[str] = []
    for index, row in enumerate(rows):
        value = max(0.0, min(max_value, dashboard_float(row.get(field))))
        x = left + (chart_w * index / max(1, count - 1))
        y = top + chart_h - (chart_h * value / max_value)
        points.append(f"{x:.1f},{y:.1f}")
    return " ".join(points)


def dashboard_history_chart(rows: list[dict[str, Any]], lang: str) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    title = t("Графики живого сжатия", "Live Compression Charts")
    if len(rows) < 2:
        return (
            f"<section class='panel chart-panel'><h2>{html.escape(title)}</h2>"
            f"<p>{html.escape(t('Пока мало точек. Открой страницу на минуту, и линия появится.', 'Not enough points yet. Keep the page open for a minute and the line will appear.'))}</p>"
            "</section>"
        )

    width = 920
    height = 230
    left = 46
    right = 14
    top = 18
    bottom = 34
    chart_h = height - top - bottom
    clean_rows = rows[-DASHBOARD_HISTORY_MAX_POINTS:]
    latest = clean_rows[-1]
    latest_pct = dashboard_float(latest.get("clean_compression_pct"))
    saved = dashboard_int(latest.get("clean_saved_tokens"))
    total = dashboard_int(latest.get("clean_total_tokens"))
    accepts = dashboard_int(latest.get("clean_cpu_accepts"))
    active_false_accepts = dashboard_int(latest.get("active_false_accepts"))
    shadow_false_accepts = dashboard_int(
        latest.get("shadow_false_accepts", latest.get("clean_false_accepts"))
    )
    first_label = html.escape(str(clean_rows[0].get("timestamp") or "")[11:19])
    last_label = html.escape(str(clean_rows[-1].get("timestamp") or "")[11:19])
    grid = []
    for pct in (0, 25, 50, 75, 100):
        y = top + chart_h - (chart_h * pct / 100)
        grid.append(
            f"<line x1='{left}' y1='{y:.1f}' x2='{width - right}' y2='{y:.1f}' class='chart-grid'/>"
            f"<text x='8' y='{y + 4:.1f}' class='chart-label'>{pct}%</text>"
        )
    caption = t(
        f"Последние {len(clean_rows)} точек, сейчас {latest_pct:.1f}% · {saved} / {total} токенов, {accepts} accepts, active false {active_false_accepts}, shadow risk {shadow_false_accepts}",
        f"Last {len(clean_rows)} points, now {latest_pct:.1f}% · {saved} / {total} tokens, {accepts} accepts, active false {active_false_accepts}, shadow risk {shadow_false_accepts}",
    )
    compression_line = line_points(
        clean_rows,
        "clean_compression_pct",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=100.0,
    )
    max_accepts = max(1.0, max(dashboard_float(row.get("clean_cpu_accepts")) for row in clean_rows))
    accepts_line = line_points(
        clean_rows,
        "clean_cpu_accepts",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=max_accepts,
    )
    false_max = max(
        1.0,
        max(dashboard_float(row.get("active_false_accepts")) for row in clean_rows),
    )
    false_line = line_points(
        clean_rows,
        "active_false_accepts",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=false_max,
    )
    return f"""
  <section class="panel chart-panel">
    <h2>{html.escape(title)}</h2>
    <p>{html.escape(caption)}</p>
    <div class="legend">
      <span><i class="legend-green"></i>{html.escape(t('сжатие токенов', 'token compression'))}</span>
      <span><i class="legend-blue"></i>{html.escape(t('CPU accepts', 'CPU accepts'))}</span>
      <span><i class="legend-red"></i>{html.escape(t('active false accepts', 'active false accepts'))}</span>
    </div>
    <svg class="chart-svg" viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(title)}">
      {''.join(grid)}
      <polyline class="chart-line chart-green" points="{compression_line}"/>
      <polyline class="chart-line chart-blue" points="{accepts_line}"/>
      <polyline class="chart-line chart-red" points="{false_line}"/>
      <text x="{left}" y="{height - 8}" class="chart-label">{first_label}</text>
      <text x="{width - right - 48}" y="{height - 8}" class="chart-label">{last_label}</text>
    </svg>
  </section>"""


def dashboard_pool_chart(rows: list[dict[str, Any]], lang: str) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    title = t("Динамика майнера: quarantine / exportable / final_hot", "Miner Pool: quarantine / exportable / final_hot")
    clean_rows = rows[-DASHBOARD_HISTORY_MAX_POINTS:]
    if len(clean_rows) < 2:
        return ""
    width = 920
    height = 230
    left = 70
    right = 14
    top = 18
    bottom = 34
    max_tokens = max(
        1.0,
        max(
            dashboard_float(row.get("quarantined_tokens"))
            + dashboard_float(row.get("exportable_tokens"))
            + dashboard_float(row.get("final_hot_tokens"))
            for row in clean_rows
        ),
    )
    quarantine_line = line_points(
        clean_rows,
        "quarantined_tokens",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=max_tokens,
    )
    exportable_line = line_points(
        clean_rows,
        "exportable_tokens",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=max_tokens,
    )
    final_hot_line = line_points(
        clean_rows,
        "final_hot_tokens",
        width=width,
        height=height,
        left=left,
        right=right,
        top=top,
        bottom=bottom,
        max_value=max_tokens,
    )
    latest = clean_rows[-1]
    first_label = html.escape(str(clean_rows[0].get("timestamp") or "")[11:19])
    last_label = html.escape(str(clean_rows[-1].get("timestamp") or "")[11:19])
    q_tokens = dashboard_int(latest.get("quarantined_tokens"))
    e_tokens = dashboard_int(latest.get("exportable_tokens"))
    f_tokens = dashboard_int(latest.get("final_hot_tokens"))
    caption = t(
        f"quarantine {q_tokens}, exportable {e_tokens}, final_hot {f_tokens}",
        f"quarantine {q_tokens}, exportable {e_tokens}, final_hot {f_tokens}",
    )
    grid = []
    for ratio in (0.0, 0.25, 0.5, 0.75, 1.0):
        y = top + (height - top - bottom) - ((height - top - bottom) * ratio)
        label = int(max_tokens * ratio)
        grid.append(
            f"<line x1='{left}' y1='{y:.1f}' x2='{width - right}' y2='{y:.1f}' class='chart-grid'/>"
            f"<text x='8' y='{y + 4:.1f}' class='chart-label'>{label}</text>"
        )
    return f"""
  <section class="panel chart-panel">
    <h2>{html.escape(title)}</h2>
    <p>{html.escape(caption)}</p>
    <div class="legend">
      <span><i class="legend-yellow"></i>quarantine</span>
      <span><i class="legend-blue"></i>exportable</span>
      <span><i class="legend-green"></i>final_hot</span>
    </div>
    <svg class="chart-svg" viewBox="0 0 {width} {height}" role="img" aria-label="{html.escape(title)}">
      {''.join(grid)}
      <polyline class="chart-line chart-yellow" points="{quarantine_line}"/>
      <polyline class="chart-line chart-blue" points="{exportable_line}"/>
      <polyline class="chart-line chart-green" points="{final_hot_line}"/>
      <text x="{left}" y="{height - 8}" class="chart-label">{first_label}</text>
      <text x="{width - right - 48}" y="{height - 8}" class="chart-label">{last_label}</text>
    </svg>
  </section>"""


def dashboard_table(rows: list[dict[str, Any]], title: str, lang: str) -> str:
    body = []
    for row in rows[:8]:
        body.append(
            "<tr>"
            f"<td>{html.escape(str(row.get('class') or row.get('kind') or row.get('profile_id') or ('unknown' if lang == 'en' else 'неизвестно')))}</td>"
            f"<td>{dashboard_int(row.get('unique_cpu_accepts_over_exact_cache'))}</td>"
            f"<td>{dashboard_int(row.get('tokens_saved'))}</td>"
            f"<td>{dashboard_int(row.get('false_accepts'))}</td>"
            "</tr>"
        )
    if not body:
        body.append(
            "<tr><td colspan='4'>no rows yet</td></tr>"
            if lang == "en"
            else "<tr><td colspan='4'>пока нет строк</td></tr>"
        )
    class_header = "class/profile" if lang == "en" else "класс/профиль"
    token_header = "tokens" if lang == "en" else "токены"
    return (
        f"<section class='panel'><h2>{html.escape(title)}</h2>"
        f"<table><thead><tr><th>{class_header}</th><th>accepts</th><th>{token_header}</th><th>false</th></tr></thead>"
        f"<tbody>{''.join(body)}</tbody></table></section>"
    )


def promotion_manifest_summary(manifest: dict[str, Any]) -> dict[str, Any]:
    return {
        "allowed": bool(manifest.get("allowed")),
        "blocker": str(manifest.get("blocker") or "none"),
        "promoted": dashboard_int(manifest.get("promoted_candidate_count")),
        "quarantined": dashboard_int(manifest.get("quarantined_candidate_count")),
        "tokens": dashboard_int(manifest.get("tokens_saved")),
        "accepts": dashboard_int(manifest.get("unique_cpu_accepts_over_exact_cache")),
        "false_accepts": dashboard_int(manifest.get("false_accepts")),
        "parity": dashboard_int(manifest.get("runtime_parity_mismatches")),
        "disabled": bool(manifest.get("live_score_only_disabled")),
        "disable_reason": str(manifest.get("live_score_only_disable_reason") or ""),
    }


def promotion_blocker_panel(lang: str) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    clean = read_json_file(CLEAN_PROMOTION_MANIFEST_JSON)
    call_token = read_json_file(CALL_TOKEN_PROMOTION_ACTIVE_MANIFEST_JSON)
    manifests = [
        (t("clean promotion", "clean promotion"), promotion_manifest_summary(clean)),
        (t("call-token active", "call-token active"), promotion_manifest_summary(call_token)),
    ]
    rows = []
    for name, item in manifests:
        state = "ok" if item["allowed"] else "watch"
        if item["disabled"]:
            state = "bad"
        blocker = item["disable_reason"] or item["blocker"]
        rows.append(
            "<tr>"
            f"<td>{html.escape(name)}</td>"
            f"<td><b class='{state}'>{dashboard_bool(item['allowed'], lang)}</b></td>"
            f"<td>{item['promoted']}</td>"
            f"<td>{item['quarantined']}</td>"
            f"<td>{item['accepts']}</td>"
            f"<td>{item['tokens']}</td>"
            f"<td>{item['false_accepts']}</td>"
            f"<td>{item['parity']}</td>"
            f"<td><code>{html.escape(blocker)}</code></td>"
            "</tr>"
        )
    return (
        f"<section class='panel'><h2>{html.escape(t('Promotion blockers', 'Promotion Blockers'))}</h2>"
        "<table><thead><tr>"
        f"<th>{html.escape(t('manifest', 'manifest'))}</th>"
        f"<th>{html.escape(t('allowed', 'allowed'))}</th>"
        f"<th>{html.escape(t('promoted', 'promoted'))}</th>"
        f"<th>{html.escape(t('quarantine', 'quarantine'))}</th>"
        "<th>accepts</th><th>tokens</th><th>false</th><th>parity</th>"
        f"<th>{html.escape(t('blocker', 'blocker'))}</th>"
        "</tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></section>"
    )


def clamp_score(score: int) -> int:
    return max(0, min(10, score))


def score_badge(score: int) -> str:
    if score >= 10:
        return "ok"
    if score >= 8:
        return "watch"
    return "bad"


def architecture_knee_scores(
    status: dict[str, Any],
    metrics: dict[str, Any],
    provider_watch: dict[str, Any],
    lang: str,
) -> list[dict[str, Any]]:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    bridge = status.get("bridge") if isinstance(status.get("bridge"), dict) else {}
    verify = status.get("verify") if isinstance(status.get("verify"), dict) else {}
    evidence = status.get("provider_evidence") if isinstance(status.get("provider_evidence"), dict) else {}
    class_rows = normalized_rows(metrics.get("operator_class_token_ranking"))
    clean_saved = dashboard_int(metrics.get("stable_clean_token_compression_saved_tokens"))
    clean_total = dashboard_int(metrics.get("stable_clean_token_compression_total_tokens"))
    clean_pct = clean_saved * 100 / clean_total if clean_total > 0 else 0.0
    clean_accepts = dashboard_int(
        metrics.get("stable_clean_token_compression_unique_cpu_accepts_over_exact_cache")
    )
    active_false = dashboard_int(metrics.get("product_hot_score_only_post_quarantine_false_accepts"))
    shadow_false = dashboard_int(metrics.get("stable_clean_token_compression_false_accepts"))
    clean_manifest_safe = bool(metrics.get("clean_promotion_manifest_safe")) or (
        bool(metrics.get("final_hot_runtime_available"))
        and bool(metrics.get("product_hot_score_only_runtime_active"))
        and active_false == 0
        and dashboard_int(metrics.get("product_hot_score_only_tokens_saved")) > 0
    )
    gateway_accepts = dashboard_int(metrics.get("gateway_local_accept_events"))
    provider_accepts = dashboard_int(metrics.get("provider_bridge_v2_local_accept_events"))
    stable_rows = dashboard_int(metrics.get("stable_decision_log_rows"))
    stable_candidates = dashboard_int(metrics.get("stable_decision_log_score_candidate_events"))
    append_rows = dashboard_int(metrics.get("append_parsed_rows"))
    append_candidates = dashboard_int(metrics.get("append_score_candidate_events"))
    hidden_quarantine = class_tokens(class_rows, "hidden_state:quarantined")
    hidden_exportable = class_tokens(class_rows, "hidden_state:exportable")
    hidden_final = class_tokens(class_rows, "hidden_state:final_hot")
    observable_quarantine = class_tokens(class_rows, "observable_subcenter:quarantined")
    observable_exportable = class_tokens(class_rows, "observable_subcenter:exportable")
    observable_final = class_tokens(class_rows, "observable_subcenter:final_hot")
    quarantine_tokens = hidden_quarantine + observable_quarantine
    exportable_tokens = hidden_exportable + observable_exportable + class_tokens(class_rows, "observable_primary:exportable")
    final_hot_tokens = hidden_final + observable_final
    safe_pool_tokens = exportable_tokens + final_hot_tokens
    provider_evidence_ready = bool(evidence.get("provider_billing_evidence_present"))
    upstream_configured = bool(bridge.get("upstream_configured"))
    provider_watch_healthy = (
        str(provider_watch.get("report_kind") or "")
        == "phase_stream_online_miner_portfolio_provider_export_watch_v1"
        and dashboard_int(provider_watch.get("cycles_completed")) > 0
        and bool(provider_watch.get("serving_runtime_changed")) is False
        and bool(provider_watch.get("local_accept_enabled")) is False
    )
    server_dashboard_healthy = bool(bridge.get("health_ok")) and dashboard_enabled()

    rows: list[dict[str, Any]] = []

    def add(index: int, ru_name: str, en_name: str, score: int, ru_evidence: str, en_evidence: str, ru_next: str, en_next: str) -> None:
        rows.append(
            {
                "index": index,
                "name": t(ru_name, en_name),
                "score": clamp_score(score),
                "evidence": t(ru_evidence, en_evidence),
                "next": t(ru_next, en_next),
            }
        )

    add(
        1,
        "Event Sources",
        "Event Sources",
        6 + int(gateway_accepts > 0) + int(provider_accepts > 0) + int(stable_rows > 0) + int(append_rows > 0),
        f"gateway={gateway_accepts}, provider_v2={provider_accepts}, stable_rows={stable_rows}, append_rows={append_rows}",
        f"gateway={gateway_accepts}, provider_v2={provider_accepts}, stable_rows={stable_rows}, append_rows={append_rows}",
        "market gate: upstream для широкого provider потока",
        "market gate: upstream for broad provider traffic",
    )
    add(
        2,
        "L1 Surface Capture",
        "L1 Surface Capture",
        6 + int(stable_rows >= 1000) + int(stable_candidates > 0) + int(append_rows > 0) + int(append_candidates > 0),
        f"stable_rows={stable_rows}, stable_candidates={stable_candidates}, append_rows={append_rows}",
        f"stable_rows={stable_rows}, stable_candidates={stable_candidates}, append_rows={append_rows}",
        "до 10: больше verifier-rich событий из реального agent loop",
        "to 10: more verifier-rich real agent-loop events",
    )
    add(
        3,
        "L2 Hidden State Packer",
        "L2 Hidden State Packer",
        6 + int(hidden_exportable > 0) + int(hidden_final > 0) + int(clean_manifest_safe) + int(active_false == 0),
        f"hidden quarantine={hidden_quarantine}, exportable={hidden_exportable}, final_hot={hidden_final}",
        f"hidden quarantine={hidden_quarantine}, exportable={hidden_exportable}, final_hot={hidden_final}",
        "до 10: держать clean manifest safe и 0 false при росте hidden backlog",
        "to 10: keep clean manifest safe and 0 false while hidden backlog grows",
    )
    add(
        4,
        "Online Miner",
        "Online Miner",
        5 + int(clean_pct >= 50) + int(clean_pct >= 70) + int(clean_accepts >= 50) + int(active_false == 0) + int(bool(metrics.get("miner_saturation_control_enabled"))),
        f"compression={clean_pct:.1f}%, accepts={clean_accepts}, active_false={active_false}, shadow_risk={shadow_false}",
        f"compression={clean_pct:.1f}%, accepts={clean_accepts}, active_false={active_false}, shadow_risk={shadow_false}",
        "до 10: удерживать >70% и 0 false на длинном окне",
        "to 10: keep >70% and 0 false on a longer window",
    )
    add(
        5,
        "Subcenter Split",
        "Subcenter Split",
        6 + int(observable_exportable > 0) + int(clean_manifest_safe) + int(active_false == 0) + int(clean_pct >= 70),
        f"quarantine={quarantine_tokens}, exportable={exportable_tokens}, final_hot={final_hot_tokens}",
        f"quarantine={quarantine_tokens}, exportable={exportable_tokens}, final_hot={final_hot_tokens}",
        "до 10: clean manifest safe, 0 false и >70% token compression",
        "to 10: clean manifest safe, 0 false, and >70% token compression",
    )
    add(
        6,
        "Candidate Lifecycle",
        "Candidate Lifecycle",
        6 + int(exportable_tokens > 0) + int(final_hot_tokens > 0) + int(bool(metrics.get("final_hot_runtime_available"))) + int(active_false == 0),
        f"exportable={exportable_tokens}, final_hot={final_hot_tokens}, runtime={dashboard_bool(metrics.get('final_hot_runtime_available'), lang)}",
        f"exportable={exportable_tokens}, final_hot={final_hot_tokens}, runtime={dashboard_bool(metrics.get('final_hot_runtime_available'), lang)}",
        "до 10: автоматический quarantine->exportable->final_hot promotion gate",
        "to 10: automatic quarantine->exportable->final_hot promotion gate",
    )
    add(
        7,
        "Shadow / Promotion Gate",
        "Shadow / Promotion Gate",
        7 + int(bool(verify.get("compression_claim_allowed"))) + int(shadow_false == 0) + int(active_false == 0),
        f"compression_claim={dashboard_bool(verify.get('compression_claim_allowed'), lang)}, active_false={active_false}, shadow_risk={shadow_false}",
        f"compression_claim={dashboard_bool(verify.get('compression_claim_allowed'), lang)}, active_false={active_false}, shadow_risk={shadow_false}",
        "до 10: не ослаблять gate; держать 0 false на future split",
        "to 10: do not weaken gate; keep 0 false on future split",
    )
    add(
        8,
        ".nwpc Package",
        ".nwpc Package",
        7 + int(bool(metrics.get("final_hot_runtime_available"))) + int(bool(metrics.get("product_hot_score_only_runtime_active"))) + int(not bool(metrics.get("product_runtime_changed"))),
        f"final_hot_runtime={dashboard_bool(metrics.get('final_hot_runtime_available'), lang)}, product_runtime_changed={dashboard_bool(metrics.get('product_runtime_changed'), lang)}",
        f"final_hot_runtime={dashboard_bool(metrics.get('final_hot_runtime_available'), lang)}, product_runtime_changed={dashboard_bool(metrics.get('product_runtime_changed'), lang)}",
        "до 10: package/runtime parity отчёт на каждом deploy",
        "to 10: package/runtime parity report on every deploy",
    )
    add(
        9,
        "Hot Runtime",
        "Hot Runtime",
        6 + int(bool(bridge.get("local_accept_enabled"))) + int(bool(metrics.get("product_hot_score_only_runtime_active"))) + int(gateway_accepts > 0) + int(active_false == 0),
        f"local_accept={dashboard_bool(bridge.get('local_accept_enabled'), lang)}, gateway_accepts={gateway_accepts}",
        f"local_accept={dashboard_bool(bridge.get('local_accept_enabled'), lang)}, gateway_accepts={gateway_accepts}",
        "до 10: p99/RSS hot-runtime graph отдельно от HTTP bridge",
        "to 10: p99/RSS hot-runtime graph separated from HTTP bridge",
    )
    add(
        10,
        "Server / Dashboard",
        "Server / Dashboard",
        7 + int(bool(verify.get("compression_claim_allowed"))) + int(server_dashboard_healthy) + int(provider_watch_healthy),
        f"token_claim={dashboard_bool(verify.get('compression_claim_allowed'), lang)}, server={dashboard_bool(server_dashboard_healthy, lang)}, export_watch={dashboard_bool(provider_watch_healthy, lang)}, market_upstream={dashboard_bool(upstream_configured, lang)}, money_evidence={dashboard_bool(provider_evidence_ready, lang)}",
        f"token_claim={dashboard_bool(verify.get('compression_claim_allowed'), lang)}, server={dashboard_bool(server_dashboard_healthy, lang)}, export_watch={dashboard_bool(provider_watch_healthy, lang)}, market_upstream={dashboard_bool(upstream_configured, lang)}, money_evidence={dashboard_bool(provider_evidence_ready, lang)}",
        "market gate: upstream + provider billing/export evidence",
        "market gate: upstream + provider billing/export evidence",
    )
    return rows


def architecture_scorecard_panel(
    status: dict[str, Any],
    metrics: dict[str, Any],
    provider_watch: dict[str, Any],
    lang: str,
) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    rows = []
    scores = architecture_knee_scores(status, metrics, provider_watch, lang)
    for row in scores:
        score = dashboard_int(row.get("score"))
        rows.append(
            "<tr>"
            f"<td>{dashboard_int(row.get('index'))}</td>"
            f"<td>{html.escape(str(row.get('name') or ''))}</td>"
            f"<td><b class='{score_badge(score)}'>{score}/10</b></td>"
            f"<td>{html.escape(str(row.get('evidence') or ''))}</td>"
            f"<td>{html.escape(str(row.get('next') or ''))}</td>"
            "</tr>"
        )
    avg = sum(dashboard_int(row.get("score")) for row in scores) / max(1, len(scores))
    return (
        f"<section class='panel scorecard-panel'><h2>{html.escape(t('Оценка 10 колен майнера', '10-Knee Miner Scorecard'))}</h2>"
        f"<p>{html.escape(t('Средняя оценка', 'Average score'))}: <b>{avg:.1f}/10</b></p>"
        "<table><thead><tr>"
        f"<th>#</th><th>{html.escape(t('колено', 'knee'))}</th><th>{html.escape(t('эффективность', 'efficiency'))}</th><th>{html.escape(t('доказательство', 'evidence'))}</th><th>{html.escape(t('до 10', 'to 10'))}</th>"
        "</tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></section>"
    )


def split_panel(metrics: dict[str, Any], lang: str) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    class_rows = normalized_rows(metrics.get("operator_class_token_ranking"))
    quarantined = class_tokens(class_rows, "hidden_state:quarantined") + class_tokens(
        class_rows, "observable_subcenter:quarantined"
    )
    exportable = (
        class_tokens(class_rows, "hidden_state:exportable")
        + class_tokens(class_rows, "observable_subcenter:exportable")
        + class_tokens(class_rows, "observable_primary:exportable")
    )
    final_hot = class_tokens(class_rows, "hidden_state:final_hot") + class_tokens(
        class_rows, "observable_subcenter:final_hot"
    )
    total = max(1, quarantined + exportable + final_hot)
    q_pct = quarantined * 100 / total
    e_pct = exportable * 100 / total
    f_pct = final_hot * 100 / total
    return f"""
  <section class="panel split-panel">
    <h2>{html.escape(t('Разделение майнера', 'Miner Split'))}</h2>
    <div class="split-bar" aria-label="miner split">
      <span class="split-q" style="width:{q_pct:.2f}%"></span>
      <span class="split-e" style="width:{e_pct:.2f}%"></span>
      <span class="split-f" style="width:{f_pct:.2f}%"></span>
    </div>
    <p><b>{quarantined}</b> {html.escape(t('токенов в quarantine', 'tokens in quarantine'))}</p>
    <p><b>{exportable}</b> {html.escape(t('токенов exportable', 'exportable tokens'))}</p>
    <p><b>{final_hot}</b> {html.escape(t('токенов final_hot', 'final_hot tokens'))}</p>
  </section>"""


def dashboard_metric_panel(title: str, rows: list[tuple[str, str, str]], lang: str) -> str:
    if not rows:
        rows = [("status", "unknown" if lang == "en" else "неизвестно", "")]
    body = []
    for label, value, hint in rows:
        css_class = "metric metric-wide" if label == "selector_mode" else "metric"
        body.append(
            f"<div class='{css_class}'>"
            f"<span>{html.escape(label)}</span>"
            f"<b>{html.escape(value)}</b>"
            f"<small>{html.escape(hint)}</small>"
            "</div>"
        )
    return (
        f"<section class='panel metric-panel'><h2>{html.escape(title)}</h2>"
        f"<div class='metric-grid'>{''.join(body)}</div></section>"
    )


def traffic_frame_panel(
    status: dict[str, Any],
    metrics: dict[str, Any],
    active_manifest: dict[str, Any],
    lang: str,
) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    bridge = status.get("bridge") if isinstance(status.get("bridge"), dict) else {}
    stable_rows = dashboard_int(metrics.get("stable_decision_log_rows"))
    stable_tokens = dashboard_int(metrics.get("stable_decision_log_total_tokens"))
    stable_saved = dashboard_int(metrics.get("stable_decision_log_tokens_saved"))
    stable_accepts = dashboard_int(metrics.get("stable_decision_log_unique_cpu_accepts_over_exact_cache"))
    stable_false = dashboard_int(metrics.get("stable_decision_log_false_accepts"))
    clean_rows = dashboard_int(metrics.get("stable_decision_log_clean_suffix_rows"))
    clean_tokens = dashboard_int(metrics.get("stable_clean_token_compression_total_tokens"))
    clean_saved = dashboard_int(metrics.get("stable_clean_token_compression_saved_tokens"))
    clean_accepts = dashboard_int(metrics.get("stable_clean_token_compression_unique_cpu_accepts_over_exact_cache"))
    clean_false = dashboard_int(metrics.get("stable_clean_token_compression_false_accepts"))
    score_candidates = dashboard_int(metrics.get("stable_decision_log_score_candidate_events"))
    clean_score_candidates = dashboard_int(metrics.get("stable_decision_log_clean_suffix_score_candidate_events"))
    append_rows = dashboard_int(metrics.get("append_parsed_rows"))
    append_candidates = dashboard_int(metrics.get("append_score_candidate_events"))
    gateway_rows = dashboard_int(metrics.get("gateway_decision_window_rows"))
    provider_rows = dashboard_int(metrics.get("provider_bridge_decision_window_rows"))
    boundary_rows = dashboard_int(metrics.get("provider_bridge_boundary_window_rows"))
    future_rows = dashboard_int(metrics.get("future_shadow_billing_request_rows"))
    future_tokens = dashboard_int(metrics.get("future_shadow_billing_request_tokens"))

    clean_token_coverage = dashboard_pct(clean_tokens, stable_tokens)
    clean_row_coverage = dashboard_pct(clean_rows, stable_rows)
    capture_state = (
        t("полный provider frame подключён", "full provider frame joined")
        if bool(bridge.get("upstream_configured")) and boundary_rows > 0
        else t("частичный Nando-frame: gateway/provider bridge/appender", "partial Nando frame: gateway/provider bridge/appender")
    )
    traffic_rows = [
        (
            t("весь Nando-frame", "full Nando frame"),
            f"{stable_rows} rows",
            f"{stable_tokens} tokens",
            f"{stable_accepts} accepts / {stable_saved} saved / shadow false {stable_false}",
        ),
        (
            t("текущий clean suffix", "current clean suffix"),
            f"{clean_rows} rows",
            f"{clean_tokens} tokens",
            f"{clean_accepts} accepts / {clean_saved} saved / shadow risk {clean_false} / {clean_token_coverage} tokens of frame",
        ),
        (
            t("кандидаты майнера", "miner candidates"),
            f"{score_candidates} events",
            f"{clean_score_candidates} clean",
            f"append {append_rows} rows / {append_candidates} candidates",
        ),
        (
            t("входные источники", "ingress sources"),
            f"gateway {gateway_rows}",
            f"provider {provider_rows}",
            f"boundary {boundary_rows} / future {future_rows} rows, {future_tokens} tokens",
        ),
        (
            t("накопленный .nwpc", "accumulated .nwpc"),
            f"{active_manifest['promoted']} profiles",
            f"{active_manifest['tokens']} tokens",
            f"{active_manifest['accepts']} accepts / false {active_manifest['false_accepts']}",
        ),
    ]
    body = "".join(
        "<tr>"
        f"<td>{html.escape(name)}</td>"
        f"<td>{html.escape(left)}</td>"
        f"<td>{html.escape(mid)}</td>"
        f"<td>{html.escape(right)}</td>"
        "</tr>"
        for name, left, mid, right in traffic_rows
    )
    return (
        f"<section class='panel wide-panel'><h2>{html.escape(t('Карта покрытия трафика', 'Traffic Coverage Map'))}</h2>"
        f"<p>{html.escape(t('Здесь видно, какую часть потока Nando реально видит, а какую часть текущий clean-window использует для безопасного сжатия.', 'This shows what traffic Nando sees, and what part the current clean window uses for safe compression.'))}</p>"
        f"<p>{html.escape(t('статус захвата', 'capture state'))}: <b>{html.escape(capture_state)}</b> · clean rows: <b>{clean_row_coverage}</b> {html.escape(t('от frame', 'of frame'))}</p>"
        "<table><thead><tr>"
        f"<th>{html.escape(t('слой', 'layer'))}</th>"
        f"<th>{html.escape(t('события', 'events'))}</th>"
        "<th>tokens</th>"
        f"<th>{html.escape(t('смысл', 'meaning'))}</th>"
        "</tr></thead>"
        f"<tbody>{body}</tbody></table></section>"
    )


def decision_pipeline_panel(
    status: dict[str, Any],
    metrics: dict[str, Any],
    active_manifest: dict[str, Any],
    lang: str,
) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    bridge = status.get("bridge") if isinstance(status.get("bridge"), dict) else {}
    gateway_rows = dashboard_int(metrics.get("gateway_decision_window_rows"))
    provider_rows = dashboard_int(metrics.get("provider_bridge_decision_window_rows"))
    boundary_rows = dashboard_int(metrics.get("provider_bridge_boundary_window_rows"))
    append_rows = dashboard_int(metrics.get("append_parsed_rows"))
    append_candidates = dashboard_int(metrics.get("append_score_candidate_events"))
    stable_rows = dashboard_int(metrics.get("stable_decision_log_rows"))
    stable_candidates = dashboard_int(metrics.get("stable_decision_log_score_candidate_events"))
    hot_profiles = dashboard_int(
        metrics.get("product_hot_score_only_active_profile_count")
        or metrics.get("final_hot_profile_count")
    )
    hot_candidates = dashboard_int(metrics.get("product_hot_score_only_post_quarantine_score_candidate_events"))
    hot_accepts = dashboard_int(metrics.get("product_hot_score_only_unique_cpu_accepts_over_exact_cache"))
    hot_tokens = dashboard_int(metrics.get("product_hot_score_only_tokens_saved"))
    hot_false = dashboard_int(metrics.get("product_hot_score_only_post_quarantine_false_accepts"))
    clean_rows = dashboard_int(metrics.get("stable_decision_log_clean_suffix_rows"))
    clean_accepts = dashboard_int(
        metrics.get("stable_clean_token_compression_unique_cpu_accepts_over_exact_cache")
    )
    clean_saved = dashboard_int(metrics.get("stable_clean_token_compression_saved_tokens"))
    clean_total = dashboard_int(metrics.get("stable_clean_token_compression_total_tokens"))
    clean_false = dashboard_int(metrics.get("stable_clean_token_compression_false_accepts"))
    recovery_events = dashboard_int(metrics.get("quarantine_recovery_discovery_events"))
    recovery_tokens = dashboard_int(metrics.get("quarantine_recovery_discovery_tokens"))
    recovery_observe = dashboard_int(metrics.get("quarantine_recovery_auto_subcenter_observe_events"))
    gateway_accepts = dashboard_int(metrics.get("gateway_local_accept_events"))
    provider_accepts = dashboard_int(metrics.get("provider_bridge_v2_local_accept_events"))
    blocker = str(metrics.get("product_hot_compression_claim_blocker") or "none")

    def stage_class(kind: str) -> str:
        return {
            "ok": "stage-ok",
            "watch": "stage-watch",
            "bad": "stage-bad",
        }.get(kind, "stage-watch")

    steps = [
        (
            "ok" if gateway_rows + provider_rows + append_rows + stable_rows > 0 else "watch",
            t("1. Вход", "1. Ingress"),
            f"gateway={gateway_rows}, provider={provider_rows}, boundary={boundary_rows}",
            f"append={append_rows}, stable={stable_rows}",
        ),
        (
            "ok" if append_candidates > 0 and stable_candidates > 0 else "watch",
            t("2. Atoms → candidates", "2. Atoms -> candidates"),
            f"append candidates={append_candidates}",
            f"stable candidates={stable_candidates}",
        ),
        (
            "bad" if hot_false > 0 else ("ok" if hot_candidates > 0 and hot_profiles > 0 else "watch"),
            t("3. Hot score", "3. Hot score"),
            f"profiles={hot_profiles}, score events={hot_candidates}",
            f"accepts={hot_accepts}, tokens={hot_tokens}, false={hot_false}",
        ),
        (
            "ok" if recovery_events > 0 else "watch",
            t("4. Recovery split", "4. Recovery split"),
            f"events={recovery_events}, tokens={recovery_tokens}",
            f"subcenter observes={recovery_observe}",
        ),
        (
            "bad" if clean_false > 0 else ("ok" if clean_accepts > 0 else "watch"),
            t("5. Clean safety window", "5. Clean safety window"),
            f"rows={clean_rows}, accepts={clean_accepts}",
            f"tokens={clean_saved}/{clean_total}, shadow false={clean_false}",
        ),
        (
            "bad" if hot_false > 0 else ("ok" if gateway_accepts + provider_accepts > 0 else "watch"),
            t("6. Решение", "6. Decision"),
            f"gateway accepts={gateway_accepts}, provider v2 accepts={provider_accepts}",
            f".nwpc accepts={active_manifest['accepts']}, blocker={blocker}, local_accept={dashboard_bool(bridge.get('local_accept_enabled'), lang)}",
        ),
    ]
    cards = "".join(
        "<div class='pipeline-step {css}'>"
        "<h3>{title}</h3>"
        "<b>{main}</b>"
        "<small>{hint}</small>"
        "</div>".format(
            css=stage_class(state),
            title=html.escape(title),
            main=html.escape(main),
            hint=html.escape(hint),
        )
        for state, title, main, hint in steps
    )
    return (
        f"<section class='panel pipeline-panel'><h2>{html.escape(t('Поток решений', 'Decision Pipeline'))}</h2>"
        f"<p>{html.escape(t('Сверху видно, что входит в сервер, где становится кандидатом, где скорится, что уходит в recovery, и где появляется accept/fallback.', 'This shows what enters the server, where it becomes a candidate, where it is scored, what goes to recovery, and where accept/fallback happens.'))}</p>"
        f"<div class='pipeline-grid'>{cards}</div></section>"
    )


def recovery_backlog_panel(
    metrics: dict[str, Any],
    quarantined_rows: list[dict[str, Any]],
    lang: str,
) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    def stuck_reason(row: dict[str, Any]) -> str:
        if bool(row.get("rejected")) or dashboard_int(row.get("false_accepts")) > 0:
            return t("false/rejected: держать в quarantine", "false/rejected: keep quarantined")
        if not bool(row.get("shadow_ready")):
            return t("мало shadow-доказательства", "not enough shadow evidence")
        if dashboard_int(row.get("trust_drift_micro")) > 100_000:
            return t("высокий drift: нужен более узкий split", "high drift: needs narrower split")
        if dashboard_int(row.get("negative_events")) > 0:
            return t("смешаны positive/negative события", "mixed positive/negative events")
        if not bool(row.get("exportable")):
            return t("ждёт promotion gate", "waiting for promotion gate")
        return t("наблюдать", "watch")

    def next_action(row: dict[str, Any]) -> str:
        if bool(row.get("rejected")) or dashboard_int(row.get("false_accepts")) > 0:
            return t("авто-изоляция и deeper split", "auto-isolate and split deeper")
        if not bool(row.get("shadow_ready")):
            return t("копить будущий поток", "collect future stream")
        if dashboard_int(row.get("trust_drift_micro")) > 100_000:
            return t("искать child subcenter", "search child subcenter")
        if dashboard_int(row.get("negative_events")) > 0:
            return t("развести отрицательные атомы", "separate negative atoms")
        return t("promotion audit", "promotion audit")

    rows = []
    for row in quarantined_rows[:10]:
        false_accepts = dashboard_int(row.get("false_accepts"))
        drift = dashboard_int(row.get("trust_drift_micro"))
        css = "bad" if false_accepts > 0 or bool(row.get("rejected")) else "watch"
        flags = []
        for name in ("active", "candidate", "shadow_ready", "exportable", "final_hot", "rejected"):
            if bool(row.get(name)):
                flags.append(name)
        rows.append(
            "<tr>"
            f"<td><code>{html.escape(str(row.get('profile_id') or 'unknown'))}</code></td>"
            f"<td>{html.escape(str(row.get('kind') or 'unknown'))}</td>"
            f"<td>{dashboard_metric_text(row.get('tokens_saved'), lang)}</td>"
            f"<td>{dashboard_metric_text(row.get('unique_cpu_accepts_over_exact_cache'), lang)}</td>"
            f"<td>{dashboard_metric_text(row.get('events_seen'), lang)}</td>"
            f"<td>{dashboard_metric_text(row.get('negative_events'), lang)}</td>"
            f"<td><b class='{css}'>{false_accepts}</b></td>"
            f"<td>{dashboard_metric_text(drift, lang)}</td>"
            f"<td><code>{html.escape(','.join(flags) or '-')}</code></td>"
            f"<td>{html.escape(stuck_reason(row))}</td>"
            f"<td>{html.escape(next_action(row))}</td>"
            "</tr>"
        )
    if not rows:
        rows.append(
            f"<tr><td colspan='11'>{html.escape(t('quarantine backlog пуст', 'quarantine backlog is empty'))}</td></tr>"
        )
    recovery_events = dashboard_int(metrics.get("quarantine_recovery_discovery_events"))
    recovery_tokens = dashboard_int(metrics.get("quarantine_recovery_discovery_tokens"))
    recovery_observe = dashboard_int(metrics.get("quarantine_recovery_auto_subcenter_observe_events"))
    return (
        f"<section class='panel wide-panel'><h2>{html.escape(t('Где застряло: Recovery Backlog', 'Where It Is Stuck: Recovery Backlog'))}</h2>"
        f"<p>{html.escape(t('Это не ручной список задач: это очередь автоматического recovery. Если bucket здесь, майнер уже видит ценность, но ещё не имеет безопасного promotion.', 'This is not a manual todo list: it is the automatic recovery queue. If a bucket is here, the miner sees value but does not yet have safe promotion evidence.'))}</p>"
        f"<p>recovery events: <b>{recovery_events}</b> · tokens: <b>{recovery_tokens}</b> · subcenter observes: <b>{recovery_observe}</b></p>"
        "<table><thead><tr>"
        "<th>bucket</th><th>kind</th><th>tokens</th><th>accepts</th><th>events</th><th>negative</th><th>false</th><th>drift</th><th>flags</th>"
        f"<th>{html.escape(t('почему stuck', 'why stuck'))}</th><th>{html.escape(t('следующее auto-действие', 'next auto action'))}</th>"
        "</tr></thead>"
        f"<tbody>{''.join(rows)}</tbody></table></section>"
    )


def live_miner_panels(
    status: dict[str, Any],
    metrics: dict[str, Any],
    class_rows: list[dict[str, Any]],
    profile_rows: list[dict[str, Any]],
    quarantined_rows: list[dict[str, Any]],
    lang: str,
) -> str:
    is_en = lang == "en"

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    bridge = status.get("bridge") if isinstance(status.get("bridge"), dict) else {}
    verify = status.get("verify") if isinstance(status.get("verify"), dict) else {}
    scorecard = status.get("scorecard") if isinstance(status.get("scorecard"), dict) else {}
    clean_saved = dashboard_int(metrics.get("stable_clean_token_compression_saved_tokens"))
    clean_total = dashboard_int(metrics.get("stable_clean_token_compression_total_tokens"))
    clean_accepts = dashboard_int(
        metrics.get("stable_clean_token_compression_unique_cpu_accepts_over_exact_cache")
    )
    if clean_accepts == 0:
        clean_accepts = dashboard_int(scorecard.get("unique_cpu_accepts_over_exact_cache"))
    active_false = dashboard_int(metrics.get("product_hot_score_only_post_quarantine_false_accepts"))
    shadow_false = dashboard_int(metrics.get("stable_clean_token_compression_false_accepts"))
    call_denominator = (
        dashboard_int(metrics.get("stable_decision_log_rows"))
        or dashboard_int(metrics.get("append_parsed_rows"))
        or dashboard_int(scorecard.get("stable_rows"))
    )
    top_profile = (profile_rows or class_rows)[0] if (profile_rows or class_rows) else {}
    top_quarantine = quarantined_rows[0] if quarantined_rows else {}
    selected_bucket = str(top_profile.get("profile_id") or top_profile.get("class") or top_profile.get("kind") or "none")
    selected_status = str(top_profile.get("status") or ("safe" if dashboard_int(top_profile.get("false_accepts")) == 0 else "quarantine"))
    selected_accepts = dashboard_int(top_profile.get("unique_cpu_accepts_over_exact_cache"))
    selected_tokens = dashboard_int(top_profile.get("tokens_saved"))
    selected_false = dashboard_int(top_profile.get("false_accepts"))
    selected_threshold = dashboard_metric_text(top_profile.get("learned_threshold_micro"), lang, default="-")
    selected_risk = "safe" if selected_false == 0 else "quarantine"
    debt_items: list[tuple[str, str, str]] = []
    if quarantined_rows:
        debt_items.append(
            (
                "P0",
                t("разрезать самый жирный quarantined bucket", "split the fattest quarantined bucket"),
                str(top_quarantine.get("profile_id") or top_quarantine.get("class") or "unknown"),
            )
        )
    if dashboard_int(metrics.get("product_hot_score_only_post_quarantine_score_candidate_events")) == 0:
        debt_items.append(
            (
                "P0",
                t("дать miner живое post-quarantine окно", "feed miner a live post-quarantine window"),
                str(metrics.get("product_hot_compression_claim_blocker") or "window_missing"),
            )
        )
    if active_false != 0:
        debt_items.append(
            (
                "P0",
                "false_accepts",
                t("quarantine before promotion", "quarantine before promotion"),
            )
        )
    elif shadow_false != 0:
        debt_items.append(
            (
                "P1",
                t("shadow risk events", "shadow risk events"),
                t("split or quarantine before widening claim", "split or quarantine before widening claim"),
            )
        )
    if not profile_rows:
        debt_items.append(
            (
                "P1",
                t("нет top safe profiles", "no top safe profiles"),
                t("нужен selector export", "selector export needed"),
            )
        )
    if not debt_items:
        debt_items.append(
            (
                "P2",
                t("расширять окно без ослабления verifier", "widen the window without weakening verifier"),
                "false_accepts=0",
            )
        )
    debt_rows = "".join(
        "<tr>"
        f"<td><b>{html.escape(priority)}</b></td>"
        f"<td>{html.escape(title)}</td>"
        f"<td><code>{html.escape(detail)}</code></td>"
        "</tr>"
        for priority, title, detail in debt_items
    )

    top_bucket_rows = []
    for row in (profile_rows or class_rows)[:8]:
        bucket = str(row.get("profile_id") or row.get("class") or row.get("kind") or "unknown")
        trust = row.get("learned_threshold_micro")
        status_text = str(row.get("status") or ("safe" if dashboard_int(row.get("false_accepts")) == 0 else "quarantine"))
        top_bucket_rows.append(
            "<tr>"
            f"<td>{html.escape(bucket)}</td>"
            f"<td>{dashboard_int(row.get('unique_cpu_accepts_over_exact_cache'))}</td>"
            f"<td>{dashboard_int(row.get('tokens_saved'))}</td>"
            f"<td>{dashboard_metric_text(trust, lang, default='-')}</td>"
            f"<td>{dashboard_int(row.get('false_accepts'))}</td>"
            f"<td><code>{html.escape(status_text)}</code></td>"
            "</tr>"
        )
    if not top_bucket_rows:
        top_bucket_rows.append(
            "<tr><td colspan='6'>no buckets yet</td></tr>"
            if is_en
            else "<tr><td colspan='6'>пока нет bucket-ов</td></tr>"
        )

    panels = [
        dashboard_metric_panel(
            t("Live Miner Window", "Live Miner Window"),
            [
                ("window_events", dashboard_metric_text(call_denominator, lang), "stable/append denominator"),
                ("gateway_window", dashboard_metric_first(metrics, ("gateway_decision_window_rows",), lang, default="0"), "gateway decisions"),
                ("provider_window", dashboard_metric_first(metrics, ("provider_bridge_decision_window_rows",), lang, default="0"), "provider bridge"),
                ("active_batch_rows", dashboard_metric_first(metrics, ("miner_active_batch_rows",), lang, default="0"), "current miner batch"),
                ("sleep_ms", dashboard_metric_first(metrics, ("miner_saturation_last_sleep_ms", "miner_active_batch_sleep_ms"), lang, default="0"), "saturation control"),
                ("idle_heartbeats", dashboard_metric_first(metrics, ("miner_saturation_idle_heartbeats",), lang, default="0"), "daemon liveness"),
            ],
            lang,
        ),
        dashboard_metric_panel(
            t("Extraction / Verifier", "Extraction / Verifier"),
            [
                ("parsed_events", dashboard_metric_first(metrics, ("append_parsed_rows", "stable_decision_log_rows"), lang, default="0"), "trace rows"),
                ("score_candidates", dashboard_metric_first(metrics, ("append_score_candidate_events", "stable_decision_log_score_candidate_events"), lang, default="0"), "phase-center input"),
                ("result_atoms", dashboard_metric_first(metrics, ("quarantine_recovery_auto_subcenter_observe_events",), lang, default="0"), "observable subcenter"),
                ("verifier_true", dashboard_metric_first(metrics, ("product_hot_score_only_post_quarantine_score_candidate_events",), lang, default="0"), "promotion window"),
                ("missing_verifier", str(max(0, call_denominator - dashboard_int(metrics.get("product_hot_score_only_post_quarantine_score_candidate_events")))), "rough gap"),
                ("extract_failures", dashboard_metric_first(metrics, ("append_false_accepts",), lang, default="0"), "must stay low"),
            ],
            lang,
        ),
        dashboard_metric_panel(
            t("Compression", "Compression"),
            [
                ("calls_saved_pct", dashboard_pct(clean_accepts, call_denominator), f"{clean_accepts} / {call_denominator}"),
                ("tokens_saved_pct", dashboard_pct(clean_saved, clean_total), f"{clean_saved} / {clean_total}"),
                ("tokens_saved", dashboard_metric_text(clean_saved or dashboard_int(scorecard.get("tokens_saved")), lang), "verified clean window"),
                ("exact_cache_hits", dashboard_metric_first(metrics, ("exact_cache_hits",), lang, default="-"), "baseline"),
                ("incremental_accepts", dashboard_metric_text(clean_accepts, lang), "over exact cache"),
                ("local_accept_mode", dashboard_bool(bridge.get("local_accept_enabled"), lang), "server policy"),
            ],
            lang,
        ),
        dashboard_metric_panel(
            "Safety",
            [
                ("active_false_accepts", dashboard_metric_text(active_false, lang), "post-quarantine hot path"),
                ("shadow_risk_events", dashboard_metric_text(shadow_false, lang), "stable diagnostic window"),
                ("post_quarantine_false", dashboard_metric_first(metrics, ("product_hot_score_only_post_quarantine_false_accepts",), lang, default="0"), "hot score only"),
                ("gateway_false", dashboard_metric_first(metrics, ("gateway_false_accepts", "provider_bridge_v2_false_accepts"), lang, default="0"), "HTTP bridge"),
                ("quarantined_buckets", dashboard_metric_text(len(quarantined_rows), lang), "needs split"),
                ("wrong_wins", dashboard_metric_first(metrics, ("wrong_wins",), lang, default="-"), "phase judge"),
                ("accept_blocker", str(metrics.get("product_hot_compression_claim_blocker") or verify.get("verdict") or "none"), "gate"),
            ],
            lang,
        ),
        dashboard_metric_panel(
            "L4 Selector",
            [
                ("selected_bucket", selected_bucket, "current best safe bucket"),
                ("bucket_status", selected_status, "final_hot/exportable/quarantine"),
                ("bucket_accepts", dashboard_metric_text(selected_accepts, lang), "unique over cache"),
                ("bucket_tokens", dashboard_metric_text(selected_tokens, lang), "token value"),
                ("bucket_false", dashboard_metric_text(selected_false, lang), "must be 0"),
                ("threshold_micro", selected_threshold, "learned selector threshold"),
                ("risk_state", selected_risk, "derived from false accepts"),
                ("selector_mode", str(metrics.get("architecture_version_key") or "phase-center"), "active architecture"),
            ],
            lang,
        ),
        dashboard_metric_panel(
            "Runtime",
            [
                ("core_p99_ns", dashboard_metric_first(metrics, ("core_p99_ns", "runtime_p99_ns"), lang, default="-"), "score loop"),
                ("worker_p99_ns", dashboard_metric_first(metrics, ("worker_p99_ns",), lang, default="-"), "worker envelope"),
                ("rss_bytes", dashboard_metric_first(metrics, ("rss_bytes",), lang, default="-"), "process"),
                ("hot_profiles", dashboard_metric_first(metrics, ("final_hot_profile_count", "product_hot_score_only_active_profile_count"), lang, default="0"), "L2 candidates"),
                ("hot_profile_bytes", dashboard_metric_first(metrics, ("hot_profile_bytes", "runtime_bytes_estimate"), lang, default="-"), "package"),
                ("cpu_percent", dashboard_metric_first(metrics, ("cpu_percent",), lang, default="-"), "daemon"),
            ],
            lang,
        ),
        (
            f"<section class='panel wide-panel'><h2>{html.escape(t('Top Buckets', 'Top Buckets'))}</h2>"
            "<table><thead><tr><th>bucket</th><th>accepts</th><th>tokens</th><th>trust</th><th>false</th><th>status</th></tr></thead>"
            f"<tbody>{''.join(top_bucket_rows)}</tbody></table></section>"
        ),
        (
            f"<section class='panel'><h2>{html.escape(t('Debt Queue', 'Debt Queue'))}</h2>"
            f"<table><thead><tr><th>P</th><th>{html.escape(t('что чинить', 'fix'))}</th><th>{html.escape(t('сигнал', 'signal'))}</th></tr></thead>"
            f"<tbody>{debt_rows}</tbody></table></section>"
        ),
    ]
    return "".join(panels)


def status_dashboard_html(request_path: str = "") -> str:
    lang = dashboard_lang_for_path(request_path)
    is_en = lang == "en"
    base_path = html.escape(urllib.parse.urlsplit(request_path).path or "/v2/status/")

    def t(ru: str, en: str) -> str:
        return en if is_en else ru

    metrics = read_json_file(METRICS_JSON)
    status = read_json_file(STATUS_JSON)
    provider_watch = read_json_file(PROVIDER_EXPORT_WATCH_REPORT)
    history_rows = read_dashboard_history(DASHBOARD_HISTORY_MAX_POINTS)
    bridge = status.get("bridge") if isinstance(status.get("bridge"), dict) else {}
    scorecard = status.get("scorecard") if isinstance(status.get("scorecard"), dict) else {}
    verify = status.get("verify") if isinstance(status.get("verify"), dict) else {}
    summary = status.get("summary") if isinstance(status.get("summary"), dict) else {}
    evidence = status.get("provider_evidence") if isinstance(status.get("provider_evidence"), dict) else {}

    clean_saved = dashboard_int(metrics.get("stable_clean_token_compression_saved_tokens"))
    clean_total = dashboard_int(metrics.get("stable_clean_token_compression_total_tokens"))
    clean_accepts = dashboard_int(
        metrics.get("stable_clean_token_compression_unique_cpu_accepts_over_exact_cache")
    )
    active_false = dashboard_int(metrics.get("product_hot_score_only_post_quarantine_false_accepts"))
    shadow_false = dashboard_int(metrics.get("stable_clean_token_compression_false_accepts"))
    stable_rows = dashboard_int(metrics.get("stable_decision_log_rows"))
    stable_tokens = dashboard_int(metrics.get("stable_decision_log_total_tokens"))
    class_rows = normalized_rows(metrics.get("operator_class_token_ranking"))
    profile_rows = normalized_rows(metrics.get("operator_profile_token_ranking"))
    quarantined_rows = normalized_rows(metrics.get("quarantined_profile_token_ranking"))
    generated = html.escape(str(status.get("generated_utc") or now_iso()))
    generated_age = html.escape(dashboard_age_text(status.get("generated_utc"), lang))
    active_manifest = promotion_manifest_summary(read_json_file(CALL_TOKEN_PROMOTION_ACTIVE_MANIFEST_JSON))
    blockers = verify.get("blockers") if isinstance(verify.get("blockers"), list) else []
    blocker_text = ", ".join(str(item) for item in blockers) or "none"
    ru_class = "active" if lang == "ru" else ""
    en_class = "active" if lang == "en" else ""
    lang_switch = (
        "<nav class='lang-switch' aria-label='language'>"
        f"<a class='{ru_class}' href='{base_path}?lang=ru'>RU</a>"
        f"<a class='{en_class}' href='{base_path}?lang=en'>ENG</a>"
        "</nav>"
    )

    claim_state = "ok" if verify.get("compression_claim_allowed") else "watch"
    clean_rows = dashboard_int(metrics.get("stable_decision_log_clean_suffix_rows"))
    stable_saved = dashboard_int(metrics.get("stable_decision_log_tokens_saved"))
    stable_accepts = dashboard_int(metrics.get("stable_decision_log_unique_cpu_accepts_over_exact_cache"))
    stable_shadow_false = dashboard_int(metrics.get("stable_decision_log_false_accepts"))
    clean_coverage = dashboard_pct(clean_total, stable_tokens)
    capture_state = (
        t("полный provider frame подключён", "full provider frame joined")
        if bool(bridge.get("upstream_configured"))
        and dashboard_int(metrics.get("provider_bridge_boundary_window_rows")) > 0
        else t("частичный Nando-frame, не весь provider-трафик", "partial Nando frame, not full provider traffic")
    )

    return f"""<!doctype html>
<html lang="{lang}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="refresh" content="10">
  <title>{t("NANDA CPU v2 Статус", "NANDA CPU v2 Status")}</title>
  <style>
    :root {{ color-scheme: dark; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 0; background: #0f1113; color: #edf1f5; }}
    main {{ width: min(1120px, calc(100vw - 28px)); margin: 0 auto; padding: 24px 0 36px; }}
    header {{ display: flex; justify-content: space-between; gap: 16px; align-items: flex-end; margin-bottom: 16px; }}
    h1 {{ font-size: 26px; line-height: 1.05; margin: 0; letter-spacing: 0; }}
    .subtitle, .stamp, .hint {{ color: #9aa7b2; font-size: 13px; line-height: 1.35; }}
    .header-actions {{ display: flex; flex-direction: column; align-items: flex-end; gap: 8px; text-align: right; }}
    .lang-switch {{ display: inline-flex; gap: 4px; padding: 3px; background: #171b1f; border: 1px solid #28313a; border-radius: 8px; }}
    .lang-switch a {{ color: #9aa7b2; text-decoration: none; font-size: 13px; font-weight: 700; padding: 6px 9px; border-radius: 6px; }}
    .lang-switch a.active {{ color: #0f1113; background: #edf1f5; }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; }}
    .hero {{ grid-template-columns: repeat(auto-fit, minmax(230px, 1fr)); }}
    .card, .panel {{ background: #171b1f; border: 1px solid #28313a; border-radius: 8px; }}
    .card {{ padding: 14px; min-height: 92px; }}
    .card h2, .panel h2 {{ color: #a8b3bd; font-size: 13px; font-weight: 650; margin: 0 0 8px; letter-spacing: 0; }}
    .card strong {{ display: block; font-size: 25px; line-height: 1.08; color: #fff; overflow-wrap: anywhere; }}
    .card p {{ margin: 8px 0 0; color: #9aa7b2; font-size: 12px; line-height: 1.35; }}
    .panel {{ margin-top: 12px; padding: 14px; overflow-x: auto; }}
    .panel p {{ margin: 6px 0; color: #d7dde3; line-height: 1.4; }}
    .big {{ font-size: clamp(30px, 5vw, 56px); line-height: .95; color: #fff; font-weight: 800; margin: 2px 0 10px; }}
    .kv {{ display: flex; flex-wrap: wrap; gap: 8px 18px; align-items: baseline; }}
    .kv span {{ color: #9aa7b2; font-size: 13px; line-height: 1.35; }}
    .kv b {{ color: #fff; font-size: 15px; }}
    .flow {{ display: flex; flex-wrap: wrap; gap: 8px; align-items: center; color: #d7dde3; }}
    .flow code {{ background: #111519; border: 1px solid #252e36; border-radius: 6px; padding: 6px 8px; }}
    .pipeline-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 8px; }}
    .pipeline-step {{ min-height: 104px; padding: 11px; background: #111519; border: 1px solid #252e36; border-left-width: 4px; border-radius: 8px; display: grid; gap: 7px; }}
    .pipeline-step h3 {{ margin: 0; color: #a8b3bd; font-size: 13px; line-height: 1.25; letter-spacing: 0; }}
    .pipeline-step b {{ color: #fff; font-size: 16px; line-height: 1.25; overflow-wrap: anywhere; }}
    .pipeline-step small {{ color: #9aa7b2; font-size: 12px; line-height: 1.3; overflow-wrap: anywhere; }}
    .stage-ok {{ border-left-color: #58d68d; }}
    .stage-watch {{ border-left-color: #f5b041; }}
    .stage-bad {{ border-left-color: #ff6b6b; }}
    .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; margin-top: 12px; align-items: start; }}
    .charts {{ display: block; }}
    .metric-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 8px; }}
    .metric {{ display: grid; gap: 5px; min-height: 78px; padding: 10px; background: #111519; border: 1px solid #252e36; border-radius: 8px; }}
    .metric span {{ color: #8e9aa5; font-size: 12px; font-weight: 700; line-height: 1.2; overflow-wrap: anywhere; }}
    .metric b {{ color: #fff; font-size: 20px; line-height: 1.05; overflow-wrap: anywhere; }}
    .metric small {{ color: #9aa7b2; font-size: 12px; line-height: 1.25; overflow-wrap: anywhere; }}
    .metric-wide {{ grid-column: 1 / -1; }}
    .metric-wide b {{ font-size: 13px; line-height: 1.35; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace; word-break: break-word; }}
    table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
    th, td {{ text-align: left; border-bottom: 1px solid #28313a; padding: 9px 8px; white-space: nowrap; }}
    th {{ color: #a8b3bd; font-weight: 650; }}
    .ok {{ color: #58d68d; }}
    .watch {{ color: #f5b041; }}
    .bad {{ color: #ff6b6b; }}
    .chart-panel p, .split-panel p {{ margin: 0 0 9px; color: #9aa7b2; font-size: 13px; }}
    .chart-svg {{ display: block; width: 100%; height: 220px; }}
    .chart-grid {{ stroke: #29323a; stroke-width: 1; }}
    .chart-line {{ fill: none; stroke-width: 3; stroke-linecap: round; stroke-linejoin: round; }}
    .chart-green {{ stroke: #58d68d; }}
    .chart-blue {{ stroke: #5dade2; }}
    .chart-red {{ stroke: #ff6b6b; }}
    .chart-yellow {{ stroke: #f5b041; }}
    .chart-label {{ fill: #9aa7b2; font-size: 12px; }}
    .legend {{ display: flex; flex-wrap: wrap; gap: 12px; color: #9aa7b2; font-size: 13px; margin-bottom: 8px; }}
    .legend i {{ display: inline-block; width: 10px; height: 10px; border-radius: 50%; margin-right: 6px; }}
    .legend-green {{ background: #58d68d; }}
    .legend-blue {{ background: #5dade2; }}
    .legend-red {{ background: #ff6b6b; }}
    .legend-yellow {{ background: #f5b041; }}
    .split-bar {{ display: flex; width: 100%; height: 12px; overflow: hidden; border-radius: 6px; background: #29323a; margin-bottom: 12px; }}
    .split-q {{ background: #f5b041; }}
    .split-e {{ background: #5dade2; }}
    .split-f {{ background: #58d68d; }}
    code {{ color: #d6e4ff; overflow-wrap: anywhere; }}
    @media (max-width: 860px) {{
      header {{ align-items: flex-start; flex-direction: column; }}
      .header-actions {{ align-items: flex-start; text-align: left; }}
      .charts {{ grid-template-columns: 1fr; }}
      th, td {{ white-space: normal; }}
    }}
  </style>
</head>
<body>
<main>
  <header>
    <div>
      <h1>NANDA CPU v2</h1>
      <div class="subtitle">{t("компактный рантайм скрытых переходов", "compact latent transition runtime")}</div>
    </div>
    <div class="header-actions">
      {lang_switch}
      <div class="stamp">{t("обновлено", "updated")}: {generated}<br>{t("возраст метрик", "metrics age")}: {generated_age} · {t("автообновление", "auto refresh")}: 10s</div>
    </div>
  </header>

  <section class="panel">
    <h2>{t("Состояние сервера", "Server State")}</h2>
    <div class="big">{'OK' if bridge.get('health_ok') else 'WATCH'}</div>
    <div class="kv">
      <span>local_accept <b>{dashboard_bool(bridge.get('local_accept_enabled'), lang)}</b></span>
      <span>client_allow <b>{dashboard_bool(bridge.get('client_allow_local_accept'), lang)}</b></span>
      <span>safety <b>{html.escape(str(bridge.get('safety_policy') or 'unknown'))}</b></span>
      <span>upstream <b>{dashboard_bool(bridge.get('upstream_configured'), lang)}</b></span>
      <span>{t("метрики", "metrics")} <b>{generated_age}</b></span>
    </div>
  </section>

  <section class="panel">
    <h2>{t("Покрытие трафика", "Traffic Coverage")}</h2>
    <div class="big">{dashboard_metric_text(stable_tokens, lang)} tokens</div>
    <p><b>{html.escape(capture_state)}</b></p>
    <div class="kv">
      <span>{t("rows во frame", "frame rows")} <b>{stable_rows}</b></span>
      <span>gateway <b>{dashboard_int(metrics.get("gateway_decision_window_rows"))}</b></span>
      <span>provider bridge <b>{dashboard_int(metrics.get("provider_bridge_decision_window_rows"))}</b></span>
      <span>provider boundary <b>{dashboard_int(metrics.get("provider_bridge_boundary_window_rows"))}</b></span>
      <span>{t("stable accepts", "stable accepts")} <b>{stable_accepts}</b></span>
      <span>{t("stable saved", "stable saved")} <b>{dashboard_metric_text(stable_saved, lang)}</b></span>
      <span>{t("stable shadow false", "stable shadow false")} <b class="{'ok' if stable_shadow_false == 0 else 'watch'}">{stable_shadow_false}</b></span>
    </div>
  </section>

  <section class="panel">
    <h2>{t("Безопасное сжатие сейчас", "Safe Compression Now")}</h2>
    <div class="big">{dashboard_pct(clean_saved, clean_total)}</div>
    <div class="kv">
      <span>clean-window <b>{clean_saved} / {clean_total}</b> tokens</span>
      <span>{t("coverage от frame", "coverage of frame")} <b>{clean_coverage}</b></span>
      <span>{t("clean rows", "clean rows")} <b>{clean_rows}</b></span>
      <span>{t("CPU accepts", "CPU accepts")} <b>{clean_accepts}</b></span>
      <span>active false <b class="{'ok' if active_false == 0 else 'bad'}">{active_false}</b></span>
      <span>shadow risk <b class="{'ok' if shadow_false == 0 else 'watch'}">{shadow_false}</b></span>
      <span>{t("claim", "claim")} <b class="{claim_state}">{dashboard_bool(verify.get('compression_claim_allowed'), lang)}</b></span>
    </div>
  </section>

  <section class="panel">
    <h2>{t("Накопленный .nwpc итог", "Accumulated .nwpc Total")}</h2>
    <div class="big">{dashboard_metric_text(active_manifest["tokens"], lang)} tokens</div>
    <div class="kv">
      <span>profiles <b>{active_manifest['promoted']}</b></span>
      <span>accepts <b>{active_manifest['accepts']}</b></span>
      <span>false <b class="{'ok' if active_manifest['false_accepts'] == 0 else 'bad'}">{active_manifest['false_accepts']}</b></span>
      <span>parity <b>{active_manifest['parity']}</b></span>
      <span>allowed <b class="{'ok' if active_manifest['allowed'] else 'watch'}">{dashboard_bool(active_manifest['allowed'], lang)}</b></span>
      <span>hot profiles <b>{dashboard_int(metrics.get("final_hot_profile_count"))}</b></span>
    </div>
  </section>

  <section class="panel">
    <h2>{t("Как работает", "How It Works")}</h2>
    <div class="flow">
      <code>{t("поток событий", "event stream")}</code>
      <span>→</span>
      <code>online phase-center miner</code>
      <span>→</span>
      <code>.nwpc</code>
      <span>→</span>
      <code>{t("CPU accept или fallback", "CPU accept or fallback")}</code>
    </div>
    <p class="hint">{t("Боевой accept разрешён только при verifier/gate и active false = 0.", "Product accept is allowed only with verifier/gate and active false = 0.")}</p>
  </section>

  {traffic_frame_panel(status, metrics, active_manifest, lang)}
  {decision_pipeline_panel(status, metrics, active_manifest, lang)}
  {recovery_backlog_panel(metrics, quarantined_rows, lang)}

  {dashboard_history_chart(history_rows, lang)}
  {dashboard_pool_chart(history_rows, lang)}

  <section class="panel">
    <h2>{t("Все важные показатели", "All Key Indicators")}</h2>
    <table><tbody>
      <tr><th>{t("server health", "server health")}</th><td>{'OK' if bridge.get('health_ok') else 'WATCH'}</td></tr>
      <tr><th>blockers</th><td><code>{html.escape(blocker_text)}</code></td></tr>
      <tr><th>miner</th><td>{t("пауза", "paused") if not metrics.get("miner_saturation_active") else t("активен", "active")} · sleep_ms={dashboard_int(metrics.get('miner_saturation_last_sleep_ms'))} · batch={dashboard_int(metrics.get('miner_active_batch_rows'))}</td></tr>
      <tr><th>candidates</th><td>stable={dashboard_int(metrics.get('stable_decision_log_score_candidate_events'))} · clean={dashboard_int(metrics.get('stable_decision_log_clean_suffix_score_candidate_events'))} · append={dashboard_int(metrics.get('append_score_candidate_events'))}</td></tr>
      <tr><th>provider</th><td>v2_accepts={dashboard_int(metrics.get('provider_bridge_v2_local_accept_events'))} · false={dashboard_int(metrics.get('provider_bridge_v2_false_accepts'))} · boundary_rows={dashboard_int(metrics.get('provider_bridge_boundary_window_rows'))}</td></tr>
      <tr><th>gateway</th><td>accepts={dashboard_int(metrics.get('gateway_local_accept_events'))} · false={dashboard_int(metrics.get('gateway_false_accepts'))}</td></tr>
      <tr><th>history</th><td>{len(history_rows)} points · compact limit {DASHBOARD_HISTORY_COMPACT_MAX_BYTES} bytes</td></tr>
      <tr><th>next</th><td><code>{html.escape(str(summary.get('next_action') or metrics.get('product_hot_compression_claim_blocker') or 'none'))}</code></td></tr>
    </tbody></table>
  </section>

  {split_panel(metrics, lang)}
  {promotion_blocker_panel(lang)}
  {live_miner_panels(status, metrics, class_rows, profile_rows, quarantined_rows, lang)}
</main>
</body>
</html>"""
