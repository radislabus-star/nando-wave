#!/usr/bin/env python3
"""Adversarial source-backed tests for the stable M3 composite gate."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
OPS = ROOT / "ops" / "phase-center-test-server"
GATE = OPS / "bin" / "nando-live-transition-gate"
BASE_PROFILE = OPS / "gates" / "nando-live-transition-gate.profile.json"
DAY = 86_400

LEDGER_SPEC = importlib.util.spec_from_file_location(
    "nando_economics_ledger_m3_test", OPS / "bin" / "nando-economics-ledger.py"
)
LEDGER = importlib.util.module_from_spec(LEDGER_SPEC)
assert LEDGER_SPEC.loader is not None
LEDGER_SPEC.loader.exec_module(LEDGER)


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


class StableM3WindowGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.temp = Path(self.temp_dir.name)
        self.registry_path = self.temp / "registry.json"
        self.miner_path = self.temp / "miner.json"
        self.economics_path = self.temp / "economics.json"
        self.profile_path = self.temp / "profile.json"
        self.admission_path = self.temp / "admission.json"
        self.ledger_path = self.temp / "economics-terminal.jsonl"
        self.gateway_path = self.temp / "economics-access.jsonl"
        self.execution_path = self.temp / "execution-events.jsonl"
        self.false_path = self.temp / "metrics.json"
        self.now = int(time.time())
        self.today = self.now // DAY * DAY

        fake_bin = self.temp / "bin"
        fake_bin.mkdir()
        fake_curl = fake_bin / "curl"
        fake_curl.write_text(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"${NANDO_TEST_HEALTH_JSON}\"\n",
            encoding="utf-8",
        )
        fake_curl.chmod(0o755)
        self.env = os.environ.copy()
        self.env["PATH"] = f"{fake_bin}:{self.env['PATH']}"
        self.env["NANDO_LIVE_GATE_PROFILE"] = str(self.profile_path)
        self.env["NANDO_TRANSITION_ADMISSION_JSON"] = str(self.admission_path)
        self.env["NANDO_TEST_HEALTH_JSON"] = json.dumps(
            {
                "response_executor_cache_ready": True,
                "response_active_profiles": 1,
                "response_cache_error": "",
            }
        )

        self.registry = {
            "schema": "nando.response-registry.v6",
            "revision": 1,
            "packages": [
                {
                    "package_id": "test-package",
                    "state": "active",
                    "origin": "grounded_synthesis",
                    "verifier": {"schema": "test-verifier.v1"},
                    "required_routing_atom_ids": ["route-a"],
                    "proof": {
                        "support_rows": 32,
                        "future_rows": 32,
                        "distinct_sessions": 3,
                        "distinct_surfaces": 2,
                        "wrong_accepts": 0,
                        "runtime_parity_failures": 0,
                        "exact_cache_overlap": 0,
                        "wave_causal_pass": True,
                    },
                }
            ],
        }
        self.miner = {
            "automatic_lifecycle": True,
            "manual_profile_approval": False,
            "active_packages": 1,
            "future_wrong_accepts": 0,
            "cross_family_negative_accepts": 0,
            "runtime_parity_failures": 0,
            "missing_verifier_receipts": 0,
            "response_authority_candidate": {
                "schema": "nando.response-authority-candidate.v1",
                "authority_schema": "nando.response-authority.v2",
                "registry_schema": "nando.response-registry.v6",
                "registry_revision": 1,
                "registry_sha256": "a" * 64,
                "packages": [{"package_id": "test-package"}],
                "execution_authority": False,
            },
        }
        profile = json.loads(BASE_PROFILE.read_text(encoding="utf-8"))
        profile["required_checks"] = ["response_runtime"]
        profile["runtime"]["admission_status"] = str(self.admission_path)
        profile["response_runtime"].update(
            {
                "registry": str(self.registry_path),
                "miner_status": str(self.miner_path),
                "economics": str(self.economics_path),
                "m3_authority_sources": {
                    "terminal_ledger": str(self.ledger_path),
                    "gateway_terminal": str(self.gateway_path),
                    "execution_events": str(self.execution_path),
                    "false_accept_metrics": str(self.false_path),
                },
                # Keep fixtures small; production thresholds are asserted separately.
                "minimum_m3_eligible_client_intents": 2,
                "minimum_m3_verified_avoided_calls": 1,
            }
        )
        profile["deployment"]["health_url"] = "http://m3.test/health"
        gate_build = self.temp / "gate-build"
        runtime_build = self.temp / "runtime-build"
        gate_build.write_bytes(b"gate")
        runtime_build.write_bytes(b"runtime")
        profile["deployment"]["gate_build"] = str(gate_build)
        profile["deployment"]["response_runtime_build"] = str(runtime_build)
        self.profile_path.write_text(json.dumps(profile), encoding="utf-8")
        self.base_economics = self.build_economics()

    @staticmethod
    def write_jsonl(path: Path, rows: list[Any]) -> None:
        path.write_text(
            "".join(
                json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n"
                for row in rows
            ),
            encoding="utf-8",
        )

    def build_economics(
        self,
        *,
        days: int = 3,
        local_tokens: int = 100,
        upstream_tokens: int = 100,
        include_local: bool = True,
        stale: bool = False,
        false_accepts: int = 0,
        recent_unsettled: bool = False,
    ) -> dict[str, Any]:
        ledger_rows: list[dict[str, Any]] = []
        gateway_rows: list[dict[str, Any]] = []
        execution_rows: list[dict[str, Any]] = []
        if recent_unsettled:
            first_start = self.today
        else:
            first_start = self.today - (days + 1 + (1 if stale else 0)) * DAY
        for day_index in range(days):
            start = first_start + day_index * DAY
            if include_local:
                intent = f"local-{day_index}"
                timestamp = start + 10
                request_hash = digest(f"request:{intent}")
                package_id = "test-package"
                projector = digest(f"projector:{intent}")
                post_receipt = post_verifier_receipt(
                    package_id=package_id,
                    request_sha256=request_hash,
                    output_sha256=projector,
                )
                verification = LEDGER.post_verifier_receipt_id(post_receipt)
                ledger_rows.append(
                    {
                        "schema": LEDGER.LEDGER_SCHEMA,
                        "timestamp_unix": timestamp,
                        "client_intent_id": intent,
                        "intent_dedupe_eligible": True,
                        "provider_attempt_id": None,
                        "request_sha256": request_hash,
                        "route": "local_response_actor",
                        "local_route": "response_actor:test-package",
                        "package_id": package_id,
                        "terminal_state": "delivered",
                        "traffic_source": "ordinary",
                        "input_tokens": local_tokens,
                        "input_token_accounting": "byte_estimate_v1",
                        "upstream_socket_opened": False,
                        "avoided_call": True,
                        "verification_status": "verified",
                        "verification_receipt_id": verification,
                        "projector_receipt_id": projector,
                        "verifier_schema": "response_actor_independent_verifier.v1",
                        LEDGER.POST_VERIFIER_RECEIPT_FIELD: post_receipt,
                    }
                )
                execution_rows.extend(
                    [
                        {
                            "schema": "nando.transition-execution-event.v1",
                            "timestamp_unix": timestamp,
                            "event": "bridge_request",
                            "client_intent_id": intent,
                            "request_sha256": request_hash,
                            "tokens": local_tokens,
                            "traffic_source": "ordinary",
                        },
                        {
                            "schema": "nando.transition-execution-event.v1",
                            "timestamp_unix": timestamp,
                            "event": "local_accept",
                            "request_sha256": request_hash,
                            "tokens": local_tokens,
                            "route": "response_actor:test-package",
                            "package_id": package_id,
                            "verification_receipt_id": verification,
                            "projector_receipt_id": projector,
                            "verifier_schema": "response_actor_independent_verifier.v1",
                            LEDGER.POST_VERIFIER_RECEIPT_FIELD: post_receipt,
                        },
                    ]
                )
                gateway_rows.append(
                    {
                        "schema": "nando.nginx-terminal.v1",
                        "timestamp_unix": timestamp,
                        "request_id": intent,
                        "status": 200,
                        "upstream_status": "200",
                        "upstream_addr": "127.0.0.1:18789",
                    }
                )

            intent = f"upstream-{day_index}"
            timestamp = start + 30
            request_hash = digest(f"request:{intent}")
            execution_rows.append(
                {
                    "schema": "nando.transition-execution-event.v1",
                    "timestamp_unix": timestamp,
                    "event": "bridge_request",
                    "client_intent_id": intent,
                    "request_sha256": request_hash,
                    "tokens": upstream_tokens,
                    "traffic_source": "ordinary",
                }
            )
            gateway_rows.append(
                {
                    "schema": "nando.nginx-terminal.v1",
                    "timestamp_unix": timestamp,
                    "request_id": intent,
                    "status": 200,
                    "upstream_status": "418, 200",
                    "upstream_addr": "127.0.0.1:18789, 192.0.2.10:443",
                }
            )

        self.write_jsonl(self.ledger_path, ledger_rows)
        self.write_jsonl(self.gateway_path, gateway_rows)
        self.write_jsonl(self.execution_path, execution_rows)
        self.false_path.write_text(
            json.dumps(
                {"false_accepts": false_accepts, "timestamp_unix": self.now},
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        return LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now,
        )

    def run_gate(
        self,
        economics: dict[str, Any],
        *,
        registry_schema: str = "nando.response-registry.v6",
    ) -> dict[str, Any]:
        registry = copy.deepcopy(self.registry)
        registry["schema"] = registry_schema
        self.registry_path.write_text(json.dumps(registry), encoding="utf-8")
        self.miner_path.write_text(json.dumps(self.miner), encoding="utf-8")
        self.economics_path.write_text(json.dumps(economics), encoding="utf-8")
        completed = subprocess.run(
            [str(GATE), "--status-mode", "--project-root", str(ROOT)],
            cwd=ROOT,
            env=self.env,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        try:
            return json.loads(completed.stdout)
        except json.JSONDecodeError as error:
            self.fail(
                f"gate returned invalid JSON: {error}\n{completed.stdout}\n{completed.stderr}"
            )

    def assert_response(
        self, report: dict[str, Any], *, verdict: str, m3_verdict: str
    ) -> dict[str, Any]:
        self.assertEqual(report["schema"], "nando.live-transition-composite-gate.v2")
        response = report["sections"]["response_runtime"]
        self.assertEqual(response["verdict"], verdict, response)
        self.assertEqual(response["m3_verdict"], m3_verdict, response)
        self.assertEqual(report["m3_complete"], m3_verdict == "PASS", report)
        if verdict == "PASS":
            self.assertTrue(report["eligible_for_local_accept"], report)
            self.assertEqual(
                report["response_authority"]["schema"], "nando.response-authority.v2"
            )
            self.assertEqual(len(report["response_authority"]["packages"]), 1)
        return response

    def rewrite_evidence(
        self, economics: dict[str, Any], records: list[dict[str, Any]]
    ) -> dict[str, Any]:
        forged = copy.deepcopy(economics)
        records = sorted(
            records,
            key=lambda record: (
                record.get("client_intent_id_sha256") or "",
                record.get("timestamp_unix") or -1,
                record["source_id"],
                record["source_row_sha256"],
                record["record_sha256"],
            ),
        )
        forged["m3_evidence"].update(
            {
                "record_count": len(records),
                "records_sha256": LEDGER.evidence_rows_sha256(records),
                "records": records,
            }
        )
        old_reconciliation = forged["source_reconciliation"]
        reconciliation = LEDGER._build_source_reconciliation(
            old_reconciliation["sources"],
            records,
            [],
            generated_at_unix=old_reconciliation["generated_at_unix"],
            records_truncated=False,
        )
        forged["source_reconciliation"] = reconciliation
        forged["dropped_incomplete_intents"] = reconciliation[
            "dropped_incomplete_intents"
        ]
        forged["dedupe_conflicts"] = reconciliation["conflicting_intents"]
        forged["receipt_provenance_failures"] = reconciliation[
            "receipt_provenance_failures"
        ]
        forged["m3_windows"] = LEDGER.build_m3_windows(
            [],
            as_of_unix=self.now,
            evidence_records=records,
            source_reconciliation=reconciliation,
            false_accept_evidence=forged["false_accept_evidence"],
        )
        return forged

    def test_production_profile_keeps_stable_m3_thresholds(self) -> None:
        policy = json.loads(BASE_PROFILE.read_text(encoding="utf-8"))["response_runtime"]
        self.assertEqual(policy["m3_window_duration_seconds"], DAY)
        self.assertEqual(policy["m3_window_settlement_seconds"], DAY)
        self.assertEqual(policy["m3_required_consecutive_windows"], 3)
        self.assertEqual(policy["minimum_m3_eligible_client_intents"], 10_000)
        self.assertEqual(policy["minimum_m3_verified_avoided_calls"], 100)
        self.assertEqual(policy["minimum_m3_input_token_saving_share_milli"], 500)
        self.assertEqual(policy["minimum_m3_receipt_coverage_milli"], 1000)
        self.assertEqual(
            policy["m3_post_verifier_receipt_schema"],
            LEDGER.POST_VERIFIER_RECEIPT_SCHEMA,
        )
        self.assertEqual(
            set(policy["m3_authority_sources"]),
            {
                "terminal_ledger",
                "gateway_terminal",
                "execution_events",
                "false_accept_metrics",
            },
        )

    def test_three_source_backed_windows_pass(self) -> None:
        response = self.assert_response(
            self.run_gate(self.base_economics), verdict="PASS", m3_verdict="PASS"
        )
        evaluation = response["m3_window_evaluation"]
        self.assertTrue(evaluation["source_authority_valid"])
        self.assertTrue(evaluation["false_accept_authority_valid"])
        self.assertTrue(evaluation["evidence_authority_valid"])
        self.assertEqual(evaluation["passing_window_count"], 3)

    def test_two_windows_and_complete_empty_window_set_are_safe_watch(self) -> None:
        two_windows = self.build_economics(days=2)
        self.assert_response(
            self.run_gate(two_windows), verdict="PASS", m3_verdict="WATCH"
        )
        empty = self.build_economics(days=1, recent_unsettled=True)
        self.assertEqual(empty["m3_windows"], [])
        response = self.assert_response(
            self.run_gate(empty), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertFalse(response["m3_window_evaluation"]["safety_veto"])

    def test_missing_window_authority_is_veto_not_lifetime_scalar_pass(self) -> None:
        forged = copy.deepcopy(self.base_economics)
        forged.pop("m3_windows")
        forged["input_token_saving_share_milli"] = 1000
        response = self.assert_response(
            self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertIn("m3_windows_missing", response["m3_blockers"])

        emptied = copy.deepcopy(self.base_economics)
        emptied["m3_windows"] = []
        response = self.assert_response(
            self.run_gate(emptied), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertIn("m3_window_source_inventory_mismatch", response["m3_blockers"])

    def test_low_share_and_no_local_receipt_denominator_are_safe_watch(self) -> None:
        low_share = self.build_economics(local_tokens=99, upstream_tokens=101)
        response = self.assert_response(
            self.run_gate(low_share), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertFalse(response["m3_window_evaluation"]["safety_veto"])

        upstream_only = self.build_economics(include_local=False)
        response = self.assert_response(
            self.run_gate(upstream_only), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertFalse(response["m3_window_evaluation"]["safety_veto"])

    def test_all_zero_forged_or_missing_evidence_hash_vetoes(self) -> None:
        for name, value in (
            ("zero", "0" * 64),
            ("forged", "f" * 64),
            ("missing", None),
        ):
            with self.subTest(name=name):
                forged = copy.deepcopy(self.base_economics)
                if value is None:
                    forged["m3_windows"][-1].pop("evidence_rows_sha256")
                else:
                    forged["m3_windows"][-1]["evidence_rows_sha256"] = value
                response = self.assert_response(
                    self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
                )
                self.assertTrue(response["m3_window_evaluation"]["safety_veto"])

    def test_fully_rehashed_dropped_intent_is_detected_against_source_inventory(self) -> None:
        records = copy.deepcopy(self.base_economics["m3_evidence"]["records"])
        records.pop(0)
        forged = self.rewrite_evidence(self.base_economics, records)
        response = self.assert_response(
            self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertIn(
            "m3_evidence_source_inventory_mismatch",
            response["m3_window_evaluation"]["m3_blockers"],
        )

    def test_fully_rehashed_forged_record_is_detected_against_source_inventory(self) -> None:
        records = copy.deepcopy(self.base_economics["m3_evidence"]["records"])
        records[0]["input_tokens"] += 10_000
        records[0]["record_sha256"] = LEDGER.canonical_sha256(
            {
                key: value
                for key, value in records[0].items()
                if key != "record_sha256"
            }
        )
        forged = self.rewrite_evidence(self.base_economics, records)
        response = self.assert_response(
            self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertIn(
            "m3_evidence_source_inventory_mismatch",
            response["m3_window_evaluation"]["m3_blockers"],
        )

    def test_scalar_counter_forgery_is_independently_recomputed(self) -> None:
        forged = copy.deepcopy(self.base_economics)
        forged["m3_windows"][-1]["verified_avoided_input_tokens"] += 1
        forged["m3_windows"][-1]["input_token_saving_share_milli"] = 500
        response = self.assert_response(
            self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
        )
        blockers = response["m3_window_evaluation"]["m3_blockers"]
        self.assertTrue(any("window_counter_mismatch" in blocker for blocker in blockers))

    def test_source_prefix_is_stable_across_append_but_mutation_vetoes(self) -> None:
        with self.ledger_path.open("ab") as handle:
            handle.write(b"{malformed\n")
        response = self.assert_response(
            self.run_gate(self.base_economics), verdict="PASS", m3_verdict="PASS"
        )
        self.assertTrue(response["source_authority_valid"])

        raw = self.ledger_path.read_bytes()
        self.ledger_path.write_bytes(raw.replace(b'"schema"', b'"schemA"', 1))
        response = self.assert_response(
            self.run_gate(self.base_economics), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertFalse(response["source_authority_valid"])

    def test_missing_source_receipt_vetoes_baseline(self) -> None:
        forged = copy.deepcopy(self.base_economics)
        forged["source_reconciliation"]["sources"] = forged[
            "source_reconciliation"
        ]["sources"][:-1]
        forged["source_reconciliation"]["source_receipts_sha256"] = (
            LEDGER.canonical_sha256(forged["source_reconciliation"]["sources"])
        )
        forged["source_reconciliation"]["reconciliation_sha256"] = (
            LEDGER.canonical_sha256(
                {
                    key: value
                    for key, value in forged["source_reconciliation"].items()
                    if key != "reconciliation_sha256"
                }
            )
        )
        self.assert_response(
            self.run_gate(forged), verdict="VETO", m3_verdict="WATCH"
        )

    def test_foreign_stale_malformed_and_nonzero_false_accept_evidence_vetoes(self) -> None:
        foreign_path = self.temp / "foreign-metrics.json"
        foreign_path.write_text(
            json.dumps({"false_accepts": 0, "timestamp_unix": self.now}),
            encoding="utf-8",
        )
        foreign_path_snapshot = copy.deepcopy(self.base_economics)
        foreign_receipt = LEDGER.read_false_accept_evidence(
            foreign_path, observed_at_unix=self.now
        )
        foreign_path_snapshot["false_accept_evidence"] = foreign_receipt
        for window in foreign_path_snapshot["m3_windows"]:
            window["false_accept_evidence_receipt_sha256"] = foreign_receipt[
                "receipt_sha256"
            ]
        self.assert_response(
            self.run_gate(foreign_path_snapshot),
            verdict="VETO",
            m3_verdict="WATCH",
        )

        foreign = copy.deepcopy(self.base_economics)
        receipt = foreign["false_accept_evidence"]
        receipt["source_id"] = "foreign_metrics"
        receipt["receipt_sha256"] = LEDGER.canonical_sha256(
            {key: value for key, value in receipt.items() if key != "receipt_sha256"}
        )
        self.assert_response(
            self.run_gate(foreign), verdict="VETO", m3_verdict="WATCH"
        )

        stale = copy.deepcopy(self.base_economics)
        receipt = stale["false_accept_evidence"]
        receipt["source_timestamp_unix"] = self.now - DAY - 1
        receipt["receipt_sha256"] = LEDGER.canonical_sha256(
            {key: value for key, value in receipt.items() if key != "receipt_sha256"}
        )
        response = self.assert_response(
            self.run_gate(stale), verdict="VETO", m3_verdict="WATCH"
        )
        self.assertIn("false_accept_evidence_stale", response["m3_blockers"])

        self.false_path.write_text("{", encoding="utf-8")
        self.assert_response(
            self.run_gate(self.base_economics), verdict="VETO", m3_verdict="WATCH"
        )

        nonzero = self.build_economics(false_accepts=1)
        self.assert_response(
            self.run_gate(nonzero), verdict="VETO", m3_verdict="WATCH"
        )

    def test_missing_socket_and_forged_receipt_source_rows_veto_gate(self) -> None:
        incomplete = self.build_economics()
        rows = [json.loads(line) for line in self.ledger_path.read_text().splitlines()]
        rows[0].pop("upstream_socket_opened")
        self.write_jsonl(self.ledger_path, rows)
        incomplete = LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now,
        )
        self.assert_response(
            self.run_gate(incomplete), verdict="PASS", m3_verdict="WATCH"
        )

        forged_receipt = self.build_economics()
        rows = [json.loads(line) for line in self.ledger_path.read_text().splitlines()]
        rows[0]["verification_receipt_id"] = digest("forged-only-in-ledger")
        self.write_jsonl(self.ledger_path, rows)
        forged_receipt = LEDGER.reduce_source_paths(
            ledger_path=self.ledger_path,
            gateway_path=self.gateway_path,
            execution_path=self.execution_path,
            false_accept_path=self.false_path,
            as_of_unix=self.now,
        )
        self.assert_response(
            self.run_gate(forged_receipt), verdict="PASS", m3_verdict="WATCH"
        )

    def test_nonordinary_window_tamper_is_authority_veto(self) -> None:
        forged = copy.deepcopy(self.base_economics)
        forged["m3_windows"][-1]["traffic_class"] = "controlled"
        response = self.assert_response(
            self.run_gate(forged), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertTrue(response["m3_window_evaluation"]["safety_veto"])

    def test_stale_source_backed_windows_keep_safety_pass_but_watch_m3(self) -> None:
        stale = self.build_economics(stale=True)
        response = self.assert_response(
            self.run_gate(stale), verdict="PASS", m3_verdict="WATCH"
        )
        self.assertIn("latest_window_stale", response["m3_blockers"])
        self.assertFalse(response["m3_window_evaluation"]["safety_veto"])

    def test_registry_v4_has_no_execution_authority(self) -> None:
        response = self.assert_response(
            self.run_gate(
                self.base_economics, registry_schema="nando.response-registry.v4"
            ),
            verdict="VETO",
            m3_verdict="WATCH",
        )
        self.assertIn("response_registry_not_v6", response["m3_blockers"])


if __name__ == "__main__":
    unittest.main()
