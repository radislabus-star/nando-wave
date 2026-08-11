#!/usr/bin/env python3

from __future__ import annotations

import copy
import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_s1c1_resource_v2.py")
SPEC = importlib.util.spec_from_file_location("verify_s1c1_resource_v2", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)


def run(
    ordinal: int,
    *,
    test: str,
    binary: str,
    matched: int,
    no_match_field: str,
    no_match: int,
    hard_max: int,
) -> dict[str, object]:
    passed = matched <= 1_000_000 and no_match <= 250_000 and hard_max <= 2_000_000
    return {
        "ordinal": ordinal,
        "test": test,
        "binary_sha256": binary,
        "samples": 4_096,
        "matched_p99_ns": matched,
        no_match_field: no_match,
        "hard_max_ns": hard_max,
        "exit_code": 0 if passed else 101,
    }


def passing_document() -> dict[str, object]:
    targeted = [
        run(
            ordinal,
            test=VERIFY.TARGETED_TEST,
            binary=VERIFY.TARGETED_BINARY_SHA256,
            matched=25_000,
            no_match_field="no_goal_p99_ns",
            no_match=2_000,
            hard_max=40_000,
        )
        for ordinal in range(1, 4)
    ]
    pairs = []
    for ordinal, order in enumerate(VERIFY.PAIR_ORDERS, start=1):
        baseline = run(
            ordinal,
            test=VERIFY.INHERITED_TEST,
            binary=VERIFY.BASELINE_BINARY_SHA256,
            matched=1_050_000,
            no_match_field="no_match_p99_ns",
            no_match=260_000,
            hard_max=1_500_000,
        )
        candidate = run(
            ordinal,
            test=VERIFY.INHERITED_TEST,
            binary=VERIFY.CANDIDATE_BINARY_SHA256,
            matched=1_040_000,
            no_match_field="no_match_p99_ns",
            no_match=255_000,
            hard_max=1_490_000,
        )
        pairs.append(
            {
                "ordinal": ordinal,
                "order": order,
                "baseline": baseline,
                "candidate": candidate,
            }
        )
    return {
        "schema": VERIFY.SCHEMA,
        "protocol_commit": "1" * 64,
        "source_manifest_root": VERIFY.SOURCE_MANIFEST_ROOT,
        "targeted_runs": targeted,
        "inherited_pairs": pairs,
        "safety": {
            "production_service_survival": True,
            "connector_survival": True,
            "false_accepts": 0,
            "runtime_parity_failures": 0,
            "serving_parity_equal": True,
            "structural_verdict": "PASS",
        },
    }


class ResourceVerifierTests(unittest.TestCase):
    def test_accepts_relative_non_regression_when_both_absolute_sides_fail(self) -> None:
        result, code = VERIFY.verify_measurements(passing_document())
        self.assertEqual(code, 0)
        self.assertEqual(result["verdict"], "PASS")
        self.assertEqual(result["inherited_absolute_environment"], "FAIL")
        self.assertFalse(result["deployment_allowed"])

    def test_targeted_absolute_failure_is_veto(self) -> None:
        document = passing_document()
        document["targeted_runs"][1]["matched_p99_ns"] = 1_000_001
        document["targeted_runs"][1]["exit_code"] = 101
        result, code = VERIFY.verify_measurements(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["targeted_gate"], "VETO")

    def test_more_than_ten_percent_median_regression_is_veto(self) -> None:
        document = passing_document()
        for pair in document["inherited_pairs"]:
            pair["candidate"]["matched_p99_ns"] = 1_200_000
            pair["candidate"]["exit_code"] = 101
        result, code = VERIFY.verify_measurements(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["inherited_regression_gate"], "VETO")

    def test_candidate_cannot_lose_an_absolute_pass(self) -> None:
        document = passing_document()
        baseline = document["inherited_pairs"][0]["baseline"]
        candidate = document["inherited_pairs"][0]["candidate"]
        baseline.update(
            matched_p99_ns=900_000,
            no_match_p99_ns=200_000,
            hard_max_ns=1_000_000,
            exit_code=0,
        )
        candidate.update(
            matched_p99_ns=1_000_001,
            no_match_p99_ns=220_000,
            hard_max_ns=1_100_000,
            exit_code=101,
        )
        result, code = VERIFY.verify_measurements(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["inherited_regression_gate"], "VETO")

    def test_safety_watch_is_veto(self) -> None:
        document = passing_document()
        document["safety"]["structural_verdict"] = "WATCH"
        result, code = VERIFY.verify_measurements(document)
        self.assertEqual(code, 1)
        self.assertEqual(result["safety_gate"], "VETO")

    def test_identity_drift_is_invalid(self) -> None:
        document = passing_document()
        document["targeted_runs"][0]["binary_sha256"] = "f" * 64
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "binary_mismatch"):
            VERIFY.verify_measurements(document)

    def test_unknown_field_is_invalid(self) -> None:
        document = copy.deepcopy(passing_document())
        document["unexpected"] = True
        with self.assertRaisesRegex(VERIFY.InvalidMeasurements, "extra"):
            VERIFY.verify_measurements(document)


if __name__ == "__main__":
    unittest.main()
