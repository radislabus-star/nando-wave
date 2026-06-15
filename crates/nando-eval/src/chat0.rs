use crate::{Chat0Result, format_chat0_result};
use nando_core::STAGE2_TOP_K;

use super::BYTE_CONTEXT_TASKS;

/// First Chat-0 report: short response generation with feedback logging.
#[derive(Debug, Clone, PartialEq)]
pub struct Chat0EvalReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub random: Chat0Result,
    pub mono192_prompt: Chat0Result,
    pub no_snapshot: Chat0Result,
    pub wrong_snapshot: Chat0Result,
    pub corrupted_snapshot: Chat0Result,
    pub prompt_cloud_snapshot: Chat0Result,
    pub feedback_log_entries: usize,
    pub prompt_cloud_over_best_control: f32,
    pub prompt_cloud_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Route-quality report for manual Chat-0 prompts.
#[derive(Debug, Clone, PartialEq)]
pub struct Chat0RouteEvalReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub random: Chat0Result,
    pub mono192_prompt: Chat0Result,
    pub snapshot_classifier: Chat0Result,
    pub prompt_cloud_lock_bank: Chat0Result,
    pub hybrid_route: Chat0Result,
    pub lock_bank_route_count: usize,
    pub feedback_log_entries: usize,
    pub lock_bank_over_snapshot: f32,
    pub hybrid_over_best_control: f32,
    pub mode_status: &'static str,
}

/// One feedback row collected by Chat-0 shell/one-shot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat0FeedbackEntry {
    pub prompt: String,
    pub response: String,
    pub expected: String,
    pub feedback_correct: bool,
}

/// Eval-gated feedback promotion report.
#[derive(Debug, Clone, PartialEq)]
pub struct Chat0PromoteEvalReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub feedback_entries: usize,
    pub correction_entries: usize,
    pub replay_base: Chat0Result,
    pub replay_candidate: Chat0Result,
    pub route_eval: Chat0RouteEvalReport,
    pub replay_improvement: f32,
    pub mode_status: &'static str,
}

/// Holdout report for promoted Chat-0 state generalization.
#[derive(Debug, Clone, PartialEq)]
pub struct Chat0PromotedHoldoutEvalReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub feedback_entries: usize,
    pub promoted_entries: usize,
    pub base: Chat0Result,
    pub exact_overlay: Chat0Result,
    pub harmonic_transfer_overlay: Chat0Result,
    pub selective_harmonic_transfer_overlay: Chat0Result,
    pub cell_signature_transfer_overlay: Chat0Result,
    pub trajectory_transfer_overlay: Chat0Result,
    pub task_hint_overlay: Chat0Result,
    pub harmonic_transfer_ablation_min_accuracy: f32,
    pub harmonic_transfer_ablation_min_drop: f32,
    pub harmonic_transfer_ablation_max_drop: f32,
    pub cell_signature_ablation_min_accuracy: f32,
    pub cell_signature_ablation_min_drop: f32,
    pub cell_signature_ablation_max_drop: f32,
    pub trajectory_ablation_min_accuracy: f32,
    pub trajectory_ablation_min_drop: f32,
    pub trajectory_ablation_max_drop: f32,
    pub selective_harmonic_best_threshold: f32,
    pub selective_harmonic_best_accuracy: f32,
    pub selective_harmonic_best_over_base: f32,
    pub exact_overlay_applied: usize,
    pub harmonic_transfer_applied: usize,
    pub selective_harmonic_transfer_applied: usize,
    pub cell_signature_transfer_applied: usize,
    pub trajectory_transfer_applied: usize,
    pub task_hint_overlay_applied: usize,
    pub exact_over_base: f32,
    pub harmonic_transfer_over_base: f32,
    pub selective_harmonic_transfer_over_base: f32,
    pub cell_signature_transfer_over_base: f32,
    pub trajectory_transfer_over_base: f32,
    pub task_hint_over_base: f32,
    pub mode_status: &'static str,
}

/// One exact correction promoted after an eval gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat0PromotedEntry {
    pub prompt: String,
    pub expected: String,
    pub target: u8,
}

