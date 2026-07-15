//! Nando-owned fail-closed runtime for compact RSEF0001 expression packages.

use serde::Serialize;
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

type ObjectPair<'a> = (&'a Map<String, Value>, &'a Map<String, Value>);

pub type NodeId = u32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Constructor {
    StateRole,
    ActionRole,
    CollectionRole,
    ActionObjectRole,
    Add,
    Subtract,
    Multiply,
    Minimum,
    Maximum,
    Negate,
    Absolute,
    Count,
    Filter,
    MergeMatchingFields,
    SetRecordFieldFromAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueKind {
    Null,
    Bool,
    Int,
    Float,
    String,
    List,
    Object,
}

#[derive(Clone, Debug)]
struct Node {
    constructor: Constructor,
    output: ValueKind,
    children: Vec<NodeId>,
    bindings: Vec<String>,
    depth: u8,
}

#[derive(Clone, Debug)]
struct Program {
    target: ValueKind,
    root: NodeId,
    support: u64,
}

#[derive(Clone, Debug)]
pub struct ExpressionRuntime {
    nodes: Vec<Node>,
    programs: Vec<Program>,
    package_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeExecution {
    pub status: &'static str,
    pub reason: &'static str,
    pub after: Option<Value>,
}

impl ExpressionRuntime {
    pub fn load(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(bytes);
        if cursor.take(8)? != b"RSEF0001" {
            return Err("unsupported_expression_header".into());
        }
        let node_count = cursor.u32()? as usize;
        let program_count = cursor.u32()? as usize;
        let mut nodes: Vec<Node> = Vec::with_capacity(node_count);
        for expected in 0..node_count {
            let constructor = constructor(cursor.u8()?)?;
            let output = kind(cursor.u8()?)?;
            let encoded_depth = cursor.u8()?;
            let child_count = cursor.u8()? as usize;
            let children = (0..child_count)
                .map(|_| cursor.u32())
                .collect::<Result<Vec<_>, _>>()?;
            if children.iter().any(|child| *child as usize >= expected) {
                return Err("expression_child_not_prior_node".into());
            }
            let binding_count = cursor.u8()? as usize;
            let bindings = (0..binding_count)
                .map(|_| cursor.string())
                .collect::<Result<Vec<_>, _>>()?;
            let computed_depth = children
                .iter()
                .map(|id| nodes[*id as usize].depth)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("expression_depth_overflow")?;
            if computed_depth != encoded_depth {
                return Err("expression_depth_mismatch".into());
            }
            nodes.push(Node {
                constructor,
                output,
                children,
                bindings,
                depth: encoded_depth,
            });
        }
        let programs = (0..program_count)
            .map(|_| {
                let target = kind(cursor.u8()?)?;
                let root = cursor.u32()?;
                let support = cursor.u64()?;
                if root as usize >= nodes.len() {
                    return Err("expression_root_missing".into());
                }
                Ok(Program {
                    target,
                    root,
                    support,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if cursor.remaining() != 0 {
            return Err("expression_trailing_bytes".into());
        }
        if programs.is_empty() {
            return Err("expression_programs_empty".into());
        }
        Ok(Self {
            nodes,
            programs,
            package_sha256: format!("{:x}", Sha256::digest(bytes)),
        })
    }

    #[must_use]
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }

    #[must_use]
    pub fn program_count(&self) -> usize {
        self.programs.len()
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn support_total(&self) -> u64 {
        self.programs.iter().map(|program| program.support).sum()
    }

    #[must_use]
    pub fn execute(&self, before: &Value, action: &Value) -> RuntimeExecution {
        let mut outputs = Vec::new();
        for program in &self.programs {
            if let Some(after) = self.execute_one(program, before, action) {
                outputs.push(after);
            }
        }
        let Some(first) = outputs.first() else {
            return RuntimeExecution {
                status: "ABSTAIN",
                reason: "no_expression_applies",
                after: None,
            };
        };
        if outputs.iter().skip(1).any(|after| after != first) {
            return RuntimeExecution {
                status: "ABSTAIN",
                reason: "expression_outputs_disagree",
                after: None,
            };
        }
        RuntimeExecution {
            status: "EXECUTED",
            reason: "nando_expression_verified",
            after: Some(first.clone()),
        }
    }

    fn execute_one(&self, program: &Program, before: &Value, action: &Value) -> Option<Value> {
        let before_object = before.as_object()?;
        let value = self.evaluate(program.root, before, action, 0)?;
        if value_kind(&value) != self.nodes[program.root as usize].output {
            return None;
        }
        let targets = before_object
            .iter()
            .filter(|(_, value)| value_kind(value) == program.target)
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        if targets.len() != 1 || before_object.get(targets[0]) == Some(&value) {
            return None;
        }
        let mut after = before.clone();
        after.as_object_mut()?.insert(targets[0].clone(), value);
        Some(after)
    }

    fn evaluate(&self, id: NodeId, before: &Value, action: &Value, depth: usize) -> Option<Value> {
        if depth > 16 {
            return None;
        }
        let node = self.nodes.get(id as usize)?;
        match node.constructor {
            Constructor::StateRole => unique_top_level(before, node.output),
            Constructor::ActionRole => unique_scalar(action, node.output),
            Constructor::CollectionRole => unique_top_level(before, ValueKind::List),
            Constructor::ActionObjectRole => unique_action_object(action),
            _ => {
                let children = node
                    .children
                    .iter()
                    .map(|child| self.evaluate(*child, before, action, depth + 1))
                    .collect::<Option<Vec<_>>>()?;
                apply(node, &children)
            }
        }
    }
}

fn apply(node: &Node, children: &[Value]) -> Option<Value> {
    match node.constructor {
        Constructor::Add
        | Constructor::Subtract
        | Constructor::Multiply
        | Constructor::Minimum
        | Constructor::Maximum => numeric_binary(node.constructor, children),
        Constructor::Negate | Constructor::Absolute => numeric_unary(node.constructor, children),
        Constructor::Count => Some(Value::Number(Number::from(
            children.first()?.as_array()?.len() as u64,
        ))),
        Constructor::Filter => filter_unique(children),
        Constructor::MergeMatchingFields => merge_matching_fields(children),
        Constructor::SetRecordFieldFromAction => set_record_field(children, &node.bindings),
        _ => None,
    }
}

fn numeric_binary(op: Constructor, children: &[Value]) -> Option<Value> {
    if children.len() != 2 {
        return None;
    }
    if let (Some(left), Some(right)) = (children[0].as_i64(), children[1].as_i64()) {
        let value = match op {
            Constructor::Add => left.checked_add(right),
            Constructor::Subtract => left.checked_sub(right),
            Constructor::Multiply => left.checked_mul(right),
            Constructor::Minimum => Some(left.min(right)),
            Constructor::Maximum => Some(left.max(right)),
            _ => None,
        }?;
        return Some(Value::Number(Number::from(value)));
    }
    let (left, right) = (children[0].as_f64()?, children[1].as_f64()?);
    let value = match op {
        Constructor::Add => left + right,
        Constructor::Subtract => left - right,
        Constructor::Multiply => left * right,
        Constructor::Minimum => left.min(right),
        Constructor::Maximum => left.max(right),
        _ => return None,
    };
    Number::from_f64(value).map(Value::Number)
}

fn numeric_unary(op: Constructor, children: &[Value]) -> Option<Value> {
    if children.len() != 1 {
        return None;
    }
    if let Some(value) = children[0].as_i64() {
        return Some(Value::Number(Number::from(match op {
            Constructor::Negate => value.checked_neg()?,
            Constructor::Absolute => value.checked_abs()?,
            _ => return None,
        })));
    }
    let value = children[0].as_f64()?;
    Number::from_f64(if op == Constructor::Negate {
        -value
    } else {
        value.abs()
    })
    .map(Value::Number)
}

fn filter_unique(children: &[Value]) -> Option<Value> {
    if children.len() != 2 {
        return None;
    }
    let rows = children[0].as_array()?;
    if rows.is_empty() || rows.iter().any(|row| !row.is_object()) {
        return None;
    }
    let selector = &children[1];
    let first = rows[0].as_object()?;
    let fields = first
        .keys()
        .filter(|field| {
            rows.iter().all(|row| {
                row.get(*field)
                    .is_some_and(|value| value_kind(value) == value_kind(selector))
            }) && rows.iter().any(|row| row.get(*field) == Some(selector))
        })
        .collect::<Vec<_>>();
    if fields.len() != 1 {
        return None;
    }
    Some(Value::Array(
        rows.iter()
            .filter(|row| row.get(fields[0]) == Some(selector))
            .cloned()
            .collect(),
    ))
}

fn object_children(children: &[Value]) -> Option<ObjectPair<'_>> {
    if children.len() != 2 {
        return None;
    }
    let rows = children[0].as_array()?;
    if rows.len() != 1 {
        return None;
    }
    Some((rows[0].as_object()?, children[1].as_object()?))
}

fn merge_matching_fields(children: &[Value]) -> Option<Value> {
    let (record, action) = object_children(children)?;
    let mut output = record.clone();
    let mut changed = 0;
    for (field, value) in action {
        if output.get(field).is_some_and(|before| before != value) {
            output.insert(field.clone(), value.clone());
            changed += 1;
        }
    }
    (changed == 1).then(|| Value::Array(vec![Value::Object(output)]))
}

fn set_record_field(children: &[Value], bindings: &[String]) -> Option<Value> {
    let (record, action) = object_children(children)?;
    if bindings.len() != 2 {
        return None;
    }
    let before = record.get(&bindings[0])?;
    let after = action.get(&bindings[1])?;
    if value_kind(before) != value_kind(after) || before == after {
        return None;
    }
    let mut output = record.clone();
    output.insert(bindings[0].clone(), after.clone());
    Some(Value::Array(vec![Value::Object(output)]))
}

fn unique_top_level(value: &Value, expected: ValueKind) -> Option<Value> {
    let values = value
        .as_object()?
        .values()
        .filter(|value| value_kind(value) == expected)
        .collect::<Vec<_>>();
    (values.len() == 1).then(|| values[0].clone())
}

fn unique_action_object(action: &Value) -> Option<Value> {
    let values = action
        .as_object()?
        .values()
        .filter(|value| value.is_object())
        .collect::<Vec<_>>();
    (values.len() == 1).then(|| values[0].clone())
}

fn unique_scalar(value: &Value, expected: ValueKind) -> Option<Value> {
    fn collect(value: &Value, expected: ValueKind, output: &mut Vec<Value>) {
        match value {
            Value::Object(object) => object
                .values()
                .for_each(|value| collect(value, expected, output)),
            Value::Array(_) => {}
            _ if value_kind(value) == expected => output.push(value.clone()),
            _ => {}
        }
    }
    let mut values = Vec::new();
    collect(value, expected, &mut values);
    (values.len() == 1).then(|| values.remove(0))
}

fn value_kind(value: &Value) -> ValueKind {
    match value {
        Value::Null => ValueKind::Null,
        Value::Bool(_) => ValueKind::Bool,
        Value::Number(number) if number.is_i64() || number.is_u64() => ValueKind::Int,
        Value::Number(_) => ValueKind::Float,
        Value::String(_) => ValueKind::String,
        Value::Array(_) => ValueKind::List,
        Value::Object(_) => ValueKind::Object,
    }
}

fn constructor(value: u8) -> Result<Constructor, String> {
    match value {
        1 => Ok(Constructor::StateRole),
        2 => Ok(Constructor::ActionRole),
        3 => Ok(Constructor::CollectionRole),
        4 => Ok(Constructor::ActionObjectRole),
        5 => Ok(Constructor::Add),
        6 => Ok(Constructor::Subtract),
        7 => Ok(Constructor::Multiply),
        8 => Ok(Constructor::Minimum),
        9 => Ok(Constructor::Maximum),
        10 => Ok(Constructor::Negate),
        11 => Ok(Constructor::Absolute),
        12 => Ok(Constructor::Count),
        13 => Ok(Constructor::Filter),
        14 => Ok(Constructor::MergeMatchingFields),
        15 => Ok(Constructor::SetRecordFieldFromAction),
        _ => Err("unknown_expression_constructor".into()),
    }
}

fn kind(value: u8) -> Result<ValueKind, String> {
    match value {
        0 => Ok(ValueKind::Null),
        1 => Ok(ValueKind::Bool),
        2 => Ok(ValueKind::Int),
        3 => Ok(ValueKind::Float),
        4 => Ok(ValueKind::String),
        5 => Ok(ValueKind::List),
        6 => Ok(ValueKind::Object),
        _ => Err("unknown_expression_type".into()),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or("expression_offset_overflow")?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or("expression_artifact_truncated")?;
        self.offset = end;
        Ok(result)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| "invalid_u16")?,
        ))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| "invalid_u32")?,
        ))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| "invalid_u64")?,
        ))
    }
    fn string(&mut self) -> Result<String, String> {
        let length = self.u16()? as usize;
        String::from_utf8(self.take(length)?.to_vec())
            .map_err(|_| "expression_binding_not_utf8".into())
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_and_unknown_packages() {
        assert!(ExpressionRuntime::load(b"RSEF0001").is_err());
        assert!(ExpressionRuntime::load(b"BAD!0001xxxxxxxx").is_err());
    }
}
