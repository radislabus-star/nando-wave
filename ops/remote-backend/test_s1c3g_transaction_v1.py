#!/usr/bin/env python3
"""Focused stable-projection, ownership and authority tests for S1C-3G."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import s1c3g_remote_transaction_v1 as executor
import verify_s1c3g_transaction_v1 as verifier


def health_snapshot() -> dict[str, object]:
    serving = {
        "ok": True,
        "service": "nando-transition-serving",
        "mode": "CPU",
        "admission_verdict": "PASS",
        "transition_active_profiles": 5,
        "response_active_profiles": 2,
        "response_executor_cache_ready": True,
    }
    return {
        "control": {
            "url": "http://127.0.0.1:18788/health",
            "raw_sha256": "a" * 64,
            "semantic": {
                "ok": True,
                "service": "nando-gateway-control",
                "mode": "CPU",
                "admission_verdict": None,
                "transition_active_profiles": None,
                "response_active_profiles": None,
                "response_executor_cache_ready": None,
            },
        },
        "cpu": {
            "url": "http://192.168.3.94:8787/cpu-health",
            "raw_sha256": "b" * 64,
            "semantic": dict(serving),
        },
        "gateway": {
            "url": "http://192.168.3.94:8787/health",
            "raw_sha256": "c" * 64,
            "semantic": {
                "ok": True,
                "service": "nando-nginx-gateway",
                "mode": None,
                "admission_verdict": None,
                "transition_active_profiles": None,
                "response_active_profiles": None,
                "response_executor_cache_ready": None,
            },
        },
        "hot": {
            "url": "http://127.0.0.1:18789/health",
            "raw_sha256": "d" * 64,
            "semantic": dict(serving),
        },
    }


class StableProjectionTests(unittest.TestCase):
    def test_executor_and_verifier_projection_agree(self) -> None:
        snapshot = health_snapshot()
        self.assertEqual(
            executor.stable_health_projection(snapshot),
            verifier.stable_health_projection(snapshot, "snapshot"),
        )
        self.assertEqual(executor.projection_contract(), verifier.projection_contract())

    def test_dynamic_fields_and_raw_hash_are_observed_not_compared(self) -> None:
        before = health_snapshot()
        after = health_snapshot()
        after["hot"]["raw_sha256"] = "e" * 64
        after["cpu"]["raw_sha256"] = "f" * 64
        after["hot"]["semantic"]["transition_active_profiles"] = 9
        after["cpu"]["semantic"]["transition_active_profiles"] = 9
        after["hot"]["semantic"]["requests"] = 100
        after["cpu"]["semantic"]["requests"] = 100
        self.assertTrue(executor.semantic_health_equal(before, after))

    def test_serving_authority_change_is_rejected(self) -> None:
        before = health_snapshot()
        after = health_snapshot()
        after["hot"]["semantic"]["admission_verdict"] = "BLOCK"
        after["cpu"]["semantic"]["admission_verdict"] = "BLOCK"
        self.assertFalse(executor.semantic_health_equal(before, after))

    def test_missing_stable_field_is_rejected(self) -> None:
        value = health_snapshot()
        del value["hot"]["semantic"]["response_active_profiles"]
        with self.assertRaisesRegex(executor.base.GateFailure, "stable_field_missing"):
            executor.stable_health_projection(value)

    def test_wrong_endpoint_url_is_rejected(self) -> None:
        value = health_snapshot()
        value["gateway"]["url"] = "http://wrong/health"
        with self.assertRaisesRegex(executor.base.GateFailure, "endpoint_url"):
            executor.stable_health_projection(value)

    def test_partial_endpoint_set_is_rejected(self) -> None:
        value = health_snapshot()
        del value["control"]
        with self.assertRaisesRegex(executor.base.GateFailure, "endpoint_set"):
            executor.stable_health_projection(value)

    def test_hot_cpu_disagreement_is_rejected(self) -> None:
        value = health_snapshot()
        value["cpu"]["semantic"]["response_active_profiles"] = 1
        with self.assertRaisesRegex(executor.base.GateFailure, "hot_cpu"):
            executor.stable_health_projection(value)

    def test_stable_receipt_rejects_extra_raw_object(self) -> None:
        receipt = executor.stable_health_projection(health_snapshot())
        receipt["hot"]["raw_sha256"] = "a" * 64
        with self.assertRaisesRegex(verifier.InvalidReceipt, "hot_fields"):
            verifier.verify_stable_receipt(receipt, "route")

    def test_route_receipt_is_compact_read_only_projection(self) -> None:
        snapshot = health_snapshot()
        with mock.patch.object(executor.base, "health_snapshot", return_value=snapshot):
            receipt = executor.read_only_route_receipt()
        self.assertEqual(receipt, executor.stable_health_projection(snapshot))
        self.assertEqual(set(receipt["hot"]), {"url", "stable"})
        source = Path(executor.__file__).read_text().split(
            "def read_only_route_receipt", 1
        )[1].split("def verify_parent", 1)[0]
        self.assertNotIn("/v1/responses", source)
        self.assertNotIn("POST", source)

    def test_authority_renewal_requires_expiry_advance_and_stable_health(self) -> None:
        snapshots = [
            {
                "ok": True,
                "admission_verdict": "PASS",
                "response_executor_cache_ready": True,
                "response_active_profiles": 2,
                "response_admission_expires_at_unix": 100,
            },
            {
                "ok": True,
                "admission_verdict": "PASS",
                "response_executor_cache_ready": True,
                "response_active_profiles": 2,
                "response_admission_expires_at_unix": 130,
            },
        ]
        health = health_snapshot()
        projection = executor.stable_health_projection(health)
        with (
            mock.patch.object(executor, "HEALTH_HTTP_JSON", side_effect=snapshots),
            mock.patch.object(executor.base, "health_snapshot", return_value=health),
            mock.patch.object(executor.time, "sleep"),
            mock.patch.object(executor.time, "monotonic", side_effect=[0.0, 0.0, 0.1]),
        ):
            receipt = executor.wait_for_authority_renewal(projection, timeout=1.0)
        self.assertEqual(receipt["advanced_seconds"], 30)
        verifier.verify_authority_renewal(receipt)

    def test_authority_renewal_rejects_not_ready_authority(self) -> None:
        value = {
            "ok": True,
            "admission_verdict": "PASS",
            "response_executor_cache_ready": False,
            "response_active_profiles": 2,
            "response_admission_expires_at_unix": 100,
        }
        with (
            mock.patch.object(executor, "HEALTH_HTTP_JSON", return_value=value),
            self.assertRaisesRegex(executor.base.GateFailure, "authority_not_ready"),
        ):
            executor.authority_lease_observation()

    def test_authority_renewal_timeout_is_terminal(self) -> None:
        value = {
            "ok": True,
            "admission_verdict": "PASS",
            "response_executor_cache_ready": True,
            "response_active_profiles": 2,
            "response_admission_expires_at_unix": 100,
        }
        projection = executor.stable_health_projection(health_snapshot())
        with (
            mock.patch.object(executor, "HEALTH_HTTP_JSON", return_value=value),
            mock.patch.object(executor.time, "sleep"),
            mock.patch.object(
                executor.time,
                "monotonic",
                side_effect=[0.0, 0.0, 0.1, 1.1],
            ),
            self.assertRaisesRegex(executor.base.GateFailure, "renewal_timeout"),
        ):
            executor.wait_for_authority_renewal(projection, timeout=1.0)

    def test_authority_renewal_receipt_rejects_non_advance(self) -> None:
        observation = {
            "endpoint": executor.ENDPOINT_CONTRACT["hot"]["url"],
            "expires_at_unix": 100,
            "admission_verdict": "PASS",
            "response_executor_cache_ready": True,
            "response_active_profiles": 2,
        }
        value = {
            "schema": "nando.s1c3g-authority-renewal-receipt.v1",
            "before": dict(observation),
            "after": dict(observation),
            "advanced_seconds": 0,
            "observation_seconds": 1.0,
            "stable_health_projection_root_sha256": executor.PROJECTION_ROOT,
            "stable_health_preserved": True,
        }
        with self.assertRaisesRegex(verifier.InvalidReceipt, "authority_advanced"):
            verifier.verify_authority_renewal(value)


class OwnershipAndAuthorityTests(unittest.TestCase):
    def test_both_inherited_comparison_paths_use_s1c3g_owner(self) -> None:
        self.assertIs(executor.base.semantic_health_equal, executor.semantic_health_equal)
        self.assertIs(executor.base.route_probe, executor.read_only_route_receipt)

    def test_parent_copy_failure_is_terminal_before_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "transaction"
            parent_dir = Path(directory) / "parent"
            root.mkdir()
            parent_dir.mkdir()
            for name in executor.PARENT_FILE_SHA256:
                (parent_dir / name).write_text("evidence")
            args = SimpleNamespace(
                transaction_directory=str(root), transaction_id="test-transaction"
            )

            def prepared(_: object) -> int:
                executor.base.write_json(
                    root / "preparation.json",
                    {"preparation_root_sha256": "old"},
                    0o400,
                )
                executor.base.write_json(
                    root / "transaction-state.json",
                    {"schema": executor.STATE_SCHEMA, "state": "PREPARED"},
                    0o600,
                )
                return 0

            with (
                mock.patch.object(executor.base, "prepare", side_effect=prepared),
                mock.patch.object(executor, "PARENT_DIRECTORY", parent_dir),
                mock.patch.object(executor.shutil, "copy2", side_effect=OSError("copy failed")),
                self.assertRaisesRegex(OSError, "copy failed"),
            ):
                executor.prepare(args)
            state = json.loads((root / "transaction-state.json").read_text())
            self.assertEqual(state["state"], "PREFLIGHT_FAILURE")

    def test_state_namespace_promotion_removes_s1c3e_filename(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.base.write_json(
                root / "s1c3e-state.json",
                {
                    "schema": "nando.s1c3e-state.v1",
                    "verdict": "S1C3E_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH",
                    "state_root_sha256": "old",
                },
                0o400,
            )
            executor._promote_state_namespace(root)
            self.assertFalse((root / "s1c3e-state.json").exists())
            state = json.loads((root / "s1c3g-state.json").read_text())
            self.assertEqual(state["schema"], executor.STATE_SCHEMA)
            self.assertEqual(
                state["verdict"], "S1C3G_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
            )

    def test_projection_contract_forbids_whole_object_equality(self) -> None:
        contract = executor.projection_contract()
        self.assertFalse(contract["whole_object_equality"])
        self.assertEqual(
            contract["projection_root_sha256"], executor.PROJECTION_ROOT
        )

    def test_installation_never_grants_scientific_authority(self) -> None:
        source = Path(executor.__file__).read_text() + Path(verifier.__file__).read_text()
        self.assertNotIn('"scientific_authority": True', source)
        self.assertNotIn('"model_training": True', source)
        self.assertNotIn('"phase_mutation": True', source)

    def test_freeze_binds_new_layer_and_frozen_dependencies(self) -> None:
        value = verifier.create_freeze(
            "a" * 40, "b" * 40, Path(verifier.__file__).resolve().parent
        )
        names = [row["path"] for row in value["files"]]
        self.assertEqual(names, list(verifier.IMPLEMENTATION_FILES))
        self.assertIn("s1c3g_remote_transaction_v1.py", names)
        self.assertIn("s1c3f_remote_transaction_v1.py", names)
        self.assertIn("s1c3_remote_transaction_v7.py", names)


if __name__ == "__main__":
    unittest.main()