/// Small persisted overlay for Chat-0 feedback learning.
///
/// This is intentionally explicit and tiny: it is not hidden weight mutation.
/// The state can only override exact prompts whose expected response maps to a
/// known Chat-0 target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat0PromotedState {
    pub version: u16,
    pub train_seed: u64,
    pub cases_per_split: usize,
    pub entries: Vec<Chat0PromotedEntry>,
}

impl Chat0EvalReport {
    /// Render a stable report for the first Chat-0 loop.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_chat0_result(self.random));
        output.push_str(&format_chat0_result(self.mono192_prompt));
        output.push_str(&format_chat0_result(self.no_snapshot));
        output.push_str(&format_chat0_result(self.wrong_snapshot));
        output.push_str(&format_chat0_result(self.corrupted_snapshot));
        output.push_str(&format_chat0_result(self.prompt_cloud_snapshot));
        output.push_str(&format!(
            "feedback_log_entries: {}\n",
            self.feedback_log_entries
        ));
        output.push_str(&format!(
            "prompt_cloud_over_best_control: {:.6}\n",
            self.prompt_cloud_over_best_control
        ));
        output.push_str(&format!(
            "prompt_cloud_over_wrong_snapshot: {:.6}\n",
            self.prompt_cloud_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl Chat0PromotedState {
    /// Build a promoted overlay from correction feedback only.
    #[must_use]
    pub fn from_feedback(train_seed: u64, cases: usize, feedback: &[Chat0FeedbackEntry]) -> Self {
        let mut entries = Vec::new();
        for entry in feedback {
            if entry.feedback_correct {
                continue;
            }
            let Some(target) = chat0_target_for_response(&entry.expected) else {
                continue;
            };
            if entries
                .iter()
                .any(|promoted: &Chat0PromotedEntry| promoted.prompt == entry.prompt)
            {
                continue;
            }
            entries.push(Chat0PromotedEntry {
                prompt: entry.prompt.clone(),
                expected: entry.expected.clone(),
                target,
            });
        }

        Self {
            version: 1,
            train_seed,
            cases_per_split: cases,
            entries,
        }
    }

    /// Render this state to a stable line-oriented text format.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 promoted state\n");
        output.push_str(&format!("version: {}\n", self.version));
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("entry_count: {}\n", self.entries.len()));
        for entry in &self.entries {
            output.push_str(&format!(
                "entry: prompt_hex={} expected_hex={} target={}\n",
                hex_encode(entry.prompt.as_bytes()),
                hex_encode(entry.expected.as_bytes()),
                entry.target as char
            ));
        }
        output
    }

    /// Parse a promoted state from [`Chat0PromotedState::to_text`].
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        match lines.next() {
            Some("Nando Wave chat-0 promoted state") => {}
            _ => return Err(String::from("bad promoted state header")),
        }

        let version = parse_prefixed_u16(lines.next(), "version: ")?;
        if version != 1 {
            return Err(format!("unsupported promoted state version {version}"));
        }
        let train_seed = parse_prefixed_u64(lines.next(), "train_seed: ")?;
        let cases_per_split = parse_prefixed_usize(lines.next(), "cases_per_split: ")?;
        let entry_count = parse_prefixed_usize(lines.next(), "entry_count: ")?;
        let mut entries = Vec::with_capacity(entry_count);

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let rest = line
                .strip_prefix("entry: ")
                .ok_or_else(|| format!("bad promoted state line '{line}'"))?;
            let prompt_hex = promoted_state_field(rest, "prompt_hex=", " expected_hex=")?;
            let expected_hex = promoted_state_field(rest, " expected_hex=", " target=")?;
            let target = promoted_state_field_to_end(rest, " target=")?;
            let target = parse_target_char(target)?;
            let prompt = String::from_utf8(hex_decode(prompt_hex)?)
                .map_err(|error| format!("invalid prompt utf8: {error}"))?;
            let expected = String::from_utf8(hex_decode(expected_hex)?)
                .map_err(|error| format!("invalid expected utf8: {error}"))?;
            entries.push(Chat0PromotedEntry {
                prompt,
                expected,
                target,
            });
        }

