use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TransitionOperation {
    SetField {
        match_field: String,
        target_slot: String,
        target_field: String,
        value_slot: String,
    },
    IncrementField {
        match_field: String,
        target_slot: String,
        target_field: String,
        amount_slot: String,
    },
    AppendRecord {
        record_bindings: BTreeMap<String, String>,
    },
    DeleteRecord {
        match_field: String,
        target_slot: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransitionProgram {
    pub schema: String,
    pub action_kind: String,
    pub operation: TransitionOperation,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub slot_types: BTreeMap<String, ValueKind>,
}

impl TransitionProgram {
    #[must_use]
    pub fn new(action_kind: impl Into<String>, operation: TransitionOperation) -> Self {
        Self {
            schema: "nando.transition-program.v1".to_owned(),
            action_kind: action_kind.into(),
            operation,
            slot_types: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_slot_type(mut self, slot: impl Into<String>, kind: ValueKind) -> Self {
        self.slot_types.insert(slot.into(), kind);
        self
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != "nando.transition-program.v1" {
            return Err("unsupported_program_schema");
        }
        if self.action_kind.is_empty() {
            return Err("empty_action_kind");
        }
        match &self.operation {
            TransitionOperation::SetField {
                match_field,
                target_slot,
                target_field,
                value_slot,
            } => validate_nonempty([match_field, target_slot, target_field, value_slot]),
            TransitionOperation::IncrementField {
                match_field,
                target_slot,
                target_field,
                amount_slot,
            } => validate_nonempty([match_field, target_slot, target_field, amount_slot]),
            TransitionOperation::AppendRecord { record_bindings } => {
                if record_bindings.is_empty() {
                    return Err("empty_record_bindings");
                }
                if !record_bindings.contains_key("id") {
                    return Err("record_id_binding_missing");
                }
                validate_nonempty(record_bindings.keys().chain(record_bindings.values()))
            }
            TransitionOperation::DeleteRecord {
                match_field,
                target_slot,
            } => validate_nonempty([match_field, target_slot]),
        }
    }

    pub fn artifact_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn description_bytes(&self) -> Result<usize, serde_json::Error> {
        self.artifact_bytes().map(|bytes| bytes.len())
    }
}

fn validate_nonempty<'a, I>(values: I) -> Result<(), &'static str>
where
    I: IntoIterator<Item = &'a String>,
{
    if values.into_iter().any(String::is_empty) {
        Err("empty_program_field")
    } else {
        Ok(())
    }
}
