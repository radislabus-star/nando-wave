#!/usr/bin/env python3
"""Fail-closed source reconciliation tests for stable M3 economics."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import tempfile
import time
import unittest
from pathlib import Path
from typing import Any


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
SPEC = importlib.util.spec_from_file_location(
    "nando_economics_ledger",
    BIN_DIR / "nando-economics-ledger.py",
)
LEDGER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(LEDGER)

DAY = 86_400


def digest(label: str) -> str:
    return hashlib.sha256(label.encode("utf-8")).hexdigest()


def post_verifier_receipt(
    *, package_id: str, request_sha256: str, output_sha256: str
) -> dict[str, str]:
    receipt = {
        "schema": LEDGER.POST_VERIFIER_RECEIPT_SCHEMA,
        "actor_sha256": digest(f"actor:{package_id}"),
        "verifier_sha256": digest(f"verifier:{package_id}"),
        "evidence_sha256": request_sha256,
        "output_sha256": output_sha256,
        "package_id_sha256": digest(package_id),
    }
    receipt["admission_binding_sha256"] = LEDGER.post_verifier_admission_binding(
        receipt
    )
    return receipt


def local_source_rows(
    intent: str,
    timestamp_unix: int,
    *,
    tokens: int = 100,
    request_sha256: str | None = None,
    verification_receipt_id: str | None = None,
    projector_receipt_id: str | None = None,
    traffic_source: str = "ordinary",
    **row_changes: Any,
) -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, Any]]:
    request_hash = request_sha256 or digest(f"request:{intent}:{timestamp_unix}")
    package_id = "test-package"
    projector = projector_receipt_id or digest(f"projector:{intent}:{timestamp_unix}")
    post_receipt = post_verifier_receipt(
        package_id=package_id,
        request_sha256=request_hash,
        output_sha256=projector,
    )
    verification = verification_receipt_id or LEDGER.post_verifier_receipt_id(
        post_receipt
    )
    row = {
        "schema": LEDGER.LEDGER_SCHEMA,
        "timestamp_unix": timestamp_unix,
        "client_intent_id": intent,
        "intent_dedupe_eligible": traffic_source == "ordinary",
        "provider_attempt_id": None,
        "request_sha256": request_hash,
        "route": "local_response_actor",
        "local_route": "response_actor:test-package",
        "package_id": package_id,
        "terminal_state": "delivered",
        "traffic_source": traffic_source,
        "input_tokens": tokens,
        "input_token_accounting": "byte_estimate_v1",
        "upstream_socket_opened": False,
        "avoided_call": True,
        "verification_status": "verified",
        "verification_receipt_id": verification,
        "projector_receipt_id": projector,
        "verifier_schema": "response_actor_independent_verifier.v1",
        LEDGER.POST_VERIFIER_RECEIPT_FIELD: post_receipt,
    }
    row.update(row_changes)
    ingress = {
        "schema": "nando.transition-execution-event.v1",
        "timestamp_unix": timestamp_unix,
        "event": "bridge_request",
        "client_intent_id": intent,
        "request_sha256": request_hash,
        "tokens": tokens,
        "traffic_source": traffic_source,
    }
    decision = {
        "schema": "nando.transition-execution-event.v1",
        "timestamp_unix": timestamp_unix,
        "event": "local_accept",
        "request_sha256": request_hash,
        "tokens": tokens,
        "route": "response_actor:test-package",
        "package_id": package_id,
        "verification_receipt_id": verification,
        "projector_receipt_id": projector,
        "verifier_schema": "response_actor_independent_verifier.v1",
        LEDGER.POST_VERIFIER_RECEIPT_FIELD: post_receipt,
    }
    gateway = {
        "schema": "nando.nginx-terminal.v1",
        "timestamp_unix": timestamp_unix,
        "request_id": intent,
        "status": 200,
        "upstream_status": "200",
        "upstream_addr": "127.0.0.1:18789",
    }
    return row, [ingress, decision], gateway


def upstream_source_rows(
    intent: str, timestamp_unix: int, *, tokens: int = 100
) -> tuple[dict[str, Any], dict[str, Any]]:
    request_hash = digest(f"request:{intent}:{timestamp_unix}")
    ingress = {
        "schema": "nando.transition-execution-event.v1",
        "timestamp_unix": timestamp_unix,
        "event": "bridge_request",
        "client_intent_id": intent,
        "request_sha256": request_hash,
        "tokens": tokens,
        "traffic_source": "ordinary",
    }
    gateway = {
        "schema": "nando.nginx-terminal.v1",
        "timestamp_unix": timestamp_unix,
        "request_id": intent,
        "status": 200,
        "upstream_status": "418, 200",
        "upstream_addr": "127.0.0.1:18789, 192.0.2.10:443",
    }
    return ingress, gateway


class EconomicsLedgerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.ledger_path = self.root / "economics-terminal.jsonl"
        self.gateway_path = self.root / "economics-access.jsonl"
        self.execution_path = self.root / "execution-events.jsonl"
        self.false_path = self.root / "metrics.json"
        self.now = int(time.time())
        self.today = self.now // DAY * DAY

    @staticmethod
    def write_jsonl(path: Path, rows: list[Any]) -> None:
        path.write_text(
            "".join(
                json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
                for row in rows
            ),
            encoding="utf-8",
        )

    def snapshot(
        self,
        ledger_rows: list[Any],
        gateway_rows: list[Any],
        execution_rows: list[Any],
        *,
        false_payload: Any | None = None,
        as_of_unix: int | None = None,
    ) -> dict[str, Any]:
        self.write_jsonl(self.ledger_path, ledger_rows)
        self.write_jsonl(self.gateway_path, gateway_rows)
        self.write_jsonl(self.execution_path, execution_rows)
        payload = (
            {"false_accepts": 0, "timestamp_unix": self.now}
            if false_payload is None
            else false_payload
        )
        self.false_path.write_text(json.dumps(payload), encoding="utf-8")
        return LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now if as_of_unix is None else as_of_unix,
        )

    def complete_day(
        self, *, start: int | None = None, local_tokens: int = 100, upstream_tokens: int = 100
    ) -> tuple[list[Any], list[Any], list[Any]]:
        timestamp = (start if start is not None else self.today - 2 * DAY) + 10
        local, local_events, local_gateway = local_source_rows(
            "ordinary-local", timestamp, tokens=local_tokens
        )
        upstream_event, upstream_gateway = upstream_source_rows(
            "ordinary-upstream", timestamp + 20, tokens=upstream_tokens
        )
        return [local], [local_gateway, upstream_gateway], [*local_events, upstream_event]

    def test_complete_receipt_chain_counts_avoidance_and_preserves_lifetime_v1(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        snapshot = self.snapshot(ledger_rows, gateway_rows, execution_rows)
        self.assertTrue(snapshot["source_reconciliation"]["complete"])
        self.assertTrue(snapshot["false_accept_evidence_available"])
        self.assertTrue(snapshot["hard_gate_pass"])
        self.assertEqual(snapshot["unique_client_intents"], 2)
        self.assertEqual(snapshot["dedupe_eligible_client_intents"], 2)
        self.assertEqual(snapshot["actual_local_accepts"], 1)
        self.assertEqual(snapshot["verified_local_accepts"], 1)
        self.assertEqual(snapshot["avoided_calls"], 1)
        self.assertEqual(snapshot["avoided_input_tokens"], 100)
        self.assertEqual(snapshot["input_token_saving_share_milli"], 500)
        self.assertEqual(snapshot["receipt_provenance_failures"], 0)
        window = snapshot["m3_windows"][0]
        self.assertEqual(window["eligible_client_intents"], 2)
        self.assertEqual(window["verified_avoided_calls"], 1)
        self.assertEqual(window["input_token_saving_share_milli"], 500)
        self.assertEqual(window["receipt_coverage_milli"], 1000)

    def test_gateway_upstream_row_requires_authoritative_timestamp_and_decision(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        ingress, gateway = upstream_source_rows("upstream", timestamp)
        terminal, companions, anomalies = LEDGER.reconcile_gateway_rows(
            [gateway], [ingress]
        )
        self.assertEqual(len(terminal), 1)
        self.assertEqual(len(companions), 1)
        self.assertEqual(anomalies, [])
        self.assertTrue(terminal[0]["upstream_socket_opened"])
        self.assertEqual(terminal[0]["timestamp_unix"], timestamp)

        for field in ("timestamp_unix", "upstream_status"):
            with self.subTest(field=field):
                malformed = dict(gateway)
                malformed.pop(field)
                terminal, _, anomalies = LEDGER.reconcile_gateway_rows(
                    [malformed], [ingress]
                )
                self.assertEqual(terminal, [])
                self.assertTrue(anomalies)

    def test_missing_unreadable_and_malformed_sources_are_visible(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        self.write_jsonl(self.ledger_path, ledger_rows)
        self.write_jsonl(self.execution_path, execution_rows)
        self.false_path.write_text(
            json.dumps({"false_accepts": 0, "timestamp_unix": self.now}),
            encoding="utf-8",
        )
        missing = LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now,
        )
        gateway_receipt = next(
            receipt
            for receipt in missing["source_reconciliation"]["sources"]
            if receipt["source_id"] == "gateway_terminal"
        )
        self.assertEqual(gateway_receipt["status"], "missing")
        self.assertFalse(missing["source_reconciliation"]["complete"])
        self.assertFalse(missing["hard_gate_pass"])

        unreadable_path = self.root / "gateway-directory"
        unreadable_path.mkdir()
        _, unreadable_receipt, _ = LEDGER.read_jsonl_source(
            unreadable_path, "gateway_terminal"
        )
        self.assertEqual(unreadable_receipt["status"], "unreadable")

        self.write_jsonl(self.gateway_path, gateway_rows)
        with self.ledger_path.open("ab") as handle:
            handle.write(b"{not-json\n")
        malformed = LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now,
        )
        reconciliation = malformed["source_reconciliation"]
        self.assertGreater(reconciliation["source_anomaly_count"], 0)
        self.assertIn("source_anomalies_present", reconciliation["blockers"])
        self.assertFalse(malformed["hard_gate_pass"])
        serialized = json.dumps(reconciliation["source_anomaly_samples"])
        self.assertNotIn("not-json", serialized)

        self.write_jsonl(self.ledger_path, [[]])
        _, non_object_receipt, non_object_anomalies = LEDGER.read_jsonl_source(
            self.ledger_path, "terminal_ledger"
        )
        self.assertEqual(non_object_receipt["parsed_record_count"], 0)
        self.assertEqual(non_object_receipt["malformed_record_count"], 1)
        self.assertTrue(non_object_anomalies)

        self.write_jsonl(self.ledger_path, [{"schema": "foreign.terminal.v1"}])
        _, foreign_receipt, foreign_anomalies = LEDGER.read_jsonl_source(
            self.ledger_path, "terminal_ledger"
        )
        self.assertEqual(foreign_receipt["malformed_record_count"], 1)
        self.assertTrue(foreign_anomalies)

    def test_missing_timestamp_or_non_boolean_socket_never_counts_as_avoided(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        for name, changes in (
            ("missing_timestamp", {"timestamp_unix": None}),
            ("missing_socket", {"upstream_socket_opened": None}),
            ("integer_socket", {"upstream_socket_opened": 0}),
        ):
            with self.subTest(name=name):
                row, events, gateway = local_source_rows(f"intent-{name}", timestamp)
                row.update(changes)
                snapshot = self.snapshot([row], [gateway], events)
                self.assertEqual(snapshot["avoided_calls"], 0)
                self.assertFalse(snapshot["hard_gate_pass"])
                self.assertFalse(snapshot["source_reconciliation"]["complete"])
                reasons = [
                    reason
                    for record in snapshot["m3_evidence"]["records"]
                    for reason in record["incomplete_reasons"]
                ]
                expected = (
                    "authoritative_timestamp_missing_or_invalid"
                    if name == "missing_timestamp"
                    else "upstream_socket_opened_missing_or_non_boolean"
                )
                self.assertIn(expected, reasons)

    def test_execution_fallback_diagnostics_are_valid_but_never_authority(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        execution_rows.append(
            {
                "schema": "nando.response-actor-fallback.v1",
                "event": "response_actor_fallback",
                "reason": "no_phase_routed_profile",
                "stage": "router",
                "cpu_decidability_class": "not_executable_current_evidence",
                "cpu_decidability_reason": "no_post_user_grounded_observation",
                "cpu_decidability_contract": "state_before_observation_only.v1",
                "request_sha256": digest("diagnostic-only"),
                "timestamp_unix": self.today - 2 * DAY + 30,
            }
        )
        snapshot = self.snapshot(ledger_rows, gateway_rows, execution_rows)
        receipt = next(
            source
            for source in snapshot["source_reconciliation"]["sources"]
            if source["source_id"] == "execution_events"
        )
        self.assertEqual(receipt["malformed_record_count"], 0)
        self.assertTrue(snapshot["source_reconciliation"]["complete"])
        self.assertEqual(snapshot["unique_client_intents"], 2)
        self.assertEqual(snapshot["actual_local_accepts"], 1)
        self.assertEqual(
            snapshot["cpu_decidability"]["classes"][
                "not_executable_current_evidence"
            ],
            1,
        )
        self.assertEqual(snapshot["cpu_decidability"]["classified_fallback_requests"], 1)

    def test_gateway_terminal_matches_delayed_exact_request_but_not_stale_or_early(self) -> None:
        ingress, gateway = upstream_source_rows(
            "delayed-request", self.today - 2 * DAY + 10
        )
        gateway["timestamp_unix"] = ingress["timestamp_unix"] + 90
        terminal, _, anomalies = LEDGER.reconcile_gateway_rows([gateway], [ingress])
        self.assertEqual(len(terminal), 1)
        self.assertEqual(anomalies, [])

        for offset in (-1, LEDGER.MAX_GATEWAY_REQUEST_DURATION_SECONDS + 1):
            with self.subTest(offset=offset):
                candidate = dict(gateway)
                candidate["timestamp_unix"] = ingress["timestamp_unix"] + offset
                terminal, _, anomalies = LEDGER.reconcile_gateway_rows(
                    [candidate], [ingress]
                )
                self.assertEqual(terminal, [])
                self.assertTrue(anomalies)

    def test_missing_gateway_companion_cannot_flip_local_to_avoided(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows("local", timestamp)
        for name, gateway_rows in (
            ("missing", []),
            ("missing_timestamp", [{key: value for key, value in gateway.items() if key != "timestamp_unix"}]),
            ("missing_decision", [{key: value for key, value in gateway.items() if key != "upstream_status"}]),
        ):
            with self.subTest(name=name):
                snapshot = self.snapshot([row], gateway_rows, events)
                self.assertEqual(snapshot["avoided_calls"], 0)
                self.assertGreater(
                    snapshot["source_reconciliation"]["gateway_companion_failures"], 0
                )
                self.assertFalse(snapshot["hard_gate_pass"])

    def test_gateway_upstream_outcome_overrides_local_backend_attempt(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows("local-timeout", timestamp)
        gateway["upstream_status"] = "504 : 200"
        gateway["upstream_addr"] = "127.0.0.1:18789 : 192.0.2.10:443"
        snapshot = self.snapshot([row], [gateway], events)

        self.assertEqual(snapshot["actual_local_accepts"], 0)
        self.assertEqual(snapshot["verified_local_accepts"], 0)
        self.assertEqual(snapshot["avoided_calls"], 0)
        self.assertEqual(snapshot["receipt_provenance_failures"], 0)
        record = next(
            record
            for record in snapshot["m3_evidence"]["records"]
            if record["reported_route_class"] == "local"
        )
        self.assertEqual(record["route_class"], "upstream")

    def test_conflicting_intent_stays_in_reconciliation_denominator(self) -> None:
        start = self.today - 2 * DAY + 10
        first, first_events, first_gateway = local_source_rows(
            "conflict", start, request_sha256=digest("request-a")
        )
        second, second_events, second_gateway = local_source_rows(
            "conflict", start + 20, request_sha256=digest("request-b")
        )
        snapshot = self.snapshot(
            [first, second],
            [first_gateway, second_gateway],
            [*first_events, *second_events],
        )
        reconciliation = snapshot["source_reconciliation"]
        self.assertEqual(reconciliation["observed_client_intents"], 1)
        self.assertEqual(reconciliation["conflicting_intents"], 1)
        self.assertEqual(reconciliation["reconciled_client_intents"], 0)
        self.assertFalse(reconciliation["complete"])
        window = snapshot["m3_windows"][0]
        self.assertEqual(window["observed_client_intents"], 1)
        self.assertEqual(window["dedupe_conflicts"], 1)
        self.assertEqual(window["eligible_client_intents"], 0)

    def test_incomplete_intent_stays_visible_in_window_denominator(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows(
            "incomplete", timestamp, request_sha256="forged"
        )
        snapshot = self.snapshot([row], [gateway], events)
        window = snapshot["m3_windows"][0]
        self.assertEqual(window["observed_client_intents"], 1)
        self.assertEqual(window["dropped_incomplete_intents"], 1)
        self.assertEqual(window["eligible_client_intents"], 0)
        self.assertFalse(snapshot["source_reconciliation"]["complete"])

    def test_receipt_ids_must_match_independent_execution_companion(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows("forged-receipt", timestamp)
        events[1]["verification_receipt_id"] = digest("different-verification")
        snapshot = self.snapshot([row], [gateway], events)
        self.assertEqual(snapshot["verified_local_accepts"], 0)
        self.assertEqual(snapshot["avoided_calls"], 0)
        self.assertEqual(snapshot["receipt_provenance_failures"], 1)
        self.assertFalse(snapshot["hard_gate_pass"])

    def test_legacy_arbitrary_64_hex_receipt_never_counts_as_covered(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows("legacy-receipt", timestamp)
        legacy_id = digest("legacy-arbitrary-receipt")
        row["verification_receipt_id"] = legacy_id
        row.pop(LEDGER.POST_VERIFIER_RECEIPT_FIELD)
        events[1]["verification_receipt_id"] = legacy_id
        events[1].pop(LEDGER.POST_VERIFIER_RECEIPT_FIELD)
        snapshot = self.snapshot([row], [gateway], events)
        evidence = snapshot["m3_evidence"]["records"][0]
        self.assertIsNone(evidence[LEDGER.POST_VERIFIER_RECEIPT_FIELD])
        self.assertIsNone(evidence["receipt_binding_sha256"])
        self.assertEqual(snapshot["verified_local_accepts"], 0)
        self.assertEqual(snapshot["avoided_calls"], 0)
        self.assertFalse(snapshot["hard_gate_pass"])

    def test_post_verifier_receipt_exact_contract_is_recomputed(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        for name in ("extra", "evidence", "output", "package", "admission"):
            with self.subTest(name=name):
                row, events, gateway = local_source_rows(
                    f"post-verifier-{name}", timestamp
                )
                receipt = copy.deepcopy(row[LEDGER.POST_VERIFIER_RECEIPT_FIELD])
                if name == "extra":
                    receipt["legacy_id"] = digest("legacy")
                elif name == "evidence":
                    receipt["evidence_sha256"] = digest("foreign-evidence")
                elif name == "output":
                    receipt["output_sha256"] = digest("foreign-output")
                elif name == "package":
                    receipt["package_id_sha256"] = digest("foreign-package")
                    receipt["admission_binding_sha256"] = (
                        LEDGER.post_verifier_admission_binding(receipt)
                    )
                else:
                    receipt["admission_binding_sha256"] = digest("forged-admission")
                receipt_id = LEDGER.post_verifier_receipt_id(receipt) or digest(
                    f"invalid:{name}"
                )
                row[LEDGER.POST_VERIFIER_RECEIPT_FIELD] = receipt
                row["verification_receipt_id"] = receipt_id
                events[1][LEDGER.POST_VERIFIER_RECEIPT_FIELD] = receipt
                events[1]["verification_receipt_id"] = receipt_id
                snapshot = self.snapshot([row], [gateway], events)
                self.assertEqual(snapshot["verified_local_accepts"], 0)
                self.assertEqual(snapshot["avoided_calls"], 0)
                self.assertFalse(snapshot["hard_gate_pass"])

    def test_all_zero_receipts_are_rejected_even_when_companion_matches(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        row, events, gateway = local_source_rows(
            "zero-receipt",
            timestamp,
            verification_receipt_id="0" * 64,
            projector_receipt_id="0" * 64,
        )
        snapshot = self.snapshot([row], [gateway], events)
        record = snapshot["m3_evidence"]["records"][0]
        self.assertIsNone(record["receipt_binding_sha256"])
        self.assertEqual(snapshot["avoided_calls"], 0)
        self.assertFalse(snapshot["hard_gate_pass"])

    def test_evidence_records_are_canonical_deterministic_and_metadata_only(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        ledger_rows[0]["raw_prompt"] = "TOP SECRET customer text"
        first = self.snapshot(ledger_rows, gateway_rows, execution_rows)
        records = first["m3_evidence"]["records"]
        self.assertEqual(
            first["m3_evidence"]["records_sha256"],
            LEDGER.evidence_rows_sha256(records),
        )
        serialized = json.dumps(records, sort_keys=True)
        self.assertNotIn("TOP SECRET", serialized)
        self.assertNotIn("raw_prompt", serialized)
        self.assertNotIn("ordinary-local", serialized)
        self.assertTrue(
            all(LEDGER.nonzero_sha256(record["record_sha256"]) for record in records)
        )

        reordered = self.snapshot(
            list(reversed(ledger_rows)),
            list(reversed(gateway_rows)),
            list(reversed(execution_rows)),
        )
        self.assertEqual(
            first["m3_evidence"]["records_sha256"],
            reordered["m3_evidence"]["records_sha256"],
        )

    def test_false_accept_evidence_has_schema_hash_count_and_freshness(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        snapshot = self.snapshot(ledger_rows, gateway_rows, execution_rows)
        receipt = snapshot["false_accept_evidence"]
        self.assertEqual(receipt["schema"], LEDGER.FALSE_ACCEPT_EVIDENCE_SCHEMA)
        self.assertEqual(receipt["source_record_count"], 1)
        self.assertTrue(receipt["fresh"])
        self.assertTrue(LEDGER.nonzero_sha256(receipt["source_sha256"]))
        self.assertEqual(
            receipt["receipt_sha256"],
            LEDGER.canonical_sha256(
                {key: value for key, value in receipt.items() if key != "receipt_sha256"}
            ),
        )

    def test_missing_malformed_stale_or_nonzero_false_accept_evidence_vetoes(self) -> None:
        ledger_rows, gateway_rows, execution_rows = self.complete_day()
        cases = (
            ("malformed", [], "malformed"),
            (
                "stale",
                {"false_accepts": 0, "timestamp_unix": self.now - DAY - 1},
                "ok",
            ),
            (
                "nonzero",
                {"false_accepts": 1, "timestamp_unix": self.now},
                "ok",
            ),
        )
        for name, payload, status in cases:
            with self.subTest(name=name):
                snapshot = self.snapshot(
                    ledger_rows, gateway_rows, execution_rows, false_payload=payload
                )
                self.assertEqual(snapshot["false_accept_evidence"]["status"], status)
                self.assertFalse(snapshot["hard_gate_pass"])
        stale = self.snapshot(
            ledger_rows,
            gateway_rows,
            execution_rows,
            false_payload={"false_accepts": 0, "timestamp_unix": self.now - DAY - 1},
        )
        self.assertFalse(stale["false_accept_evidence"]["fresh"])

        self.false_path.unlink()
        missing_receipt = LEDGER.read_false_accept_evidence(self.false_path)
        self.assertEqual(missing_receipt["status"], "missing")

    def test_retries_are_assigned_only_to_global_first_seen_utc_day(self) -> None:
        first_start = self.today - 3 * DAY
        first, first_events, first_gateway = local_source_rows(
            "retry", first_start + 10, request_sha256=digest("retry-request")
        )
        retry, retry_events, retry_gateway = local_source_rows(
            "retry",
            first_start + DAY + 10,
            request_sha256=digest("retry-request"),
            verification_receipt_id=first["verification_receipt_id"],
            projector_receipt_id=first["projector_receipt_id"],
        )
        other, other_events, other_gateway = local_source_rows(
            "day-two", first_start + DAY + 30
        )
        snapshot = self.snapshot(
            [first, retry, other],
            [first_gateway, retry_gateway, other_gateway],
            [*first_events, *retry_events, *other_events],
        )
        windows = snapshot["m3_windows"]
        self.assertEqual(len(windows), 2)
        self.assertEqual(windows[0]["eligible_client_intents"], 1)
        self.assertEqual(windows[1]["eligible_client_intents"], 1)
        self.assertEqual(windows[0]["evidence_record_count"], 2)
        self.assertEqual(windows[1]["evidence_record_count"], 1)

    def test_zero_traffic_gap_day_is_explicit_in_denominator(self) -> None:
        first_start = self.today - 4 * DAY
        first, first_events, first_gateway = local_source_rows(
            "before-gap", first_start + 10
        )
        last, last_events, last_gateway = local_source_rows(
            "after-gap", first_start + 2 * DAY + 10
        )
        snapshot = self.snapshot(
            [first, last],
            [first_gateway, last_gateway],
            [*first_events, *last_events],
        )
        windows = snapshot["m3_windows"]
        self.assertEqual(len(windows), 3)
        self.assertEqual(windows[1]["observed_client_intents"], 0)
        self.assertEqual(windows[1]["eligible_client_intents"], 0)
        self.assertEqual(windows[1]["evidence_record_count"], 0)
        self.assertEqual(
            windows[1]["evidence_rows_sha256"], LEDGER.evidence_rows_sha256([])
        )

    def test_controlled_rows_remain_outside_commercial_window(self) -> None:
        timestamp = self.today - 2 * DAY + 10
        controlled, events, gateway = local_source_rows(
            "controlled", timestamp, traffic_source="controlled_probe"
        )
        snapshot = self.snapshot([controlled], [gateway], events)
        self.assertEqual(snapshot["m3_windows"], [])
        self.assertEqual(snapshot["m3_evidence"]["records"], [])
        self.assertTrue(snapshot["hard_gate_pass"])


if __name__ == "__main__":
    unittest.main()
