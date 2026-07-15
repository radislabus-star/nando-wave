use std::collections::BTreeMap;
use std::error::Error;

use serde_json::{Map, Value, json};

use super::*;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn set_field_executes_and_verifies() -> TestResult {
    let before = canonical_state(json!([
        {"id": "a", "status": "open", "count": 2},
        {"id": "b", "status": "open", "count": 4}
    ]))?;
    let action = canonical_action(json!({
        "kind": "set_status",
        "target": "b",
        "new_status": "closed"
    }))?;
    let result = execute_canonical(&set_program(), &before, &action);
    assert_eq!(result.status, ExecutionStatus::Executed);
    assert_eq!(
        result
            .after_records
            .as_ref()
            .and_then(|rows| rows[1].get("status")),
        Some(&json!("closed"))
    );
    assert_eq!(result.proof.get("postcondition"), Some(&json!("verified")));
    Ok(())
}

#[test]
fn increment_append_and_delete_execute() -> TestResult {
    let before = canonical_state(json!([
        {"id": "a", "status": "open", "count": 2},
        {"id": "b", "status": "open", "count": 4}
    ]))?;

    let increment = TransitionProgram::new(
        "increment_count",
        TransitionOperation::IncrementField {
            match_field: "id".to_owned(),
            target_slot: "target".to_owned(),
            target_field: "count".to_owned(),
            amount_slot: "amount".to_owned(),
        },
    )
    .with_slot_type("amount", ValueKind::Number);
    let increment_action = canonical_action(json!({
        "kind": "increment_count",
        "target": "a",
        "amount": 3
    }))?;
    let incremented = execute_canonical(&increment, &before, &increment_action);
    assert_eq!(incremented.status, ExecutionStatus::Executed);
    assert_eq!(
        incremented
            .after_records
            .as_ref()
            .and_then(|rows| rows[0].get("count")),
        Some(&json!(5))
    );

    let append = TransitionProgram::new(
        "append_item",
        TransitionOperation::AppendRecord {
            record_bindings: BTreeMap::from([
                ("id".to_owned(), "record.id".to_owned()),
                ("status".to_owned(), "record.status".to_owned()),
                ("count".to_owned(), "record.count".to_owned()),
            ]),
        },
    );
    let append_action = canonical_action(json!({
        "kind": "append_item",
        "record": {"id": "c", "status": "new", "count": 1}
    }))?;
    let appended = execute_canonical(&append, &before, &append_action);
    assert_eq!(appended.status, ExecutionStatus::Executed);
    assert_eq!(appended.after_records.as_ref().map(Vec::len), Some(3));

    let delete = TransitionProgram::new(
        "delete_item",
        TransitionOperation::DeleteRecord {
            match_field: "id".to_owned(),
            target_slot: "target".to_owned(),
        },
    );
    let delete_action = canonical_action(json!({"kind": "delete_item", "target": "a"}))?;
    let deleted = execute_canonical(&delete, &before, &delete_action);
    assert_eq!(deleted.status, ExecutionStatus::Executed);
    assert_eq!(deleted.after_records.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        deleted
            .after_records
            .as_ref()
            .and_then(|rows| rows[0].get("id")),
        Some(&json!("b"))
    );
    Ok(())
}

#[test]
fn hard_negatives_abstain() -> TestResult {
    let before = canonical_state(json!([
        {"id": "a", "status": "open", "count": 2},
        {"id": "b", "status": "open", "count": 4}
    ]))?;
    let cases = [
        json!({"kind": "wrong", "target": "a", "new_status": "closed"}),
        json!({"kind": "set_status", "target": "missing", "new_status": "closed"}),
        json!({"kind": "set_status", "target": "a", "new_status": "open"}),
        json!({"kind": "set_status", "target": "a", "new_status": 7}),
    ];
    for case in cases {
        let action = canonical_action(case)?;
        assert_eq!(
            execute_canonical(&set_program(), &before, &action).status,
            ExecutionStatus::Abstain
        );
    }

    let ambiguous = canonical_state(json!([
        {"id": "a", "status": "open"},
        {"id": "a", "status": "closed"}
    ]))?;
    let action = canonical_action(json!({
        "kind": "set_status",
        "target": "a",
        "new_status": "held"
    }))?;
    assert_eq!(
        execute_canonical(&set_program(), &ambiguous, &action).reason,
        "target_ambiguous"
    );
    Ok(())
}

