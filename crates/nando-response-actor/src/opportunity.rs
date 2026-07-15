use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::TeacherTransition;

pub const OPPORTUNITY_BOARD_SCHEMA_V2: &str = "nando.opportunity-board.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReducibilityClass {
    CpuVerified,
    ExecutableCandidate,
    MissingDslPrimitive,
    MissingExternalVerifier,
    InsufficientRepetition,
    UnexploredMultiSource,
    AmbiguousPreActionState,
    NonDeterministicOrCreative,
    StaleOrInvalidEvidence,
    UnclassifiedBug,
}

impl ReducibilityClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuVerified => "CPU_VERIFIED",
            Self::ExecutableCandidate => "EXECUTABLE_CANDIDATE",
            Self::MissingDslPrimitive => "MISSING_DSL_PRIMITIVE",
            Self::MissingExternalVerifier => "MISSING_EXTERNAL_VERIFIER",
            Self::InsufficientRepetition => "INSUFFICIENT_REPETITION",
            Self::UnexploredMultiSource => "UNEXPLORED_MULTI_SOURCE",
            Self::AmbiguousPreActionState => "AMBIGUOUS_PRE_ACTION_STATE",
            Self::NonDeterministicOrCreative => "NON_DETERMINISTIC_OR_CREATIVE",
            Self::StaleOrInvalidEvidence => "STALE_OR_INVALID_EVIDENCE",
            Self::UnclassifiedBug => "UNCLASSIFIED_BUG",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpportunityBoardConfig {
    pub minimum_window_intents: usize,
    pub minimum_window_seconds: u64,
    pub maximum_window_intents: usize,
    pub required_m3_windows: usize,
}

