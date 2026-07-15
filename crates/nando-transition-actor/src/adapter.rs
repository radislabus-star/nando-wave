use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::runtime::{CanonicalRecord, CanonicalState, get_slot};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Map,
    List,
    Columns,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ActionRule {
    pub concrete_kind: String,
    pub canonical_kind: String,
    #[serde(default)]
    pub slot_paths: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub slot_constants: BTreeMap<String, Value>,
    #[serde(default)]
    pub record_paths: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SurfaceAdapter {
    pub name: String,
    pub layout: Layout,
    pub root_path: Vec<String>,
    pub field_map: BTreeMap<String, String>,
    pub action_kind_path: Vec<String>,
    pub action_rules: Vec<ActionRule>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptedState {
    pub records: CanonicalState,
    pub original: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterError(pub String);

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AdapterError {}

impl SurfaceAdapter {
    pub fn validate(&self) -> Result<(), AdapterError> {
        if self.name.is_empty() {
            return Err(AdapterError("empty_adapter_name".to_owned()));
        }
        if self.root_path.is_empty() {
            return Err(AdapterError("empty_state_root_path".to_owned()));
        }
        if self.action_kind_path.is_empty() {
            return Err(AdapterError("empty_action_kind_path".to_owned()));
        }
        let Some(id_source) = self.field_map.get("id") else {
            return Err(AdapterError("canonical_id_mapping_missing".to_owned()));
        };
        if self.layout == Layout::Map && id_source != "$key" {
            return Err(AdapterError("map_id_must_use_key".to_owned()));
        }
        if self.action_rules.is_empty() {
            return Err(AdapterError("action_rules_missing".to_owned()));
        }
        Ok(())
    }

    pub fn adapt_state(&self, state: &Value) -> Result<AdaptedState, AdapterError> {
        self.validate()?;
        let root = get_path(state, &self.root_path)?;
        let records = match self.layout {
            Layout::Map => self.adapt_map(root)?,
            Layout::List => self.adapt_list(root)?,
            Layout::Columns => self.adapt_columns(root)?,
        };
        Ok(AdaptedState {
            records,
            original: state.clone(),
        })
    }

    pub fn adapt_action(&self, action: &Value) -> Result<Map<String, Value>, AdapterError> {
        self.validate()?;
        let concrete_kind = get_path(action, &self.action_kind_path)?
            .as_str()
            .ok_or_else(|| AdapterError("action_kind_must_be_string".to_owned()))?;
        let rule = self
            .action_rules
            .iter()
            .find(|rule| rule.concrete_kind == concrete_kind)
            .ok_or_else(|| AdapterError(format!("unknown_action_kind:{concrete_kind}")))?;

        let mut canonical = Map::new();
        canonical.insert(
            "kind".to_owned(),
            Value::String(rule.canonical_kind.clone()),
        );
        for (slot, path) in &rule.slot_paths {
            canonical.insert(slot.clone(), get_path(action, path)?.clone());
        }
        for (slot, value) in &rule.slot_constants {
            canonical.insert(slot.clone(), value.clone());
        }
        if !rule.record_paths.is_empty() {
            let mut record = Map::new();
            for (role, path) in &rule.record_paths {
                record.insert(role.clone(), get_path(action, path)?.clone());
            }
            canonical.insert("record".to_owned(), Value::Object(record));
        }
        Ok(canonical)
    }

    pub fn project(
        &self,
        records: &CanonicalState,
        context: &AdaptedState,
    ) -> Result<Value, AdapterError> {
        let original_root = get_path(&context.original, &self.root_path)?;
        let projected = match self.layout {
            Layout::Map => self.project_map(records, original_root)?,
            Layout::List => self.project_list(records, original_root)?,
            Layout::Columns => self.project_columns(records, original_root)?,
        };
        let mut output = context.original.clone();
        set_path(&mut output, &self.root_path, projected)?;
        Ok(output)
    }

    pub fn encode_state(&self, records: &CanonicalState) -> Result<Value, AdapterError> {
        self.validate()?;
        let empty_root = match self.layout {
            Layout::Map | Layout::Columns => Value::Object(Map::new()),
            Layout::List => Value::Array(Vec::new()),
        };
        let projected = match self.layout {
            Layout::Map => self.project_map(records, &empty_root)?,
            Layout::List => self.project_list(records, &empty_root)?,
            Layout::Columns => self.project_columns(records, &empty_root)?,
        };
        let mut output = Value::Object(Map::new());
        set_path(&mut output, &self.root_path, projected)?;
        Ok(output)
    }

    pub fn encode_action(&self, action: &Map<String, Value>) -> Result<Value, AdapterError> {
        self.validate()?;
        let canonical_kind = action
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| AdapterError("canonical_action_kind_missing".to_owned()))?;
        let rule = self
            .action_rules
            .iter()
            .find(|rule| rule.canonical_kind == canonical_kind)
            .ok_or_else(|| {
                AdapterError(format!("canonical_action_rule_missing:{canonical_kind}"))
            })?;
        let mut output = Value::Object(Map::new());
        set_path(
            &mut output,
            &self.action_kind_path,
            Value::String(rule.concrete_kind.clone()),
        )?;
        for (slot, path) in &rule.slot_paths {
            let value = get_slot(action, slot)
                .ok_or_else(|| AdapterError(format!("canonical_slot_missing:{slot}")))?;
            set_path(&mut output, path, value.clone())?;
        }
        for (role, path) in &rule.record_paths {
            let slot = format!("record.{role}");
            let value = get_slot(action, &slot)
                .ok_or_else(|| AdapterError(format!("canonical_slot_missing:{slot}")))?;
            set_path(&mut output, path, value.clone())?;
        }
        Ok(output)
    }

    fn adapt_map(&self, root: &Value) -> Result<CanonicalState, AdapterError> {
        let rows = root
            .as_object()
            .ok_or_else(|| AdapterError("map_root_must_be_object".to_owned()))?;
        let mut records = Vec::with_capacity(rows.len());
        for (key, raw_value) in rows {
            let raw = raw_value
                .as_object()
                .ok_or_else(|| AdapterError("map_row_must_be_object".to_owned()))?;
            let mut record = CanonicalRecord::new();
            for (role, concrete) in &self.field_map {
                if concrete == "$key" {
                    record.insert(role.clone(), Value::String(key.clone()));
                } else if let Some(value) = raw.get(concrete) {
                    record.insert(role.clone(), value.clone());
                }
            }
            records.push(record);
        }
        Ok(records)
    }

    fn adapt_list(&self, root: &Value) -> Result<CanonicalState, AdapterError> {
        let rows = root
            .as_array()
            .ok_or_else(|| AdapterError("list_root_must_be_array".to_owned()))?;
        rows.iter()
            .map(|raw_value| {
                let raw = raw_value
                    .as_object()
                    .ok_or_else(|| AdapterError("list_row_must_be_object".to_owned()))?;
                let mut record = CanonicalRecord::new();
                for (role, concrete) in &self.field_map {
                    if let Some(value) = raw.get(concrete) {
                        record.insert(role.clone(), value.clone());
                    }
                }
                Ok(record)
            })
            .collect()
    }

    fn adapt_columns(&self, root: &Value) -> Result<CanonicalState, AdapterError> {
        let columns = root
            .as_object()
            .ok_or_else(|| AdapterError("columns_root_must_be_object".to_owned()))?;
        let id_column = self
            .field_map
            .get("id")
            .ok_or_else(|| AdapterError("canonical_id_mapping_missing".to_owned()))?;
        let ids = columns
            .get(id_column)
            .and_then(Value::as_array)
            .ok_or_else(|| AdapterError("columns_id_must_be_array".to_owned()))?;
        let mut records = Vec::with_capacity(ids.len());
        for index in 0..ids.len() {
            let mut record = CanonicalRecord::new();
            for (role, concrete) in &self.field_map {
                let column = columns
                    .get(concrete)
                    .and_then(Value::as_array)
                    .ok_or_else(|| AdapterError(format!("column_must_be_array:{concrete}")))?;
                if column.len() != ids.len() {
                    return Err(AdapterError(format!("ragged_column:{concrete}")));
                }
                record.insert(role.clone(), column[index].clone());
            }
            records.push(record);
        }
        Ok(records)
    }

    fn project_map(
        &self,
        records: &CanonicalState,
        original_root: &Value,
    ) -> Result<Value, AdapterError> {
        let original = original_root.as_object();
        let mut result = Map::new();
        for record in records {
            let id = record
                .get("id")
                .ok_or_else(|| AdapterError("canonical_record_id_missing".to_owned()))?;
            let key = scalar_key(id)?;
            let mut raw = original
                .and_then(|rows| rows.get(&key))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for (role, concrete) in &self.field_map {
                if concrete != "$key"
                    && let Some(value) = record.get(role)
                {
                    raw.insert(concrete.clone(), value.clone());
                }
            }
            result.insert(key, Value::Object(raw));
        }
        Ok(Value::Object(result))
    }

    fn project_list(
        &self,
        records: &CanonicalState,
        original_root: &Value,
    ) -> Result<Value, AdapterError> {
        let original_rows = original_root.as_array();
        let id_field = self
            .field_map
            .get("id")
            .ok_or_else(|| AdapterError("canonical_id_mapping_missing".to_owned()))?;
        let mut result = Vec::with_capacity(records.len());
        for record in records {
            let id = record
                .get("id")
                .ok_or_else(|| AdapterError("canonical_record_id_missing".to_owned()))?;
            let mut raw = original_rows
                .and_then(|rows| {
                    rows.iter().find(|row| {
                        row.as_object().and_then(|object| object.get(id_field)) == Some(id)
                    })
                })
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for (role, concrete) in &self.field_map {
                if let Some(value) = record.get(role) {
                    raw.insert(concrete.clone(), value.clone());
                }
            }
            result.push(Value::Object(raw));
        }
        Ok(Value::Array(result))
    }

    fn project_columns(
        &self,
        records: &CanonicalState,
        original_root: &Value,
    ) -> Result<Value, AdapterError> {
        let original = original_root.as_object();
        let id_column = self
            .field_map
            .get("id")
            .ok_or_else(|| AdapterError("canonical_id_mapping_missing".to_owned()))?;
        let original_ids = original
            .and_then(|columns| columns.get(id_column))
            .and_then(Value::as_array);
        let mut result = Map::new();

        if let Some(columns) = original {
            for (column_name, old_value) in columns {
                let Some(old_values) = old_value.as_array() else {
                    result.insert(column_name.clone(), old_value.clone());
                    continue;
                };
                let values = records
                    .iter()
                    .map(|record| {
                        let index = record.get("id").and_then(|id| {
                            original_ids.and_then(|ids| ids.iter().position(|value| value == id))
                        });
                        index
                            .and_then(|index| old_values.get(index))
                            .cloned()
                            .unwrap_or(Value::Null)
                    })
                    .collect();
                result.insert(column_name.clone(), Value::Array(values));
            }
        }

        for (role, concrete) in &self.field_map {
            let values = records
                .iter()
                .map(|record| record.get(role).cloned().unwrap_or(Value::Null))
                .collect();
            result.insert(concrete.clone(), Value::Array(values));
        }
        Ok(Value::Object(result))
    }
}

fn get_path<'a>(value: &'a Value, path: &[String]) -> Result<&'a Value, AdapterError> {
    let mut current = value;
    for part in path {
        current = current
            .as_object()
            .and_then(|object| object.get(part))
            .ok_or_else(|| AdapterError(format!("path_missing:{}", path.join("."))))?;
    }
    Ok(current)
}

fn set_path(value: &mut Value, path: &[String], item: Value) -> Result<(), AdapterError> {
    let Some((last, parents)) = path.split_last() else {
        return Err(AdapterError("empty_path".to_owned()));
    };
    let mut current = value;
    for part in parents {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let object = current
            .as_object_mut()
            .ok_or_else(|| AdapterError("path_parent_not_object".to_owned()))?;
        current = object
            .entry(part.clone())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    current
        .as_object_mut()
        .ok_or_else(|| AdapterError("path_parent_not_object".to_owned()))?
        .insert(last.clone(), item);
    Ok(())
}

fn scalar_key(value: &Value) -> Result<String, AdapterError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => {
            Err(AdapterError("map_id_must_be_scalar".to_owned()))
        }
    }
}