#[test]
fn verifier_rejects_tampered_transition() -> TestResult {
    let before = canonical_state(json!([{"id": "a", "status": "open", "count": 2}]))?;
    let action = canonical_action(json!({
        "kind": "set_status",
        "target": "a",
        "new_status": "closed"
    }))?;
    let tampered = canonical_state(json!([{"id": "a", "status": "closed", "count": 99}]))?;
    assert_eq!(
        verify_transition(&set_program(), &before, &action, &tampered),
        Err(VerificationError("postcondition_state_mismatch"))
    );
    Ok(())
}

#[test]
fn adapters_commute_and_preserve_surface_frames() -> TestResult {
    let before = canonical_state(json!([
        {"id": "a", "status": "open", "count": 2},
        {"id": "b", "status": "open", "count": 4}
    ]))?;
    let action = canonical_action(json!({
        "kind": "set_status",
        "target": "b",
        "new_status": "closed"
    }))?;
    let canonical_result = execute_canonical(&set_program(), &before, &action);
    let canonical_after = canonical_result
        .after_records
        .as_ref()
        .ok_or("canonical actor did not execute")?;

    for layout in [Layout::Map, Layout::List, Layout::Columns] {
        let adapter = adapter(layout);
        let mut concrete_before = adapter.encode_state(&before)?;
        add_surface_frame(&mut concrete_before, layout)?;
        let concrete_action = adapter.encode_action(&action)?;
        assert_eq!(adapter.adapt_action(&concrete_action)?, action);

        let result = execute_surface(&set_program(), &adapter, &concrete_before, &concrete_action);
        assert_eq!(result.status, ExecutionStatus::Executed);
        assert_eq!(result.after_records.as_ref(), Some(canonical_after));
        let concrete_after = result
            .concrete_after
            .as_ref()
            .ok_or("surface projection missing")?;
        assert_surface_frame(concrete_after, layout)?;
        assert_eq!(
            adapter.adapt_state(concrete_after)?.records,
            *canonical_after
        );
    }
    Ok(())
}

#[test]
fn transition_programs_are_compact_and_roundtrip() -> TestResult {
    let programs = [
        set_program(),
        TransitionProgram::new(
            "increment_count",
            TransitionOperation::IncrementField {
                match_field: "id".to_owned(),
                target_slot: "target".to_owned(),
                target_field: "count".to_owned(),
                amount_slot: "amount".to_owned(),
            },
        ),
        TransitionProgram::new(
            "append_item",
            TransitionOperation::AppendRecord {
                record_bindings: BTreeMap::from([
                    ("id".to_owned(), "record.id".to_owned()),
                    ("status".to_owned(), "record.status".to_owned()),
                ]),
            },
        ),
        TransitionProgram::new(
            "delete_item",
            TransitionOperation::DeleteRecord {
                match_field: "id".to_owned(),
                target_slot: "target".to_owned(),
            },
        ),
    ];
    for program in programs {
        let bytes = program.artifact_bytes()?;
        assert!(
            bytes.len() <= 512,
            "typed actor artifact exceeded 512 bytes"
        );
        let decoded: TransitionProgram = serde_json::from_slice(&bytes)?;
        assert_eq!(decoded, program);
    }
    Ok(())
}

fn set_program() -> TransitionProgram {
    TransitionProgram::new(
        "set_status",
        TransitionOperation::SetField {
            match_field: "id".to_owned(),
            target_slot: "target".to_owned(),
            target_field: "status".to_owned(),
            value_slot: "new_status".to_owned(),
        },
    )
    .with_slot_type("target", ValueKind::String)
    .with_slot_type("new_status", ValueKind::String)
}

