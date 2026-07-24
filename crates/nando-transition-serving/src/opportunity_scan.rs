use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use nando_operator_kernel::{CollectionOutputRenderer, ResponseOperation, ResponseValueSelector};
use nando_operator_learning::CollectionSynthesisExample;
use nando_response_actor::{
    diagnose_response_dynamic_coverage, enumerate_source_neutral_response_programs,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SCHEMA: &str = "nando.streaming-opportunity-scan.v13";
const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_FINAL_BYTES: usize = 16 * 1024;
const MAX_TURN_BYTES: usize = 512 * 1024;
const MAX_SOURCE_BYTES_PER_PASS: u64 = 32 * 1024 * 1024;
const MAX_SOURCE_TURNS_PER_PASS: u64 = 256;
const MAX_ALL_SOURCE_BYTES_PER_PASS: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct SourceCheckpoint {
    offset: u64,
    turns: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ScanCheckpoint {
    schema: String,
    #[serde(default)]
    next_source_index: usize,
    sources: BTreeMap<String, SourceCheckpoint>,
    status: OpportunityScanStatus,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpportunityClassStatus {
    pub examples: u64,
    pub potential_input_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpportunityScanStatus {
    pub ready: bool,
    pub busy: bool,
    pub source_files_seen: u64,
    pub source_files_complete: u64,
    pub turns_scanned: u64,
    pub turns_with_tool_output: u64,
    pub turns_with_final: u64,
    pub observed_input_tokens: u64,
    pub classified_input_tokens: u64,
    pub classification_identity: bool,
    pub synthesis_examples: u64,
    pub synthesis_input_tokens: u64,
    pub exact_program_examples: u64,
    pub exact_program_potential_input_tokens: u64,
    pub policy_rejected_exact_examples: u64,
    pub policy_rejection_reasons: BTreeMap<String, u64>,
    pub policy_rejected_examples_by_reason: BTreeMap<String, OpportunityClassStatus>,
    pub policy_rejected_examples_by_reason_and_dynamic: BTreeMap<String, OpportunityClassStatus>,
    pub static_text_rejection_reasons: BTreeMap<String, u64>,
    pub static_text_rejected_examples_by_reason: BTreeMap<String, OpportunityClassStatus>,
    pub synthesis_errors: BTreeMap<String, u64>,
    pub scalar_overlap_examples: u64,
    pub unsupported_examples: u64,
    pub malformed_rows: u64,
    pub oversized_turns: u64,
    pub classes: BTreeMap<String, OpportunityClassStatus>,
    pub turn_classes: BTreeMap<String, OpportunityClassStatus>,
    pub dynamic_coverage_classes: BTreeMap<String, OpportunityClassStatus>,
    pub last_error: String,
}

#[derive(Default)]
struct TurnSample {
    request: Option<Value>,
    outputs: Vec<Value>,
    expected: Option<String>,
    input_tokens: u64,
    retained_bytes: usize,
    overflow: bool,
}

impl TurnSample {
    fn clear(&mut self) {
        *self = Self::default();
    }

    fn observe(&mut self, row: &Value, row_bytes: usize) {
        self.retained_bytes = self.retained_bytes.saturating_add(row_bytes);
        if self.retained_bytes > MAX_TURN_BYTES {
            self.overflow = true;
            self.outputs.clear();
            self.expected = None;
            return;
        }
        let row_type = row.get("type").and_then(Value::as_str).unwrap_or("");
        let Some(payload) = row.get("payload") else {
            return;
        };
        let payload_type = payload.get("type").and_then(Value::as_str).unwrap_or("");
        if row_type == "event_msg"
            && payload_type == "user_message"
            && let Some(text) = payload.get("message").and_then(Value::as_str)
        {
            self.request = request_item(text);
        }
        if row_type == "response_item"
            && payload_type == "message"
            && payload.get("role").and_then(Value::as_str) == Some("user")
            && let Some(text) = message_text(payload.get("content"))
        {
            self.request = request_item(&text);
        }
        if row_type == "response_item"
            && matches!(
                payload_type,
                "function_call_output" | "custom_tool_call_output"
            )
            && let Some(text) = payload.get("output").and_then(bounded_output_text)
        {
            self.outputs.push(serde_json::json!({
                "type": "function_call_output",
                "output": text,
            }));
        }
        if row_type == "event_msg"
            && payload_type == "agent_message"
            && payload.get("phase").and_then(Value::as_str) == Some("final_answer")
        {
            self.expected = payload
                .get("message")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty() && text.len() <= MAX_FINAL_BYTES)
                .map(str::to_owned);
        }
        if row_type == "response_item"
            && payload_type == "message"
            && payload.get("role").and_then(Value::as_str) == Some("assistant")
            && payload.get("phase").and_then(Value::as_str) == Some("final_answer")
        {
            self.expected = message_text(payload.get("content"));
        }
        if row_type == "event_msg" && payload_type == "token_count" {
            self.input_tokens = payload
                .get("info")
                .and_then(|info| info.get("last_token_usage"))
                .and_then(|usage| usage.get("input_tokens"))
                .and_then(Value::as_u64)
                .unwrap_or(self.input_tokens);
        }
    }
}

fn bounded_output_text(output: &Value) -> Option<String> {
    match output {
        Value::String(text) if !text.is_empty() && text.len() <= MAX_OUTPUT_BYTES => {
            Some(text.clone())
        }
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            let mut combined = String::new();
            for part in parts {
                if !matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) {
                    return None;
                }
                let text = part.get("text").and_then(Value::as_str)?;
                if combined.len().saturating_add(text.len()) > MAX_OUTPUT_BYTES {
                    return None;
                }
                combined.push_str(text);
            }
            (!combined.is_empty()).then_some(combined)
        }
        _ => None,
    }
}

pub fn spawn_opportunity_scan(
    root: PathBuf,
    checkpoint_path: PathBuf,
    shared_status: Arc<RwLock<OpportunityScanStatus>>,
) -> Result<(), String> {
    thread::Builder::new()
        .name("nando-opportunity-scan".to_owned())
        .spawn(move || {
            loop {
                set_busy(&shared_status, true);
                match run_pass(&root, &checkpoint_path, &shared_status) {
                    Ok(all_complete) => {
                        set_busy(&shared_status, false);
                        thread::sleep(if all_complete {
                            Duration::from_secs(30)
                        } else {
                            Duration::from_millis(250)
                        });
                    }
                    Err(error) => {
                        if let Ok(mut status) = shared_status.write() {
                            status.busy = false;
                            status.last_error = error;
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        })
        .map_err(|error| format!("opportunity_scan_thread:{error}"))?;
    Ok(())
}

fn run_pass(
    root: &Path,
    checkpoint_path: &Path,
    shared_status: &Arc<RwLock<OpportunityScanStatus>>,
) -> Result<bool, String> {
    let mut checkpoint = load_checkpoint(checkpoint_path)?;
    let mut paths = session_files(root);
    paths.sort_by(|left, right| {
        let left_modified = fs::metadata(left).and_then(|value| value.modified()).ok();
        let right_modified = fs::metadata(right).and_then(|value| value.modified()).ok();
        right_modified
            .cmp(&left_modified)
            .then_with(|| left.cmp(right))
    });
    checkpoint.status.source_files_seen = paths.len() as u64;
    let mut all_complete = true;
    let mut pass_bytes = 0_u64;
    let source_count = paths.len();
    let start = if source_count == 0 {
        0
    } else {
        checkpoint.next_source_index % source_count
    };
    let mut processed_sources = 0_usize;
    for step in 0..source_count {
        let path = paths[(start + step) % source_count].clone();
        let key = path.to_string_lossy().into_owned();
        let source = checkpoint.sources.entry(key).or_default();
        let previous_offset = source.offset;
        let complete = scan_source(&path, source, &mut checkpoint.status)?;
        pass_bytes = pass_bytes.saturating_add(source.offset.saturating_sub(previous_offset));
        all_complete &= complete;
        persist_checkpoint(checkpoint_path, &checkpoint)?;
        publish(shared_status, &checkpoint.status);
        processed_sources = processed_sources.saturating_add(1);
        if pass_bytes >= MAX_ALL_SOURCE_BYTES_PER_PASS {
            all_complete = false;
            break;
        }
    }
    if source_count > 0 {
        checkpoint.next_source_index = (start + processed_sources) % source_count;
    }
    checkpoint.status.source_files_complete = checkpoint
        .sources
        .iter()
        .filter(|(path, source)| {
            fs::metadata(path).is_ok_and(|metadata| source.offset >= metadata.len())
        })
        .count() as u64;
    checkpoint.status.ready = true;
    persist_checkpoint(checkpoint_path, &checkpoint)?;
    publish(shared_status, &checkpoint.status);
    Ok(all_complete)
}

fn scan_source(
    path: &Path,
    source: &mut SourceCheckpoint,
    status: &mut OpportunityScanStatus,
) -> Result<bool, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("opportunity_scan_open:{}:{error}", path.display()))?;
    let length = file
        .metadata()
        .map_err(|error| format!("opportunity_scan_metadata:{}:{error}", path.display()))?
        .len();
    if source.offset > length {
        *source = SourceCheckpoint::default();
    }
    file.seek(SeekFrom::Start(source.offset))
        .map_err(|error| format!("opportunity_scan_seek:{}:{error}", path.display()))?;
    let pass_start = source.offset;
    let pass_turn_start = source.turns;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut turn = TurnSample::default();
    let mut line = String::new();
    loop {
        line.clear();
        let position = reader
            .stream_position()
            .map_err(|error| format!("opportunity_scan_position:{error}"))?;
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("opportunity_scan_read:{}:{error}", path.display()))?;
        if bytes == 0 || !line.ends_with('\n') {
            source.offset = position;
            if bytes == 0 {
                finish_turn(&mut turn, status)?;
            }
            return Ok(source.offset >= length);
        }
        source.offset = position.saturating_add(bytes as u64);
        let Ok(row) = serde_json::from_str::<Value>(line.trim_end()) else {
            status.malformed_rows = status.malformed_rows.saturating_add(1);
            continue;
        };
        if is_turn_boundary(&row) {
            finish_turn(&mut turn, status)?;
            source.turns = source.turns.saturating_add(1);
            if source.offset.saturating_sub(pass_start) >= MAX_SOURCE_BYTES_PER_PASS
                || source.turns.saturating_sub(pass_turn_start) >= MAX_SOURCE_TURNS_PER_PASS
            {
                return Ok(false);
            }
        }
        turn.observe(&row, bytes);
    }
}

fn finish_turn(turn: &mut TurnSample, status: &mut OpportunityScanStatus) -> Result<(), String> {
    if turn.retained_bytes == 0 {
        return Ok(());
    }
    status.turns_scanned = status.turns_scanned.saturating_add(1);
    status.observed_input_tokens = status
        .observed_input_tokens
        .saturating_add(turn.input_tokens);
    if turn.overflow {
        status.oversized_turns = status.oversized_turns.saturating_add(1);
        classify_turn(status, "oversized", turn.input_tokens);
        turn.clear();
        return Ok(());
    }
    if !turn.outputs.is_empty() {
        status.turns_with_tool_output = status.turns_with_tool_output.saturating_add(1);
    }
    if turn.expected.is_some() {
        status.turns_with_final = status.turns_with_final.saturating_add(1);
    }
    let Some(expected_response) = turn.expected.take() else {
        classify_turn(status, "no_final_response", turn.input_tokens);
        turn.clear();
        return Ok(());
    };
    if turn.outputs.is_empty() && turn.request.is_none() {
        classify_turn(status, "final_without_grounded_input", turn.input_tokens);
        turn.clear();
        return Ok(());
    }
    status.synthesis_examples = status.synthesis_examples.saturating_add(1);
    status.synthesis_input_tokens = status
        .synthesis_input_tokens
        .saturating_add(turn.input_tokens);
    let has_tool_output = !turn.outputs.is_empty();
    let mut input = Vec::new();
    if let Some(request) = turn.request.take() {
        input.push(request);
    }
    input.append(&mut turn.outputs);
    let example = CollectionSynthesisExample {
        provider_payload: serde_json::json!({"input": input}),
        expected_response,
    };
    let dynamic_coverage_class = observe_dynamic_coverage(status, &example, turn.input_tokens);
    let scalar_overlap = has_scalar_overlap(&example);
    let space = match enumerate_source_neutral_response_programs(&example) {
        Ok(space) => space,
        Err(error) => {
            status.unsupported_examples = status.unsupported_examples.saturating_add(1);
            *status.synthesis_errors.entry(error.to_owned()).or_default() += 1;
            classify_turn(status, "synthesis_error", turn.input_tokens);
            turn.clear();
            return Ok(());
        }
    };
    if !space.programs.is_empty() {
        classify_turn(status, "exact_program", turn.input_tokens);
        status.exact_program_examples = status.exact_program_examples.saturating_add(1);
        status.exact_program_potential_input_tokens = status
            .exact_program_potential_input_tokens
            .saturating_add(turn.input_tokens);
        for class in space.programs.iter().map(program_class) {
            let entry = status.classes.entry(class).or_default();
            entry.examples = entry.examples.saturating_add(1);
            entry.potential_input_tokens = entry
                .potential_input_tokens
                .saturating_add(turn.input_tokens);
        }
    } else {
        status.unsupported_examples = status.unsupported_examples.saturating_add(1);
        let class = if space.policy_rejected_exact_matches > 0 {
            "policy_rejected_exact_program"
        } else if !has_tool_output {
            "request_only_unsupported"
        } else if scalar_overlap {
            "tool_output_partial_overlap"
        } else {
            "tool_output_no_scalar_overlap"
        };
        classify_turn(status, class, turn.input_tokens);
    }
    if space.policy_rejected_exact_matches > 0 {
        status.policy_rejected_exact_examples =
            status.policy_rejected_exact_examples.saturating_add(1);
        for (reason, count) in space.policy_rejection_reasons {
            let entry = status
                .policy_rejection_reasons
                .entry(reason.clone())
                .or_default();
            *entry = entry.saturating_add(count as u64);
            observe_class_status(
                &mut status.policy_rejected_examples_by_reason,
                reason.clone(),
                turn.input_tokens,
            );
            observe_class_status(
                &mut status.policy_rejected_examples_by_reason_and_dynamic,
                format!("{reason}.{dynamic_coverage_class}"),
                turn.input_tokens,
            );
        }
        for (reason, count) in space.static_text_rejection_reasons {
            let entry = status
                .static_text_rejection_reasons
                .entry(reason.clone())
                .or_default();
            *entry = entry.saturating_add(count as u64);
            observe_class_status(
                &mut status.static_text_rejected_examples_by_reason,
                reason,
                turn.input_tokens,
            );
        }
    }
    if scalar_overlap {
        status.scalar_overlap_examples = status.scalar_overlap_examples.saturating_add(1);
    }
    turn.clear();
    Ok(())
}

fn classify_turn(status: &mut OpportunityScanStatus, class: &str, input_tokens: u64) {
    let entry = status.turn_classes.entry(class.to_owned()).or_default();
    entry.examples = entry.examples.saturating_add(1);
    entry.potential_input_tokens = entry.potential_input_tokens.saturating_add(input_tokens);
    status.classified_input_tokens = status.classified_input_tokens.saturating_add(input_tokens);
    status.classification_identity = status.classified_input_tokens == status.observed_input_tokens;
}

fn observe_dynamic_coverage(
    status: &mut OpportunityScanStatus,
    example: &CollectionSynthesisExample,
    input_tokens: u64,
) -> String {
    let diagnostic = diagnose_response_dynamic_coverage(example);
    let percent = diagnostic
        .dynamic_bytes
        .saturating_mul(100)
        .checked_div(diagnostic.response_bytes)
        .unwrap_or(0);
    let band = match percent {
        0 => "000",
        1..=9 => "001_009",
        10..=24 => "010_024",
        25..=49 => "025_049",
        50..=74 => "050_074",
        75..=89 => "075_089",
        90..=99 => "090_099",
        _ => "100",
    };
    let source = if diagnostic.request_dynamic_bytes == 0 && diagnostic.tool_dynamic_bytes == 0 {
        "none"
    } else if diagnostic.request_dynamic_bytes > diagnostic.tool_dynamic_bytes.saturating_mul(2) {
        "request"
    } else if diagnostic.tool_dynamic_bytes > diagnostic.request_dynamic_bytes.saturating_mul(2) {
        "tool"
    } else {
        "mixed"
    };
    let class = format!("dynamic_{band}.{source}");
    observe_class_status(
        &mut status.dynamic_coverage_classes,
        class.clone(),
        input_tokens,
    );
    class
}

fn observe_class_status(
    classes: &mut BTreeMap<String, OpportunityClassStatus>,
    class: String,
    input_tokens: u64,
) {
    let entry = classes.entry(class).or_default();
    entry.examples = entry.examples.saturating_add(1);
    entry.potential_input_tokens = entry.potential_input_tokens.saturating_add(input_tokens);
}

fn program_class(program: &nando_response_actor::ResponseProgram) -> String {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue {
            selector, renderer, ..
        } => format!(
            "project.{}.{}",
            selector_class(selector),
            renderer_class(renderer)
        ),
        ResponseOperation::ProjectStatus { selector, .. } => {
            format!("status.{}", selector_class(selector))
        }
        ResponseOperation::ComposeCollection { steps, .. } => steps
            .iter()
            .filter_map(|step| serde_json::to_value(step).ok())
            .filter_map(|step| step.get("step").and_then(Value::as_str).map(str::to_owned))
            .collect::<Vec<_>>()
            .join("->"),
        _ => "other".to_owned(),
    }
}

fn selector_class(selector: &ResponseValueSelector) -> &'static str {
    match selector {
        ResponseValueSelector::ContinuationHandle { .. } => "continuation_handle",
        ResponseValueSelector::UniqueScalar { .. } => "unique_scalar",
        ResponseValueSelector::UniqueTurnScalar { .. } => "unique_turn_scalar",
        ResponseValueSelector::ContentLinePrefix { .. } => "content_line_prefix",
        ResponseValueSelector::JsonField { .. } => "json_field",
        ResponseValueSelector::JsonScalarOrdinal { .. } => "json_scalar_ordinal",
        ResponseValueSelector::UniqueTurnJsonField { .. } => "unique_turn_json_field",
        ResponseValueSelector::UniqueActiveTurnJsonField { .. } => "active_turn_json_field",
        ResponseValueSelector::RequestReferencedJsonField { .. } => "request_referenced_json_field",
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. } => {
            "request_referenced_json_field_ordinal"
        }
        ResponseValueSelector::TurnOutputLine { .. } => "turn_output_line",
        ResponseValueSelector::TurnOutputScalarOrdinal { .. } => "turn_output_scalar_ordinal",
        ResponseValueSelector::LatestTurnOutputLine { .. } => "latest_turn_output_line",
        ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. } => {
            "latest_turn_output_scalar_ordinal"
        }
        ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. } => {
            "latest_turn_output_scalar_from_end"
        }
        ResponseValueSelector::CommandOutputBody => "command_output_body",
        ResponseValueSelector::RequestLastToken => "request_last_token",
        ResponseValueSelector::RequestUniqueLiteral => "request_unique_literal",
    }
}

