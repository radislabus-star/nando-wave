use std::collections::{BTreeMap, BTreeSet};

use nando_transition_actor::Layout;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransitionTrace {
    pub before: Value,
    pub action: Value,
    pub after: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutShape {
    Map,
    List,
    Columns,
}

impl From<LayoutShape> for Layout {
    fn from(value: LayoutShape) -> Self {
        match value {
            LayoutShape::Map => Self::Map,
            LayoutShape::List => Self::List,
            LayoutShape::Columns => Self::Columns,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SurfaceShape {
    pub layout: LayoutShape,
    pub root_path: Vec<String>,
    pub record_fields: Vec<String>,
    pub id_sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RawRecord {
    pub key: Option<String>,
    pub fields: Map<String, Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ScalarPath {
    pub path: Vec<String>,
}

pub(crate) fn discover_surface(state: &Value) -> Result<SurfaceShape, &'static str> {
    let mut candidates = Vec::new();
    collect_collections(state, &mut Vec::new(), &mut candidates);
    candidates.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| left.0.root_path.cmp(&right.0.root_path))
    });
    let Some((shape, _, _)) = candidates.into_iter().next() else {
        return Err("state_collection_not_found");
    };
    Ok(shape)
}

pub(crate) fn records_for(
    state: &Value,
    shape: &SurfaceShape,
) -> Result<Vec<RawRecord>, &'static str> {
    let root = get_path(state, &shape.root_path).ok_or("state_root_missing")?;
    match shape.layout {
        LayoutShape::Map => records_from_map(root),
        LayoutShape::List => records_from_list(root),
        LayoutShape::Columns => records_from_columns(root),
    }
}

pub(crate) fn scalar_paths(value: &Value) -> Vec<ScalarPath> {
    let mut out = Vec::new();
    collect_scalar_paths(value, &mut Vec::new(), &mut out);
    out.sort();
    out
}

pub(crate) fn value_at<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    get_path(value, path)
}

fn collect_collections(
    value: &Value,
    path: &mut Vec<String>,
    out: &mut Vec<(SurfaceShape, usize, usize)>,
) {
    if let Some((layout, fields, rows)) = collection_shape(value) {
        let mut id_sources = fields.clone();
        if layout == LayoutShape::Map {
            id_sources.push("$key".to_owned());
        }
        id_sources.sort();
        id_sources.dedup();
        out.push((
            SurfaceShape {
                layout,
                root_path: path.clone(),
                record_fields: fields.clone(),
                id_sources,
            },
            fields.len(),
            rows * fields.len(),
        ));
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                path.push(key.clone());
                collect_collections(child, path, out);
                path.pop();
            }
        }
        Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                path.push(index.to_string());
                collect_collections(child, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn collection_shape(value: &Value) -> Option<(LayoutShape, Vec<String>, usize)> {
    if let Some(array) = value.as_array()
        && !array.is_empty()
        && array.iter().all(Value::is_object)
    {
        let fields = common_object_fields(array.iter().filter_map(Value::as_object));
        if !fields.is_empty() {
            return Some((LayoutShape::List, fields, array.len()));
        }
    }
    let object = value.as_object()?;
    if object.is_empty() {
        return None;
    }
    if object.values().all(Value::is_object) {
        let fields = common_object_fields(object.values().filter_map(Value::as_object));
        if !fields.is_empty() {
            return Some((LayoutShape::Map, fields, object.len()));
        }
    }
    if object.values().all(Value::is_array) {
        let lengths = object
            .values()
            .filter_map(Value::as_array)
            .map(Vec::len)
            .collect::<BTreeSet<_>>();
        if lengths.len() == 1 {
            let rows = lengths.into_iter().next().unwrap_or(0);
            if rows > 0 {
                return Some((LayoutShape::Columns, object.keys().cloned().collect(), rows));
            }
        }
    }
    None
}

fn common_object_fields<'a>(objects: impl Iterator<Item = &'a Map<String, Value>>) -> Vec<String> {
    let mut iterator = objects;
    let Some(first) = iterator.next() else {
        return Vec::new();
    };
    let mut fields = first.keys().cloned().collect::<BTreeSet<_>>();
    for object in iterator {
        fields.retain(|field| object.contains_key(field));
    }
    fields.into_iter().collect()
}

fn records_from_map(root: &Value) -> Result<Vec<RawRecord>, &'static str> {
    let object = root.as_object().ok_or("map_root_not_object")?;
    object
        .iter()
        .map(|(key, value)| {
            Ok(RawRecord {
                key: Some(key.clone()),
                fields: value.as_object().ok_or("map_record_not_object")?.clone(),
            })
        })
        .collect()
}

fn records_from_list(root: &Value) -> Result<Vec<RawRecord>, &'static str> {
    root.as_array()
        .ok_or("list_root_not_array")?
        .iter()
        .map(|value| {
            Ok(RawRecord {
                key: None,
                fields: value.as_object().ok_or("list_record_not_object")?.clone(),
            })
        })
        .collect()
}

fn records_from_columns(root: &Value) -> Result<Vec<RawRecord>, &'static str> {
    let columns = root.as_object().ok_or("columns_root_not_object")?;
    let row_count = columns
        .values()
        .next()
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or("columns_empty")?;
    let mut rows = vec![
        RawRecord {
            key: None,
            fields: Map::new(),
        };
        row_count
    ];
    for (field, values) in columns {
        let values = values.as_array().ok_or("column_not_array")?;
        if values.len() != row_count {
            return Err("column_length_mismatch");
        }
        for (row, value) in rows.iter_mut().zip(values) {
            row.fields.insert(field.clone(), value.clone());
        }
    }
    Ok(rows)
}

fn collect_scalar_paths(value: &Value, path: &mut Vec<String>, out: &mut Vec<ScalarPath>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                path.push(key.clone());
                collect_scalar_paths(child, path, out);
                path.pop();
            }
        }
        Value::Array(_) => {}
        _ => out.push(ScalarPath { path: path.clone() }),
    }
}

fn get_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut cursor = value;
    for segment in path {
        cursor = match cursor {
            Value::Object(object) => object.get(segment)?,
            Value::Array(array) => array.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cursor)
}

pub(crate) fn id_value(record: &RawRecord, source: &str) -> Option<Value> {
    if source == "$key" {
        record.key.clone().map(Value::String)
    } else {
        record.fields.get(source).cloned()
    }
}

pub(crate) fn index_by_id(records: &[RawRecord], source: &str) -> Option<BTreeMap<String, usize>> {
    let mut out = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        let value = id_value(record, source)?;
        let key = serde_json::to_string(&value).ok()?;
        if out.insert(key, index).is_some() {
            return None;
        }
    }
    Some(out)
}
