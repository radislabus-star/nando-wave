use serde_json::{Map, Value, json};

use crate::{LayoutShape, TransitionTrace};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum A2Operator {
    Set,
    Increment,
    Append,
    Delete,
}

pub(crate) const A2_OPERATORS: [A2Operator; 4] = [
    A2Operator::Set,
    A2Operator::Increment,
    A2Operator::Append,
    A2Operator::Delete,
];

#[derive(Clone, Debug)]
pub(crate) struct A2SurfaceSpec {
    pub index: usize,
    pub corpus_seed: u64,
    pub layout: LayoutShape,
    pub outer: String,
    pub root: String,
    pub id: String,
    pub status: String,
    pub count: String,
    pub owner: String,
    pub note: String,
    pub command: String,
    pub kind: String,
    pub target: String,
    pub value: String,
    pub amount: String,
    pub noise: String,
    pub set_kind: String,
    pub increment_kind: String,
    pub append_kind: String,
    pub delete_kind: String,
}

impl A2SurfaceSpec {
    pub(crate) fn new(index: usize, layout: LayoutShape, corpus_seed: u64) -> Self {
        Self {
            index,
            corpus_seed,
            layout,
            outer: seeded_name("space", index, corpus_seed),
            root: seeded_name("rows", index, corpus_seed),
            id: seeded_name("identity", index, corpus_seed),
            status: seeded_name("condition", index, corpus_seed),
            count: seeded_name("quantity", index, corpus_seed),
            owner: seeded_name("holder", index, corpus_seed),
            note: seeded_name("memo", index, corpus_seed),
            command: seeded_name("request", index, corpus_seed),
            kind: seeded_name("verb", index, corpus_seed),
            target: seeded_name("subject", index, corpus_seed),
            value: seeded_name("destination", index, corpus_seed),
            amount: seeded_name("step", index, corpus_seed),
            noise: seeded_name("channel", index, corpus_seed),
            set_kind: seeded_name("commit", index, corpus_seed),
            increment_kind: seeded_name("raise", index, corpus_seed),
            append_kind: seeded_name("insert", index, corpus_seed),
            delete_kind: seeded_name("remove", index, corpus_seed),
        }
    }

    pub(crate) fn kind_for(&self, operator: A2Operator) -> &str {
        match operator {
            A2Operator::Set => &self.set_kind,
            A2Operator::Increment => &self.increment_kind,
            A2Operator::Append => &self.append_kind,
            A2Operator::Delete => &self.delete_kind,
        }
    }
}

pub(crate) fn traces_for(
    spec: &A2SurfaceSpec,
    seed_start: usize,
    per_operator: usize,
) -> Vec<TransitionTrace> {
    let mut traces = Vec::with_capacity(per_operator * A2_OPERATORS.len());
    for (operator_index, operator) in A2_OPERATORS.iter().copied().enumerate() {
        for offset in 0..per_operator {
            traces.push(make_trace(
                spec,
                seed_start + operator_index * per_operator + offset,
                operator,
            ));
        }
    }
    traces
}

pub(crate) fn weak_observe(trace: &TransitionTrace, spec: &A2SurfaceSpec) -> TransitionTrace {
    let mut observed = trace.clone();
    mask_irrelevant_fields(&mut observed.before, spec);
    mask_irrelevant_fields(&mut observed.after, spec);
    if let Some(root) = observed.before.as_object_mut() {
        root.insert(
            "weak_observation_noise".to_owned(),
            Value::String(format!("noise-{}", spec.corpus_seed)),
        );
    }
    if let Some(root) = observed.after.as_object_mut() {
        root.insert(
            "weak_observation_noise".to_owned(),
            Value::String(format!("noise-{}", spec.corpus_seed)),
        );
    }
    if let Some(body) = observed
        .action
        .get_mut(&spec.command)
        .and_then(Value::as_object_mut)
    {
        body.insert(
            "irrelevant_observation_lane".to_owned(),
            Value::String(format!("noise-{}", spec.index)),
        );
    }
    observed
}