fn adapter(layout: Layout) -> SurfaceAdapter {
    let field_map = match layout {
        Layout::Map => BTreeMap::from([
            ("id".to_owned(), "$key".to_owned()),
            ("status".to_owned(), "state".to_owned()),
            ("count".to_owned(), "qty".to_owned()),
        ]),
        Layout::List => BTreeMap::from([
            ("id".to_owned(), "uuid".to_owned()),
            ("status".to_owned(), "state".to_owned()),
            ("count".to_owned(), "qty".to_owned()),
        ]),
        Layout::Columns => BTreeMap::from([
            ("id".to_owned(), "keys".to_owned()),
            ("status".to_owned(), "states".to_owned()),
            ("count".to_owned(), "counts".to_owned()),
        ]),
    };
    SurfaceAdapter {
        name: format!("{layout:?}_adapter"),
        layout,
        root_path: vec!["data".to_owned()],
        field_map,
        action_kind_path: vec!["action".to_owned(), "type".to_owned()],
        action_rules: vec![ActionRule {
            concrete_kind: "change".to_owned(),
            canonical_kind: "set_status".to_owned(),
            slot_paths: BTreeMap::from([
                (
                    "target".to_owned(),
                    vec!["action".to_owned(), "target".to_owned()],
                ),
                (
                    "new_status".to_owned(),
                    vec!["action".to_owned(), "value".to_owned()],
                ),
            ]),
            slot_constants: BTreeMap::new(),
            record_paths: BTreeMap::new(),
        }],
    }
}

fn add_surface_frame(state: &mut Value, layout: Layout) -> TestResult {
    let root = state
        .as_object_mut()
        .ok_or("surface root is not an object")?;
    root.insert("meta".to_owned(), json!({"trace": "keep"}));
    let data = root.get_mut("data").ok_or("surface data missing")?;
    match layout {
        Layout::Map => {
            for row in data
                .as_object_mut()
                .ok_or("map data is not an object")?
                .values_mut()
            {
                row.as_object_mut()
                    .ok_or("map row is not an object")?
                    .insert("untouched".to_owned(), json!(true));
            }
        }
        Layout::List => {
            for row in data.as_array_mut().ok_or("list data is not an array")? {
                row.as_object_mut()
                    .ok_or("list row is not an object")?
                    .insert("untouched".to_owned(), json!(true));
            }
        }
        Layout::Columns => {
            let columns = data
                .as_object_mut()
                .ok_or("columns data is not an object")?;
            columns.insert("notes".to_owned(), json!(["left", "right"]));
            columns.insert("timezone".to_owned(), json!("UTC"));
        }
    }
    Ok(())
}

fn assert_surface_frame(state: &Value, layout: Layout) -> TestResult {
    assert_eq!(state.pointer("/meta/trace"), Some(&json!("keep")));
    match layout {
        Layout::Map => {
            assert_eq!(state.pointer("/data/a/untouched"), Some(&json!(true)));
            assert_eq!(state.pointer("/data/b/untouched"), Some(&json!(true)));
        }
        Layout::List => {
            assert_eq!(state.pointer("/data/0/untouched"), Some(&json!(true)));
            assert_eq!(state.pointer("/data/1/untouched"), Some(&json!(true)));
        }
        Layout::Columns => {
            assert_eq!(
                state.pointer("/data/notes"),
                Some(&json!(["left", "right"]))
            );
            assert_eq!(state.pointer("/data/timezone"), Some(&json!("UTC")));
        }
    }
    Ok(())
}

fn canonical_state(value: Value) -> Result<CanonicalState, serde_json::Error> {
    serde_json::from_value(value)
}

fn canonical_action(value: Value) -> Result<Map<String, Value>, serde_json::Error> {
    serde_json::from_value(value)
}