fn renderer_class(renderer: &CollectionOutputRenderer) -> &'static str {
    match renderer {
        CollectionOutputRenderer::Direct => "direct",
        CollectionOutputRenderer::RenderTemplate { .. } => "template",
        CollectionOutputRenderer::RenderSequence { .. } => "sequence",
        CollectionOutputRenderer::RequestTemplate { .. } => "request_template",
    }
}

fn has_scalar_overlap(example: &CollectionSynthesisExample) -> bool {
    example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("output").and_then(Value::as_str))
        .any(|output| {
            output
                .split(|character: char| {
                    character.is_whitespace()
                        || matches!(character, ':' | '=' | ',' | ';' | '[' | ']' | '{' | '}')
                })
                .any(|value| {
                    value.len() >= 2
                        && value.len() <= 128
                        && example.expected_response.contains(value)
                })
        })
}

fn message_text(content: Option<&Value>) -> Option<String> {
    let text = content?
        .as_array()?
        .iter()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty() && text.len() <= MAX_FINAL_BYTES).then_some(text)
}

fn request_item(message: &str) -> Option<Value> {
    (!message.is_empty() && message.len() <= MAX_FINAL_BYTES).then(|| {
        serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{"type":"input_text", "text":message}],
        })
    })
}

fn is_turn_boundary(row: &Value) -> bool {
    row.get("type").and_then(Value::as_str) == Some("turn_context")
}

