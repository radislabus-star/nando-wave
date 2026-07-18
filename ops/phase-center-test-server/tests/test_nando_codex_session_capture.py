import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
SPEC = importlib.util.spec_from_file_location(
    "nando_codex_session_capture",
    BIN_DIR / "nando-codex-session-capture.py",
)
CAPTURE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(CAPTURE)


class CodexSessionCaptureTest(unittest.TestCase):
    def test_generic_relation_scheduler_keeps_hot_file_and_rotates_backlog(self) -> None:
        paths = [Path(f"rollout-{index}.jsonl") for index in range(6)]
        first, cursor, hot = CAPTURE.generic_relation_schedule(paths, 0, 4)
        second, next_cursor, second_hot = CAPTURE.generic_relation_schedule(
            paths, cursor, 4
        )
        self.assertEqual(first[0], paths[-1])
        self.assertEqual(second[0], paths[-1])
        self.assertEqual(len(set(first)), 4)
        self.assertEqual(len(set(second)), 4)
        self.assertEqual(hot, 1)
        self.assertEqual(second_hot, 1)
        self.assertNotEqual(first[1:], second[1:])
        self.assertNotEqual(cursor, next_cursor)

    def test_generic_relation_scheduler_honors_expanded_file_budget(self) -> None:
        paths = [Path(f"rollout-{index}.jsonl") for index in range(32)]
        selected, _, hot = CAPTURE.generic_relation_schedule(paths, 0, 16)
        self.assertEqual(len(selected), 16)
        self.assertEqual(len(set(selected)), 16)
        self.assertEqual(selected[0], paths[-1])
        self.assertEqual(hot, 1)

    def test_capture_lock_prevents_concurrent_state_writers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary) / "state.json"
            first = CAPTURE.acquire_capture_lock(state)
            self.assertIsNotNone(first)
            self.assertIsNone(CAPTURE.acquire_capture_lock(state))
            assert first is not None
            os.close(first)
            third = CAPTURE.acquire_capture_lock(state)
            self.assertIsNotNone(third)
            assert third is not None
            os.close(third)

    def test_relation_frame_append_is_idempotent_after_state_loss(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "frames.jsonl"
            frame = {
                "schema": CAPTURE.RELATION_FRAME_SCHEMA,
                "frame_id_sha256": "a" * 64,
            }
            path.write_text(json.dumps(frame) + "\n", encoding="utf-8")
            state = CAPTURE.empty_state()
            state["relation_frame_outbox"].append(frame)
            self.assertEqual(CAPTURE.append_relation_frames(path, state, 64), 0)
            self.assertEqual(len(path.read_text(encoding="utf-8").splitlines()), 1)
            self.assertEqual(state["relation_frame_outbox"], [])
            self.assertIn(frame["frame_id_sha256"], state["delivered_relation_frame_ids"])

    def test_classifier_version_change_schedules_automatic_wait_backfill(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "state.json"
            state = CAPTURE.empty_state()
            state["wait_relation_classifier_version"] = "old-version"
            state["wait_relation_files"] = {
                "rollout.jsonl": {
                    "offset": 99,
                    "pending": {"cell_id": "1"},
                    "calls": {"x": "y"},
                }
            }
            path.write_text(json.dumps(state), encoding="utf-8")
            loaded = CAPTURE.load_state(path)
            self.assertEqual(
                loaded["wait_relation_classifier_version"],
                CAPTURE.WAIT_RELATION_CLASSIFIER_VERSION,
            )
            self.assertEqual(loaded["wait_relation_files"]["rollout.jsonl"]["offset"], 0)
            self.assertIsNone(loaded["wait_relation_files"]["rollout.jsonl"]["pending"])
            self.assertEqual(loaded["wait_relation_files"]["rollout.jsonl"]["calls"], {})

    def test_pending_wait_calls_are_compact_and_legacy_payloads_are_dropped(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "state.json"
            state = CAPTURE.empty_state()
            calls = {
                f"call-{index}": {
                    "surface": "shell_batch",
                    "command_sha256": f"{index:064x}",
                    "call_shape": "custom_tool_call",
                }
                for index in range(80)
            }
            calls["legacy"] = "very large raw source"
            state["wait_relation_files"] = {
                "rollout.jsonl": {"offset": 99, "pending": None, "calls": calls}
            }
            path.write_text(json.dumps(state), encoding="utf-8")
            loaded = CAPTURE.load_state(path)
            compact = loaded["wait_relation_files"]["rollout.jsonl"]["calls"]
            self.assertEqual(len(compact), CAPTURE.MAX_PENDING_WAIT_CALLS_PER_FILE)
            self.assertNotIn("legacy", compact)

    def write_fixture(self, path: Path) -> None:
        rows = [
            {
                "timestamp": "2026-07-10T10:00:00Z",
                "type": "session_meta",
                "payload": {"id": "session-secret-id"},
            },
            {
                "timestamp": "2026-07-10T10:00:01Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "call_id": "secret-call-id",
                    "name": "exec",
                    "input": 'await tools.exec_command({cmd:"cargo test secret-project"})',
                },
            },
            {
                "timestamp": "2026-07-10T10:00:02Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "secret-call-id",
                    "output": [{"type": "text", "text": "secret raw output"}],
                },
            },
        ]
        path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")

    def test_harvest_is_incremental_grounded_and_raw_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-test.jsonl"
            self.write_fixture(path)
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_file(path, state, 10), 1)
            self.assertEqual(CAPTURE.harvest_file(path, state, 10), 0)
            self.assertEqual(len(state["outbox"]), 1)
            observation = state["outbox"][0]
            self.assertEqual(observation["action"]["event"]["kind"], "test")
            self.assertEqual(
                observation["evidence"]["verifier"],
                "codex_session_call_output_pair_v1",
            )
            serialized = json.dumps(state)
            self.assertNotIn("session-secret-id", serialized)
            self.assertNotIn("secret-call-id", serialized)
            self.assertNotIn("secret-project", serialized)
            self.assertNotIn("secret raw output", serialized)

    def test_nested_tools_keep_natural_surface_families(self) -> None:
        cases = [
            ("tools.exec_command", "exec_command", "shell"),
            ("tools.apply_patch", "apply_patch", "file_edit"),
            ("tools.view_image", "view_file", "file_read"),
            ("tools.update_plan", "update_plan", "tool_call"),
        ]
        for marker, expected_tool, expected_family in cases:
            tool, _ = CAPTURE.normalized_tool("exec", f"await {marker}({{}})")
            self.assertEqual(tool, expected_tool)
            self.assertEqual(CAPTURE.ADAPTER.tool_family(tool), expected_family)

    def test_tool_names_inside_strings_and_comments_are_not_calls(self) -> None:
        source = """
        const patch = "tools.apply_patch({fake:true})";
        // tools.view_image({fake:true})
        const result = await tools.exec_command({cmd: patch});
        """
        self.assertEqual(CAPTURE.nested_tool_calls(source), ["exec_command"])
        tool, _ = CAPTURE.normalized_tool("exec", source)
        self.assertEqual(tool, "exec_command")

    def test_distinct_nested_calls_are_not_misattributed(self) -> None:
        tool, _ = CAPTURE.normalized_tool(
            "exec",
            "await tools.exec_command({}); await tools.update_plan({});",
        )
        self.assertEqual(tool, "multi_tool")

    def test_incomplete_json_line_is_retried(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-test.jsonl"
            path.write_bytes(b'{"type":"session_meta"')
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_file(path, state, 10), 0)
            self.assertEqual(state["files"][str(path)]["offset"], 0)

    def test_provider_request_correlates_to_terminal_outcome_without_raw_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            session_id = "private-session-id"
            turn_id = "private-turn-id"
            prompt = "private prompt"
            answer = "private answer"
            session_digest = CAPTURE.sha256_text(session_id)
            turn_digest = CAPTURE.sha256_text(turn_id)
            prompt_digest = CAPTURE.sha256_text(prompt)
            shape = {
                "schema": "nando.provider-request-shape.v1",
                "client_identity_sha256": {
                    "session_id": session_digest,
                    "thread_id": session_digest,
                    "turn_id": turn_digest,
                },
                "request_text_sha256": prompt_digest,
                "request_text_bytes": len(prompt),
                "raw_text_stored": False,
            }
            provider_events = root / "provider-events.jsonl"
            provider_events.write_text(
                json.dumps(
                    {
                        "event": "bridge_request",
                        "timestamp_unix": 1,
                        "request_sha256": CAPTURE.sha256_text("request-body"),
                        "request_shape": shape,
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            rollout = root / "rollout-test.jsonl"
            rows = [
                {"type": "session_meta", "payload": {"id": session_id}},
                {"type": "turn_context", "payload": {"turn_id": turn_id}},
                {
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "user",
                        "content": [{"type": "input_text", "text": prompt}],
                    },
                },
                {
                    "timestamp": "2026-07-11T10:00:00Z",
                    "type": "event_msg",
                    "payload": {
                        "type": "task_complete",
                        "turn_id": turn_id,
                        "last_agent_message": answer,
                    },
                },
            ]
            rollout.write_text(
                "".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8"
            )
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_provider_events(provider_events, state, 10), 1)
            self.assertEqual(CAPTURE.harvest_terminal_outcomes(rollout, state, 10), 1)
            self.assertEqual(CAPTURE.reconcile_provider_outcomes(state, 10), 1)
            self.assertEqual(len(state["outcome_outbox"]), 1)
            outcome = state["outcome_outbox"][0]
            self.assertEqual(outcome["schema"], "nando.provider-outcome.v1")
            self.assertEqual(outcome["provider_request_count"], 1)
            self.assertEqual(outcome["request_text_sha256"], prompt_digest)
            self.assertEqual(outcome["outcome"]["sha256"], CAPTURE.sha256_text(answer))
            self.assertFalse(outcome["raw_text_stored"])
            serialized = json.dumps(outcome)
            self.assertNotIn(session_id, serialized)
            self.assertNotIn(turn_id, serialized)
            self.assertNotIn(prompt, serialized)
            self.assertNotIn(answer, serialized)

            outcome_path = root / "provider-outcomes.jsonl"
            self.assertEqual(CAPTURE.append_provider_outcomes(outcome_path, state, 10), 1)
            written = json.loads(outcome_path.read_text(encoding="utf-8"))
            self.assertEqual(written["outcome_id"], outcome["outcome_id"])
            state["outcome_outbox"].append(outcome)
            self.assertEqual(CAPTURE.append_provider_outcomes(outcome_path, state, 10), 0)
            self.assertEqual(len(outcome_path.read_text(encoding="utf-8").splitlines()), 1)

    def test_provider_outcome_requires_matching_request_text_hash(self) -> None:
        state = CAPTURE.empty_state()
        session_digest = CAPTURE.sha256_text("session")
        turn_digest = CAPTURE.sha256_text("turn")
        key = CAPTURE.provider_turn_key(session_digest, turn_digest)
        state["provider_events"]["by_turn"][key] = [
            {
                "request_sha256": CAPTURE.sha256_text("request"),
                "request_text_sha256": CAPTURE.sha256_text("different"),
            }
        ]
        state["pending_terminals"][key] = {
            "request_text_sha256": CAPTURE.sha256_text("prompt")
        }
        self.assertEqual(CAPTURE.reconcile_provider_outcomes(state, 10), 0)
        self.assertIn(key, state["pending_terminals"])

    def test_response_relation_extracts_program_without_raw_filler(self) -> None:
        relation = CAPTURE.build_response_relation(
            "Reply exactly: PRIVATE-FILLER",
            "PRIVATE-FILLER",
            CAPTURE.sha256_text("session"),
            CAPTURE.sha256_text("turn"),
            "2026-07-11T10:00:00Z",
        )
        self.assertIsNotNone(relation)
        assert relation is not None
        self.assertEqual(relation["relation"], "outcome_equals_request_suffix")
        self.assertEqual(relation["program_hint"]["prefix"], "reply exactly:")
        serialized = json.dumps(relation)
        self.assertNotIn("PRIVATE-FILLER", serialized)
        self.assertFalse(relation["raw_request_stored"])
        self.assertFalse(relation["raw_outcome_stored"])

    def test_response_relation_abstains_when_outcome_is_not_request_suffix(self) -> None:
        relation = CAPTURE.build_response_relation(
            "Explain the project",
            "A long generated explanation",
            CAPTURE.sha256_text("session"),
            CAPTURE.sha256_text("turn"),
            "2026-07-11T10:00:00Z",
        )
        self.assertIsNone(relation)

    def test_future_shadow_records_correct_and_wrong_without_raw_text(self) -> None:
        candidates = [{"package_id": "package-1", "prefixes": ["reply exactly:"]}]
        correct = CAPTURE.build_response_shadows(
            "Reply exactly: VALUE-1",
            "VALUE-1",
            CAPTURE.sha256_text("session"),
            CAPTURE.sha256_text("turn-1"),
            "2026-07-11T10:00:00Z",
            candidates,
        )
        wrong = CAPTURE.build_response_shadows(
            "Reply exactly: VALUE-2",
            "different",
            CAPTURE.sha256_text("session"),
            CAPTURE.sha256_text("turn-2"),
            "2026-07-11T10:01:00Z",
            candidates,
        )
        self.assertEqual(len(correct), 1)
        self.assertTrue(correct[0]["verifier_ok"])
        self.assertFalse(wrong[0]["verifier_ok"])
        serialized = json.dumps(correct + wrong)
        self.assertNotIn("VALUE-1", serialized)
        self.assertNotIn("VALUE-2", serialized)
        self.assertNotIn("different", serialized)

    def pending_action_rows(self, action: dict) -> list[dict]:
        return [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-1",
                    "input": "await tools.exec_command({cmd:'cargo test'})",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "exec-1",
                    "output": (
                        "Script running with cell ID cell-859\n"
                        "Wall time 10.0 seconds\nOutput:\n"
                    ),
                },
            },
            {
                "timestamp": "2026-07-11T10:00:00Z",
                "type": "response_item",
                "payload": action,
            },
        ]

    def test_pending_output_followed_by_non_wait_action_is_negative(self) -> None:
        secret = "private-next-action-value"
        cases = [
            (
                "function_call",
                {
                    "type": "function_call",
                    "name": "lookup",
                    "call_id": "lookup-1",
                    "arguments": json.dumps({"query": secret}),
                },
                {"kind": "action_function", "value": "lookup"},
                14,
            ),
            (
                "function_call",
                {
                    "type": "function_call",
                    "name": "x" * (CAPTURE.MAX_RELATION_ACTION_NAME_BYTES + 1),
                    "call_id": "long-name-1",
                    "arguments": json.dumps({"query": secret}),
                },
                None,
                13,
            ),
            (
                "custom_tool_call",
                {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-2",
                    "input": f"await tools.exec_command({{cmd:'{secret}'}})",
                },
                None,
                13,
            ),
        ]
        for response_shape, action, expected_action_atom, expected_atom_count in cases:
            with self.subTest(response_shape=response_shape, name=action["name"][:16]):
                with tempfile.TemporaryDirectory() as temporary:
                    path = Path(temporary) / "rollout-non-wait.jsonl"
                    rows = self.pending_action_rows(action)
                    path.write_text(
                        "".join(json.dumps(row) + "\n" for row in rows),
                        encoding="utf-8",
                    )
                    state = CAPTURE.empty_state()

                    self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 1)
                    self.assertEqual(len(state["response_relation_outbox"]), 1)
                    self.assertEqual(len(state["relation_frame_outbox"]), 1)
                    self.assertFalse(state["response_relation_outbox"][0]["verifier_ok"])
                    frame = state["relation_frame_outbox"][0]
                    self.assertFalse(frame["verifier_label"])
                    self.assertEqual(len(frame["atoms"]), expected_atom_count)
                    self.assertIn(
                        {"kind": "response_shape", "value": response_shape},
                        frame["atoms"],
                    )
                    self.assertIn(
                        {
                            "kind": "cardinality",
                            "role": "active_pending_handle_count_band",
                            "count": 1,
                        },
                        frame["atoms"],
                    )
                    if expected_action_atom is None:
                        self.assertFalse(
                            any(atom["kind"] == "action_function" for atom in frame["atoms"])
                        )
                    else:
                        self.assertIn(expected_action_atom, frame["atoms"])
                    self.assertFalse(
                        any(
                            atom["kind"] in {
                                "action_role_argument",
                                "action_integer_argument",
                                "slot_equality",
                            }
                            or (
                                atom["kind"] == "typed_slot"
                                and atom.get("source") == "action"
                            )
                            for atom in frame["atoms"]
                        )
                    )
                    serialized = json.dumps(state)
                    self.assertNotIn(secret, serialized)
                    self.assertNotIn("cell-859", serialized)
                    self.assertNotIn("Wall time", serialized)
                    self.assertIsNone(state["wait_relation_files"][str(path)]["pending"])

    def test_non_wait_action_can_seed_the_next_pending_relation(self) -> None:
        secret = "private-second-command"
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-two-pending.jsonl"
            next_action = {
                "type": "custom_tool_call",
                "name": "exec",
                "call_id": "exec-2",
                "input": f"await tools.exec_command({{cmd:'pytest {secret}'}})",
            }
            rows = self.pending_action_rows(next_action)
            rows.extend(
                [
                    {
                        "type": "response_item",
                        "payload": {
                            "type": "custom_tool_call_output",
                            "call_id": "exec-2",
                            "output": "Script running with cell ID cell-860\n",
                        },
                    },
                    {
                        "timestamp": "2026-07-11T10:00:01Z",
                        "type": "response_item",
                        "payload": {
                            "type": "function_call",
                            "name": "wait",
                            "call_id": "wait-2",
                            "arguments": json.dumps(
                                {
                                    "cell_id": "cell-860",
                                    "yield_time_ms": 1_000,
                                    "max_tokens": 5_000,
                                }
                            ),
                        },
                    },
                ]
            )
            path.write_text(
                "".join(json.dumps(row) + "\n" for row in rows),
                encoding="utf-8",
            )
            state = CAPTURE.empty_state()

            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 1), 1)
            file_state = state["wait_relation_files"][str(path)]
            self.assertEqual(len(file_state["calls"]), 1)
            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 1), 1)
            self.assertEqual(
                [frame["verifier_label"] for frame in state["relation_frame_outbox"]],
                [False, True],
            )
            self.assertIsNone(file_state["pending"])
            self.assertEqual(file_state["calls"], {})
            self.assertNotIn(secret, json.dumps(state))

    def test_malformed_or_unbounded_wait_action_fails_closed(self) -> None:
        secret = "private-unsupported-argument"
        cases = [
            ("{", False),
            ("[]", False),
            (
                json.dumps(
                    {"cell_id": "cell-859", "yield_time_ms": True, "max_tokens": 5_000}
                ),
                True,
            ),
            (
                json.dumps({"cell_id": "cell-859", "unsupported": secret}),
                True,
            ),
        ]
        for arguments, has_action_function in cases:
            with self.subTest(arguments=arguments[:32]):
                with tempfile.TemporaryDirectory() as temporary:
                    path = Path(temporary) / "rollout-invalid-wait.jsonl"
                    action = {
                        "type": "function_call",
                        "name": "wait",
                        "call_id": "wait-invalid",
                        "arguments": arguments,
                    }
                    path.write_text(
                        "".join(
                            json.dumps(row) + "\n"
                            for row in self.pending_action_rows(action)
                        ),
                        encoding="utf-8",
                    )
                    state = CAPTURE.empty_state()

                    self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 1)
                    relation = state["response_relation_outbox"][0]
                    frame = state["relation_frame_outbox"][0]
                    self.assertFalse(relation["verifier_ok"])
                    self.assertFalse(frame["verifier_label"])
                    self.assertEqual(
                        any(atom["kind"] == "action_function" for atom in frame["atoms"]),
                        has_action_function,
                    )
                    self.assertFalse(
                        any(
                            atom["kind"] in {
                                "action_role_argument",
                                "action_integer_argument",
                                "slot_equality",
                            }
                            or (
                                atom["kind"] == "typed_slot"
                                and atom.get("source") == "action"
                            )
                            for atom in frame["atoms"]
                        )
                    )
                    self.assertNotIn(secret, json.dumps(state))

    def test_wait_relation_is_grounded_and_raw_free(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-wait.jsonl"
            rows = [
                {"type":"session_meta","payload":{"id":"private-session"}},
                {"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"exec-1","input":"await tools.exec_command({cmd:'cargo test'})"}},
                {"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"exec-1","output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"}},
                {"timestamp":"2026-07-11T10:00:00Z","type":"response_item","payload":{"type":"function_call","name":"wait","call_id":"wait-1","arguments":json.dumps({"cell_id":"859","yield_time_ms":1000,"max_tokens":5000})}},
            ]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 1)
            relation = state["response_relation_outbox"][0]
            self.assertTrue(relation["verifier_ok"])
            self.assertEqual(relation["program_hint"]["op"], "wait_on_yielded_cell")
            self.assertEqual(relation["guard_schema"], "wait_long_running_batch_guard.v5")
            serialized = json.dumps(relation)
            self.assertNotIn("private-session", serialized)
            self.assertNotIn("Script running", serialized)
            frame = state["relation_frame_outbox"][0]
            self.assertEqual(frame["schema"], CAPTURE.RELATION_FRAME_SCHEMA)
            self.assertEqual(frame["extractor_version"], CAPTURE.RELATION_EXTRACTOR_VERSION)
            self.assertEqual(len(frame["atoms"]), 19)
            self.assertTrue(frame["verifier_label"])
            atom_kinds = {atom["kind"] for atom in frame["atoms"]}
            self.assertIn("action_function", atom_kinds)
            self.assertIn("action_role_argument", atom_kinds)
            self.assertIn("action_integer_argument", atom_kinds)
            self.assertIn("observation_selector", atom_kinds)
            self.assertIn(
                {"kind": "observation_call_shape", "value": "custom_tool_call"},
                frame["atoms"],
            )
            context = {
                atom["role"]: atom["count"]
                for atom in frame["atoms"]
                if atom["kind"] == "cardinality"
            }
            self.assertEqual(
                context,
                {
                    "turn_call_count_band": 1,
                    "turn_output_count_band": 1,
                    "turn_pending_count_band": 1,
                    "active_pending_handle_count_band": 1,
                    "turn_message_count_band": 0,
                    "turn_call_shape_count_band": 1,
                },
            )
            frame_text = json.dumps(frame)
            self.assertNotIn("private-session", frame_text)
            self.assertIn("Script running with cell ID ", frame_text)
            self.assertNotIn("Wall time", frame_text)
            self.assertNotIn('"859"', frame_text)
            self.assertNotIn("cargo test", frame_text)
            self.assertNotIn("program_hint", frame_text)

    def test_count_band_is_bounded_and_power_of_two(self) -> None:
        self.assertEqual([CAPTURE.count_band(value) for value in range(10)], [0, 1, 2, 2, 4, 4, 4, 4, 8, 8])

    def test_any_wait_relation_discovers_non_build_surface(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-any-wait.jsonl"
            rows = [
                {"type":"session_meta","payload":{"id":"private-session"}},
                {"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"exec-2","input":"await tools.exec_command({cmd:'journalctl -f'})"}},
                {"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"exec-2","output":"Script running with cell ID service-42\nWall time 10.0 seconds\nOutput:\n"}},
                {"timestamp":"2026-07-11T10:00:00Z","type":"response_item","payload":{"type":"function_call","name":"wait","call_id":"wait-2","arguments":json.dumps({"cell_id":"service-42","yield_time_ms":1000,"max_tokens":5000})}},
            ]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 1)
            relation = state["response_relation_outbox"][0]
            self.assertTrue(relation["verifier_ok"])
            self.assertEqual(relation["program_hint"]["op"], "wait_on_yielded_surfaces")
            self.assertEqual(relation["program_hint"]["prefix"], "service_observation")
            self.assertEqual(relation["guard_schema"], "wait_yielded_surface_guard.v3")

    def test_new_user_turn_cancels_pending_wait_relation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-interrupted-wait.jsonl"
            rows = [
                {"type":"session_meta","payload":{"id":"private-session"}},
                {"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"exec-1","input":"await tools.exec_command({cmd:'cargo test'})"}},
                {"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"exec-1","output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"}},
                {"type":"event_msg","payload":{"type":"user_message"}},
                {"timestamp":"2026-07-11T10:00:00Z","type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"exec-2","input":"await tools.exec_command({cmd:'git status'})"}},
            ]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 0)
            self.assertEqual(state["response_relation_outbox"], [])
            self.assertEqual(state["relation_frame_outbox"], [])

    def test_wrong_wait_handle_remains_negative_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-wrong-wait.jsonl"
            rows = [
                {"type":"session_meta","payload":{"id":"private-session"}},
                {"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"exec-1","input":"await tools.exec_command({cmd:'cargo test'})"}},
                {"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"exec-1","output":"Script running with cell ID 859\nWall time 10.0 seconds\nOutput:\n"}},
                {"timestamp":"2026-07-11T10:00:00Z","type":"response_item","payload":{"type":"function_call","name":"wait","call_id":"wait-1","arguments":json.dumps({"cell_id":"different","yield_time_ms":1000,"max_tokens":5000})}},
            ]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_wait_relations(path, state, 10), 1)
            self.assertFalse(state["response_relation_outbox"][0]["verifier_ok"])
            self.assertFalse(state["relation_frame_outbox"][0]["verifier_label"])

    def test_generic_relation_adapter_feeds_the_shared_frame_outbox(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-generic.jsonl"
            rows = [
                {"type": "session_meta", "payload": {"id": "private-session"}},
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "lookup",
                        "call_id": "lookup-1",
                        "arguments": "{}",
                    },
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "call_id": "lookup-1",
                        "output": '{"value":"private result"}',
                    },
                },
                {
                    "timestamp": "2026-07-12T11:00:00Z",
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "consume",
                        "call_id": "consume-1",
                        "arguments": json.dumps({"value": "private result"}),
                    },
                },
            ]
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = CAPTURE.empty_state()
            self.assertEqual(CAPTURE.harvest_generic_response_relations(path, state, 10), 0)
            file_state = state["generic_relation_files"][str(path)]
            self.assertEqual(len(file_state["pending_model_action"]), 1)
            self.assertEqual(file_state["ready_frames"], [])
            with path.open("a", encoding="utf-8") as handle:
                handle.write(
                    json.dumps(
                        {
                            "type": "event_msg",
                            "payload": {
                                "type": "token_count",
                                "info": {"last_token_usage": {"input_tokens": 123}},
                            },
                        }
                    )
                    + "\n"
                )
            self.assertEqual(CAPTURE.harvest_generic_response_relations(path, state, 10), 1)
            self.assertEqual(CAPTURE.harvest_generic_response_relations(path, state, 10), 0)
            self.assertEqual(file_state["pending_model_action"], [])
            self.assertEqual(file_state["ready_frames"], [])
            self.assertEqual(len(state["relation_frame_outbox"]), 1)
            serialized = json.dumps(state)
            self.assertNotIn("private result", serialized)
            self.assertEqual(
                state["generic_relation_classifier_version"],
                CAPTURE.GENERIC_RELATION_CLASSIFIER_VERSION,
            )

    def test_generic_relation_backfill_bootstraps_from_a_bounded_recent_tail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-large.jsonl"
            prefix = "".join(
                json.dumps({"type": "event_msg", "payload": {"index": index}}) + "\n"
                for index in range(256)
            )
            rows = [
                {"type": "session_meta", "payload": {"id": "private-session"}},
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "lookup",
                        "call_id": "lookup-1",
                        "arguments": "{}",
                    },
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "call_id": "lookup-1",
                        "output": '{"value":"private result"}',
                    },
                },
                {
                    "timestamp": "2026-07-12T11:00:00Z",
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "consume",
                        "call_id": "consume-1",
                        "arguments": json.dumps({"value": "private result"}),
                    },
                },
                {
                    "type": "event_msg",
                    "payload": {
                        "type": "token_count",
                        "info": {"last_token_usage": {"input_tokens": 123}},
                    },
                },
            ]
            path.write_text(
                prefix + "".join(json.dumps(row) + "\n" for row in rows),
                encoding="utf-8",
            )
            state = CAPTURE.empty_state()
            self.assertEqual(
                CAPTURE.harvest_generic_response_relations(
                    path, state, 10, backfill_bytes=2_048
                ),
                1,
            )
            file_state = state["generic_relation_files"][str(path)]
            self.assertGreater(file_state["offset"], len(prefix) - 2_048)
            self.assertRegex(file_state["session_id_sha256"], r"^[0-9a-f]{64}$")
            self.assertEqual(file_state["pending_model_action"], [])
            self.assertEqual(file_state["ready_frames"], [])
            self.assertNotIn("private result", json.dumps(state))


if __name__ == "__main__":
    unittest.main()
