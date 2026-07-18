import importlib.util
import json
import os
import sys
import unittest
from pathlib import Path


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
sys.path.insert(0, str(BIN_DIR))
os.environ["NANDO_PHASE_CENTER_ENV"] = "/nonexistent/nando-test.env"

ADAPTER_SPEC = importlib.util.spec_from_file_location(
    "nando_codex_transition_adapter",
    BIN_DIR / "nando-codex-transition-adapter.py",
)
ADAPTER = importlib.util.module_from_spec(ADAPTER_SPEC)
assert ADAPTER_SPEC.loader is not None
ADAPTER_SPEC.loader.exec_module(ADAPTER)

BRIDGE_SPEC = importlib.util.spec_from_file_location(
    "nando_provider_bridge_for_adapter_test",
    BIN_DIR / "nando-provider-bridge.py",
)
BRIDGE = importlib.util.module_from_spec(BRIDGE_SPEC)
assert BRIDGE_SPEC.loader is not None
BRIDGE_SPEC.loader.exec_module(BRIDGE)


class CodexTransitionAdapterTest(unittest.TestCase):
    def test_post_tool_use_builds_content_bound_transition(self) -> None:
        observation = ADAPTER.build_observation(
            {
                "hook_event_name": "PostToolUse",
                "session_id": "session-1",
                "turn_id": "turn-1",
                "tool_name": "Bash",
                "tool_input": {"command": "cargo test"},
                "tool_response": {"exit_code": 0, "output": "secret raw output"},
            }
        )
        self.assertIsNotNone(observation)
        assert observation is not None
        self.assertEqual(observation["after"]["tool_runs"][0]["status"], "succeeded")
        self.assertEqual(observation["action"]["event"]["kind"], "test")
        self.assertEqual(observation["usage"]["total_tokens"], 0)
        self.assertEqual(
            len(observation["provenance"]["source_session_id_sha256"]), 64
        )
        self.assertEqual(
            len(observation["provenance"]["source_event_id_sha256"]), 64
        )
        self.assertNotIn("secret raw output", json.dumps(observation))
        expected = BRIDGE.grounded_evidence_receipt(
            observation["before"],
            observation["action"],
            observation["after"],
            observation["evidence"]["source"],
            observation["evidence"]["verifier"],
            observation["evidence"]["receipt_schema"],
            observation["observed_at"],
            observation["provenance"],
        )
        self.assertEqual(observation["evidence"]["receipt_sha256"], expected)
        self.assertEqual(
            observation["evidence"]["receipt_schema"],
            "nando.grounded-transition-receipt.v2",
        )

    def test_failed_tool_is_recorded_without_claiming_tokens(self) -> None:
        observation = ADAPTER.build_observation(
            {
                "hook_event_name": "PostToolUse",
                "session_id": "session-2",
                "tool_name": "Bash",
                "tool_input": {"command": "false"},
                "tool_response": {"exit_code": 1, "output": "failed"},
            }
        )
        assert observation is not None
        self.assertEqual(observation["action"]["event"]["value"], "failed")
        self.assertEqual(observation["usage"]["total_tokens"], 0)

    def test_non_post_event_is_ignored(self) -> None:
        self.assertIsNone(
            ADAPTER.build_observation(
                {"hook_event_name": "SessionStart", "session_id": "session-3"}
            )
        )

    def test_natural_tool_families_keep_distinct_surfaces(self) -> None:
        events = [
            ("Bash", {"command": "cargo check"}, "tool_runs"),
            ("apply_patch", {"patch": "x"}, "file_operations"),
            ("view_file", {"path": "x"}, "read_requests"),
            ("custom_tool", {"value": "x"}, "tool_calls"),
        ]
        roots = []
        for index, (tool_name, tool_input, expected_root) in enumerate(events):
            observation = ADAPTER.build_observation(
                {
                    "hook_event_name": "PostToolUse",
                    "session_id": f"session-{index}",
                    "tool_name": tool_name,
                    "tool_input": tool_input,
                    "tool_response": {"status": "ok"},
                }
            )
            assert observation is not None
            self.assertIn(expected_root, observation["before"])
            roots.append(next(iter(observation["before"])))
        self.assertEqual(len(set(roots)), 4)


if __name__ == "__main__":
    unittest.main()
