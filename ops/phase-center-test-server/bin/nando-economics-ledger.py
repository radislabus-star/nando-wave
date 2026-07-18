#!/usr/bin/env python3
"""Build fail-closed, source-reconciled transition economics receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


LEDGER_SCHEMA = "nando.economics-terminal.v1"
SNAPSHOT_SCHEMA = "nando.economics-snapshot.v1"
M3_WINDOW_SCHEMA = "nando.economics-m3-window.v1"
SOURCE_RECEIPT_SCHEMA = "nando.economics-source-receipt.v1"
SOURCE_RECONCILIATION_SCHEMA = "nando.economics-source-reconciliation.v1"
SOURCE_ANOMALY_SCHEMA = "nando.economics-source-anomaly.v1"
FALSE_ACCEPT_EVIDENCE_SCHEMA = "nando.false-accept-evidence-receipt.v1"
M3_EVIDENCE_SCHEMA = "nando.economics-m3-evidence.v1"
INTENT_EVIDENCE_SCHEMA = "nando.economics-intent-evidence.v1"
EXECUTION_COMPANION_SCHEMA = "nando.economics-execution-companion.v1"
GATEWAY_COMPANION_SCHEMA = "nando.economics-gateway-companion.v1"
RECEIPT_BINDING_SCHEMA = "nando.economics-receipt-binding.v1"
POST_VERIFIER_RECEIPT_FIELD = "post_verifier_receipt"
POST_VERIFIER_RECEIPT_SCHEMA = "nando.response-post-verifier-receipt.v1"
POST_VERIFIER_ADMISSION_SCHEMA = "nando.response-admission-binding.v1"
POST_VERIFIER_RECEIPT_HASH_FIELDS = (
    "actor_sha256",
    "verifier_sha256",
    "evidence_sha256",
    "output_sha256",
    "package_id_sha256",
    "admission_binding_sha256",
)
POST_VERIFIER_RECEIPT_FIELDS = ("schema", *POST_VERIFIER_RECEIPT_HASH_FIELDS)

UTC_DAY_SECONDS = 86_400
M3_SETTLEMENT_SECONDS = UTC_DAY_SECONDS
FALSE_ACCEPT_MAX_AGE_SECONDS = UTC_DAY_SECONDS
MAX_GATEWAY_REQUEST_DURATION_SECONDS = 600
COMPANION_TIMESTAMP_SKEW_SECONDS = 5
MAX_JSONL_LINE_BYTES = 1_048_576
MAX_FALSE_ACCEPT_SOURCE_BYTES = 1_048_576
MAX_EVIDENCE_RECORDS = 100_000
MAX_ANOMALY_RECORDS = 256
M3_EVIDENCE_RETENTION_DAYS = 6
SOURCE_ID_KEY = "_economics_source_id"
REQUIRED_SOURCE_IDS = ("terminal_ledger", "gateway_terminal", "execution_events")
EXPECTED_SOURCE_SCHEMAS = {
    "terminal_ledger": frozenset({LEDGER_SCHEMA}),
    "gateway_terminal": frozenset({"nando.nginx-terminal.v1"}),
    "execution_events": frozenset(
        {
            "nando.transition-execution-event.v1",
            "nando.response-actor-fallback.v1",
        }
    ),
}
LOCAL_ROUTES = {"local_actor", "local_response_actor"}
VERIFIER_SCHEMAS = {
    "typed_actor_independent_verifier.v1",
    "response_actor_independent_verifier.v1",
    "continue_handle_external_evidence.v1",
    "source_value_external_evidence.v1",
    "custom_tool_external_evidence.v1",
    "value_projection_external_evidence.v1",
    "status_projection_external_evidence.v1",
}


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def nonzero_sha256(value: Any) -> bool:
    return valid_sha256(value) and value != "0" * 64


def canonical_json(value: Any) -> str:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    )


def canonical_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def with_hash(value: dict[str, Any], field: str) -> dict[str, Any]:
    result = dict(value)
    result[field] = canonical_sha256(result)
    return result


def package_id_sha256(package_id: Any) -> str | None:
    if not isinstance(package_id, str) or not package_id or len(package_id) > 256:
        return None
    return hashlib.sha256(package_id.encode("utf-8")).hexdigest()


def post_verifier_admission_binding(receipt: Any) -> str | None:
    if not isinstance(receipt, dict):
        return None
    required = ("actor_sha256", "verifier_sha256", "package_id_sha256")
    if any(not nonzero_sha256(receipt.get(field)) for field in required):
        return None
    return canonical_sha256(
        {
            "schema": POST_VERIFIER_ADMISSION_SCHEMA,
            **{field: receipt[field] for field in required},
        }
    )


def canonical_post_verifier_receipt(value: Any) -> dict[str, str] | None:
    if (
        not isinstance(value, dict)
        or set(value) != set(POST_VERIFIER_RECEIPT_FIELDS)
        or value.get("schema") != POST_VERIFIER_RECEIPT_SCHEMA
        or any(
            not nonzero_sha256(value.get(field))
            for field in POST_VERIFIER_RECEIPT_HASH_FIELDS
        )
        or value.get("admission_binding_sha256")
        != post_verifier_admission_binding(value)
    ):
        return None
    return {field: value[field] for field in POST_VERIFIER_RECEIPT_FIELDS}


def post_verifier_receipt_id(value: Any) -> str | None:
    receipt = canonical_post_verifier_receipt(value)
    return canonical_sha256(receipt) if receipt is not None else None


def post_verifier_receipt_covered(row: Any) -> bool:
    if not isinstance(row, dict):
        return False
    receipt = canonical_post_verifier_receipt(row.get(POST_VERIFIER_RECEIPT_FIELD))
    receipt_id = post_verifier_receipt_id(receipt)
    package_hash = package_id_sha256(row.get("package_id"))
    return (
        receipt is not None
        and nonzero_sha256(receipt_id)
        and row.get("verification_receipt_id") == receipt_id
        and row.get("request_sha256") == receipt["evidence_sha256"]
        and row.get("projector_receipt_id") == receipt["output_sha256"]
        and package_hash == receipt["package_id_sha256"]
    )


def ratio_milli(numerator: int, denominator: int) -> int:
    return (max(0, numerator) * 1000 // denominator) if denominator > 0 else 0


def traffic_source_dedupe_eligible(traffic_source: str) -> bool:
    return (
        not traffic_source.startswith("controlled_")
        and not traffic_source.startswith("dogfood_")
        and traffic_source not in {"smoke", "fixture", "audit"}
    )


def _strict_json_loads(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-finite JSON value: {value}")

    return json.loads(text, parse_constant=reject_constant)


def _timestamp_value(value: Any) -> int | None:
    if type(value) is int and value >= 0:
        return value
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(text)
        if parsed.tzinfo is None:
            return None
        timestamp = int(parsed.timestamp())
    except (OverflowError, OSError, ValueError):
        return None
    return timestamp if timestamp >= 0 else None


def row_timestamp_unix(row: Any) -> int | None:
    if not isinstance(row, dict):
        return None
    supplied: list[int] = []
    for field in ("timestamp_unix", "timestamp"):
        if field not in row:
            continue
        parsed = _timestamp_value(row[field])
        if parsed is None:
            return None
        supplied.append(parsed)
    if not supplied or len(set(supplied)) != 1:
        return None
    return supplied[0]


def utc_day_bounds(timestamp_unix: int) -> tuple[int, int]:
    if type(timestamp_unix) is not int:
        raise TypeError("timestamp_unix must be an integer")
    start = timestamp_unix - (timestamp_unix % UTC_DAY_SECONDS)
    return start, start + UTC_DAY_SECONDS


def m3_input_accounting(row: Any) -> tuple[int, str] | None:
    if not isinstance(row, dict):
        return None
    tokens = row.get("input_tokens")
    primary = row.get("input_token_accounting")
    legacy = row.get("input_token_measurement")
    if primary is not None and legacy is not None and primary != legacy:
        return None
    accounting = primary if primary is not None else legacy
    if (
        type(tokens) is not int
        or tokens <= 0
        or not isinstance(accounting, str)
        or not accounting
        or len(accounting) > 128
    ):
        return None
    return tokens, accounting


def m3_row_eligible(
    row: Any, *, window_start_unix: int, window_end_unix: int
) -> bool:
    timestamp_unix = row_timestamp_unix(row)
    return (
        isinstance(row, dict)
        and row.get("schema") == LEDGER_SCHEMA
        and isinstance(row.get("client_intent_id"), str)
        and bool(row["client_intent_id"])
        and row.get("traffic_source") == "ordinary"
        and row.get("intent_dedupe_eligible") is True
        and valid_sha256(row.get("request_sha256"))
        and m3_input_accounting(row) is not None
        and type(row.get("upstream_socket_opened")) is bool
        and timestamp_unix is not None
        and window_start_unix <= timestamp_unix < window_end_unix
    )


def m3_receipt_covered(row: Any) -> bool:
    return (
        isinstance(row, dict)
        and row.get("verification_status") == "verified"
        and row.get("verifier_schema") in VERIFIER_SCHEMAS
        and nonzero_sha256(row.get("verification_receipt_id"))
        and nonzero_sha256(row.get("projector_receipt_id"))
        and post_verifier_receipt_covered(row)
    )


def false_accept_value(row: Any) -> int | None:
    if not isinstance(row, dict):
        return None
    value = row.get("false_accepts")
    if type(value) is int and value >= 0:
        return value
    if row.get("event") == "false_accept" or row.get("false_accept") is True:
        return 1
    return None


def scoped_false_accepts(
    rows: Iterable[Any], *, window_start_unix: int, window_end_unix: int
) -> tuple[int, list[dict[str, Any]]]:
    count = 0
    evidence: list[dict[str, Any]] = []
    for row in rows:
        timestamp_unix = row_timestamp_unix(row)
        value = false_accept_value(row)
        if (
            isinstance(row, dict)
            and row.get("traffic_source") == "ordinary"
            and timestamp_unix is not None
            and window_start_unix <= timestamp_unix < window_end_unix
            and value is not None
        ):
            count += value
            evidence.append(row)
    return count, evidence


def evidence_rows_sha256(rows: Iterable[Any]) -> str:
    canonical_rows = sorted(canonical_json(row) for row in rows)
    return hashlib.sha256("\n".join(canonical_rows).encode("utf-8")).hexdigest()


def _anomaly(source_id: str, reason: str, **fields: Any) -> dict[str, Any]:
    value = {
        "schema": SOURCE_ANOMALY_SCHEMA,
        "source_id": source_id,
        "reason": reason,
        **fields,
    }
    return with_hash(value, "anomaly_sha256")


def read_jsonl_source(
    path: Path,
    source_id: str,
    *,
    observed_at_unix: int | None = None,
    prefix_size_bytes: int | None = None,
) -> tuple[list[Any], dict[str, Any], list[dict[str, Any]]]:
    observed_at = int(time.time()) if observed_at_unix is None else observed_at_unix
    resolved = str(path.expanduser().resolve(strict=False))
    rows: list[Any] = []
    anomalies: list[dict[str, Any]] = []
    hasher = hashlib.sha256()
    size_bytes = 0
    record_count = 0
    parsed_count = 0
    status = "ok"
    before = None
    after = None
    try:
        before = path.stat()
        boundary_size = before.st_size if prefix_size_bytes is None else prefix_size_bytes
        if type(boundary_size) is not int or boundary_size < 0 or before.st_size < boundary_size:
            raise ValueError("source_prefix_boundary_unavailable")
        remaining = boundary_size
        with path.open("rb") as handle:
            line_number = 0
            while remaining > 0:
                raw_line = handle.readline(min(remaining + 1, MAX_JSONL_LINE_BYTES + 1))
                line_number += 1
                if not raw_line or len(raw_line) > remaining:
                    raise ValueError("source_prefix_not_line_aligned")
                remaining -= len(raw_line)
                record_count += 1
                size_bytes += len(raw_line)
                hasher.update(raw_line)
                raw_hash = hashlib.sha256(raw_line).hexdigest()
                try:
                    if len(raw_line) > MAX_JSONL_LINE_BYTES:
                        raise ValueError("line_too_large")
                    text = raw_line.decode("utf-8")
                    if not text.strip():
                        raise ValueError("blank_line")
                    value = _strict_json_loads(text)
                    if not isinstance(value, dict):
                        raise ValueError("record_not_object")
                    expected_schemas = EXPECTED_SOURCE_SCHEMAS.get(source_id)
                    if (
                        expected_schemas is not None
                        and value.get("schema") not in expected_schemas
                    ):
                        raise ValueError("record_schema_mismatch")
                    rows.append(value)
                    parsed_count += 1
                except (UnicodeError, ValueError, json.JSONDecodeError) as error:
                    anomalies.append(
                        _anomaly(
                            source_id,
                            "malformed_jsonl_record",
                            line_number=line_number,
                            raw_record_sha256=raw_hash,
                            detail=type(error).__name__,
                        )
                    )
        if remaining != 0:
            raise ValueError("source_prefix_boundary_unavailable")
        after = path.stat()
        if before.st_ino != after.st_ino or after.st_size < boundary_size:
            status = "changed_during_read"
        elif after.st_size == before.st_size and before.st_mtime_ns != after.st_mtime_ns:
            status = "changed_during_read"
    except FileNotFoundError:
        status = "missing"
    except ValueError:
        status = "prefix_boundary_invalid"
    except (OSError, UnicodeError):
        status = "unreadable"

    receipt = {
        "schema": SOURCE_RECEIPT_SCHEMA,
        "source_id": source_id,
        "source_kind": "jsonl",
        "source_boundary_kind": "immutable_prefix",
        "source_path": resolved,
        "required": True,
        "status": status,
        "observed_at_unix": observed_at,
        "source_sha256": hasher.hexdigest() if before is not None else None,
        "source_size_bytes": size_bytes if before is not None else None,
        "source_modified_at_unix_ns": after.st_mtime_ns if after is not None else None,
        "source_record_count": record_count,
        "parsed_record_count": parsed_count,
        "malformed_record_count": len(anomalies),
    }
    return rows, with_hash(receipt, "receipt_sha256"), anomalies


def read_false_accept_evidence(
    path: Path, *, observed_at_unix: int | None = None
) -> dict[str, Any]:
    observed_at = int(time.time()) if observed_at_unix is None else observed_at_unix
    resolved = str(path.expanduser().resolve(strict=False))
    status = "ok"
    raw = b""
    value: Any = None
    before = None
    after = None
    try:
        before = path.stat()
        raw = path.read_bytes()
        after = path.stat()
        if len(raw) > MAX_FALSE_ACCEPT_SOURCE_BYTES:
            status = "oversized"
        elif (
            before.st_ino != after.st_ino
            or before.st_size != after.st_size
            or before.st_mtime_ns != after.st_mtime_ns
        ):
            status = "changed_during_read"
        else:
            value = _strict_json_loads(raw.decode("utf-8"))
    except FileNotFoundError:
        status = "missing"
    except UnicodeError:
        status = "unreadable"
    except (OSError, ValueError, json.JSONDecodeError):
        status = "malformed"

    count = value.get("false_accepts") if isinstance(value, dict) else None
    if status == "ok" and (type(count) is not int or count < 0):
        status = "malformed"
        count = None
    content_timestamp = None
    if isinstance(value, dict):
        content_timestamp = _timestamp_value(
            value.get("timestamp_unix", value.get("generated_at_unix"))
        )
    source_timestamp = (
        content_timestamp
        if content_timestamp is not None
        else (after.st_mtime_ns // 1_000_000_000 if after is not None else None)
    )
    age = (
        observed_at - source_timestamp if type(source_timestamp) is int else None
    )
    fresh = (
        status == "ok"
        and type(age) is int
        and -COMPANION_TIMESTAMP_SKEW_SECONDS <= age <= FALSE_ACCEPT_MAX_AGE_SECONDS
    )
    receipt = {
        "schema": FALSE_ACCEPT_EVIDENCE_SCHEMA,
        "source_id": "false_accept_metrics",
        "source_kind": "json",
        "source_path": resolved,
        "status": status,
        "observed_at_unix": observed_at,
        "source_timestamp_unix": source_timestamp,
        "source_timestamp_kind": (
            "content" if content_timestamp is not None else "filesystem_mtime"
        ),
        "max_age_seconds": FALSE_ACCEPT_MAX_AGE_SECONDS,
        "fresh": fresh,
        "source_sha256": hashlib.sha256(raw).hexdigest() if before is not None else None,
        "source_size_bytes": len(raw) if before is not None else None,
        "source_modified_at_unix_ns": after.st_mtime_ns if after is not None else None,
        "source_record_count": 1 if isinstance(value, dict) else 0,
        "false_accepts": count,
    }
    return with_hash(receipt, "receipt_sha256")


def read_false_accepts(path: Path) -> tuple[int | None, bool, str | None]:
    """Compatibility wrapper over the explicit false-accept receipt."""
    receipt = read_false_accept_evidence(path)
    available = receipt["status"] == "ok" and receipt["fresh"] is True
    reason = None if available else f"false_accept_evidence_{receipt['status']}"
    return receipt["false_accepts"], available, reason


def _public_execution_companion(row: Any) -> tuple[str | None, dict[str, Any]] | None:
    if (
        not isinstance(row, dict)
        or row.get("schema") != "nando.transition-execution-event.v1"
        or row.get("event") not in {"bridge_request", "local_accept"}
    ):
        return None
    timestamp = row_timestamp_unix(row)
    request_hash = row.get("request_sha256")
    tokens = row.get("tokens")
    if timestamp is None or not valid_sha256(request_hash) or type(tokens) is not int or tokens <= 0:
        return None
    event = row["event"]
    client_intent_id = row.get("client_intent_id")
    if event == "bridge_request" and (
        not isinstance(client_intent_id, str) or not client_intent_id
    ):
        return None
    post_verifier_receipt = (
        canonical_post_verifier_receipt(row.get(POST_VERIFIER_RECEIPT_FIELD))
        if event == "local_accept"
        else None
    )
    public = {
        "schema": EXECUTION_COMPANION_SCHEMA,
        "kind": "ingress" if event == "bridge_request" else "decision",
        "timestamp_unix": timestamp,
        "client_intent_id_sha256": (
            hashlib.sha256(client_intent_id.encode("utf-8")).hexdigest()
            if isinstance(client_intent_id, str)
            else None
        ),
        "request_sha256": request_hash,
        "input_tokens": tokens,
        "traffic_source": row.get("traffic_source") if event == "bridge_request" else None,
        "route_sha256": (
            hashlib.sha256(row["route"].encode("utf-8")).hexdigest()
            if isinstance(row.get("route"), str) and row["route"]
            else None
        ),
        "verification_receipt_id": row.get("verification_receipt_id"),
        "projector_receipt_id": row.get("projector_receipt_id"),
        "verifier_schema": row.get("verifier_schema"),
        "package_id_sha256": package_id_sha256(row.get("package_id")),
        POST_VERIFIER_RECEIPT_FIELD: post_verifier_receipt,
    }
    return client_intent_id, with_hash(public, "companion_sha256")


def execution_companions(
    rows: Iterable[Any],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    ingress: list[dict[str, Any]] = []
    decisions: list[dict[str, Any]] = []
    anomalies: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict) or row.get("schema") != "nando.transition-execution-event.v1":
            continue
        if row.get("event") not in {"bridge_request", "local_accept"}:
            continue
        companion = _public_execution_companion(row)
        if companion is None:
            anomalies.append(
                _anomaly(
                    "execution_events",
                    "execution_companion_malformed",
                    source_row_sha256=canonical_sha256(row),
                )
            )
            continue
        raw_intent, public = companion
        private = {"raw_client_intent_id": raw_intent, "record": public}
        (ingress if public["kind"] == "ingress" else decisions).append(private)
    return ingress, decisions, anomalies


def _gateway_decision(row: dict[str, Any]) -> tuple[str, bool] | None:
    if type(row.get("status")) is not int:
        return None
    status_text = row.get("upstream_status")
    address_text = row.get("upstream_addr")
    if not isinstance(status_text, str) or not status_text.strip():
        return None
    if not isinstance(address_text, str) or not address_text.strip():
        return None
    statuses = [
        value for value in re.split(r"\s+:\s+|\s*,\s*", status_text) if value
    ]
    addresses = [
        value
        for value in re.split(r"\s+:\s+|\s*,\s*", address_text)
        if value and value != "-"
    ]
    if statuses and statuses[0] in {"418", "502", "503", "504"}:
        return ("upstream", True) if len(addresses) >= 2 else None
    return ("local", False) if len(addresses) == 1 else None


def reconcile_gateway_rows(
    gateway_rows: Iterable[Any], execution_rows: Iterable[Any]
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    ingress, _, execution_anomalies = execution_companions(execution_rows)
    by_intent: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for companion in ingress:
        raw_intent = companion["raw_client_intent_id"]
        if isinstance(raw_intent, str):
            by_intent[raw_intent].append(companion)

    terminal: list[dict[str, Any]] = []
    companions: list[dict[str, Any]] = []
    anomalies = list(execution_anomalies)
    for row in gateway_rows:
        if not isinstance(row, dict) or row.get("schema") != "nando.nginx-terminal.v1":
            continue
        request_id = row.get("request_id")
        gateway_timestamp = row_timestamp_unix(row)
        decision = _gateway_decision(row)
        if not isinstance(request_id, str) or not request_id or gateway_timestamp is None or decision is None:
            anomalies.append(
                _anomaly(
                    "gateway_terminal",
                    "gateway_companion_malformed",
                    source_row_sha256=canonical_sha256(row),
                )
            )
            continue
        matches = [
            candidate
            for candidate in by_intent.get(request_id, [])
            if candidate["record"]["timestamp_unix"] <= gateway_timestamp
            <= candidate["record"]["timestamp_unix"]
            + MAX_GATEWAY_REQUEST_DURATION_SECONDS
        ]
        if len(matches) != 1:
            anomalies.append(
                _anomaly(
                    "gateway_terminal",
                    "gateway_ingress_companion_missing_or_ambiguous",
                    source_row_sha256=canonical_sha256(row),
                    candidate_count=len(matches),
                )
            )
            continue
        ingress_record = matches[0]["record"]
        decision_name, upstream_opened = decision
        public = {
            "schema": GATEWAY_COMPANION_SCHEMA,
            "timestamp_unix": gateway_timestamp,
            "ingress_timestamp_unix": ingress_record["timestamp_unix"],
            "client_intent_id_sha256": ingress_record["client_intent_id_sha256"],
            "request_sha256": ingress_record["request_sha256"],
            "status": row["status"],
            "decision": decision_name,
            "upstream_socket_opened": upstream_opened,
            "upstream_status_sha256": hashlib.sha256(
                row["upstream_status"].encode("utf-8")
            ).hexdigest(),
            "upstream_addr_sha256": hashlib.sha256(
                row["upstream_addr"].encode("utf-8")
            ).hexdigest(),
            "ingress_companion_sha256": ingress_record["companion_sha256"],
        }
        public = with_hash(public, "companion_sha256")
        companions.append({"raw_client_intent_id": request_id, "record": public})
        if not upstream_opened:
            continue
        traffic_source = ingress_record.get("traffic_source")
        terminal.append(
            {
                SOURCE_ID_KEY: "gateway_terminal",
                "schema": LEDGER_SCHEMA,
                "timestamp_unix": ingress_record["timestamp_unix"],
                "client_intent_id": request_id,
                "intent_dedupe_eligible": (
                    traffic_source_dedupe_eligible(traffic_source)
                    if isinstance(traffic_source, str)
                    else False
                ),
                "provider_attempt_id": f"nginx:{request_id}",
                "request_sha256": ingress_record["request_sha256"],
                "route": "upstream",
                "terminal_state": "delivered" if 200 <= row["status"] < 400 else "failed",
                "input_tokens": ingress_record["input_tokens"],
                "input_token_accounting": "byte_estimate_v1",
                "traffic_source": traffic_source,
                "upstream_socket_opened": True,
                "avoided_call": False,
                "verification_status": "not_applicable",
                "verification_receipt_id": None,
                "projector_receipt_id": None,
                "gateway_status": row["status"],
                "gateway_companion_sha256": public["companion_sha256"],
            }
        )
    return terminal, companions, anomalies


def gateway_terminal_rows(
    gateway_rows: Iterable[Any], execution_rows: Iterable[Any]
) -> list[dict[str, Any]]:
    terminal, _, _ = reconcile_gateway_rows(gateway_rows, execution_rows)
    return terminal


def source_timestamp_fields(*sources: Any) -> dict[str, Any]:
    for source in sources:
        if not isinstance(source, dict):
            continue
        fields = {
            key: source[key]
            for key in ("timestamp_unix", "timestamp")
            if key in source
        }
        if fields:
            return fields
    return {}


def _raw_source_row(row: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in row.items() if key != SOURCE_ID_KEY}


def _traffic_class(row: dict[str, Any]) -> str:
    source = row.get("traffic_source")
    eligible = row.get("intent_dedupe_eligible")
    if source == "ordinary" and eligible is True:
        return "ordinary"
    if isinstance(source, str) and source != "ordinary" and eligible is False:
        return "excluded"
    return "unknown"


def _single_match(values: list[dict[str, Any]]) -> dict[str, Any] | None:
    return values[0] if len(values) == 1 else None


def _record_hash_without(record: dict[str, Any], field: str) -> str:
    return canonical_sha256({key: value for key, value in record.items() if key != field})


def build_evidence_records(
    rows: Iterable[Any],
    *,
    execution_rows: Iterable[Any] = (),
    gateway_companions: Iterable[dict[str, Any]] = (),
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    ingress, decisions, anomalies = execution_companions(execution_rows)
    gateway = list(gateway_companions)
    records: list[dict[str, Any]] = []
    for supplied in rows:
        if not isinstance(supplied, dict) or supplied.get("schema") != LEDGER_SCHEMA:
            continue
        source_id = supplied.get(SOURCE_ID_KEY, "terminal_ledger")
        row = _raw_source_row(supplied)
        traffic_class = _traffic_class(row)
        intent = row.get("client_intent_id")
        intent_hash = (
            hashlib.sha256(intent.encode("utf-8")).hexdigest()
            if isinstance(intent, str) and intent
            else None
        )
        timestamp = row_timestamp_unix(row)
        request_hash = row.get("request_sha256")
        accounting = m3_input_accounting(row)
        tokens = accounting[0] if accounting is not None else None
        accounting_name = accounting[1] if accounting is not None else None
        upstream_opened = row.get("upstream_socket_opened")
        route = row.get("route")
        route_class = "local" if route in LOCAL_ROUTES else "upstream" if route == "upstream" else "other"
        delivered = row.get("terminal_state") == "delivered"
        incomplete: list[str] = []
        if traffic_class != "excluded":
            if traffic_class != "ordinary":
                incomplete.append("ordinary_traffic_authority_missing")
            if intent_hash is None:
                incomplete.append("client_intent_id_missing")
            if timestamp is None:
                incomplete.append("authoritative_timestamp_missing_or_invalid")
            elif timestamp > int(time.time()) + COMPANION_TIMESTAMP_SKEW_SECONDS:
                incomplete.append("authoritative_timestamp_in_future")
            if not valid_sha256(request_hash):
                incomplete.append("request_sha256_missing_or_invalid")
            if accounting is None:
                incomplete.append("input_token_accounting_missing_or_invalid")
            if type(upstream_opened) is not bool:
                incomplete.append("upstream_socket_opened_missing_or_non_boolean")

        ingress_matches = [
            item["record"]
            for item in ingress
            if intent is not None
            and item["raw_client_intent_id"] == intent
            and item["record"]["request_sha256"] == request_hash
            and item["record"]["input_tokens"] == tokens
            and timestamp is not None
            and abs(item["record"]["timestamp_unix"] - timestamp)
            <= COMPANION_TIMESTAMP_SKEW_SECONDS
        ]
        ingress_record = _single_match(ingress_matches)
        gateway_matches = [
            item["record"]
            for item in gateway
            if intent is not None
            and item["raw_client_intent_id"] == intent
            and item["record"]["request_sha256"] == request_hash
            and timestamp is not None
            and abs(item["record"]["ingress_timestamp_unix"] - timestamp)
            <= COMPANION_TIMESTAMP_SKEW_SECONDS
        ]
        gateway_record = _single_match(gateway_matches)
        reported_route_class = route_class
        if gateway_record is not None:
            route_class = (
                "upstream"
                if gateway_record["upstream_socket_opened"] is True
                else "local"
            )
        if traffic_class != "excluded":
            if ingress_record is None:
                incomplete.append("ingress_companion_missing_or_ambiguous")
            if gateway_record is None:
                incomplete.append("gateway_companion_missing_or_ambiguous")

        verifier_schema = row.get("verifier_schema")
        verification_receipt = row.get("verification_receipt_id")
        projector_receipt = row.get("projector_receipt_id")
        post_verifier_receipt = canonical_post_verifier_receipt(
            row.get(POST_VERIFIER_RECEIPT_FIELD)
        )
        package_hash = package_id_sha256(row.get("package_id"))
        decision_record = None
        binding_hash = None
        if route_class == "local" and delivered:
            if traffic_class != "excluded" and not m3_receipt_covered(row):
                incomplete.append("verifier_or_projector_receipt_invalid")
            decision_matches = [
                item["record"]
                for item in decisions
                if item["record"]["request_sha256"] == request_hash
                and item["record"]["input_tokens"] == tokens
                and item["record"]["verification_receipt_id"] == verification_receipt
                and item["record"]["projector_receipt_id"] == projector_receipt
                and item["record"]["verifier_schema"] == verifier_schema
                and item["record"][POST_VERIFIER_RECEIPT_FIELD]
                == post_verifier_receipt
                and item["record"]["package_id_sha256"] == package_hash
                and timestamp is not None
                and abs(item["record"]["timestamp_unix"] - timestamp)
                <= COMPANION_TIMESTAMP_SKEW_SECONDS
            ]
            decision_record = _single_match(decision_matches)
            if traffic_class != "excluded" and decision_record is None:
                incomplete.append("execution_decision_companion_missing_or_ambiguous")
            if (
                m3_receipt_covered(row)
                and ingress_record is not None
                and decision_record is not None
                and gateway_record is not None
                and gateway_record["decision"] == "local"
                and gateway_record["upstream_socket_opened"] is False
                and intent_hash is not None
                and timestamp is not None
                and tokens is not None
            ):
                binding = {
                    "schema": RECEIPT_BINDING_SCHEMA,
                    "client_intent_id_sha256": intent_hash,
                    "source_row_sha256": canonical_sha256(row),
                    "timestamp_unix": timestamp,
                    "request_sha256": request_hash,
                    "input_tokens": tokens,
                    "verifier_schema": verifier_schema,
                    "verification_receipt_id": verification_receipt,
                    "projector_receipt_id": projector_receipt,
                    "ingress_companion_sha256": ingress_record["companion_sha256"],
                    "decision_companion_sha256": decision_record["companion_sha256"],
                    "gateway_companion_sha256": gateway_record["companion_sha256"],
                }
                binding_hash = canonical_sha256(binding)

        record = {
            "schema": INTENT_EVIDENCE_SCHEMA,
            "source_id": source_id,
            "source_row_sha256": canonical_sha256(row),
            "client_intent_id_sha256": intent_hash,
            "timestamp_unix": timestamp,
            "traffic_class": traffic_class,
            "intent_dedupe_eligible": row.get("intent_dedupe_eligible"),
            "request_sha256": request_hash if valid_sha256(request_hash) else None,
            "input_tokens": tokens,
            "input_token_accounting": accounting_name,
            "route_class": route_class,
            "reported_route_class": reported_route_class,
            "terminal_state": "delivered" if delivered else "other",
            "upstream_socket_opened": (
                upstream_opened if type(upstream_opened) is bool else None
            ),
            "verification_status": row.get("verification_status"),
            "verifier_schema": verifier_schema,
            "verification_receipt_id": verification_receipt,
            "projector_receipt_id": projector_receipt,
            "package_id_sha256": package_hash,
            POST_VERIFIER_RECEIPT_FIELD: post_verifier_receipt,
            "ingress_companion_sha256": (
                ingress_record["companion_sha256"] if ingress_record else None
            ),
            "decision_companion_sha256": (
                decision_record["companion_sha256"] if decision_record else None
            ),
            "gateway_companion_sha256": (
                gateway_record["companion_sha256"] if gateway_record else None
            ),
            "receipt_binding_sha256": binding_hash,
            "incomplete_reasons": sorted(set(incomplete)),
        }
        record["record_sha256"] = canonical_sha256(record)
        records.append(record)

    records.sort(
        key=lambda record: (
            record.get("client_intent_id_sha256") or "",
            record.get("timestamp_unix") if type(record.get("timestamp_unix")) is int else -1,
            record["source_id"],
            record["source_row_sha256"],
            record["record_sha256"],
        )
    )
    return records, anomalies


def _intent_groups(records: Iterable[dict[str, Any]]) -> tuple[
    dict[str, list[dict[str, Any]]], list[dict[str, Any]]
]:
    all_grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    unassigned: list[dict[str, Any]] = []
    for record in records:
        intent_hash = record.get("client_intent_id_sha256")
        timestamp = record.get("timestamp_unix")
        if not valid_sha256(intent_hash) or type(timestamp) is not int:
            if record.get("traffic_class") != "excluded":
                unassigned.append(record)
            continue
        all_grouped[intent_hash].append(record)
    grouped = {
        intent_hash: intent_records
        for intent_hash, intent_records in all_grouped.items()
        if any(record.get("traffic_class") != "excluded" for record in intent_records)
    }
    return grouped, unassigned


def _group_conflicts(records: list[dict[str, Any]]) -> list[str]:
    conflicts: list[str] = []
    for field in ("request_sha256", "input_tokens", "input_token_accounting"):
        values = {record.get(field) for record in records if record.get(field) is not None}
        if len(values) > 1:
            conflicts.append(f"{field}_conflict")
    if len({record.get("traffic_class") for record in records}) > 1:
        conflicts.append("traffic_class_conflict")
    if len({record.get("intent_dedupe_eligible") for record in records}) > 1:
        conflicts.append("intent_dedupe_eligibility_conflict")
    if any(count > 1 for count in Counter(record["source_row_sha256"] for record in records).values()):
        conflicts.append("duplicate_source_rows")
    local_bindings = [
        record["receipt_binding_sha256"]
        for record in records
        if record["route_class"] == "local"
        and record["terminal_state"] == "delivered"
        and record.get("receipt_binding_sha256") is not None
    ]
    if any(count > 1 for count in Counter(local_bindings).values()):
        conflicts.append("receipt_binding_reused")
    return conflicts


def _window_counters(records: list[dict[str, Any]]) -> dict[str, int]:
    grouped, unassigned = _intent_groups(records)
    counters = {
        "observed_client_intents": len(grouped) + len(unassigned),
        "dropped_incomplete_intents": len(unassigned),
        "dedupe_conflicts": 0,
        "eligible_client_intents": 0,
        "eligible_input_tokens": 0,
        "verified_avoided_calls": 0,
        "verified_avoided_input_tokens": 0,
        "eligible_local_delivered_intents": 0,
        "receipt_covered_local_delivered_intents": 0,
        "unresolved_local_outcomes": 0,
    }
    for intent_records in grouped.values():
        conflicts = _group_conflicts(intent_records)
        incomplete = any(record["incomplete_reasons"] for record in intent_records)
        if conflicts:
            counters["dedupe_conflicts"] += 1
            continue
        if incomplete or any(record["traffic_class"] != "ordinary" for record in intent_records):
            counters["dropped_incomplete_intents"] += 1
            continue
        tokens = intent_records[0]["input_tokens"]
        if type(tokens) is not int or tokens <= 0:
            counters["dropped_incomplete_intents"] += 1
            continue
        counters["eligible_client_intents"] += 1
        counters["eligible_input_tokens"] += tokens
        delivered_local = [
            record
            for record in intent_records
            if record["route_class"] == "local"
            and record["terminal_state"] == "delivered"
        ]
        receipt_covered = bool(delivered_local) and all(
            nonzero_sha256(record.get("receipt_binding_sha256"))
            for record in delivered_local
        )
        gateway_covered = all(
            nonzero_sha256(record.get("gateway_companion_sha256"))
            for record in intent_records
        )
        upstream_opened = any(
            record.get("upstream_socket_opened") is True for record in intent_records
        )
        if delivered_local:
            counters["eligible_local_delivered_intents"] += 1
            if receipt_covered:
                counters["receipt_covered_local_delivered_intents"] += 1
            if not receipt_covered or not gateway_covered or upstream_opened:
                counters["unresolved_local_outcomes"] += 1
        if delivered_local and receipt_covered and gateway_covered and not upstream_opened:
            counters["verified_avoided_calls"] += 1
            counters["verified_avoided_input_tokens"] += tokens
    return counters


def _bounded_m3_evidence_records(
    records: list[dict[str, Any]], *, as_of_unix: int
) -> tuple[list[dict[str, Any]], int]:
    today_start, _ = utc_day_bounds(as_of_unix)
    retention_start = today_start - M3_EVIDENCE_RETENTION_DAYS * UTC_DAY_SECONDS
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    unassigned: list[dict[str, Any]] = []
    for record in records:
        intent_hash = record.get("client_intent_id_sha256")
        if valid_sha256(intent_hash):
            grouped[intent_hash].append(record)
        elif record.get("traffic_class") != "excluded":
            unassigned.append(record)
    selected = list(unassigned)
    for intent_records in grouped.values():
        candidate = [
            record
            for record in intent_records
            if record.get("traffic_class") != "excluded"
        ]
        if not candidate:
            continue
        timestamps = [
            record["timestamp_unix"]
            for record in candidate
            if type(record.get("timestamp_unix")) is int
        ]
        if not timestamps or min(timestamps) >= retention_start:
            selected.extend(intent_records)
    selected.sort(
        key=lambda record: (
            record.get("client_intent_id_sha256") or "",
            record.get("timestamp_unix") if type(record.get("timestamp_unix")) is int else -1,
            record["source_id"],
            record["source_row_sha256"],
            record["record_sha256"],
        )
    )
    return selected, retention_start


def _build_source_reconciliation(
    source_receipts: Iterable[dict[str, Any]],
    evidence_records: list[dict[str, Any]],
    anomalies: Iterable[dict[str, Any]],
    *,
    generated_at_unix: int,
    records_truncated: bool,
) -> dict[str, Any]:
    sources = sorted(source_receipts, key=lambda receipt: str(receipt.get("source_id")))
    anomaly_values = sorted(
        anomalies, key=lambda anomaly: str(anomaly.get("anomaly_sha256"))
    )
    anomaly_samples = anomaly_values[:MAX_ANOMALY_RECORDS]
    candidate_records = [
        record for record in evidence_records if record["traffic_class"] != "excluded"
    ]
    counters = _window_counters(candidate_records)
    receipt_failures = sum(
        1
        for record in candidate_records
        if record["route_class"] == "local"
        and record["terminal_state"] == "delivered"
        and not nonzero_sha256(record.get("receipt_binding_sha256"))
    )
    gateway_failures = sum(
        1
        for record in candidate_records
        if not nonzero_sha256(record.get("gateway_companion_sha256"))
    )
    source_ids = [source.get("source_id") for source in sources]
    sources_complete = (
        source_ids == sorted(REQUIRED_SOURCE_IDS)
        and all(
            source.get("schema") == SOURCE_RECEIPT_SCHEMA
            and source.get("required") is True
            and source.get("source_kind") == "jsonl"
            and source.get("status") == "ok"
            and source.get("malformed_record_count") == 0
            for source in sources
        )
    )
    blockers: list[str] = []
    if not sources_complete:
        blockers.append("required_sources_incomplete")
    if anomaly_values:
        blockers.append("source_anomalies_present")
    if len(anomaly_values) > len(anomaly_samples):
        blockers.append("source_anomalies_truncated")
    if records_truncated:
        blockers.append("evidence_records_truncated")
    if counters["dropped_incomplete_intents"]:
        blockers.append("dropped_or_incomplete_intents")
    if counters["dedupe_conflicts"]:
        blockers.append("conflicting_intents")
    if receipt_failures:
        blockers.append("receipt_provenance_failures")
    if gateway_failures:
        blockers.append("gateway_companion_failures")
    reconciliation = {
        "schema": SOURCE_RECONCILIATION_SCHEMA,
        "generated_at_unix": generated_at_unix,
        "required_source_ids": list(REQUIRED_SOURCE_IDS),
        "sources": sources,
        "source_receipts_sha256": canonical_sha256(sources),
        "source_anomaly_count": len(anomaly_values),
        "source_anomaly_samples": anomaly_samples,
        "source_anomaly_samples_sha256": canonical_sha256(anomaly_samples),
        "evidence_record_count": len(evidence_records),
        "evidence_records_sha256": evidence_rows_sha256(evidence_records),
        "records_truncated": records_truncated,
        "observed_client_intents": counters["observed_client_intents"],
        "reconciled_client_intents": counters["eligible_client_intents"],
        "dropped_incomplete_intents": counters["dropped_incomplete_intents"],
        "conflicting_intents": counters["dedupe_conflicts"],
        "receipt_provenance_failures": receipt_failures,
        "gateway_companion_failures": gateway_failures,
        "blockers": sorted(set(blockers)),
        "complete": not blockers,
    }
    reconciliation["reconciliation_sha256"] = canonical_sha256(reconciliation)
    return reconciliation


def build_m3_windows(
    rows: Iterable[Any],
    *,
    as_of_unix: int,
    false_accept_rows: Iterable[Any] = (),
    evidence_records: list[dict[str, Any]] | None = None,
    source_reconciliation: dict[str, Any] | None = None,
    false_accept_evidence: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    if type(as_of_unix) is not int:
        raise TypeError("as_of_unix must be an integer")
    if evidence_records is None:
        evidence_records, _ = build_evidence_records(rows)
    grouped, _ = _intent_groups(evidence_records)
    windows: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for intent_records in grouped.values():
        first_seen = min(record["timestamp_unix"] for record in intent_records)
        window_start, _ = utc_day_bounds(first_seen)
        windows[window_start].extend(intent_records)

    false_count = (
        false_accept_evidence.get("false_accepts")
        if isinstance(false_accept_evidence, dict)
        and type(false_accept_evidence.get("false_accepts")) is int
        else None
    )
    if false_count is None:
        false_count = sum(
            value
            for row in false_accept_rows
            if (value := false_accept_value(row)) is not None
        )
    reconciliation_hash = (
        source_reconciliation.get("reconciliation_sha256")
        if isinstance(source_reconciliation, dict)
        else None
    )
    false_receipt_hash = (
        false_accept_evidence.get("receipt_sha256")
        if isinstance(false_accept_evidence, dict)
        else None
    )
    result: list[dict[str, Any]] = []
    settled_starts = [
        start
        for start in windows
        if start + UTC_DAY_SECONDS + M3_SETTLEMENT_SECONDS <= as_of_unix
    ]
    if not settled_starts:
        return result
    for window_start in range(
        min(settled_starts), max(settled_starts) + UTC_DAY_SECONDS, UTC_DAY_SECONDS
    ):
        window_end = window_start + UTC_DAY_SECONDS
        settled_at = window_end + M3_SETTLEMENT_SECONDS
        if settled_at > as_of_unix:
            continue
        records = sorted(windows.get(window_start, []), key=lambda record: record["record_sha256"])
        counters = _window_counters(records)
        eligible_tokens = counters["eligible_input_tokens"]
        avoided_tokens = counters["verified_avoided_input_tokens"]
        delivered = counters["eligible_local_delivered_intents"]
        covered = counters["receipt_covered_local_delivered_intents"]
        day = datetime.fromtimestamp(window_start, timezone.utc).strftime("%Y-%m-%d")
        result.append(
            {
                "schema": M3_WINDOW_SCHEMA,
                "window_id": f"utc-day:{day}",
                "window_start_unix": window_start,
                "window_end_unix": window_end,
                "settled_at_unix": settled_at,
                "traffic_class": "ordinary",
                "dedupe_key": "client_intent_id",
                "dedupe_scope": "global_first_seen",
                **counters,
                "input_token_saving_share_milli": ratio_milli(
                    avoided_tokens, eligible_tokens
                ),
                "receipt_coverage_milli": ratio_milli(covered, delivered),
                "false_accepts": false_count,
                "source_reconciliation_sha256": reconciliation_hash,
                "false_accept_evidence_receipt_sha256": false_receipt_hash,
                "evidence_record_count": len(records),
                "evidence_record_sha256s": [record["record_sha256"] for record in records],
                "evidence_rows_sha256": evidence_rows_sha256(records),
            }
        )
    return result


def _unverified_false_accept_receipt(
    false_accepts: int | None, reason: str | None
) -> dict[str, Any]:
    value = {
        "schema": FALSE_ACCEPT_EVIDENCE_SCHEMA,
        "source_id": "false_accept_metrics",
        "source_kind": "scalar_compatibility",
        "source_path": None,
        "status": reason or "unverified",
        "observed_at_unix": int(time.time()),
        "source_timestamp_unix": None,
        "source_timestamp_kind": None,
        "max_age_seconds": FALSE_ACCEPT_MAX_AGE_SECONDS,
        "fresh": False,
        "source_sha256": None,
        "source_size_bytes": None,
        "source_modified_at_unix_ns": None,
        "source_record_count": 0,
        "false_accepts": false_accepts if type(false_accepts) is int else None,
    }
    return with_hash(value, "receipt_sha256")


def _in_memory_source_receipts(rows: list[Any]) -> list[dict[str, Any]]:
    receipts: list[dict[str, Any]] = []
    for source_id in REQUIRED_SOURCE_IDS:
        count = len(rows) if source_id == "terminal_ledger" else 0
        value = {
            "schema": SOURCE_RECEIPT_SCHEMA,
            "source_id": source_id,
            "source_kind": "in_memory",
            "source_path": None,
            "required": True,
            "status": "unverified",
            "observed_at_unix": int(time.time()),
            "source_sha256": None,
            "source_size_bytes": None,
            "source_modified_at_unix_ns": None,
            "source_record_count": count,
            "parsed_record_count": count,
            "malformed_record_count": 0,
        }
        receipts.append(with_hash(value, "receipt_sha256"))
    return receipts


def reduce_terminal_rows(
    rows: Iterable[Any],
    *,
    false_accepts: int | None = 0,
    false_accept_evidence_available: bool = True,
    false_accept_evidence_reason: str | None = None,
    false_accept_rows: Iterable[Any] | None = None,
    as_of_unix: int | None = None,
    execution_rows: Iterable[Any] = (),
    gateway_companions: Iterable[dict[str, Any]] = (),
    source_receipts: Iterable[dict[str, Any]] | None = None,
    source_anomalies: Iterable[dict[str, Any]] = (),
    false_accept_evidence: dict[str, Any] | None = None,
    extra_reconciliation_anomalies: Iterable[dict[str, Any]] = (),
) -> dict[str, Any]:
    supplied_rows = list(rows)
    execution_values = list(execution_rows)
    fallback_decidability_by_request: dict[str, str] = {}
    fallback_decidability_reasons: Counter[str] = Counter()
    allowed_decidability_classes = {
        "potentially_cpu_executable",
        "not_executable_current_evidence",
        "unsupported_by_current_dsl",
    }
    for row in execution_values:
        if (
            isinstance(row, dict)
            and row.get("schema") == "nando.response-actor-fallback.v1"
            and nonzero_sha256(row.get("request_sha256"))
        ):
            class_name = row.get("cpu_decidability_class")
            reason = row.get("cpu_decidability_reason")
            if class_name not in allowed_decidability_classes:
                class_name = "unclassified"
            fallback_decidability_by_request[row["request_sha256"]] = class_name
            if isinstance(reason, str) and reason:
                fallback_decidability_reasons[reason] += 1
    fallback_decidability_counts = Counter(fallback_decidability_by_request.values())
    gateway_values = list(gateway_companions)
    generated_at = int(time.time())
    all_evidence_records, evidence_anomalies = build_evidence_records(
        supplied_rows,
        execution_rows=execution_values,
        gateway_companions=gateway_values,
    )
    evidence_as_of = generated_at if as_of_unix is None else as_of_unix
    evidence_records, evidence_retention_start = _bounded_m3_evidence_records(
        all_evidence_records, as_of_unix=evidence_as_of
    )
    records_truncated = len(evidence_records) > MAX_EVIDENCE_RECORDS
    if records_truncated:
        evidence_records = evidence_records[:MAX_EVIDENCE_RECORDS]
    receipts = (
        list(source_receipts)
        if source_receipts is not None
        else _in_memory_source_receipts(supplied_rows)
    )
    anomalies = [
        *source_anomalies,
        *extra_reconciliation_anomalies,
        *evidence_anomalies,
    ]
    reconciliation = _build_source_reconciliation(
        receipts,
        evidence_records,
        anomalies,
        generated_at_unix=generated_at,
        records_truncated=records_truncated,
    )
    false_receipt = false_accept_evidence or _unverified_false_accept_receipt(
        false_accepts,
        None
        if false_accept_evidence_available
        else false_accept_evidence_reason or "unavailable",
    )
    false_count = false_receipt.get("false_accepts")
    false_available = (
        false_receipt.get("schema") == FALSE_ACCEPT_EVIDENCE_SCHEMA
        and false_receipt.get("source_kind") == "json"
        and false_receipt.get("status") == "ok"
        and false_receipt.get("fresh") is True
        and type(false_count) is int
        and false_count >= 0
        and nonzero_sha256(false_receipt.get("source_sha256"))
        and nonzero_sha256(false_receipt.get("receipt_sha256"))
    )

    valid = [
        _raw_source_row(row)
        for row in supplied_rows
        if isinstance(row, dict)
        and row.get("schema") == LEDGER_SCHEMA
        and isinstance(row.get("client_intent_id"), str)
        and row["client_intent_id"]
    ]
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in valid:
        grouped[row["client_intent_id"]].append(row)
    evidence_by_source = defaultdict(list)
    for record in all_evidence_records:
        evidence_by_source[record["source_row_sha256"]].append(record)

    provider_attempt_ids = {
        row["provider_attempt_id"]
        for row in valid
        if isinstance(row.get("provider_attempt_id"), str)
    }
    dedupe_eligible_intents = 0
    dedupe_ineligible_intents = 0
    total_input_tokens = 0
    actual_local_accepts = 0
    verified_local_accepts = 0
    avoided_calls = 0
    avoided_input_tokens = 0
    unresolved_local_outcomes = 0
    missing_evidence_receipts = 0
    audited_calls_counted_as_avoided = 0

    for intent_rows in grouped.values():
        eligible = all(row.get("intent_dedupe_eligible") is True for row in intent_rows)
        if eligible:
            dedupe_eligible_intents += 1
        else:
            dedupe_ineligible_intents += 1
        input_tokens = max(
            (row.get("input_tokens") for row in intent_rows if type(row.get("input_tokens")) is int),
            default=0,
        )
        if eligible:
            total_input_tokens += input_tokens
        explicit_upstream = all(
            type(row.get("upstream_socket_opened")) is bool for row in intent_rows
        )
        upstream_opened = any(row.get("upstream_socket_opened") is True for row in intent_rows)
        delivered_local = [
            row
            for row in intent_rows
            if row.get("route") in LOCAL_ROUTES and row.get("terminal_state") == "delivered"
            and any(
                record.get("route_class") == "local"
                for record in evidence_by_source.get(canonical_sha256(row), [])
            )
        ]
        provenance_covered = []
        for row in delivered_local:
            records = evidence_by_source.get(canonical_sha256(row), [])
            provenance_covered.append(
                len(records) == 1
                and nonzero_sha256(records[0].get("receipt_binding_sha256"))
                and nonzero_sha256(records[0].get("gateway_companion_sha256"))
            )
        syntactic_receipts = [m3_receipt_covered(row) for row in delivered_local]
        verified = (
            bool(delivered_local)
            and all(syntactic_receipts)
            and all(provenance_covered)
            and explicit_upstream
            and not upstream_opened
        )
        if delivered_local:
            actual_local_accepts += 1
            if verified:
                verified_local_accepts += 1
            else:
                unresolved_local_outcomes += 1
            missing_evidence_receipts += sum(
                1
                for syntax, provenance in zip(
                    syntactic_receipts, provenance_covered, strict=True
                )
                if not syntax or not provenance
            )
        if eligible and verified:
            avoided_calls += 1
            avoided_input_tokens += input_tokens

        audited_calls_counted_as_avoided += sum(
            1
            for row in intent_rows
            if row.get("route") in {"shadow", "audit", "canary", "local_canary"}
            and row.get("avoided_call") is True
        )

    verification_coverage_milli = ratio_milli(
        verified_local_accepts, actual_local_accepts
    )
    token_saving_share_milli = ratio_milli(avoided_input_tokens, total_input_tokens)
    call_saving_share_milli = ratio_milli(avoided_calls, dedupe_eligible_intents)
    hard_gate_pass = (
        reconciliation["complete"] is True
        and false_available
        and false_count == 0
        and actual_local_accepts == verified_local_accepts
        and unresolved_local_outcomes == 0
        and missing_evidence_receipts == 0
        and audited_calls_counted_as_avoided == 0
    )
    m1_pass = (
        hard_gate_pass
        and dedupe_eligible_intents >= 10_000
        and avoided_calls >= 100
        and token_saving_share_milli >= 10
    )
    m3_evidence = {
        "schema": M3_EVIDENCE_SCHEMA,
        "record_schema": INTENT_EVIDENCE_SCHEMA,
        "record_count": len(evidence_records),
        "retention_start_unix": evidence_retention_start,
        "post_verifier_receipt_schema": POST_VERIFIER_RECEIPT_SCHEMA,
        "post_verifier_receipt_fields": list(POST_VERIFIER_RECEIPT_FIELDS),
        "records_truncated": records_truncated,
        "records_sha256": evidence_rows_sha256(evidence_records),
        "records": evidence_records,
    }
    snapshot = {
        "schema": SNAPSHOT_SCHEMA,
        "generated_at_unix": generated_at,
        "terminal_rows": len(valid),
        "unique_client_intents": len(grouped),
        "dedupe_eligible_client_intents": dedupe_eligible_intents,
        "dedupe_ineligible_client_intents": dedupe_ineligible_intents,
        "unique_provider_attempts": len(provider_attempt_ids),
        "global_input_tokens": total_input_tokens,
        "actual_local_accepts": actual_local_accepts,
        "verified_local_accepts": verified_local_accepts,
        "avoided_calls": avoided_calls,
        "avoided_input_tokens": avoided_input_tokens,
        "call_saving_share_milli": call_saving_share_milli,
        "input_token_saving_share_milli": token_saving_share_milli,
        "verification_coverage_milli": verification_coverage_milli,
        "unresolved_local_outcomes": unresolved_local_outcomes,
        "missing_evidence_receipts": missing_evidence_receipts,
        "audited_calls_counted_as_avoided": audited_calls_counted_as_avoided,
        "receipt_provenance_failures": reconciliation["receipt_provenance_failures"],
        "dropped_incomplete_intents": reconciliation["dropped_incomplete_intents"],
        "dedupe_conflicts": reconciliation["conflicting_intents"],
        "false_accepts": false_count if type(false_count) is int else None,
        "false_accept_evidence_available": false_available,
        "false_accept_evidence": false_receipt,
        "cpu_decidability": {
            "schema": "nando.cpu-decidability-summary.v1",
            "contract": "state_before_observation_only.v1",
            "classified_fallback_requests": sum(
                count
                for class_name, count in fallback_decidability_counts.items()
                if class_name != "unclassified"
            ),
            "unclassified_fallback_requests": fallback_decidability_counts.get(
                "unclassified", 0
            ),
            "classes": {
                class_name: fallback_decidability_counts.get(class_name, 0)
                for class_name in sorted(allowed_decidability_classes)
            },
            "reasons": dict(sorted(fallback_decidability_reasons.items())),
        },
        "source_reconciliation": reconciliation,
        "m3_evidence": m3_evidence,
        "hard_gate_pass": hard_gate_pass,
        "product_m1_pass": m1_pass,
        "boundary": "deduplicated client-intent economics; only source-reconciled verified local delivery with explicit no-upstream authority counts as avoided",
    }
    if not false_available:
        snapshot["false_accept_evidence_reason"] = (
            false_accept_evidence_reason
            or f"false_accept_evidence_{false_receipt.get('status', 'unavailable')}"
        )
    snapshot["m3_windows"] = build_m3_windows(
        valid,
        as_of_unix=generated_at if as_of_unix is None else as_of_unix,
        false_accept_rows=valid if false_accept_rows is None else false_accept_rows,
        evidence_records=evidence_records,
        source_reconciliation=reconciliation,
        false_accept_evidence=false_receipt,
    )
    return snapshot


def reduce_source_paths(
    *,
    ledger_path: Path,
    gateway_path: Path,
    execution_path: Path,
    false_accept_path: Path,
    as_of_unix: int | None = None,
    source_prefix_sizes: dict[str, int] | None = None,
) -> dict[str, Any]:
    observed_at = int(time.time())
    boundaries = source_prefix_sizes or {}
    ledger_rows, ledger_receipt, ledger_anomalies = read_jsonl_source(
        ledger_path,
        "terminal_ledger",
        observed_at_unix=observed_at,
        prefix_size_bytes=boundaries.get("terminal_ledger"),
    )
    gateway_rows, gateway_receipt, gateway_anomalies = read_jsonl_source(
        gateway_path,
        "gateway_terminal",
        observed_at_unix=observed_at,
        prefix_size_bytes=boundaries.get("gateway_terminal"),
    )
    execution_rows, execution_receipt, execution_anomalies = read_jsonl_source(
        execution_path,
        "execution_events",
        observed_at_unix=observed_at,
        prefix_size_bytes=boundaries.get("execution_events"),
    )
    gateway_terminal, gateway_companions, reconciliation_anomalies = (
        reconcile_gateway_rows(gateway_rows, execution_rows)
    )
    tagged_ledger = [
        {**row, SOURCE_ID_KEY: "terminal_ledger"}
        if isinstance(row, dict)
        else row
        for row in ledger_rows
    ]
    false_receipt = read_false_accept_evidence(
        false_accept_path, observed_at_unix=observed_at
    )
    return reduce_terminal_rows(
        [*tagged_ledger, *gateway_terminal],
        false_accepts=false_receipt.get("false_accepts"),
        false_accept_evidence_available=(
            false_receipt.get("status") == "ok" and false_receipt.get("fresh") is True
        ),
        false_accept_evidence_reason=(
            None
            if false_receipt.get("status") == "ok" and false_receipt.get("fresh") is True
            else f"false_accept_evidence_{false_receipt.get('status', 'unavailable')}"
        ),
        as_of_unix=as_of_unix,
        execution_rows=execution_rows,
        gateway_companions=gateway_companions,
        source_receipts=[ledger_receipt, gateway_receipt, execution_receipt],
        source_anomalies=[
            *ledger_anomalies,
            *gateway_anomalies,
            *execution_anomalies,
        ],
        false_accept_evidence=false_receipt,
        extra_reconciliation_anomalies=reconciliation_anomalies,
    )


def read_jsonl(path: Path) -> list[Any]:
    """Compatibility reader; callers needing claims must use its source receipt."""
    rows, _, _ = read_jsonl_source(path, "compatibility")
    return rows


def atomic_write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, ensure_ascii=False, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, 0o644)
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--ledger",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/economics-terminal.jsonl"),
    )
    parser.add_argument(
        "--gateway-log",
        type=Path,
        default=Path("/var/lib/nando-gateway/economics-access.jsonl"),
    )
    parser.add_argument(
        "--execution-events",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/execution-events.jsonl"),
    )
    parser.add_argument(
        "--transition-metrics",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/metrics.json"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("/var/lib/nando-wave/transition/economics.json"),
    )
    args = parser.parse_args()
    snapshot = reduce_source_paths(
        ledger_path=args.ledger,
        gateway_path=args.gateway_log,
        execution_path=args.execution_events,
        false_accept_path=args.transition_metrics,
    )
    atomic_write_json(args.output, snapshot)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