fn make_trace(spec: &A2SurfaceSpec, seed: usize, operator: A2Operator) -> TransitionTrace {
    let selected_original = seed % 4;
    let base_ids = (0..4)
        .map(|row| format!("entity_{}_{}_{}", spec.index, seed, row))
        .collect::<Vec<_>>();
    let order = seeded_row_order(spec.corpus_seed, seed);
    let mut ids = order
        .iter()
        .map(|row| base_ids[*row].clone())
        .collect::<Vec<_>>();
    let mut rows = order
        .iter()
        .map(|row| full_row(spec, seed, *row, &base_ids[*row]))
        .collect::<Vec<_>>();
    let selected = order
        .iter()
        .position(|row| *row == selected_original)
        .unwrap_or(0);
    let selected_id = base_ids[selected_original].clone();
    let before = surface_state(spec, &ids, &rows, seed);
    let mut action_body = Map::from_iter([
        (
            spec.kind.clone(),
            Value::String(spec.kind_for(operator).to_owned()),
        ),
        (spec.target.clone(), Value::String(selected_id.clone())),
        (
            spec.noise.clone(),
            Value::String("weak_trace_channel".to_owned()),
        ),
    ]);

    match operator {
        A2Operator::Set => {
            let value = format!("done_{}", seed % 3);
            action_body.insert(spec.value.clone(), Value::String(value.clone()));
            rows[selected].insert(spec.status.clone(), Value::String(value));
        }
        A2Operator::Increment => {
            let amount = (seed % 3 + 1) as u64;
            action_body.insert(spec.amount.clone(), Value::from(amount));
            let current = rows[selected]
                .get(&spec.count)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            rows[selected].insert(spec.count.clone(), Value::from(current + amount));
        }
        A2Operator::Append => {
            let new_id = format!("new_entity_{}_{}", spec.index, seed);
            let value = format!("queued_{}", seed % 5);
            action_body.insert(spec.target.clone(), Value::String(new_id.clone()));
            action_body.insert(spec.value.clone(), Value::String(value.clone()));
            ids.push(new_id.clone());
            let mut appended = Map::from_iter([
                (spec.id.clone(), Value::String(new_id)),
                (spec.status.clone(), Value::String(value)),
            ]);
            if spec.layout == LayoutShape::Columns {
                appended.insert(spec.count.clone(), Value::Null);
                appended.insert(spec.owner.clone(), Value::Null);
                appended.insert(spec.note.clone(), Value::Null);
            }
            rows.push(appended);
        }
        A2Operator::Delete => {
            ids.remove(selected);
            rows.remove(selected);
        }
    }

    TransitionTrace {
        before,
        action: Value::Object(Map::from_iter([(
            spec.command.clone(),
            Value::Object(action_body),
        )])),
        after: surface_state(spec, &ids, &rows, seed),
    }
}

fn full_row(spec: &A2SurfaceSpec, seed: usize, row: usize, id: &str) -> Map<String, Value> {
    Map::from_iter([
        (spec.id.clone(), Value::String(id.to_owned())),
        (
            spec.status.clone(),
            Value::String(format!("open_{}", (seed + row) % 5)),
        ),
        (spec.count.clone(), Value::from((seed + row + 10) as u64)),
        (
            spec.owner.clone(),
            Value::String(format!("owner_{}", (seed + row) % 7)),
        ),
        (
            spec.note.clone(),
            Value::String(format!("frame_{}_{}", seed, row)),
        ),
    ])
}

fn surface_state(
    spec: &A2SurfaceSpec,
    ids: &[String],
    rows: &[Map<String, Value>],
    seed: usize,
) -> Value {
    let collection = match spec.layout {
        LayoutShape::Map => Value::Object(Map::from_iter(rows.iter().enumerate().map(
            |(index, row)| {
                let mut row = row.clone();
                row.remove(&spec.id);
                (ids[index].clone(), Value::Object(row))
            },
        ))),
        LayoutShape::List => Value::Array(rows.iter().cloned().map(Value::Object).collect()),
        LayoutShape::Columns => {
            let mut columns = Map::new();
            for field in [&spec.id, &spec.status, &spec.count, &spec.owner, &spec.note] {
                columns.insert(
                    field.clone(),
                    Value::Array(
                        rows.iter()
                            .map(|row| row.get(field).cloned().unwrap_or(Value::Null))
                            .collect(),
                    ),
                );
            }
            Value::Object(columns)
        }
    };
    json!({
        spec.outer.clone(): {
            spec.root.clone(): collection,
            "surface_frame": format!("surface-{}", spec.index)
        },
        "trace_frame": format!("trace-{seed}")
    })
}

fn mask_irrelevant_fields(state: &mut Value, spec: &A2SurfaceSpec) {
    let Some(root) = state
        .get_mut(&spec.outer)
        .and_then(Value::as_object_mut)
        .and_then(|outer| outer.get_mut(&spec.root))
    else {
        return;
    };
    match spec.layout {
        LayoutShape::Map => {
            if let Some(rows) = root.as_object_mut() {
                for row in rows.values_mut().filter_map(Value::as_object_mut) {
                    row.remove(&spec.owner);
                    row.remove(&spec.note);
                }
            }
        }
        LayoutShape::List => {
            if let Some(rows) = root.as_array_mut() {
                for row in rows.iter_mut().filter_map(Value::as_object_mut) {
                    row.remove(&spec.owner);
                    row.remove(&spec.note);
                }
            }
        }
        LayoutShape::Columns => {
            if let Some(columns) = root.as_object_mut() {
                columns.remove(&spec.owner);
                columns.remove(&spec.note);
            }
        }
    }
}

fn seeded_name(role: &str, index: usize, seed: u64) -> String {
    let salt = mix64(seed ^ stable_role_hash(role));
    match seed % 3 {
        0 => format!("{role}_{index}_{:x}", salt & 0xfff),
        1 => format!("{:x}_{role}_{index}", salt & 0xfff),
        _ => format!("{role}{:x}_{index}", salt & 0xfff),
    }
}

fn seeded_row_order(corpus_seed: u64, trace_seed: usize) -> Vec<usize> {
    let mut order = vec![0, 1, 2, 3];
    let mut state = mix64(corpus_seed ^ trace_seed as u64);
    for index in (1..order.len()).rev() {
        state = mix64(state);
        let swap = usize::try_from(state % (index as u64 + 1)).unwrap_or(0);
        order.swap(index, swap);
    }
    order
}

fn stable_role_hash(role: &str) -> u64 {
    role.as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
            (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