        if entries.len() != entry_count {
            return Err(format!(
                "entry_count mismatch: header {entry_count}, parsed {}",
                entries.len()
            ));
        }

        Ok(Self {
            version,
            train_seed,
            cases_per_split,
            entries,
        })
    }

    pub(crate) fn target_for_prompt(
        &self,
        train_seed: u64,
        cases: usize,
        prompt: &[u8],
    ) -> Option<u8> {
        if self.train_seed != train_seed || self.cases_per_split != cases {
            return None;
        }
        self.entries
            .iter()
            .find_map(|entry| (entry.prompt.as_bytes() == prompt).then_some(entry.target))
    }

    pub(crate) fn target_for_task_hint(
        &self,
        train_seed: u64,
        cases: usize,
        prompt: &[u8],
    ) -> Option<u8> {
        if self.train_seed != train_seed || self.cases_per_split != cases {
            return None;
        }
        self.entries.iter().find_map(|entry| {
            let task = chat0_task_for_target(entry.target);
            (task != "unknown" && contains_ascii_word(prompt, task.as_bytes()))
                .then_some(entry.target)
        })
    }
}

impl Chat0PromoteEvalReport {
    /// Render a stable report for eval-gated feedback promotion.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 promote eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("feedback_entries: {}\n", self.feedback_entries));
        output.push_str(&format!(
            "correction_entries: {}\n",
            self.correction_entries
        ));
        output.push_str(&format_chat0_result(self.replay_base));
        output.push_str(&format_chat0_result(self.replay_candidate));
        output.push_str(&format!(
            "route_mode_status: {}\n",
            self.route_eval.mode_status
        ));
        output.push_str(&format!(
            "route_hybrid_accuracy: {:.6}\n",
            self.route_eval.hybrid_route.exact_accuracy
        ));
        output.push_str(&format!(
            "replay_improvement: {:.6}\n",
            self.replay_improvement
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl Chat0PromotedHoldoutEvalReport {
    /// Render a stable report for promoted-state holdout behavior.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 promoted holdout eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("feedback_entries: {}\n", self.feedback_entries));
        output.push_str(&format!("promoted_entries: {}\n", self.promoted_entries));
        output.push_str(&format_chat0_result(self.base));
        output.push_str(&format_chat0_result(self.exact_overlay));
        output.push_str(&format_chat0_result(self.harmonic_transfer_overlay));
        output.push_str(&format_chat0_result(
            self.selective_harmonic_transfer_overlay,
        ));
        output.push_str(&format_chat0_result(self.cell_signature_transfer_overlay));
        output.push_str(&format_chat0_result(self.trajectory_transfer_overlay));
        output.push_str(&format_chat0_result(self.task_hint_overlay));
        output.push_str(&format!(
            "harmonic_transfer_ablation_min_accuracy: {:.6}\n",
            self.harmonic_transfer_ablation_min_accuracy
        ));
        output.push_str(&format!(
            "harmonic_transfer_ablation_min_drop: {:.6}\n",
            self.harmonic_transfer_ablation_min_drop
        ));
        output.push_str(&format!(
            "harmonic_transfer_ablation_max_drop: {:.6}\n",
            self.harmonic_transfer_ablation_max_drop
        ));
        output.push_str(&format!(
            "cell_signature_ablation_min_accuracy: {:.6}\n",
            self.cell_signature_ablation_min_accuracy
        ));
        output.push_str(&format!(
            "cell_signature_ablation_min_drop: {:.6}\n",
            self.cell_signature_ablation_min_drop
        ));
        output.push_str(&format!(
            "cell_signature_ablation_max_drop: {:.6}\n",
            self.cell_signature_ablation_max_drop
        ));
        output.push_str(&format!(
            "trajectory_ablation_min_accuracy: {:.6}\n",
            self.trajectory_ablation_min_accuracy
        ));
        output.push_str(&format!(
            "trajectory_ablation_min_drop: {:.6}\n",
            self.trajectory_ablation_min_drop
        ));
        output.push_str(&format!(
            "trajectory_ablation_max_drop: {:.6}\n",
            self.trajectory_ablation_max_drop
        ));
        output.push_str(&format!(
            "selective_harmonic_best_threshold: {:.6}\n",
            self.selective_harmonic_best_threshold
        ));
        output.push_str(&format!(
            "selective_harmonic_best_accuracy: {:.6}\n",
            self.selective_harmonic_best_accuracy
        ));
        output.push_str(&format!(
            "selective_harmonic_best_over_base: {:.6}\n",
            self.selective_harmonic_best_over_base
        ));
        output.push_str(&format!(
            "exact_overlay_applied: {}\n",
            self.exact_overlay_applied
        ));
        output.push_str(&format!(
            "harmonic_transfer_applied: {}\n",
            self.harmonic_transfer_applied
        ));
        output.push_str(&format!(
            "selective_harmonic_transfer_applied: {}\n",
            self.selective_harmonic_transfer_applied
        ));
        output.push_str(&format!(
            "cell_signature_transfer_applied: {}\n",
            self.cell_signature_transfer_applied
        ));
        output.push_str(&format!(
            "trajectory_transfer_applied: {}\n",
            self.trajectory_transfer_applied
        ));
        output.push_str(&format!(
            "task_hint_overlay_applied: {}\n",
            self.task_hint_overlay_applied
        ));
        output.push_str(&format!("exact_over_base: {:.6}\n", self.exact_over_base));
        output.push_str(&format!(
            "harmonic_transfer_over_base: {:.6}\n",
            self.harmonic_transfer_over_base
        ));
        output.push_str(&format!(
            "selective_harmonic_transfer_over_base: {:.6}\n",
            self.selective_harmonic_transfer_over_base
        ));
        output.push_str(&format!(
            "cell_signature_transfer_over_base: {:.6}\n",
            self.cell_signature_transfer_over_base
        ));
        output.push_str(&format!(
            "trajectory_transfer_over_base: {:.6}\n",
            self.trajectory_transfer_over_base
        ));
        output.push_str(&format!(
            "task_hint_over_base: {:.6}\n",
            self.task_hint_over_base
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl Chat0RouteEvalReport {
    /// Render a stable report for manual Chat-0 route quality.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 route eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_chat0_result(self.random));
        output.push_str(&format_chat0_result(self.mono192_prompt));
        output.push_str(&format_chat0_result(self.snapshot_classifier));
        output.push_str(&format_chat0_result(self.prompt_cloud_lock_bank));
        output.push_str(&format_chat0_result(self.hybrid_route));
        output.push_str(&format!(
            "lock_bank_route_count: {}\n",
            self.lock_bank_route_count
        ));
        output.push_str(&format!(
            "feedback_log_entries: {}\n",
            self.feedback_log_entries
        ));
        output.push_str(&format!(
            "lock_bank_over_snapshot: {:.6}\n",
            self.lock_bank_over_snapshot
        ));
        output.push_str(&format!(
            "hybrid_over_best_control: {:.6}\n",
            self.hybrid_over_best_control
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

/// One-shot Chat-0 trace for an interactive prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct Chat0Trace {
    pub train_seed: u64,
    pub cases_per_split: usize,
    pub prompt: String,
    pub route: &'static str,
    pub predicted_task: &'static str,
    pub predicted_target: u8,
    pub response: &'static str,
    pub expected_response: Option<String>,
    pub feedback_correct: Option<bool>,
    pub active_cell_ids: [u32; STAGE2_TOP_K],
    pub coherence: f32,
    pub spectral_entropy: f32,
    pub center_phase: f32,
    pub center_magnitude: f32,
    pub snapshot_bytes: usize,
    pub mode_status: &'static str,
}

impl Chat0Trace {
    /// Render a stable, line-oriented trace for CLI output and trace files.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave chat-0 trace\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("prompt: {}\n", self.prompt));
        output.push_str(&format!("route: {}\n", self.route));
        output.push_str(&format!("predicted_task: {}\n", self.predicted_task));
        output.push_str(&format!(
            "predicted_target: {}\n",
            self.predicted_target as char
        ));
        output.push_str(&format!("response: {}\n", self.response));
        match &self.expected_response {
            Some(expected) => output.push_str(&format!("expected_response: {expected}\n")),
            None => output.push_str("expected_response: none\n"),
        }
        match self.feedback_correct {
            Some(correct) => output.push_str(&format!("feedback_correct: {correct}\n")),
            None => output.push_str("feedback_correct: none\n"),
        }
        output.push_str(&format!("active_cell_ids: {:?}\n", self.active_cell_ids));
        output.push_str(&format!("coherence: {:.6}\n", self.coherence));
        output.push_str(&format!("spectral_entropy: {:.6}\n", self.spectral_entropy));
        output.push_str(&format!("center_phase: {:.6}\n", self.center_phase));
        output.push_str(&format!("center_magnitude: {:.6}\n", self.center_magnitude));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

pub(crate) fn chat0_response_for_target(target: u8) -> &'static str {
    match target {
        b'p' => "pong",
        b'n' => "nando",
        b't' => "now",
        b'h' => "help",
        b'e' => "echo",
        b's' => "saved",
        b'o' => "opened",
        b'c' => "closed",
        _ => "?",
    }
}

pub(crate) fn chat0_target_for_response(response: &str) -> Option<u8> {
    BYTE_CONTEXT_TASKS
        .iter()
        .find_map(|(_, target)| (chat0_response_for_target(*target) == response).then_some(*target))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(String::from("hex length must be even"));
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex byte '{}'", byte as char)),
    }
}

fn parse_prefixed_u16(line: Option<&str>, prefix: &str) -> Result<u16, String> {
    parse_prefixed(line, prefix)?
        .parse::<u16>()
        .map_err(|error| format!("invalid {prefix} value: {error}"))
}

fn parse_prefixed_u64(line: Option<&str>, prefix: &str) -> Result<u64, String> {
    parse_prefixed(line, prefix)?
        .parse::<u64>()
        .map_err(|error| format!("invalid {prefix} value: {error}"))
}

fn parse_prefixed_usize(line: Option<&str>, prefix: &str) -> Result<usize, String> {
    parse_prefixed(line, prefix)?
        .parse::<usize>()
        .map_err(|error| format!("invalid {prefix} value: {error}"))
}

fn parse_prefixed<'a>(line: Option<&'a str>, prefix: &str) -> Result<&'a str, String> {
    line.ok_or_else(|| format!("missing line '{prefix}...'"))?
        .strip_prefix(prefix)
        .ok_or_else(|| format!("missing prefix '{prefix}'"))
}

fn promoted_state_field<'a>(line: &'a str, start: &str, end: &str) -> Result<&'a str, String> {
    let start_index = line
        .find(start)
        .ok_or_else(|| format!("missing marker '{start}'"))?
        + start.len();
    let rest = &line[start_index..];
    let end_index = rest
        .find(end)
        .ok_or_else(|| format!("missing marker '{end}'"))?;
    Ok(rest[..end_index].trim())
}

fn promoted_state_field_to_end<'a>(line: &'a str, start: &str) -> Result<&'a str, String> {
    let start_index = line
        .find(start)
        .ok_or_else(|| format!("missing marker '{start}'"))?
        + start.len();
    Ok(line[start_index..].trim())
}

fn parse_target_char(value: &str) -> Result<u8, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 1 {
        return Err(format!("target must be one byte, got '{value}'"));
    }
    if chat0_task_for_target(bytes[0]) == "unknown" {
        return Err(format!("unknown target '{value}'"));
    }
    Ok(bytes[0])
}

fn contains_ascii_word(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub(crate) fn chat0_task_for_target(target: u8) -> &'static str {
    BYTE_CONTEXT_TASKS
        .iter()
        .find_map(|(task, candidate)| (*candidate == target).then_some(*task))
        .unwrap_or("unknown")
}
