#!/usr/bin/env python3
"""Independently verify source-reconciled stable M3 economics windows."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


WINDOW_SCHEMA = "nando.economics-m3-window.v1"
RESULT_SCHEMA = "nando.live-transition-m3-evaluation.v1"
SNAPSHOT_SCHEMA = "nando.economics-snapshot.v1"
SOURCE_RECEIPT_SCHEMA = "nando.economics-source-receipt.v1"
SOURCE_RECONCILIATION_SCHEMA = "nando.economics-source-reconciliation.v1"
FALSE_ACCEPT_EVIDENCE_SCHEMA = "nando.false-accept-evidence-receipt.v1"
M3_EVIDENCE_SCHEMA = "nando.economics-m3-evidence.v1"
INTENT_EVIDENCE_SCHEMA = "nando.economics-intent-evidence.v1"
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
REQUIRED_SOURCE_IDS = ("terminal_ledger", "gateway_terminal", "execution_events")
DAY_SECONDS = 86_400
FALSE_ACCEPT_MAX_AGE_SECONDS = DAY_SECONDS
TIMESTAMP_FUTURE_SKEW_SECONDS = 5
MAX_EVIDENCE_RECORDS = 100_000
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def _load_ledger() -> Any:
    path = Path(__file__).with_name("nando-economics-ledger.py")
    spec = importlib.util.spec_from_file_location("nando_economics_ledger_gate", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("economics_ledger_import_failed")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def is_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def valid_sha256(value: Any) -> bool:
    return isinstance(value, str) and SHA256_RE.fullmatch(value) is not None


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


def hash_without(value: dict[str, Any], field: str) -> str:
    return canonical_sha256({key: item for key, item in value.items() if key != field})


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


def post_verifier_receipt_covered(record: Any) -> bool:
    if not isinstance(record, dict):
        return False
    receipt = canonical_post_verifier_receipt(
        record.get(POST_VERIFIER_RECEIPT_FIELD)
    )
    return (
        receipt is not None
        and canonical_sha256(receipt) == record.get("verification_receipt_id")
        and receipt["evidence_sha256"] == record.get("request_sha256")
        and receipt["output_sha256"] == record.get("projector_receipt_id")
        and receipt["package_id_sha256"] == record.get("package_id_sha256")
    )


def evidence_rows_sha256(rows: list[dict[str, Any]]) -> str:
    material = "\n".join(sorted(canonical_json(row) for row in rows)).encode("utf-8")
    return hashlib.sha256(material).hexdigest()


def ratio_milli(numerator: int, denominator: int) -> int:
    return numerator * 1000 // denominator if denominator > 0 else 0


def _base_result() -> dict[str, Any]:
    return {
        "schema": RESULT_SCHEMA,
        "safety_veto": False,
        "source_authority_valid": False,
        "false_accept_authority_valid": False,
        "evidence_authority_valid": False,
        "m3_verdict": "WATCH",
        "candidate_window_count": 0,
        "passing_window_count": 0,
        "m3_blockers": [],
        "windows": [],
    }


def _append_unique(target: list[str], blocker: str) -> None:
    if blocker not in target:
        target.append(blocker)


def _receipt_hash_valid(receipt: Any, field: str) -> bool:
    return (
        isinstance(receipt, dict)
        and nonzero_sha256(receipt.get(field))
        and receipt[field] == hash_without(receipt, field)
    )


def _stable_source_receipt(receipt: dict[str, Any]) -> dict[str, Any]:
    fields = (
        "schema",
        "source_id",
        "source_kind",
        "source_boundary_kind",
        "source_path",
        "required",
        "status",
        "source_sha256",
        "source_size_bytes",
        "source_record_count",
        "parsed_record_count",
        "malformed_record_count",
    )
    return {field: receipt.get(field) for field in fields}


def _stable_false_receipt(receipt: dict[str, Any]) -> dict[str, Any]:
    fields = (
        "schema",
        "source_id",
        "source_kind",
        "source_path",
        "status",
        "source_timestamp_unix",
        "source_timestamp_kind",
        "max_age_seconds",
        "source_sha256",
        "source_size_bytes",
        "source_modified_at_unix_ns",
        "source_record_count",
        "false_accepts",
    )
    return {field: receipt.get(field) for field in fields}


def _fresh_source_snapshot(
    economics: dict[str, Any], policy: dict[str, Any], now_unix: int
) -> dict[str, Any]:
    reconciliation = economics.get("source_reconciliation")
    false_receipt = economics.get("false_accept_evidence")
    if not isinstance(reconciliation, dict) or not isinstance(false_receipt, dict):
        raise ValueError("authority_receipts_missing")
    sources = reconciliation.get("sources")
    if not isinstance(sources, list):
        raise ValueError("source_receipts_missing")
    receipt_paths = {
        source.get("source_id"): source.get("source_path")
        for source in sources
        if isinstance(source, dict)
    }
    configured = policy.get("m3_authority_sources")
    if not isinstance(configured, dict) or set(configured) != {
        *REQUIRED_SOURCE_IDS,
        "false_accept_metrics",
    }:
        raise ValueError("authority_source_contract_invalid")
    paths = {
        source_id: str(Path(path).expanduser().resolve(strict=False))
        for source_id, path in configured.items()
        if isinstance(path, str) and path
    }
    if set(paths) != set(configured):
        raise ValueError("authority_source_contract_invalid")
    if set(receipt_paths) != set(REQUIRED_SOURCE_IDS) or any(
        not isinstance(receipt_paths[source_id], str) or not receipt_paths[source_id]
        for source_id in REQUIRED_SOURCE_IDS
    ):
        raise ValueError("required_source_paths_invalid")
    if any(receipt_paths[source_id] != paths[source_id] for source_id in REQUIRED_SOURCE_IDS):
        raise ValueError("foreign_source_receipt")
    false_path = false_receipt.get("source_path")
    if not isinstance(false_path, str) or false_path != paths["false_accept_metrics"]:
        raise ValueError("false_accept_source_path_invalid")
    ledger = _load_ledger()
    prefix_sizes = {
        source_id: source.get("source_size_bytes")
        for source_id, source in (
            (source_id, next(
                source for source in sources
                if isinstance(source, dict) and source.get("source_id") == source_id
            ))
            for source_id in REQUIRED_SOURCE_IDS
        )
    }
    if any(type(size) is not int or size < 0 for size in prefix_sizes.values()):
        raise ValueError("source_prefix_boundary_invalid")
    return ledger.reduce_source_paths(
        ledger_path=Path(paths["terminal_ledger"]),
        gateway_path=Path(paths["gateway_terminal"]),
        execution_path=Path(paths["execution_events"]),
        false_accept_path=Path(paths["false_accept_metrics"]),
        as_of_unix=now_unix,
        source_prefix_sizes=prefix_sizes,
    )


def _validate_false_accept_receipt(
    receipt: Any,
    fresh_receipt: Any,
    now_unix: int,
    blockers: list[str],
) -> bool:
    if not isinstance(receipt, dict):
        _append_unique(blockers, "false_accept_evidence_missing")
        return False
    valid = True
    if (
        receipt.get("schema") != FALSE_ACCEPT_EVIDENCE_SCHEMA
        or receipt.get("source_id") != "false_accept_metrics"
        or receipt.get("source_kind") != "json"
        or receipt.get("status") != "ok"
        or receipt.get("source_record_count") != 1
        or receipt.get("max_age_seconds") != FALSE_ACCEPT_MAX_AGE_SECONDS
        or type(receipt.get("false_accepts")) is not int
        or receipt["false_accepts"] < 0
        or not nonzero_sha256(receipt.get("source_sha256"))
        or not _receipt_hash_valid(receipt, "receipt_sha256")
    ):
        _append_unique(blockers, "false_accept_evidence_malformed")
        valid = False
    source_timestamp = receipt.get("source_timestamp_unix")
    observed_at = receipt.get("observed_at_unix")
    if not is_integer(source_timestamp) or not is_integer(observed_at):
        _append_unique(blockers, "false_accept_evidence_timestamp_invalid")
        valid = False
    else:
        source_age = now_unix - source_timestamp
        observed_age = now_unix - observed_at
        if (
            source_age < -TIMESTAMP_FUTURE_SKEW_SECONDS
            or source_age > FALSE_ACCEPT_MAX_AGE_SECONDS
            or observed_age < -TIMESTAMP_FUTURE_SKEW_SECONDS
            or observed_age > FALSE_ACCEPT_MAX_AGE_SECONDS
            or receipt.get("fresh") is not True
        ):
            _append_unique(blockers, "false_accept_evidence_stale")
            valid = False
    if not isinstance(fresh_receipt, dict) or _stable_false_receipt(receipt) != _stable_false_receipt(fresh_receipt):
        _append_unique(blockers, "false_accept_evidence_source_mismatch")
        valid = False
    if receipt.get("false_accepts") != 0:
        _append_unique(blockers, "false_accepts_nonzero")
        valid = False
    return valid


def _intent_groups(records: list[dict[str, Any]]) -> tuple[
    dict[str, list[dict[str, Any]]], list[dict[str, Any]]
]:
    all_grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    unassigned: list[dict[str, Any]] = []
    for record in records:
        intent_hash = record.get("client_intent_id_sha256")
        timestamp = record.get("timestamp_unix")
        if not valid_sha256(intent_hash) or not is_integer(timestamp):
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
    bindings = [
        record["receipt_binding_sha256"]
        for record in records
        if record.get("route_class") == "local"
        and record.get("terminal_state") == "delivered"
        and record.get("receipt_binding_sha256") is not None
    ]
    if any(count > 1 for count in Counter(bindings).values()):
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
        if _group_conflicts(intent_records):
            counters["dedupe_conflicts"] += 1
            continue
        if any(record.get("incomplete_reasons") for record in intent_records) or any(
            record.get("traffic_class") != "ordinary" for record in intent_records
        ):
            counters["dropped_incomplete_intents"] += 1
            continue
        tokens = intent_records[0].get("input_tokens")
        if not is_integer(tokens) or tokens <= 0:
            counters["dropped_incomplete_intents"] += 1
            continue
        counters["eligible_client_intents"] += 1
        counters["eligible_input_tokens"] += tokens
        delivered_local = [
            record
            for record in intent_records
            if record.get("route_class") == "local"
            and record.get("terminal_state") == "delivered"
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


def _binding_hash(record: dict[str, Any]) -> str | None:
    required = (
        "client_intent_id_sha256",
        "source_row_sha256",
        "timestamp_unix",
        "request_sha256",
        "input_tokens",
        "verifier_schema",
        "verification_receipt_id",
        "projector_receipt_id",
        "ingress_companion_sha256",
        "decision_companion_sha256",
        "gateway_companion_sha256",
    )
    if any(record.get(field) is None for field in required):
        return None
    material = {"schema": RECEIPT_BINDING_SCHEMA}
    material.update({field: record[field] for field in required})
    return canonical_sha256(material)


def _validate_evidence_records(
    economics: dict[str, Any],
    fresh: dict[str, Any],
    blockers: list[str],
) -> tuple[bool, list[dict[str, Any]]]:
    evidence = economics.get("m3_evidence")
    fresh_evidence = fresh.get("m3_evidence")
    if not isinstance(evidence, dict) or not isinstance(fresh_evidence, dict):
        _append_unique(blockers, "m3_evidence_missing")
        return False, []
    records = evidence.get("records")
    if (
        evidence.get("schema") != M3_EVIDENCE_SCHEMA
        or evidence.get("record_schema") != INTENT_EVIDENCE_SCHEMA
        or evidence.get("post_verifier_receipt_schema")
        != POST_VERIFIER_RECEIPT_SCHEMA
        or evidence.get("post_verifier_receipt_fields")
        != list(POST_VERIFIER_RECEIPT_FIELDS)
        or not isinstance(records, list)
        or len(records) > MAX_EVIDENCE_RECORDS
        or evidence.get("record_count") != len(records)
        or not is_integer(evidence.get("retention_start_unix"))
        or evidence.get("records_truncated") is not False
        or not nonzero_sha256(evidence.get("records_sha256"))
    ):
        _append_unique(blockers, "m3_evidence_malformed")
        return False, records if isinstance(records, list) else []
    valid = True
    try:
        recomputed_hash = evidence_rows_sha256(records)
    except (TypeError, ValueError):
        recomputed_hash = None
    if recomputed_hash != evidence.get("records_sha256"):
        _append_unique(blockers, "m3_evidence_rows_sha256_mismatch")
        valid = False
    for record in records:
        if (
            not isinstance(record, dict)
            or record.get("schema") != INTENT_EVIDENCE_SCHEMA
            or not nonzero_sha256(record.get("record_sha256"))
            or record.get("record_sha256") != hash_without(record, "record_sha256")
            or not nonzero_sha256(record.get("source_row_sha256"))
            or not isinstance(record.get("incomplete_reasons"), list)
        ):
            _append_unique(blockers, "m3_evidence_record_invalid")
            valid = False
            continue
        local_delivered = (
            record.get("traffic_class") == "ordinary"
            and record.get("route_class") == "local"
            and record.get("terminal_state") == "delivered"
        )
        if local_delivered:
            receipt_hashes = (
                record.get("verification_receipt_id"),
                record.get("projector_receipt_id"),
                record.get("ingress_companion_sha256"),
                record.get("decision_companion_sha256"),
                record.get("gateway_companion_sha256"),
                record.get("receipt_binding_sha256"),
            )
            if (
                any(not nonzero_sha256(value) for value in receipt_hashes)
                or not post_verifier_receipt_covered(record)
                or record.get("receipt_binding_sha256") != _binding_hash(record)
            ):
                _append_unique(blockers, "receipt_provenance_binding_invalid")
                valid = False
    fresh_records = fresh_evidence.get("records")
    if (
        evidence.get("retention_start_unix") != fresh_evidence.get("retention_start_unix")
        or not isinstance(fresh_records, list)
        or [canonical_json(row) for row in records] != [
        canonical_json(row) for row in fresh_records
        ]
    ):
        _append_unique(blockers, "m3_evidence_source_inventory_mismatch")
        valid = False
    return valid, records


def _validate_source_reconciliation(
    economics: dict[str, Any],
    fresh: dict[str, Any],
    records: list[dict[str, Any]],
    blockers: list[str],
) -> bool:
    reconciliation = economics.get("source_reconciliation")
    fresh_reconciliation = fresh.get("source_reconciliation")
    if not isinstance(reconciliation, dict) or not isinstance(fresh_reconciliation, dict):
        _append_unique(blockers, "source_reconciliation_missing")
        return False
    sources = reconciliation.get("sources")
    fresh_sources = fresh_reconciliation.get("sources")
    if not isinstance(sources, list) or not isinstance(fresh_sources, list):
        _append_unique(blockers, "source_receipts_missing")
        return False
    valid = True
    if (
        reconciliation.get("schema") != SOURCE_RECONCILIATION_SCHEMA
        or reconciliation.get("required_source_ids") != list(REQUIRED_SOURCE_IDS)
        or reconciliation.get("complete") is not True
        or reconciliation.get("blockers") != []
        or reconciliation.get("records_truncated") is not False
        or reconciliation.get("source_anomaly_count") != 0
        or reconciliation.get("dropped_incomplete_intents") != 0
        or reconciliation.get("conflicting_intents") != 0
        or reconciliation.get("receipt_provenance_failures") != 0
        or reconciliation.get("gateway_companion_failures") != 0
        or not _receipt_hash_valid(reconciliation, "reconciliation_sha256")
    ):
        _append_unique(blockers, "source_reconciliation_incomplete")
        valid = False
    if reconciliation.get("source_receipts_sha256") != canonical_sha256(sources):
        _append_unique(blockers, "source_receipts_sha256_mismatch")
        valid = False
    samples = reconciliation.get("source_anomaly_samples")
    if not isinstance(samples, list) or reconciliation.get(
        "source_anomaly_samples_sha256"
    ) != canonical_sha256(samples):
        _append_unique(blockers, "source_anomaly_receipt_invalid")
        valid = False
    if (
        reconciliation.get("evidence_record_count") != len(records)
        or reconciliation.get("evidence_records_sha256") != evidence_rows_sha256(records)
    ):
        _append_unique(blockers, "source_evidence_inventory_mismatch")
        valid = False

    by_id = {
        source.get("source_id"): source
        for source in sources
        if isinstance(source, dict)
    }
    fresh_by_id = {
        source.get("source_id"): source
        for source in fresh_sources
        if isinstance(source, dict)
    }
    if set(by_id) != set(REQUIRED_SOURCE_IDS) or set(fresh_by_id) != set(REQUIRED_SOURCE_IDS):
        _append_unique(blockers, "required_source_receipts_missing")
        valid = False
    else:
        for source_id in REQUIRED_SOURCE_IDS:
            source = by_id[source_id]
            if (
                source.get("schema") != SOURCE_RECEIPT_SCHEMA
                or source.get("source_kind") != "jsonl"
                or source.get("source_boundary_kind") != "immutable_prefix"
                or source.get("required") is not True
                or source.get("status") != "ok"
                or source.get("malformed_record_count") != 0
                or not nonzero_sha256(source.get("source_sha256"))
                or not _receipt_hash_valid(source, "receipt_sha256")
                or _stable_source_receipt(source)
                != _stable_source_receipt(fresh_by_id[source_id])
            ):
                _append_unique(blockers, f"source_receipt_invalid:{source_id}")
                valid = False

    counters = _window_counters(
        [record for record in records if record.get("traffic_class") != "excluded"]
    )
    expected = {
        "observed_client_intents": counters["observed_client_intents"],
        "reconciled_client_intents": counters["eligible_client_intents"],
        "dropped_incomplete_intents": counters["dropped_incomplete_intents"],
        "conflicting_intents": counters["dedupe_conflicts"],
        "receipt_provenance_failures": sum(
            1
            for record in records
            if record.get("traffic_class") != "excluded"
            and record.get("route_class") == "local"
            and record.get("terminal_state") == "delivered"
            and not nonzero_sha256(record.get("receipt_binding_sha256"))
        ),
        "gateway_companion_failures": sum(
            1
            for record in records
            if record.get("traffic_class") != "excluded"
            and not nonzero_sha256(record.get("gateway_companion_sha256"))
        ),
    }
    for field, value in expected.items():
        if reconciliation.get(field) != value:
            _append_unique(blockers, f"source_reconciliation_counter_mismatch:{field}")
            valid = False
    return valid


def _expected_windows(
    records: list[dict[str, Any]], now_unix: int, settlement: int
) -> dict[int, list[dict[str, Any]]]:
    grouped, _ = _intent_groups(records)
    windows: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for intent_records in grouped.values():
        first_seen = min(record["timestamp_unix"] for record in intent_records)
        start = first_seen - first_seen % DAY_SECONDS
        if start + DAY_SECONDS + settlement <= now_unix:
            windows[start].extend(intent_records)
    for window_records in windows.values():
        window_records.sort(key=lambda record: record["record_sha256"])
    if not windows:
        return {}
    return {
        start: windows.get(start, [])
        for start in range(min(windows), max(windows) + DAY_SECONDS, DAY_SECONDS)
    }


def _evaluate_window(
    window: Any,
    *,
    index: int,
    records: list[dict[str, Any]],
    expected_records: list[dict[str, Any]] | None,
    policy: dict[str, Any],
    integer_policy: dict[str, int],
    reconciliation_hash: str,
    false_receipt_hash: str,
    false_accepts: int,
) -> tuple[dict[str, Any], bool]:
    fallback_id = f"window[{index}]"
    if not isinstance(window, dict):
        return {
            "window_id": fallback_id,
            "verdict": "VETO",
            "blockers": ["window_malformed"],
        }, True
    window_id = window.get("window_id")
    summary: dict[str, Any] = {
        "window_id": window_id if isinstance(window_id, str) else fallback_id,
        "verdict": "PASS",
        "blockers": [],
    }
    blockers: list[str] = summary["blockers"]
    unsafe = False
    expected_strings = {
        "schema": policy.get("m3_window_schema", WINDOW_SCHEMA),
        "traffic_class": policy.get("m3_traffic_class", "ordinary"),
        "dedupe_key": policy.get("m3_dedupe_key", "client_intent_id"),
        "dedupe_scope": policy.get("m3_dedupe_scope", "global_first_seen"),
    }
    for field, expected in expected_strings.items():
        if window.get(field) != expected:
            blockers.append(f"{field}_mismatch")
            unsafe = True

    start = window.get("window_start_unix")
    end = window.get("window_end_unix")
    settled_at = window.get("settled_at_unix")
    if not all(is_integer(value) for value in (start, end, settled_at)):
        blockers.append("window_bounds_malformed")
        unsafe = True
    else:
        try:
            expected_id = "utc-day:" + datetime.fromtimestamp(
                start, timezone.utc
            ).strftime("%Y-%m-%d")
        except (OverflowError, OSError, ValueError):
            expected_id = None
        if (
            start < 0
            or start % integer_policy["duration"] != 0
            or end - start != integer_policy["duration"]
            or window_id != expected_id
        ):
            blockers.append("window_bounds_not_canonical_utc_day")
            unsafe = True
        if settled_at != end + integer_policy["settlement"]:
            blockers.append("window_settlement_invalid")
            unsafe = True

    record_ids = window.get("evidence_record_sha256s")
    selected_records = expected_records or []
    if (
        expected_records is None
        or not isinstance(record_ids, list)
        or any(not nonzero_sha256(value) for value in record_ids)
        or record_ids != [record["record_sha256"] for record in selected_records]
        or window.get("evidence_record_count") != len(selected_records)
        or not nonzero_sha256(window.get("evidence_rows_sha256"))
        or window.get("evidence_rows_sha256") != evidence_rows_sha256(selected_records)
    ):
        blockers.append("window_evidence_inventory_mismatch")
        unsafe = True
    if (
        window.get("source_reconciliation_sha256") != reconciliation_hash
        or window.get("false_accept_evidence_receipt_sha256") != false_receipt_hash
    ):
        blockers.append("window_authority_receipt_mismatch")
        unsafe = True

    expected_counters = _window_counters(selected_records)
    numeric_fields = (
        "observed_client_intents",
        "dropped_incomplete_intents",
        "dedupe_conflicts",
        "eligible_client_intents",
        "eligible_input_tokens",
        "verified_avoided_calls",
        "verified_avoided_input_tokens",
        "eligible_local_delivered_intents",
        "receipt_covered_local_delivered_intents",
        "unresolved_local_outcomes",
    )
    if any(not is_integer(window.get(field)) or window[field] < 0 for field in numeric_fields):
        blockers.append("window_counters_malformed")
        unsafe = True
    else:
        for field in numeric_fields:
            if window[field] != expected_counters[field]:
                blockers.append(f"window_counter_mismatch:{field}")
                unsafe = True
    if window.get("false_accepts") != false_accepts:
        blockers.append("window_false_accept_evidence_mismatch")
        unsafe = True

    eligible_tokens = expected_counters["eligible_input_tokens"]
    avoided_tokens = expected_counters["verified_avoided_input_tokens"]
    delivered = expected_counters["eligible_local_delivered_intents"]
    covered = expected_counters["receipt_covered_local_delivered_intents"]
    share = ratio_milli(avoided_tokens, eligible_tokens)
    coverage = ratio_milli(covered, delivered)
    summary["recomputed_input_token_saving_share_milli"] = share
    if window.get("input_token_saving_share_milli") != share:
        blockers.append("input_token_saving_share_ratio_inconsistent")
        unsafe = True
    if window.get("receipt_coverage_milli") != coverage:
        blockers.append("receipt_coverage_ratio_inconsistent")
        unsafe = True
    if avoided_tokens > eligible_tokens or covered > delivered:
        blockers.append("window_counter_bounds_invalid")
        unsafe = True
    for field, blocker in (
        ("dropped_incomplete_intents", "dropped_or_incomplete_intents_nonzero"),
        ("dedupe_conflicts", "dedupe_conflicts_nonzero"),
        ("unresolved_local_outcomes", "unresolved_local_outcomes_nonzero"),
    ):
        if expected_counters[field] != 0:
            blockers.append(blocker)
            unsafe = True
    if false_accepts != 0:
        blockers.append("false_accepts_nonzero")
        unsafe = True
    if coverage < integer_policy["minimum_coverage"]:
        blockers.append("receipt_coverage_below_minimum")
        unsafe = delivered > 0 or unsafe
    if expected_counters["eligible_client_intents"] < integer_policy["minimum_intents"]:
        blockers.append("eligible_client_intents_below_minimum")
    if expected_counters["verified_avoided_calls"] < integer_policy["minimum_calls"]:
        blockers.append("verified_avoided_calls_below_minimum")
    if share < integer_policy["minimum_share"]:
        blockers.append("verified_input_token_share_below_minimum")
    if blockers:
        summary["verdict"] = "VETO" if unsafe else "WATCH"
    return summary, unsafe


def evaluate(economics: Any, profile: Any, now_unix: int) -> dict[str, Any]:
    result = _base_result()
    if not is_integer(now_unix) or now_unix < 0:
        return _invalid(result, "evaluation_timestamp_invalid")
    policy = profile.get("response_runtime") if isinstance(profile, dict) else None
    if not isinstance(policy, dict):
        return _invalid(result, "m3_profile_invalid")
    if policy.get("m3_post_verifier_receipt_schema") != POST_VERIFIER_RECEIPT_SCHEMA:
        return _invalid(result, "m3_post_verifier_receipt_contract_invalid")
    integer_policy = {
        "duration": policy.get("m3_window_duration_seconds"),
        "settlement": policy.get("m3_window_settlement_seconds"),
        "required": policy.get("m3_required_consecutive_windows"),
        "minimum_intents": policy.get("minimum_m3_eligible_client_intents"),
        "minimum_calls": policy.get("minimum_m3_verified_avoided_calls"),
        "minimum_coverage": policy.get("minimum_m3_receipt_coverage_milli"),
        "minimum_share": policy.get("minimum_m3_input_token_saving_share_milli"),
    }
    if any(not is_integer(value) or value <= 0 for value in integer_policy.values()):
        return _invalid(result, "m3_profile_invalid")
    if integer_policy["duration"] != DAY_SECONDS or integer_policy["settlement"] != DAY_SECONDS:
        return _invalid(result, "m3_profile_not_stable_utc_day_contract")
    required = integer_policy["required"]
    result["required_consecutive_windows"] = required
    if not isinstance(economics, dict) or economics.get("schema") != SNAPSHOT_SCHEMA:
        return _invalid(result, "economics_snapshot_invalid")

    try:
        fresh = _fresh_source_snapshot(economics, policy, now_unix)
    except (OSError, RuntimeError, TypeError, ValueError, json.JSONDecodeError):
        return _invalid(result, "source_authority_recompute_failed")

    authority_blockers: list[str] = []
    evidence_valid, records = _validate_evidence_records(
        economics, fresh, authority_blockers
    )
    source_valid = _validate_source_reconciliation(
        economics, fresh, records, authority_blockers
    )
    false_valid = _validate_false_accept_receipt(
        economics.get("false_accept_evidence"),
        fresh.get("false_accept_evidence"),
        now_unix,
        authority_blockers,
    )
    result["source_authority_valid"] = source_valid
    result["false_accept_authority_valid"] = false_valid
    result["evidence_authority_valid"] = evidence_valid
    result["m3_blockers"].extend(authority_blockers)
    authority_unsafe = not (source_valid and false_valid and evidence_valid)

    if "m3_windows" not in economics:
        result["m3_blockers"].append("m3_windows_missing")
        result["safety_veto"] = True
        return result
    windows = economics["m3_windows"]
    if not isinstance(windows, list):
        return _invalid(result, "m3_windows_malformed")
    if not windows:
        result["m3_blockers"].append("m3_windows_empty")
        if _expected_windows(records, now_unix, integer_policy["settlement"]):
            result["m3_blockers"].append("m3_window_source_inventory_mismatch")
            authority_unsafe = True
        result["safety_veto"] = authority_unsafe
        return result

    previous_start: int | None = None
    for window in windows:
        start = window.get("window_start_unix") if isinstance(window, dict) else None
        if not is_integer(start) or (previous_start is not None and start <= previous_start):
            return _invalid(result, "m3_windows_not_ordered")
        previous_start = start

    expected = _expected_windows(records, now_unix, integer_policy["settlement"])
    actual_starts = [window.get("window_start_unix") for window in windows]
    if actual_starts != list(expected):
        result["m3_blockers"].append("m3_window_source_inventory_mismatch")
        authority_unsafe = True

    reconciliation = economics.get("source_reconciliation", {})
    false_receipt = economics.get("false_accept_evidence", {})
    reconciliation_hash = reconciliation.get("reconciliation_sha256")
    false_receipt_hash = false_receipt.get("receipt_sha256")
    false_accepts = false_receipt.get("false_accepts")
    false_accepts = false_accepts if type(false_accepts) is int else -1
    summaries: list[dict[str, Any]] = []
    unsafe = authority_unsafe
    previous_end: int | None = None
    for index, window in enumerate(windows):
        start = window.get("window_start_unix") if isinstance(window, dict) else None
        summary, window_unsafe = _evaluate_window(
            window,
            index=index,
            records=records,
            expected_records=expected.get(start) if is_integer(start) else None,
            policy=policy,
            integer_policy=integer_policy,
            reconciliation_hash=reconciliation_hash,
            false_receipt_hash=false_receipt_hash,
            false_accepts=false_accepts,
        )
        if previous_end is not None and is_integer(start) and start != previous_end:
            summary["blockers"].append("windows_not_ordered_and_contiguous")
            summary["verdict"] = "VETO"
            window_unsafe = True
        end = window.get("window_end_unix") if isinstance(window, dict) else None
        previous_end = end if is_integer(end) else None
        summaries.append(summary)
        unsafe = unsafe or window_unsafe

    candidate_summaries = summaries[-required:]
    result["windows"] = candidate_summaries
    result["candidate_window_count"] = len(candidate_summaries)
    if len(candidate_summaries) < required:
        result["m3_blockers"].append("insufficient_consecutive_windows")
    for summary in summaries:
        result["m3_blockers"].extend(
            f"{summary['window_id']}:{blocker}" for blocker in summary["blockers"]
        )

    latest = windows[-1] if isinstance(windows[-1], dict) else {}
    latest_end = latest.get("window_end_unix")
    if is_integer(latest_end):
        ready_at = latest_end + integer_policy["settlement"]
        result["latest_window_end_unix"] = latest_end
        result["latest_window_ready_at_unix"] = ready_at
        if now_unix < ready_at:
            result["m3_blockers"].append("latest_window_not_settled")
        elif now_unix >= ready_at + integer_policy["duration"]:
            result["m3_blockers"].append("latest_window_stale")

    result["safety_veto"] = unsafe
    result["passing_window_count"] = sum(
        summary["verdict"] == "PASS" for summary in candidate_summaries
    )
    if (
        not unsafe
        and len(candidate_summaries) == required
        and not result["m3_blockers"]
        and result["passing_window_count"] == required
    ):
        result["m3_verdict"] = "PASS"
    return result


def _invalid(result: dict[str, Any], blocker: str) -> dict[str, Any]:
    result["safety_veto"] = True
    _append_unique(result["m3_blockers"], blocker)
    return result


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--economics", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--now-unix", type=int, default=int(time.time()))
    args = parser.parse_args()
    try:
        economics = _load_json(args.economics)
        profile = _load_json(args.profile)
        report = evaluate(economics, profile, args.now_unix)
    except (FileNotFoundError, OSError, UnicodeError, json.JSONDecodeError):
        report = _invalid(_base_result(), "m3_authority_artifact_unreadable")
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
