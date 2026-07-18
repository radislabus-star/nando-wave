import importlib.util
import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
SPEC = importlib.util.spec_from_file_location(
    "nando_codex_response_relations",
    BIN_DIR / "nando-codex-response-relations.py",
)
RELATIONS = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(RELATIONS)


class CodexResponseRelationsTest(unittest.TestCase):
    def harvest(self, rows: list[dict]) -> tuple[list[dict], dict]:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout.jsonl"
            path.write_text(
                "".join(json.dumps(row) + "\n" for row in rows + [{
                    "type": "turn_context", "payload": {"marker": "safe-boundary"}
                }]),
                encoding="utf-8",
            )
            state = RELATIONS.empty_file_state()
            frames = RELATIONS.harvest_relation_frames(path, state, 64)
            self.assertEqual(RELATIONS.harvest_relation_frames(path, state, 64), [])
            return frames, state

    @staticmethod
    def base_rows(output: str, function: str, argument_name: str, argument) -> list[dict]:
        return [
            {
                "timestamp": "2026-07-12T10:00:00Z",
                "type": "session_meta",
                "payload": {"id": "private-session"},
            },
            {
                "timestamp": "2026-07-12T10:00:01Z",
                "type": "response_item",
                "payload": {"type": "message", "role": "user", "content": "private prompt"},
            },
            {
                "timestamp": "2026-07-12T10:00:02Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "source_lookup",
                    "call_id": "source-call",
                    "arguments": "{}",
                },
            },
            {
                "timestamp": "2026-07-12T10:00:03Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "source-call",
                    "output": output,
                },
            },
            {
                "timestamp": "2026-07-12T10:00:04Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": function,
                    "call_id": "target-call",
                    "arguments": json.dumps({argument_name: argument}),
                },
            },
        ]

    def test_discovers_scalar_transfer_without_storing_raw_value(self) -> None:
        secret = "private result value"
        frames, state = self.harvest(
            self.base_rows('{"nested":{"result":"private result value"}}', "route_result", "payload", secret)
        )
        self.assertEqual(len(frames), 1)
        frame = frames[0]
        self.assertTrue(frame["verifier_label"])
        self.assertEqual(frame["extractor_version"], RELATIONS.RELATION_EXTRACTOR_VERSION)
        self.assertTrue(
            any(atom.get("kind") == "slot_equality" for atom in frame["atoms"])
        )
        self.assertTrue(
            any(
                atom.get("kind") == "action_role_argument"
                and atom.get("name") == "payload"
                for atom in frame["atoms"]
            )
        )
        serialized = json.dumps({"frames": frames, "state": state})
        self.assertNotIn(secret, serialized)
        self.assertNotIn("private prompt", serialized)
        self.assertNotIn("private-session", serialized)

    def test_turn_unique_json_selector_separates_concurrent_values(self) -> None:
        rows = self.base_rows('{"session_id":11}', "poll", "session_id", 11)
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:05Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "source_lookup",
                    "call_id": "source-call-2",
                    "arguments": "{}",
                },
            },
            {
                "timestamp": "2026-07-12T10:00:06Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "source-call-2",
                    "output": '{"session_id":22}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:07Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "poll",
                    "call_id": "target-call-2",
                    "arguments": json.dumps({"session_id": 11}),
                },
            },
        ])
        frames, _ = self.harvest(rows)
        self.assertEqual(len(frames), 3)
        first_selector = next(
            atom["selector"] for atom in frames[0]["atoms"]
            if atom["kind"] == "observation_selector"
        )
        negative_selectors = [
            atom["selector"]
            for frame in frames[1:]
            for atom in frame["atoms"]
            if atom["kind"] == "observation_selector"
        ]
        self.assertTrue(frames[0]["verifier_label"])
        self.assertEqual(first_selector["kind"], "unique_active_turn_json_field")
        self.assertTrue(all(not frame["verifier_label"] for frame in frames[1:]))
        self.assertIn("json_field", {selector["kind"] for selector in negative_selectors})

    def test_completed_single_handle_allows_next_active_handle(self) -> None:
        rows = self.base_rows('{"session_id":11}', "poll", "session_id", 11)
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:05Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "target-call",
                    "output": '{"exit_code":0}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:06Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "source_lookup",
                    "call_id": "source-call-2",
                    "arguments": "{}",
                },
            },
            {
                "timestamp": "2026-07-12T10:00:07Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "source-call-2",
                    "output": '{"session_id":22}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:08Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "poll",
                    "call_id": "target-call-2",
                    "arguments": json.dumps({"session_id": 22}),
                },
            },
        ])
        frames, _ = self.harvest(rows)
        positives = [frame for frame in frames if frame["verifier_label"]]
        self.assertEqual(len(positives), 2)
        selectors = [
            next(atom["selector"] for atom in frame["atoms"] if atom["kind"] == "observation_selector")
            for frame in positives
        ]
        self.assertTrue(all(selector["kind"] == "unique_active_turn_json_field" for selector in selectors))

    def test_captures_bounded_cold_collection_example_without_hot_program_label(self) -> None:
        rows = self.base_rows(
            '{"rows":[{"kind":"keep","value":3},{"kind":"drop","value":4},{"kind":"keep","value":5}]}',
            "unused",
            "unused",
            0,
        )[:4]
        rows.append({
            "timestamp": "2026-07-12T10:00:04Z",
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "phase": "final_answer",
                "content": [{"type": "output_text", "text": "[3,5]"}],
            },
        })
        frames, _ = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        cold = frames[0]["cold_collection_example"]
        self.assertEqual(cold["schema"], "nando.response-collection-synthesis-example.v1")
        self.assertEqual(cold["expected_response"], "[3,5]")
        self.assertTrue(
            any(atom["kind"] == "request_phase_atom" for atom in frames[0]["atoms"])
        )
        self.assertNotIn("private prompt", json.dumps(frames[0]))
        self.assertNotIn("operator", json.dumps(frames[0]))
        self.assertNotIn("program_hint", json.dumps(frames[0]))

    def test_current_event_messages_capture_collection_and_request_phase(self) -> None:
        rows = self.base_rows(
            '{"rows":[{"kind":"keep","value":3},{"kind":"drop","value":4},{"kind":"keep","value":5}]}',
            "unused",
            "unused",
            0,
        )[:1]
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:01Z",
                "type": "event_msg",
                "payload": {
                    "type": "user_message",
                    "message": "Return the selected values.",
                },
            },
            *self.base_rows(
                '{"rows":[{"kind":"keep","value":3},{"kind":"drop","value":4},{"kind":"keep","value":5}]}',
                "unused",
                "unused",
                0,
            )[2:4],
            {
                "timestamp": "2026-07-12T10:00:04Z",
                "type": "event_msg",
                "payload": {
                    "type": "agent_message",
                    "message": "[3,5]",
                    "phase": "final_answer",
                },
            },
        ])
        frames, _ = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        self.assertEqual(frames[0]["cold_collection_example"]["expected_response"], "[3,5]")
        self.assertTrue(any(atom["kind"] == "request_phase_atom" for atom in frames[0]["atoms"]))

    def test_current_agent_message_captures_exact_scalar_projection(self) -> None:
        rows = self.base_rows('{"candidate":42}', "unused", "unused", 0)[:4]
        rows.append({
            "timestamp": "2026-07-12T10:00:03.5Z",
            "type": "event_msg",
            "payload": {
                "type": "agent_message",
                "message": "Checking the result.",
                "phase": "commentary",
            },
        })
        rows.append({
            "timestamp": "2026-07-12T10:00:04Z",
            "type": "event_msg",
            "payload": {
                "type": "agent_message",
                "message": "42",
                "phase": "final_answer",
            },
        })
        frames, _ = self.harvest(rows)
        positives = [frame for frame in frames if frame["verifier_label"]]
        self.assertEqual(len(positives), 1)
        self.assertTrue(any(atom["kind"] == "action_value_projection" for atom in positives[0]["atoms"]))

    def test_collection_parser_accepts_unique_json_segment_and_input_text_blocks(self) -> None:
        expected = {
            "rows": [
                {"kind": "keep", "value": 3},
                {"kind": "drop", "value": 4},
            ]
        }
        self.assertEqual(
            RELATIONS._bounded_collection_json(
                "command output\n```json\n"
                + json.dumps(expected, separators=(",", ":"))
                + "\n```\ncompleted"
            ),
            expected,
        )
        self.assertEqual(
            RELATIONS._bounded_collection_json(
                [{"type": "input_text", "text": json.dumps(expected)}]
            ),
            expected,
        )

    def test_collection_parser_accepts_root_array_with_neutral_wrapper(self) -> None:
        self.assertEqual(
            RELATIONS._bounded_collection_json('[{"value":1},{"value":2}]'),
            {"items": [{"value": 1}, {"value": 2}]},
        )

    def test_collection_parser_abstains_on_distinct_json_collections(self) -> None:
        output = '\n'.join(
            [
                '{"rows":[{"value":1}]}',
                '{"rows":[{"value":2}]}',
            ]
        )
        self.assertIsNone(RELATIONS._bounded_collection_json(output))

    def test_collection_survives_later_noncollection_tool_call_in_same_turn(self) -> None:
        rows = self.base_rows(
            '{"rows":[{"kind":"keep","value":3},{"kind":"drop","value":4},{"kind":"keep","value":5}]}',
            "later_check",
            "unused",
            0,
        )
        rows.append({
            "timestamp": "2026-07-12T10:00:05Z",
            "type": "response_item",
            "payload": {
                "type": "function_call_output",
                "call_id": "target-call",
                "output": "check completed",
            },
        })
        rows.append({
            "timestamp": "2026-07-12T10:00:06Z",
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "phase": "final_answer",
                "content": [{"type": "output_text", "text": "Selected values: [3,5]."}],
            },
        })
        frames, _ = self.harvest(rows)
        cold = [frame["cold_collection_example"] for frame in frames if "cold_collection_example" in frame]
        self.assertEqual(len(cold), 1)
        self.assertEqual(cold[0]["expected_response"], "Selected values: [3,5].")

    def test_two_distinct_turn_collections_abstain(self) -> None:
        rows = self.base_rows(
            '{"rows":[{"value":1}]}',
            "second_lookup",
            "unused",
            0,
        )
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:05Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "target-call",
                    "output": '{"rows":[{"value":2}]}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:06Z",
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "phase": "final_answer",
                    "content": [{"type": "output_text", "text": "Count: 1."}],
                },
            },
        ])
        frames, _ = self.harvest(rows)
        self.assertFalse(any("cold_collection_example" in frame for frame in frames))

    def test_collection_template_privacy_matches_rust_package_boundary(self) -> None:
        self.assertTrue(RELATIONS._safe_collection_template("Selected values: ."))
        self.assertTrue(RELATIONS._safe_collection_template("Результат: ."))
        for unsafe in [
            "Authorization: Bearer AbC123",
            "email=user@example.com",
            "source=/home/ubu/private.json",
            "source=C:\\private\\data.json",
            "key=AbCdEfGhIjKlMnOpQrStUv123456",
            "digest=0123456789abcdef0123456789abcdef",
            "line one\nline two",
            "Клиент Иван Иванов:",
            "Телефон +7 999 123-45-67:",
            "Адрес Невский проспект:",
            "Client Acme Corporation:",
        ]:
            self.assertFalse(RELATIONS._safe_collection_template(unsafe), unsafe)

    def test_cold_collection_rejects_secret_like_fields_and_values(self) -> None:
        self.assertIsNone(
            RELATIONS._bounded_collection_json(
                '{"rows":[{"api_token":"AbC123456789012345678901234"}]}'
            )
        )

    def test_request_phase_encoder_matches_rust_fnv_contract(self) -> None:
        self.assertIn(
            15291052347829727369,
            RELATIONS.request_phase_atom_ids("Please count these rows"),
        )
        self.assertIn(
            13856698933407100379,
            RELATIONS.request_phase_atom_ids("Пожалуйста, посчитай строки"),
        )

    def test_renamed_function_argument_and_scalar_types_are_structural(self) -> None:
        cases = [
            ("42", 42, "integer"),
            ("true", True, "boolean"),
            ('"fresh-id"', "fresh-id", "identifier"),
            ('"fresh value"', "fresh value", "string"),
        ]
        for index, (output, argument, value_type) in enumerate(cases):
            with self.subTest(value_type=value_type):
                frames, _ = self.harvest(
                    self.base_rows(output, f"unknown_function_{index}", f"unknown_arg_{index}", argument)
                )
                self.assertEqual(len(frames), 1)
                observation = next(
                    atom
                    for atom in frames[0]["atoms"]
                    if atom.get("kind") == "typed_slot"
                    and atom.get("source") == "observation"
                )
                self.assertEqual(observation["value_type"], value_type)

    def test_multi_field_output_learns_an_exact_field_selector(self) -> None:
        frames, state = self.harvest(
            self.base_rows('{"left":"one","right":"two"}', "route_result", "payload", "one")
        )
        self.assertEqual(len(frames), 1)
        self.assertTrue(frames[0]["verifier_label"])
        self.assertTrue(
            any(
                atom.get("kind") == "observation_selector"
                and atom.get("selector", {}).get("field") == "left"
                for atom in frames[0]["atoms"]
            )
        )
        self.assertEqual(state["observations"], [])

    def test_assistant_can_learn_from_an_earlier_unique_turn_output(self) -> None:
        rows = self.base_rows('{"result":"selected"}', "inspect_more", "unused", 0)
        rows[-1]["payload"]["arguments"] = "{}"
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:05Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "target-call",
                    "output": '{"other":42}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:06Z",
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "phase": "final_answer",
                    "content": [
                        {"type": "output_text", "text": "Selected value: selected."}
                    ],
                },
            },
        ])
        frames, _ = self.harvest(rows)
        positives = [frame for frame in frames if frame["verifier_label"]]
        self.assertEqual(len(positives), 1)
        selector = next(
            atom["selector"]
            for atom in positives[0]["atoms"]
            if atom["kind"] == "observation_selector"
        )
        self.assertEqual(selector["field"], "result")
        self.assertEqual(selector["kind"], "unique_active_turn_json_field")
        projection = next(
            atom
            for atom in positives[0]["atoms"]
            if atom["kind"] == "action_value_projection"
        )
        self.assertEqual(projection["renderer"]["prefix"], "Selected value: ")
        self.assertEqual(projection["renderer"]["suffix"], ".")

    def test_changed_turn_field_does_not_replay_an_old_teacher_value(self) -> None:
        rows = self.base_rows('{"result":"selected"}', "inspect_more", "unused", 0)
        rows[-1]["payload"]["arguments"] = "{}"
        rows.extend([
            {
                "timestamp": "2026-07-12T10:00:05Z",
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "target-call",
                    "output": '{"result":"changed"}',
                },
            },
            {
                "timestamp": "2026-07-12T10:00:06Z",
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "phase": "final_answer",
                    "content": [
                        {"type": "output_text", "text": "Selected value: selected."}
                    ],
                },
            },
        ])
        frames, _ = self.harvest(rows)
        self.assertFalse(any(frame["verifier_label"] for frame in frames))

    def test_unrelated_next_action_becomes_negative_evidence(self) -> None:
        frames, _ = self.harvest(
            self.base_rows('{"result":"source"}', "other_action", "payload", "different")
        )
        self.assertGreaterEqual(len(frames), 1)
        self.assertTrue(all(frame["verifier_label"] is False for frame in frames))
        self.assertFalse(
            any(
                atom.get("kind") == "slot_equality"
                for frame in frames
                for atom in frame["atoms"]
            )
        )

    def test_new_user_turn_clears_pending_observation(self) -> None:
        rows = self.base_rows('{"result":"source"}', "route_result", "payload", "source")
        rows.insert(
            -1,
            {
                "timestamp": "2026-07-12T10:00:03.5Z",
                "type": "response_item",
                "payload": {"type": "message", "role": "user", "content": "new turn"},
            },
        )
        frames, _ = self.harvest(rows)
        self.assertEqual(frames, [])

    def test_pending_and_multiline_outputs_are_not_scalar_transfers(self) -> None:
        for output in [
            "Script running with cell ID 123\n",
            "first line\nsecond line",
            "1.5",
            "null",
        ]:
            with self.subTest(output=output):
                frames, _ = self.harvest(
                    self.base_rows(output, "route_result", "payload", output)
                )
                self.assertEqual(frames, [])

    def test_line_budget_advances_incrementally_without_unbounded_scan(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "rollout-budget.jsonl"
            irrelevant = [
                {"type": "event_msg", "payload": {"type": "progress", "index": index}}
                for index in range(20)
            ]
            rows = irrelevant + self.base_rows(
                '{"result":"value"}', "route_result", "payload", "value"
            )
            rows.append(self.token_count_row(1))
            path.write_text(
                "".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8"
            )
            state = RELATIONS.empty_file_state()
            self.assertEqual(
                RELATIONS.harvest_relation_frames(path, state, 64, max_lines=10), []
            )
            first_offset = state["offset"]
            self.assertGreater(first_offset, 0)
            self.assertLess(first_offset, path.stat().st_size)
            self.assertEqual(
                RELATIONS.harvest_relation_frames(path, state, 64, max_lines=10), []
            )
            frames = RELATIONS.harvest_relation_frames(path, state, 64, max_lines=10)
            self.assertEqual(len(frames), 1)

    def test_discovers_typed_custom_tool_continuation_from_structured_output(self) -> None:
        session_id = 82344
        rows = [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-1",
                    "input": "const r=await tools.exec_command({cmd:\"cargo test\"});text(r.output);",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "exec-1",
                    "output": [
                        {"type": "text", "text": "still running"},
                        {"type": "text", "text": f"SESSION_ID={session_id}"},
                    ],
                },
            },
            {
                "timestamp": "2026-07-12T10:00:04Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-2",
                    "input": (
                        "const r = await tools.write_stdin({session_id:82344,chars:\"\","
                        "yield_time_ms:30000,max_output_tokens:12000});\n"
                        "text(r.output);\n"
                        "if (r.session_id) text(`SESSION_ID=${r.session_id}`);\n"
                    ),
                },
            },
        ]
        frames, state = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        frame = frames[0]
        self.assertTrue(frame["verifier_label"])
        self.assertTrue(
            any(
                atom.get("kind") == "action_inner_tool"
                and atom.get("value") == "write_stdin"
                for atom in frame["atoms"]
            )
        )
        self.assertTrue(
            any(
                atom.get("kind") == "observation_call_shape"
                and atom.get("value") == "custom_tool_call"
                for atom in frame["atoms"]
            )
        )
        self.assertTrue(
            any(
                atom.get("kind") == "observation_selector"
                and atom.get("selector", {}).get("prefix") == "SESSION_ID="
                for atom in frame["atoms"]
            )
        )
        self.assertTrue(
            any(
                atom.get("kind") == "action_result_projection"
                and atom.get("continuation_field") == "session_id"
                for atom in frame["atoms"]
            )
        )
        serialized = json.dumps({"frame": frame, "state": state})
        self.assertNotIn(str(session_id), serialized)

    def test_discovers_json_result_projection_used_by_live_nested_tools(self) -> None:
        session_id = 91827
        rows = [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-1",
                    "input": "const r=await tools.exec_command({cmd:\"cargo test\"});text(JSON.stringify(r));",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "exec-1",
                    "output": [
                        {
                            "type": "input_text",
                            "text": json.dumps(
                                {
                                    "output": "still running",
                                    "session_id": session_id,
                                    "wall_time_seconds": 10.0,
                                }
                            ),
                        }
                    ],
                },
            },
            {
                "timestamp": "2026-07-12T10:00:04Z",
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-2",
                    "input": (
                        "const r = await tools.write_stdin({\"session_id\":91827,"
                        "\"chars\":\"\",\"yield_time_ms\":30000,"
                        "\"max_output_tokens\":12000});\n"
                        "text(JSON.stringify(r));"
                    ),
                },
            },
        ]
        frames, state = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        frame = frames[0]
        self.assertTrue(frame["verifier_label"])
        self.assertTrue(
            any(atom.get("kind") == "action_json_result_projection" for atom in frame["atoms"])
        )
        self.assertTrue(
            any(
                atom.get("kind") == "observation_selector"
                and atom.get("selector", {}).get("field") == "session_id"
                for atom in frame["atoms"]
            )
        )
        serialized = json.dumps({"frame": frame, "state": state})
        self.assertNotIn(str(session_id), serialized)

    def test_custom_tool_program_with_extra_code_is_negative(self) -> None:
        rows = [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-1",
                    "input": "safe source",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "exec-1",
                    "output": [{"type": "text", "text": "JOB_HANDLE=9"}],
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-2",
                    "input": (
                        "const r=await tools.write_stdin({job_handle:9});"
                        "text(r.output);if(r.session_id)text(`JOB_HANDLE=${r.session_id}`);"
                        "tools.dangerous({});"
                    ),
                },
            },
        ]
        frames, _ = self.harvest(rows)
        self.assertGreaterEqual(len(frames), 1)
        self.assertTrue(all(frame["verifier_label"] is False for frame in frames))

    def test_nonempty_custom_tool_string_constant_never_enters_relation_frame(self) -> None:
        secret = "private-command-value"
        rows = [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-1",
                    "input": "source",
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call_output",
                    "call_id": "exec-1",
                    "output": [
                        {"type": "input_text", "text": '{"job_handle":91827}'}
                    ],
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": "exec-2",
                    "input": (
                        "const r=await tools.write_stdin({\"job_handle\":91827,"
                        f"\"chars\":\"{secret}\"}});text(JSON.stringify(r));"
                    ),
                },
            },
        ]
        frames, state = self.harvest(rows)
        self.assertGreaterEqual(len(frames), 1)
        self.assertTrue(all(frame["verifier_label"] is False for frame in frames))
        self.assertNotIn(secret, json.dumps({"frames": frames, "state": state}))

    def assistant_rows(
        self, observation: str, content, phase: str | None = "final_answer"
    ) -> list[dict]:
        rows = self.base_rows(observation, "unused", "value", "unused")[:-1]
        payload = {
            "type": "message",
            "role": "assistant",
            "content": content,
        }
        if phase is not None:
            payload["phase"] = phase
        rows.append(
            {
                "timestamp": "2026-07-12T10:00:04Z",
                "type": "response_item",
                "payload": payload,
            }
        )
        return rows

    @staticmethod
    def token_count_row(input_tokens) -> dict:
        return {
            "timestamp": "2026-07-12T10:00:03.5Z",
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "last_token_usage": {"input_tokens": input_tokens},
                    "total_token_usage": {"input_tokens": 999_999_999},
                },
            },
        }

    def test_input_tokens_propagate_to_project_selected_value_frame(self) -> None:
        rows = self.assistant_rows(
            '{"candidate":"selected"}',
            [{"type": "output_text", "text": "selected"}],
        )
        rows.append(self.token_count_row(12_345))
        frames, state = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        self.assertTrue(frames[0]["verifier_label"])
        self.assertEqual(frames[0]["estimated_input_tokens"], 12_345)
        self.assertEqual(state["pending_model_action"], [])

    def test_input_tokens_propagate_to_existing_function_action(self) -> None:
        rows = self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
        rows.append(self.token_count_row(4_321))
        frames, _ = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        self.assertEqual(frames[0]["estimated_input_tokens"], 4_321)

    def test_input_tokens_are_consumed_once(self) -> None:
        rows = self.base_rows('{"result":"first"}', "route_result", "payload", "first")
        rows.append(self.token_count_row(777))
        rows.extend(
            [
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "call_id": "target-call",
                        "output": '{"result":"second"}',
                    },
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "route_result",
                        "call_id": "second-target",
                        "arguments": json.dumps({"payload": "second"}),
                    },
                },
            ]
        )
        frames, _ = self.harvest(rows)
        self.assertEqual([frame["estimated_input_tokens"] for frame in frames], [777, 0])

    def test_input_tokens_reset_on_new_turn(self) -> None:
        rows = self.base_rows('{"result":"old"}', "unused", "payload", "unused")[:-1]
        rows.extend(
            [
                self.token_count_row(888),
                {
                    "type": "response_item",
                    "payload": {"type": "message", "role": "user", "content": "new turn"},
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "source_lookup",
                        "call_id": "new-source",
                        "arguments": "{}",
                    },
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call_output",
                        "call_id": "new-source",
                        "output": '{"result":"new"}',
                    },
                },
                {
                    "type": "response_item",
                    "payload": {
                        "type": "function_call",
                        "name": "route_result",
                        "call_id": "new-target",
                        "arguments": json.dumps({"payload": "new"}),
                    },
                },
            ]
        )
        frames, _ = self.harvest(rows)
        self.assertEqual(len(frames), 1)
        self.assertEqual(frames[0]["estimated_input_tokens"], 0)

    def test_invalid_input_tokens_use_safe_zero_default(self) -> None:
        invalid = [-1, True, 1.5, RELATIONS.MAX_LAST_INPUT_TOKENS + 1]
        for value in invalid:
            with self.subTest(value=value):
                rows = self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
                rows.append(self.token_count_row(value))
                frames, state = self.harvest(rows)
                self.assertEqual(frames[0]["estimated_input_tokens"], 0)
                self.assertEqual(state["pending_model_action"], [])

    def test_orphan_token_count_is_not_persisted_for_a_future_action(self) -> None:
        rows = [
            {"type": "session_meta", "payload": {"id": "private-session"}},
            self.token_count_row(55_555),
        ]
        frames, state = self.harvest(rows)
        self.assertEqual(frames, [])
        self.assertNotIn("last_input_tokens", state["turn"])
        serialized = json.dumps(state)
        self.assertNotIn("total_token_usage", serialized)
        self.assertNotIn("last_token_usage", serialized)
        self.assertEqual(
            RELATIONS.normalize_file_state({"turn": {"last_input_tokens": 999}})["turn"].get("last_input_tokens"),
            None,
        )

    def test_project_status_maps_zero_and_bounded_nonzero_source_neutrally(self) -> None:
        cases = [
            ('{"opaque_value":0}', "success", "zero"),
            ("opaque_value=913457", "failure", "nonzero"),
        ]
        for observation, response, status_class in cases:
            with self.subTest(response=response):
                frames, state = self.harvest(
                    self.assistant_rows(
                        observation,
                        [{"type": "output_text", "text": response}],
                    )
                )
                self.assertEqual(len(frames), 1)
                self.assertTrue(frames[0]["verifier_label"])
                self.assertIn(
                    {
                        "kind": "action_status_projection",
                        "mapping": "zero_is_success",
                    },
                    frames[0]["atoms"],
                )
                self.assertFalse(
                    any(atom.get("kind") == "slot_equality" for atom in frames[0]["atoms"])
                )
                serialized = json.dumps({"frames": frames, "state": state})
                self.assertNotIn(f'"{response}"', serialized)
                self.assertNotIn("913457", serialized)
                self.assertIn(status_class, {"zero", "nonzero"})

    def test_project_status_wrong_outcome_is_negative(self) -> None:
        for observation, response in [('{"value":0}', "failure"), ('{"value":9}', "success")]:
            with self.subTest(observation=observation, response=response):
                frames, _ = self.harvest(
                    self.assistant_rows(
                        observation,
                        [{"type": "output_text", "text": response}],
                    )
                )
                self.assertEqual(len(frames), 1)
                self.assertFalse(frames[0]["verifier_label"])
                self.assertIn(
                    {"kind": "action_status_projection", "mapping": "zero_is_success"},
                    frames[0]["atoms"],
                )

    def test_project_status_multiple_integer_selectors_are_ambiguous(self) -> None:
        for observation in ['{"left":0,"right":1}', "code=0\ncode=1"]:
            with self.subTest(observation=observation):
                frames, _ = self.harvest(
                    self.assistant_rows(
                        observation,
                        [{"type": "output_text", "text": "success"}],
                    )
                )
                self.assertEqual(len(frames), 2)
                self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_project_status_rejects_types_bounds_and_noncanonical_response(self) -> None:
        cases = [
            ("true", "success"),
            ('"success"', "success"),
            ("completed successfully", "success"),
            ("-1", "failure"),
            (str(RELATIONS.MAX_PROJECT_STATUS_CODE + 1), "failure"),
            ("1.0", "failure"),
            ("0", "Success"),
            ("0", "success\n"),
        ]
        for observation, response in cases:
            with self.subTest(observation=observation, response=response):
                frames, _ = self.harvest(
                    self.assistant_rows(
                        observation,
                        [{"type": "output_text", "text": response}],
                    )
                )
                self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_project_status_requires_explicit_final_phase_and_fresh_output(self) -> None:
        for phase in (None, "analysis", "commentary"):
            with self.subTest(phase=phase):
                frames, _ = self.harvest(
                    self.assistant_rows(
                        "0",
                        [{"type": "output_text", "text": "success"}],
                        phase=phase,
                    )
                )
                self.assertEqual(frames, [])

        stale_rows = self.assistant_rows(
            "0", [{"type": "output_text", "text": "success"}]
        )
        stale_rows.insert(
            -1,
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "unknown-newer-output",
                    "output": "unrelated",
                },
            },
        )
        frames, _ = self.harvest(stale_rows)
        self.assertEqual(frames, [])

    def test_project_status_token_attribution_privacy_and_replay_are_stable(self) -> None:
        raw_code = 654321
        rows = self.assistant_rows(
            json.dumps({"opaque_value": raw_code}),
            [{"type": "output_text", "text": "failure"}],
        )
        rows.append(self.token_count_row(6_789))
        first_frames, first_state = self.harvest(rows)
        second_frames, second_state = self.harvest(rows)
        self.assertEqual(first_frames, second_frames)
        self.assertEqual(first_state, second_state)
        self.assertEqual(len(first_frames), 1)
        self.assertTrue(first_frames[0]["verifier_label"])
        self.assertEqual(first_frames[0]["estimated_input_tokens"], 6_789)
        serialized = json.dumps({"frames": first_frames, "state": first_state})
        self.assertNotIn(str(raw_code), serialized)
        self.assertNotIn('"failure"', serialized)

    def test_project_status_canonical_normalization_strips_unknown_keys(self) -> None:
        observation = {
            "value_sha256": RELATIONS.sha256_value(7),
            "value_type": "integer",
            "integer_status_class": "nonzero",
            "selector": {
                "kind": "json_field",
                "field": "opaque_value",
                "value_type": "integer",
                "raw_status": 7,
            },
            "tool_kind": "source_lookup",
            "call_shape": "function_call",
            "completion_state": "completed",
            "output_sha256": "0" * 64,
            "context_atoms": [],
            "raw_status": 7,
        }
        normalized = RELATIONS.normalize_file_state({"observations": [observation]})
        self.assertEqual(
            normalized["observations"][0],
            {
                "value_sha256": RELATIONS.sha256_value(7),
                "value_type": "integer",
                "selector": {
                    "kind": "json_field",
                    "field": "opaque_value",
                    "value_type": "integer",
                },
                "tool_kind": "source_lookup",
                "call_shape": "function_call",
                "completion_state": "completed",
                "output_sha256": "0" * 64,
                "context_atoms": [],
                "integer_status_class": "nonzero",
            },
        )
        self.assertNotIn("raw_status", json.dumps(normalized))

        frame = deepcopy(self.harvest(
            self.assistant_rows("0", [{"type": "output_text", "text": "success"}])
        )[0][0])
        atom = next(
            atom for atom in frame["atoms"] if atom["kind"] == "action_status_projection"
        )
        atom["raw_status"] = 0
        rebuilt = RELATIONS._normalize_stored_frame(frame)
        self.assertIsNotNone(rebuilt)
        assert rebuilt is not None
        self.assertIn(
            {"kind": "action_status_projection", "mapping": "zero_is_success"},
            rebuilt["atoms"],
        )
        self.assertNotIn("raw_status", json.dumps(rebuilt))

    def test_assistant_exact_plain_text_projection_is_positive(self) -> None:
        secret = "private selected value"
        frames, state = self.harvest(
            self.assistant_rows(json.dumps({"candidate": secret}), [
                {"type": "output_text", "text": secret}
            ], phase="final_answer")
        )
        self.assertEqual(len(frames), 1)
        self.assertTrue(frames[0]["verifier_label"])
        self.assertIn(
            {"kind": "action_value_projection", "format": "plain_text"},
            frames[0]["atoms"],
        )
        self.assertTrue(
            any(
                atom.get("kind") == "typed_slot"
                and atom.get("source") == "action"
                and atom.get("slot_id") == 2
                for atom in frames[0]["atoms"]
            )
        )
        self.assertIn(
            {"kind": "slot_equality", "left_slot": 1, "right_slot": 2},
            frames[0]["atoms"],
        )
        serialized = json.dumps({"frames": frames, "state": state})
        self.assertNotIn(secret, serialized)
        self.assertNotIn("private prompt", serialized)

    def test_assistant_scalar_template_becomes_typed_projection(self) -> None:
        frames, state = self.harvest(
            self.assistant_rows(
                '{"candidate":"selected"}',
                [{"type": "output_text", "text": "Result: selected."}],
            )
        )
        self.assertEqual(len(frames), 1)
        self.assertTrue(frames[0]["verifier_label"])
        self.assertIn(
            {
                "kind": "action_value_projection",
                "format": "plain_text",
                "renderer": {
                    "kind": "render_template",
                    "prefix": "Result: ",
                    "suffix": ".",
                },
            },
            frames[0]["atoms"],
        )
        self.assertNotIn("selected", json.dumps({"frames": frames, "state": state}))

    def test_assistant_multi_claim_response_becomes_typed_render_sequence(self) -> None:
        frames, state = self.harvest(
            self.assistant_rows(
                '{"count":3,"status":"passed"}',
                [{"type": "output_text", "text": "Count: 3; status: passed."}],
            )
        )
        positives = [frame for frame in frames if frame["verifier_label"]]
        self.assertEqual(len(positives), 1)
        projection = next(
            atom for atom in positives[0]["atoms"]
            if atom["kind"] == "action_value_projection"
        )
        self.assertEqual(projection["format"], "plain_text")
        renderer = projection["renderer"]
        self.assertEqual(renderer["kind"], "render_sequence")
        self.assertEqual(
            [segment["kind"] for segment in renderer["segments"]],
            ["static", "primary", "static", "selected", "static"],
        )
        serialized = json.dumps({"frames": frames, "state": state})
        self.assertNotIn('"passed"', serialized)
        self.assertNotIn('"count": 3', serialized)

    def test_assistant_multi_claim_rejects_ambiguous_dynamic_binding(self) -> None:
        frames, _ = self.harvest(
            self.assistant_rows(
                '{"left":"same","right":"same","count":2}',
                [{"type": "output_text", "text": "Values: same, same; count: 2."}],
            )
        )
        self.assertTrue(frames)
        self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_assistant_multi_claim_learns_source_neutral_turn_output_lines(self) -> None:
        frames, state = self.harvest(
            self.assistant_rows(
                "apt is blocked\nchrome hold",
                [{
                    "type": "output_text",
                    "text": "Result: apt is blocked; status: chrome hold.",
                }],
            )
        )
        positives = [frame for frame in frames if frame["verifier_label"]]
        self.assertEqual(len(positives), 1)
        projection = next(
            atom for atom in positives[0]["atoms"]
            if atom["kind"] == "action_value_projection"
        )
        selectors = [
            segment["selector"]
            for segment in projection["renderer"]["segments"]
            if segment["kind"] == "selected"
        ]
        primary = next(
            atom["selector"] for atom in positives[0]["atoms"]
            if atom["kind"] == "observation_selector"
        )
        self.assertEqual(primary["kind"], "turn_output_line")
        self.assertEqual(selectors[0]["kind"], "turn_output_line")
        serialized = json.dumps({"frames": frames, "state": state})
        self.assertNotIn("apt is blocked", serialized)
        self.assertNotIn("chrome hold", serialized)

    def test_assistant_scalar_template_rejects_unapproved_static_text(self) -> None:
        frames, _ = self.harvest(
            self.assistant_rows(
                '{"candidate":"selected"}',
                [{"type": "output_text", "text": "Customer Acme: selected."}],
            )
        )
        self.assertGreaterEqual(len(frames), 1)
        self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_assistant_non_final_phases_cannot_create_projection_or_causal_evidence(self) -> None:
        for phase in ("commentary", "analysis"):
            with self.subTest(phase=phase):
                frames, state = self.harvest(
                    self.assistant_rows(
                        '{"candidate":"selected"}',
                        [{"type": "output_text", "text": "selected"}],
                        phase=phase,
                    )
                )
                self.assertEqual(frames, [])
                self.assertEqual(state["pending_model_action"], [])
                self.assertEqual(state["observations"], [])
                self.assertEqual(state["ready_frames"], [])
                self.assertNotIn("action_value_projection", json.dumps(state))

    def test_assistant_missing_phase_cannot_create_projection_or_causal_evidence(self) -> None:
        frames, state = self.harvest(
            self.assistant_rows(
                '{"candidate":"selected"}',
                [{"type": "output_text", "text": "selected"}],
                phase=None,
            )
        )
        self.assertEqual(frames, [])
        self.assertEqual(state["pending_model_action"], [])
        self.assertEqual(state["observations"], [])
        self.assertEqual(state["ready_frames"], [])
        self.assertNotIn("action_value_projection", json.dumps(state))

    def test_assistant_exact_canonical_json_scalar_projection_is_positive(self) -> None:
        frames, _ = self.harvest(
            self.assistant_rows('{"candidate":42}', [
                {"type": "output_text", "text": "42"}
            ])
        )
        self.assertEqual(len(frames), 1)
        self.assertTrue(frames[0]["verifier_label"])
        self.assertIn(
            {"kind": "action_value_projection", "format": "canonical_json"},
            frames[0]["atoms"],
        )

    def test_assistant_hash_or_type_mismatch_is_negative(self) -> None:
        for observation, text in [('{"candidate":"left"}', "right"), ('{"candidate":"42"}', "42")]:
            with self.subTest(observation=observation, text=text):
                frames, _ = self.harvest(
                    self.assistant_rows(observation, [{"type": "output_text", "text": text}])
                )
                self.assertGreaterEqual(len(frames), 1)
                self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_assistant_ambiguous_prior_candidates_are_negative(self) -> None:
        frames, _ = self.harvest(
            self.assistant_rows('{"left":"same","right":"same"}', [
                {"type": "output_text", "text": "same"}
            ])
        )
        self.assertEqual(len(frames), 2)
        self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_assistant_multiline_non_scalar_and_ambiguous_output_are_negative(self) -> None:
        contents = [
            [{"type": "output_text", "text": "selected\nvalue"}],
            [{"type": "output_text", "text": '{"nested":"selected"}'}],
            [
                {"type": "output_text", "text": "selected"},
                {"type": "output_text", "text": "selected"},
            ],
        ]
        for content in contents:
            with self.subTest(content=content):
                frames, _ = self.harvest(
                    self.assistant_rows('{"candidate":"selected"}', content)
                )
                self.assertGreaterEqual(len(frames), 1)
                self.assertTrue(all(not frame["verifier_label"] for frame in frames))

    def test_assistant_output_is_bounded_private_and_replay_stable(self) -> None:
        secret = "x" * (RELATIONS.MAX_OUTPUT_BYTES + 1)
        rows = self.assistant_rows('{"candidate":"short"}', [
            {"type": "output_text", "text": secret}
        ])
        first_frames, first_state = self.harvest(rows)
        second_frames, second_state = self.harvest(rows)
        self.assertEqual(first_frames, second_frames)
        self.assertEqual(first_state, second_state)
        self.assertTrue(all(not frame["verifier_label"] for frame in first_frames))
        serialized = json.dumps({"frames": first_frames, "state": first_state})
        self.assertNotIn(secret, serialized)

    def test_action_tool_output_then_token_attributes_preceding_action(self) -> None:
        rows = self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
        rows.extend([
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output", "call_id": "target-call", "output": "done"
                },
            },
            self.token_count_row(2468),
        ])
        frames, _ = self.harvest(rows)
        self.assertEqual([frame["estimated_input_tokens"] for frame in frames], [2468])

    def test_second_token_count_cannot_reuse_consumed_batch(self) -> None:
        rows = self.base_rows('{"result":"first"}', "route_result", "payload", "first")
        rows.extend([self.token_count_row(101), self.token_count_row(202)])
        frames, state = self.harvest(rows)
        self.assertEqual([frame["estimated_input_tokens"] for frame in frames], [101])
        self.assertEqual(state["pending_model_action"], [])

    def test_missing_token_flushes_zero_before_next_action_without_shift(self) -> None:
        rows = self.base_rows('{"result":"first"}', "route_result", "payload", "first")
        rows.extend([
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output", "call_id": "target-call",
                    "output": '{"result":"second"}',
                },
            },
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call", "name": "route_result", "call_id": "next",
                    "arguments": json.dumps({"payload": "second"}),
                },
            },
            self.token_count_row(303),
        ])
        frames, _ = self.harvest(rows)
        self.assertEqual([frame["estimated_input_tokens"] for frame in frames], [0, 303])

    def test_incremental_eof_retains_then_completes_pending_batch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "incremental.jsonl"
            rows = self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = RELATIONS.empty_file_state()
            self.assertEqual(RELATIONS.harvest_relation_frames(path, state, 64), [])
            self.assertEqual(len(state["pending_model_action"]), 1)
            serialized = json.dumps(state)
            self.assertNotIn("selected", serialized)
            with path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(self.token_count_row(404)) + "\n")
            frames = RELATIONS.harvest_relation_frames(path, state, 64)
            self.assertEqual([frame["estimated_input_tokens"] for frame in frames], [404])
            self.assertEqual(state["pending_model_action"], [])

    def test_ready_queue_preserves_batch_across_max_events(self) -> None:
        rows = self.assistant_rows(
            '{"left":"same","right":"same"}',
            [{"type": "output_text", "text": "different"}],
        )
        rows.append(self.token_count_row(505))
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "ready.jsonl"
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            state = RELATIONS.empty_file_state()
            first = RELATIONS.harvest_relation_frames(path, state, 1)
            second = RELATIONS.harvest_relation_frames(path, state, 1)
            self.assertEqual(len(first), 1)
            self.assertEqual(len(second), 1)
            self.assertNotEqual(first[0]["frame_id_sha256"], second[0]["frame_id_sha256"])
            self.assertEqual(state["ready_frames"], [])

    def test_legacy_and_malformed_frame_state_normalizes_fail_closed(self) -> None:
        normalized = RELATIONS.normalize_file_state({
            "offset": 7,
            "turn": {"last_input_tokens": 88},
            "pending_model_action": [{"atoms": [{"raw": "private"}]}],
            "ready_frames": ["private"],
        })
        self.assertEqual(normalized["offset"], 7)
        self.assertEqual(normalized["pending_model_action"], [])
        self.assertEqual(normalized["ready_frames"], [])
        self.assertNotIn("private", json.dumps(normalized))

    def test_sensitive_selectors_never_enter_frames_or_state(self) -> None:
        selectors = [
            "https://host.invalid/x", "owner@example.invalid", "/home/user/key",
            "550e8400-e29b-41d4-a716-446655440000", "ABCDEF0123456789ABCDEF0123456789",
            "field123456", "password", "api_token", "white space",
            "control\u0001name",
        ]
        for selector in selectors:
            with self.subTest(selector=selector):
                output = json.dumps({selector: "selected"})
                frames, state = self.harvest(
                    self.base_rows(output, "route_result", "payload", "selected")
                )
                serialized = json.dumps({"frames": frames, "state": state})
                self.assertEqual(frames, [])
                persisted_selectors = [
                    atom.get("selector")
                    for frame in frames + state["pending_model_action"] + state["ready_frames"]
                    for atom in frame.get("atoms", [])
                    if atom.get("kind") == "observation_selector"
                ]
                self.assertNotIn(selector, json.dumps(persisted_selectors))

    def test_safe_structural_selector_still_works(self) -> None:
        frames, state = self.harvest(
            self.base_rows('{"result_code":"selected","status_flag":true}',
                           "route_result", "payload", "selected")
        )
        self.assertEqual(len(frames), 1)
        self.assertIn("result_code", json.dumps(frames))
        self.assertNotIn("selected", json.dumps({"frames": frames, "state": state}))

    def test_normalize_file_state_reconstructs_nested_relation_data(self) -> None:
        secret = "raw_value-secret"
        observation = {
            "value_sha256": RELATIONS.sha256_value("selected"),
            "value_type": "string",
            "selector": {
                "kind": "json_field",
                "field": "result_code",
                "value_type": "string",
                "raw_value": secret,
            },
            "tool_kind": "source_lookup",
            "call_shape": "function_call",
            "completion_state": "completed",
            "output_sha256": "0" * 64,
            "context_atoms": [
                {
                    "kind": "cardinality",
                    "role": "turn_call_count_band",
                    "count": 1,
                    "raw_value": secret,
                }
            ],
            "raw_value": secret,
        }
        frame = deepcopy(self.harvest(
            self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
        )[0][0])
        selector_atom = next(
            atom for atom in frame["atoms"] if atom["kind"] == "observation_selector"
        )
        selector_atom["selector"]["raw_value"] = secret
        frame["atoms"][0]["raw_value"] = secret
        frame["raw_value"] = secret

        normalized = RELATIONS.normalize_file_state({
            "observations": [observation],
            "pending_model_action": [frame],
            "ready_frames": [frame],
            "raw_value": secret,
        })
        serialized = json.dumps(normalized)
        self.assertNotIn(secret, serialized)
        self.assertNotIn("raw_value", serialized)
        self.assertEqual(
            set(normalized["observations"][0]),
            {
                "value_sha256", "value_type", "selector", "tool_kind",
                "call_shape", "completion_state", "output_sha256", "context_atoms",
            },
        )
        self.assertEqual(
            normalized["observations"][0]["selector"],
            {"kind": "json_field", "field": "result_code", "value_type": "string"},
        )
        self.assertEqual(
            normalized["observations"][0]["context_atoms"],
            [{"kind": "cardinality", "role": "turn_call_count_band", "count": 1}],
        )
        expected_frame_keys = {
            "schema", "frame_id_sha256", "event_id_sha256", "client_intent_id_sha256",
            "session_id_sha256", "observed_at_unix_nanos", "extractor_version",
            "estimated_input_tokens", "verifier_label", "atoms", "evidence_ref_sha256",
        }
        self.assertEqual(set(normalized["pending_model_action"][0]), expected_frame_keys)
        self.assertEqual(set(normalized["ready_frames"][0]), expected_frame_keys)

    def test_stored_frame_rejects_unknown_atom_kind_after_canonical_rebuild(self) -> None:
        secret = "raw_value-secret"
        frame = deepcopy(self.harvest(
            self.base_rows('{"result":"selected"}', "route_result", "payload", "selected")
        )[0][0])
        frame["atoms"][0]["raw_value"] = secret
        frame["raw_value"] = secret
        normalized = RELATIONS._normalize_stored_frame(frame)
        self.assertIsNotNone(normalized)
        assert normalized is not None
        self.assertNotIn(secret, json.dumps(normalized))
        self.assertNotIn("raw_value", json.dumps(normalized))
        self.assertEqual(
            normalized["atoms"][0],
            {"kind": "tool_kind", "value": "source_lookup"},
        )

        frame["atoms"].append({"kind": "future_atom", "raw_value": secret})
        self.assertIsNone(RELATIONS._normalize_stored_frame(frame))


if __name__ == "__main__":
    unittest.main()