fn session_files(root: &Path) -> Vec<PathBuf> {
    let mut output = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        let Ok(entries) = fs::read_dir(path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                output.push(path);
            }
        }
    }
    output
}

fn load_checkpoint(path: &Path) -> Result<ScanCheckpoint, String> {
    if !path.exists() {
        return Ok(ScanCheckpoint {
            schema: SCHEMA.to_owned(),
            ..ScanCheckpoint::default()
        });
    }
    let checkpoint: ScanCheckpoint = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("opportunity_scan_checkpoint_read:{error}"))?,
    )
    .map_err(|error| format!("opportunity_scan_checkpoint_decode:{error}"))?;
    if checkpoint.schema != SCHEMA {
        return Err("opportunity_scan_checkpoint_schema".to_owned());
    }
    Ok(checkpoint)
}

fn persist_checkpoint(path: &Path, checkpoint: &ScanCheckpoint) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("opportunity_scan_checkpoint_dir:{error}"))?;
    }
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary)
        .map_err(|error| format!("opportunity_scan_checkpoint_create:{error}"))?;
    serde_json::to_writer(&mut file, checkpoint)
        .map_err(|error| format!("opportunity_scan_checkpoint_encode:{error}"))?;
    file.write_all(b"\n")
        .map_err(|error| format!("opportunity_scan_checkpoint_write:{error}"))?;
    file.sync_data()
        .map_err(|error| format!("opportunity_scan_checkpoint_sync:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("opportunity_scan_checkpoint_rename:{error}"))?;
    Ok(())
}

fn publish(shared: &Arc<RwLock<OpportunityScanStatus>>, status: &OpportunityScanStatus) {
    if let Ok(mut current) = shared.write() {
        let busy = current.busy;
        *current = status.clone();
        current.busy = busy;
    }
}

fn set_busy(shared: &Arc<RwLock<OpportunityScanStatus>>, busy: bool) {
    if let Ok(mut status) = shared.write() {
        status.busy = busy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scan_finds_exact_count_without_persisting_raw_payload() {
        let root = std::env::temp_dir().join(format!(
            "nando-opportunity-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let sessions = root.join("sessions");
        fs::create_dir_all(&sessions).expect("sessions");
        let path = sessions.join("sample.jsonl");
        let mut file = File::create(&path).expect("file");
        for row in [
            json!({"type":"turn_context","payload":{}}),
            json!({"type":"response_item","payload":{"type":"function_call_output","output":"{\"private_surface\":[{\"value\":1},{\"value\":2},{\"value\":3}]}"}}),
            json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"3"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":120}}}}),
            json!({"type":"turn_context","payload":{}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"Do the work. Reply only ONLY_OK."}}),
            json!({"type":"event_msg","payload":{"type":"agent_message","phase":"final_answer","message":"ONLY_OK"}}),
            json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":80}}}}),
            json!({"type":"turn_context","payload":{}}),
        ] {
            serde_json::to_writer(&mut file, &row).expect("row");
            file.write_all(b"\n").expect("newline");
        }
        file.sync_all().expect("sync");
        let checkpoint_path = root.join("opportunities.json");
        let shared = Arc::new(RwLock::new(OpportunityScanStatus::default()));
        assert!(run_pass(&sessions, &checkpoint_path, &shared).expect("pass"));
        let status = shared.read().expect("status").clone();
        assert_eq!(status.exact_program_examples, 2, "{status:?}");
        assert_eq!(status.exact_program_potential_input_tokens, 200);
        assert_eq!(status.classified_input_tokens, 200);
        assert!(status.classification_identity);
        assert_eq!(
            status
                .turn_classes
                .get("exact_program")
                .map(|class| (class.examples, class.potential_input_tokens)),
            Some((2, 200))
        );
        assert!(
            status
                .classes
                .contains_key("project.request_last_token.direct")
        );
        let durable = fs::read_to_string(&checkpoint_path).expect("checkpoint");
        assert!(!durable.contains("private_surface"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn turn_sample_accepts_bounded_custom_output_parts() {
        let mut turn = TurnSample::default();
        turn.observe(
            &json!({
                "type":"response_item",
                "payload":{
                    "type":"custom_tool_call_output",
                    "output":[
                        {"type":"input_text","text":""},
                        {"type":"input_text","text":"{\"session_id\":60906}"}
                    ]
                }
            }),
            64,
        );
        assert_eq!(turn.outputs.len(), 1);
        assert_eq!(
            turn.outputs[0].get("output").and_then(Value::as_str),
            Some("{\"session_id\":60906}")
        );
    }
}
