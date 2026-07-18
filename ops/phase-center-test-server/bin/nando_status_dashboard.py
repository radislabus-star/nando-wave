"""Protected Nando CPU observability dashboard entrypoint.

This module owns access control and data loading only. Rendering is isolated in
``nando_status_dashboard_view`` so the dashboard cannot score traffic, compile
profiles, or change local-accept policy.
"""

from __future__ import annotations

from collections import deque
import hmac
import json
import os
from pathlib import Path
from typing import Any

from nando_status_dashboard_view import render_status_dashboard


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
LIVE_TAIL_REPORT_JSON = Path(
    os.environ.get(
        "NANDO_LIVE_TAIL_REPORT",
        "/var/lib/nando-wave/streaming/nando-phase-live-miner-tail.report.json",
    )
)
APPENDER_REPORT_JSON = Path(
    os.environ.get(
        "NANDO_APPENDER_REPORT",
        "/var/lib/nando-wave/streaming/phase-center-appender.report.json",
    )
)
TRANSITION_METRICS_JSON = Path(
    os.environ.get(
        "NANDO_TRANSITION_METRICS",
        "/var/lib/nando-wave/transition/metrics.json",
    )
)
TRANSITION_REGISTRY_JSON = Path(
    os.environ.get(
        "NANDO_TRANSITION_REGISTRY",
        "/var/lib/nando-wave/transition/registry.json",
    )
)
ECONOMICS_JSON = Path(
    os.environ.get(
        "NANDO_ECONOMICS_SNAPSHOT_JSON",
        "/var/lib/nando-wave/transition/economics.json",
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


def read_json_file(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


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


def dashboard_key_for_path(path: str) -> str:
    prefixes = ("/status/", "/v1/status/", "/v2/status/")
    for prefix in prefixes:
        if path.startswith(prefix):
            key = path[len(prefix) :].split("?", 1)[0].split("#", 1)[0]
            return key.removesuffix(".html")
    return ""


def dashboard_enabled() -> bool:
    return bool(STATUS_DASHBOARD_KEY)


def dashboard_path_authorized(path: str) -> bool:
    if not STATUS_DASHBOARD_KEY:
        return False
    key = dashboard_key_for_path(path)
    return bool(key) and hmac.compare_digest(key, STATUS_DASHBOARD_KEY)


def status_dashboard_html(request_path: str = "") -> str:
    return render_status_dashboard(
        metrics=read_json_file(METRICS_JSON),
        live_tail=read_json_file(LIVE_TAIL_REPORT_JSON),
        status=read_json_file(STATUS_JSON),
        appender=read_json_file(APPENDER_REPORT_JSON),
        transition_metrics=read_json_file(TRANSITION_METRICS_JSON),
        transition_registry=read_json_file(TRANSITION_REGISTRY_JSON),
        economics=read_json_file(ECONOMICS_JSON),
        history=read_dashboard_history(DASHBOARD_HISTORY_MAX_POINTS),
        request_path=request_path,
        upstream_base_url_configured=bool(
            os.environ.get("NANDO_PROVIDER_UPSTREAM_BASE_URL", "").strip()
        ),
    )