impl Default for OpportunityBoardConfig {
    fn default() -> Self {
        Self {
            minimum_window_intents: 10_000,
            minimum_window_seconds: 24 * 60 * 60,
            maximum_window_intents: 1_000_000,
            required_m3_windows: 3,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct IntentOpportunity {
    input_tokens: u64,
    teacher_signature_sha256: Option<String>,
    class: ReducibilityClass,
    verifier_available: bool,
    observed_at_unix: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpportunityClassReport {
    pub intents: u64,
    pub input_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherOpportunityReport {
    pub teacher_signature_sha256: String,
    pub action_symbol: String,
    pub ordinary_intents: u64,
    pub ordinary_tokens: u64,
    pub verified_intents: u64,
    pub verified_tokens: u64,
    pub marginal_uncovered_tokens: u64,
    pub exact_checks: u64,
    pub search_slices: u64,
    pub search_cost: u64,
    pub hot_bytes: u64,
    pub estimated_safe_accept_milli: u64,
    pub verifier_availability_milli: u64,
    pub transfer_probability_milli: u64,
    pub expected_verified_value_micro: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct M3WindowReport {
    pub started_at_unix: u64,
    pub ended_at_unix: u64,
    pub ordinary_intents: u64,
    pub ordinary_tokens: u64,
    pub verified_intents: u64,
    pub verified_tokens: u64,
    pub verified_token_share_milli: u64,
    pub false_accepts: u64,
    pub parity_failures: u64,
    pub pass: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OpportunityBoardReport {
    pub schema: String,
    pub window_started_at_unix: u64,
    pub ordinary_intents: u64,
    pub ordinary_tokens: u64,
    pub verified_intents: u64,
    pub verified_tokens: u64,
    pub verified_token_share_milli: u64,
    pub classified_intents: u64,
    pub classification_identity_holds: bool,
    pub capacity_overflow: u64,
    pub false_accepts: u64,
    pub parity_failures: u64,
    pub classes: BTreeMap<String, OpportunityClassReport>,
    pub teacher_programs: Vec<TeacherOpportunityReport>,
    pub completed_m3_windows: Vec<M3WindowReport>,
    pub product_m3_pass: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpportunityBoard {
    config: OpportunityBoardConfig,
    window_started_at_unix: u64,
    intents: BTreeMap<String, IntentOpportunity>,
    teacher_programs: BTreeMap<String, TeacherOpportunityReport>,
    completed_windows: Vec<M3WindowReport>,
    capacity_overflow: u64,
    false_accepts: u64,
    parity_failures: u64,
    #[serde(default)]
    false_accept_intents: BTreeSet<String>,
    #[serde(default)]
    parity_failure_intents: BTreeSet<String>,
}

pub(crate) struct SearchObservation<'a> {
    pub exact_checks: u64,
    pub search_slices: u64,
    pub hot_bytes: u64,
    pub safe_accept_milli: u64,
    pub transfer_probability_milli: u64,
    pub blocker: Option<&'a str>,
}

impl OpportunityBoard {
    #[must_use]
    pub fn new(config: OpportunityBoardConfig, now_unix: u64) -> Self {
        Self {
            config,
            window_started_at_unix: now_unix,
            intents: BTreeMap::new(),
            teacher_programs: BTreeMap::new(),
            completed_windows: Vec::new(),
            capacity_overflow: 0,
            false_accepts: 0,
            parity_failures: 0,
            false_accept_intents: BTreeSet::new(),
            parity_failure_intents: BTreeSet::new(),
        }
    }

    pub fn observe_request(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        ordinary: bool,
        now_unix: u64,
    ) {
        if !ordinary || self.intents.contains_key(intent_sha256) {
            return;
        }
        if self.intents.len() >= self.config.maximum_window_intents {
            self.capacity_overflow = self.capacity_overflow.saturating_add(1);
            return;
        }
        self.intents.insert(
            intent_sha256.to_owned(),
            IntentOpportunity {
                input_tokens,
                teacher_signature_sha256: None,
                class: ReducibilityClass::InsufficientRepetition,
                verifier_available: false,
                observed_at_unix: now_unix,
            },
        );
    }

    pub fn observe_transition(&mut self, transition: &TeacherTransition) {
        let Some(economics) = transition.economics.as_ref() else {
            return;
        };
        if economics.controlled || economics.replay || !economics.dedupe_eligible {
            return;
        }
        let now_unix = transition.outcome.completed_at_unix_nanos / 1_000_000_000;
        if economics.ordinary {
            self.observe_request(
                &transition.before.client_intent_id_sha256,
                economics.exact_input_tokens,
                true,
                now_unix,
            );
        }
        let signature = transition.outcome.action.signature_sha256.clone();
        let verifier_available = transition.outcome.verifier.accepted;
        if economics.ordinary
            && let Some(intent) = self
                .intents
                .get_mut(&transition.before.client_intent_id_sha256)
        {
            intent.teacher_signature_sha256 = Some(signature.clone());
            intent.verifier_available = verifier_available;
            intent.class = if transition.before.contains_teacher_atoms() {
                ReducibilityClass::UnclassifiedBug
            } else if !verifier_available {
                ReducibilityClass::MissingExternalVerifier
            } else {
                ReducibilityClass::ExecutableCandidate
            };
        }
        let aggregate = self
            .teacher_programs
            .entry(signature.clone())
            .or_insert_with(|| TeacherOpportunityReport {
                teacher_signature_sha256: signature,
                action_symbol: transition.outcome.action.action_symbol.clone(),
                estimated_safe_accept_milli: 1_000,
                verifier_availability_milli: u64::from(verifier_available) * 1_000,
                transfer_probability_milli: 500,
                ..TeacherOpportunityReport::default()
            });
        aggregate.ordinary_intents = aggregate.ordinary_intents.saturating_add(1);
        aggregate.ordinary_tokens = aggregate
            .ordinary_tokens
            .saturating_add(economics.exact_input_tokens);
        aggregate.marginal_uncovered_tokens = aggregate
            .marginal_uncovered_tokens
            .saturating_add(economics.exact_input_tokens);
        recompute_priority(aggregate);
    }

    pub fn classify_intent(
        &mut self,
        intent_sha256: &str,
        class: ReducibilityClass,
        blocker: Option<&str>,
    ) {
        let Some(intent) = self.intents.get_mut(intent_sha256) else {
            return;
        };
        intent.class = class;
        if let Some(signature) = intent.teacher_signature_sha256.as_ref()
            && let Some(program) = self.teacher_programs.get_mut(signature)
        {
            program.blocker = blocker.map(str::to_owned);
            recompute_priority(program);
        }
    }

    pub(crate) fn observe_search(
        &mut self,
        teacher_signature_sha256: &str,
        observation: SearchObservation<'_>,
    ) {
        let Some(program) = self.teacher_programs.get_mut(teacher_signature_sha256) else {
            return;
        };
        program.exact_checks = observation.exact_checks;
        program.search_slices = observation.search_slices;
        program.search_cost = observation
            .exact_checks
            .max(observation.search_slices)
            .max(1);
        program.hot_bytes = observation.hot_bytes;
        program.estimated_safe_accept_milli = observation.safe_accept_milli.min(1_000);
        program.transfer_probability_milli = observation.transfer_probability_milli.min(1_000);
        program.blocker = observation.blocker.map(str::to_owned);
        recompute_priority(program);
    }

    pub fn mark_verified(&mut self, intent_sha256: &str) {
        let Some(intent) = self.intents.get_mut(intent_sha256) else {
            return;
        };
        if intent.class == ReducibilityClass::CpuVerified {
            return;
        }
        intent.class = ReducibilityClass::CpuVerified;
        if let Some(signature) = intent.teacher_signature_sha256.as_ref()
            && let Some(program) = self.teacher_programs.get_mut(signature)
        {
            program.verified_intents = program.verified_intents.saturating_add(1);
            program.verified_tokens = program.verified_tokens.saturating_add(intent.input_tokens);
            program.marginal_uncovered_tokens = program
                .marginal_uncovered_tokens
                .saturating_sub(intent.input_tokens);
            recompute_priority(program);
        }
    }

    pub fn mark_false_accept(&mut self, intent_sha256: &str) {
        if self.false_accept_intents.insert(intent_sha256.to_owned()) {
            self.false_accepts = self.false_accepts.saturating_add(1);
        }
    }

    pub fn mark_parity_failure(&mut self, intent_sha256: &str) {
        if self.parity_failure_intents.insert(intent_sha256.to_owned()) {
            self.parity_failures = self.parity_failures.saturating_add(1);
        }
    }

    #[must_use]
    pub const fn authority_safe(&self) -> bool {
        self.false_accepts == 0 && self.parity_failures == 0
    }

    pub fn try_roll_window(&mut self, now_unix: u64) -> bool {
        if self.intents.len() < self.config.minimum_window_intents
            || now_unix.saturating_sub(self.window_started_at_unix)
                < self.config.minimum_window_seconds
        {
            return false;
        }
        let summary = self.current_window_report(now_unix);
        self.completed_windows.push(summary);
        if self.completed_windows.len() > self.config.required_m3_windows {
            let remove = self
                .completed_windows
                .len()
                .saturating_sub(self.config.required_m3_windows);
            self.completed_windows.drain(..remove);
        }
        self.window_started_at_unix = now_unix;
        self.intents.clear();
        self.teacher_programs.clear();
        self.capacity_overflow = 0;
        self.false_accepts = 0;
        self.parity_failures = 0;
        self.false_accept_intents.clear();
        self.parity_failure_intents.clear();
        true
    }

    #[must_use]
    pub fn report(&self, now_unix: u64) -> OpportunityBoardReport {
        let current = self.current_window_report(now_unix);
        let mut classes = BTreeMap::<String, OpportunityClassReport>::new();
        for intent in self.intents.values() {
            let class = classes.entry(intent.class.as_str().to_owned()).or_default();
            class.intents = class.intents.saturating_add(1);
            class.input_tokens = class.input_tokens.saturating_add(intent.input_tokens);
        }
        let mut teacher_programs = self.teacher_programs.values().cloned().collect::<Vec<_>>();
        teacher_programs.sort_by(|left, right| {
            right
                .expected_verified_value_micro
                .cmp(&left.expected_verified_value_micro)
                .then_with(|| {
                    right
                        .marginal_uncovered_tokens
                        .cmp(&left.marginal_uncovered_tokens)
                })
                .then_with(|| {
                    left.teacher_signature_sha256
                        .cmp(&right.teacher_signature_sha256)
                })
        });
        let completed_m3_windows = self.completed_windows.clone();
        let product_m3_pass = completed_m3_windows.len() >= self.config.required_m3_windows
            && completed_m3_windows.iter().all(|window| window.pass);
        OpportunityBoardReport {
            schema: OPPORTUNITY_BOARD_SCHEMA_V2.to_owned(),
            window_started_at_unix: self.window_started_at_unix,
            ordinary_intents: current.ordinary_intents,
            ordinary_tokens: current.ordinary_tokens,
            verified_intents: current.verified_intents,
            verified_tokens: current.verified_tokens,
            verified_token_share_milli: current.verified_token_share_milli,
            classified_intents: self.intents.len() as u64,
            classification_identity_holds: self.capacity_overflow == 0
                && current.ordinary_intents == self.intents.len() as u64,
            capacity_overflow: self.capacity_overflow,
            false_accepts: self.false_accepts,
            parity_failures: self.parity_failures,
            classes,
            teacher_programs,
            completed_m3_windows,
            product_m3_pass,
        }
    }

    fn current_window_report(&self, now_unix: u64) -> M3WindowReport {
        let ordinary_intents = self.intents.len() as u64;
        let ordinary_tokens = self
            .intents
            .values()
            .map(|intent| intent.input_tokens)
            .sum::<u64>();
        let verified_intents = self
            .intents
            .values()
            .filter(|intent| intent.class == ReducibilityClass::CpuVerified)
            .count() as u64;
        let verified_tokens = self
            .intents
            .values()
            .filter(|intent| intent.class == ReducibilityClass::CpuVerified)
            .map(|intent| intent.input_tokens)
            .sum::<u64>();
        let verified_token_share_milli = ratio_milli(verified_tokens, ordinary_tokens);
        let mature = ordinary_intents >= self.config.minimum_window_intents as u64
            && now_unix.saturating_sub(self.window_started_at_unix)
                >= self.config.minimum_window_seconds;
        M3WindowReport {
            started_at_unix: self.window_started_at_unix,
            ended_at_unix: now_unix,
            ordinary_intents,
            ordinary_tokens,
            verified_intents,
            verified_tokens,
            verified_token_share_milli,
            false_accepts: self.false_accepts,
            parity_failures: self.parity_failures,
            pass: mature
                && verified_token_share_milli >= 500
                && self.false_accepts == 0
                && self.parity_failures == 0
                && self.capacity_overflow == 0,
        }
    }
}

impl Default for OpportunityBoard {
    fn default() -> Self {
        Self::new(OpportunityBoardConfig::default(), 0)
    }
}

fn recompute_priority(program: &mut TeacherOpportunityReport) {
    let numerator = u128::from(program.marginal_uncovered_tokens)
        .saturating_mul(u128::from(program.estimated_safe_accept_milli))
        .saturating_mul(u128::from(program.verifier_availability_milli))
        .saturating_mul(u128::from(program.transfer_probability_milli));
    let denominator = u128::from(
        program
            .search_cost
            .saturating_add(program.hot_bytes)
            .saturating_add(1),
    )
    .saturating_mul(1_000_000);
    program.expected_verified_value_micro =
        u64::try_from(numerator / denominator).unwrap_or(u64::MAX);
}

fn ratio_milli(numerator: u64, denominator: u64) -> u64 {
    numerator
        .saturating_mul(1_000)
        .checked_div(denominator)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_safety_events_are_idempotent_across_checkpoint_replay() {
        let mut board = OpportunityBoard::default();
        board.mark_false_accept("intent-a");
        board.mark_false_accept("intent-a");
        board.mark_parity_failure("intent-b");
        board.mark_parity_failure("intent-b");
        let checkpoint = serde_cbor::to_vec(&board).expect("encode checkpoint");
        let mut restored: OpportunityBoard =
            serde_cbor::from_slice(&checkpoint).expect("decode checkpoint");
        restored.mark_false_accept("intent-a");
        restored.mark_parity_failure("intent-b");
        let report = restored.report(1);
        assert_eq!(report.false_accepts, 1);
        assert_eq!(report.parity_failures, 1);
    }
}
