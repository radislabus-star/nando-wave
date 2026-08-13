use std::collections::BTreeMap;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::GroundedDecisionShadowCensorV1;

pub const S1C4_NATURAL_CENSUS_CURSOR_SCHEMA_V1: &str = "nando.s1c4-natural-census-cursor.v1";
pub const S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1: &str = "nando.s1c4-natural-census-report.v1";
pub const S1C4_WINDOW_REQUESTS_V1: u64 = 1024;
pub const S1C4_WINDOW_SECONDS_V1: u64 = 24 * 60 * 60;
pub const S1C4_QUIESCENCE_SECONDS_V1: u64 = 60;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum S1c4NaturalCensusStateV1 {
    Collecting,
    Quiescing,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum S1c4NaturalCensusVerdictV1 {
    Collecting,
    EmptyGoalSurface,
    EmptyAlternativeSurface,
    InsufficientLineages,
    Pass,
    Veto,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct S1c4NaturalCensusCursorV1 {
    pub schema: String,
    pub cursor_root_sha256: String,
    pub implementation_sha256: String,
    pub deployment_receipt_root_sha256: String,
    pub opportunity_bridge_root_sha256: String,
    pub opportunity_counter_started_after_sequence: u64,
    pub opportunity_start_sequence: u64,
    pub opportunity_start_request_ordinal: u64,
    pub opportunity_start_input_tokens: u64,
    pub classification_start_rows: u64,
    pub classification_start_root_sha256: String,
    pub precommit_start_rows: u64,
    pub precommit_prefix_root_sha256: String,
    pub selected_action_start_rows: u64,
    pub selected_action_prefix_root_sha256: String,
    pub satisfaction_start_rows: u64,
    pub satisfaction_prefix_root_sha256: String,
    pub queue_overflow_start: u64,
    pub writer_failures_start: u64,
    pub disconnected_start: u64,
    pub duplicate_rows_start: u64,
    pub false_accepts_start: u64,
    pub parity_failures_start: u64,
    pub opened_at_unix: u64,
    pub deadline_at_unix: u64,
    pub maximum_request_events: u64,
    pub quiescence_seconds: u64,
}

impl S1c4NaturalCensusCursorV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        implementation_sha256: String,
        deployment_receipt_root_sha256: String,
        opportunity_bridge_root_sha256: String,
        opportunity_counter_started_after_sequence: u64,
        opportunity_start_sequence: u64,
        opportunity_start_request_ordinal: u64,
        opportunity_start_input_tokens: u64,
        classification_start_rows: u64,
        classification_start_root_sha256: String,
        precommit_start_rows: u64,
        precommit_prefix_root_sha256: String,
        selected_action_start_rows: u64,
        selected_action_prefix_root_sha256: String,
        satisfaction_start_rows: u64,
        satisfaction_prefix_root_sha256: String,
        queue_overflow_start: u64,
        writer_failures_start: u64,
        disconnected_start: u64,
        duplicate_rows_start: u64,
        false_accepts_start: u64,
        parity_failures_start: u64,
        opened_at_unix: u64,
    ) -> Result<Self, &'static str> {
        let deadline_at_unix = opened_at_unix
            .checked_add(S1C4_WINDOW_SECONDS_V1)
            .ok_or("s1c4_cursor_deadline_overflow")?;
        let mut cursor = Self {
            schema: S1C4_NATURAL_CENSUS_CURSOR_SCHEMA_V1.to_owned(),
            cursor_root_sha256: String::new(),
            implementation_sha256,
            deployment_receipt_root_sha256,
            opportunity_bridge_root_sha256,
            opportunity_counter_started_after_sequence,
            opportunity_start_sequence,
            opportunity_start_request_ordinal,
            opportunity_start_input_tokens,
            classification_start_rows,
            classification_start_root_sha256,
            precommit_start_rows,
            precommit_prefix_root_sha256,
            selected_action_start_rows,
            selected_action_prefix_root_sha256,
            satisfaction_start_rows,
            satisfaction_prefix_root_sha256,
            queue_overflow_start,
            writer_failures_start,
            disconnected_start,
            duplicate_rows_start,
            false_accepts_start,
            parity_failures_start,
            opened_at_unix,
            deadline_at_unix,
            maximum_request_events: S1C4_WINDOW_REQUESTS_V1,
            quiescence_seconds: S1C4_QUIESCENCE_SECONDS_V1,
        };
        cursor.cursor_root_sha256 = cursor.expected_root()?;
        cursor.validate()?;
        Ok(cursor)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != S1C4_NATURAL_CENSUS_CURSOR_SCHEMA_V1
            || !valid_nonzero_sha256(&self.cursor_root_sha256)
            || !valid_nonzero_sha256(&self.implementation_sha256)
            || !valid_nonzero_sha256(&self.deployment_receipt_root_sha256)
            || !valid_nonzero_sha256(&self.opportunity_bridge_root_sha256)
            || !valid_nonzero_sha256(&self.classification_start_root_sha256)
            || !valid_nonzero_sha256(&self.precommit_prefix_root_sha256)
            || !valid_nonzero_sha256(&self.selected_action_prefix_root_sha256)
            || !valid_nonzero_sha256(&self.satisfaction_prefix_root_sha256)
            || self.opportunity_start_sequence <= self.opportunity_counter_started_after_sequence
            || self.opened_at_unix == 0
            || self.deadline_at_unix != self.opened_at_unix.saturating_add(S1C4_WINDOW_SECONDS_V1)
            || self.maximum_request_events != S1C4_WINDOW_REQUESTS_V1
            || self.quiescence_seconds != S1C4_QUIESCENCE_SECONDS_V1
            || self.expected_root()? != self.cursor_root_sha256
        {
            return Err("s1c4_natural_census_cursor_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        let mut digest = self.clone();
        digest.cursor_root_sha256.clear();
        canonical_json_sha256(&digest)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct S1c4NaturalCensusReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub cursor_root_sha256: String,
    pub state: S1c4NaturalCensusStateV1,
    pub verdict: S1c4NaturalCensusVerdictV1,
    pub blocker: String,
    pub generated_at_unix: u64,
    pub closes_at_unix: u64,
    pub quiescence_deadline_unix: u64,
    pub opportunity_end_sequence: u64,
    pub opportunity_end_request_ordinal: u64,
    pub opportunity_end_input_tokens: u64,
    pub denominator_requests: u64,
    pub denominator_input_tokens: u64,
    pub classified_requests: u64,
    pub goal_bound: u64,
    pub alternative_bearing: u64,
    pub decision_episodes: u64,
    pub satisfied_episodes: u64,
    pub distinct_decision_lineages: u64,
    pub censor_counts: BTreeMap<GroundedDecisionShadowCensorV1, u64>,
    pub classification_rows_total: u64,
    pub classification_last_root_sha256: String,
    pub queue_overflow: u64,
    pub writer_failures: u64,
    pub duplicate_rows: u64,
    pub false_accepts: u64,
    pub parity_failures: u64,
    pub source_complete: bool,
    pub exact_join_complete: bool,
    pub raw_payloads_persisted: bool,
    pub k2_open: bool,
    pub s2_started: bool,
    pub model_training_allowed: bool,
    pub package_activation_allowed: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl S1c4NaturalCensusReportV1 {
    pub fn seal(mut report: Self) -> Result<Self, &'static str> {
        report.schema = S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1.to_owned();
        report.report_root_sha256.clear();
        report.report_root_sha256 = report.expected_root()?;
        report.validate()?;
        Ok(report)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let censor_total = self
            .censor_counts
            .values()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .ok_or("s1c4_report_count_overflow")?;
        if self.schema != S1C4_NATURAL_CENSUS_REPORT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.report_root_sha256)
            || !valid_nonzero_sha256(&self.cursor_root_sha256)
            || !valid_nonzero_sha256(&self.classification_last_root_sha256)
            || self.generated_at_unix == 0
            || self.classified_requests != censor_total.saturating_add(self.decision_episodes)
            || self.satisfied_episodes > self.decision_episodes
            || self.distinct_decision_lineages > self.satisfied_episodes
            || self.goal_bound > self.classified_requests
            || self.alternative_bearing > self.goal_bound
            || self.raw_payloads_persisted
            || self.k2_open
            || self.s2_started
            || self.model_training_allowed
            || self.package_activation_allowed
            || self.authority_ready
            || self.phase_mutation_allowed
            || (self.state == S1c4NaturalCensusStateV1::Terminal
                && self.verdict == S1c4NaturalCensusVerdictV1::Collecting)
            || (self.state != S1c4NaturalCensusStateV1::Terminal
                && self.verdict != S1c4NaturalCensusVerdictV1::Collecting)
            || self.expected_root()? != self.report_root_sha256
        {
            return Err("s1c4_natural_census_report_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        let mut digest = self.clone();
        digest.report_root_sha256.clear();
        canonical_json_sha256(&digest)
    }
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn s1c4_terminal_verdict_v1(
    denominator_requests: u64,
    classified_requests: u64,
    goal_bound: u64,
    alternative_bearing: u64,
    satisfied_episodes: u64,
    distinct_decision_lineages: u64,
    missing_exact_goal: u64,
    veto: bool,
) -> (S1c4NaturalCensusVerdictV1, &'static str) {
    if veto || denominator_requests == 0 || classified_requests != denominator_requests {
        return (S1c4NaturalCensusVerdictV1::Veto, "evidence_integrity_veto");
    }
    if missing_exact_goal == denominator_requests {
        return (
            S1c4NaturalCensusVerdictV1::EmptyGoalSurface,
            "no_pre_action_exact_goal_in_frozen_window",
        );
    }
    if goal_bound > 0 && alternative_bearing == 0 {
        return (
            S1c4NaturalCensusVerdictV1::EmptyAlternativeSurface,
            "no_applicable_certified_k1_alternative_in_frozen_window",
        );
    }
    if satisfied_episodes >= 2 && distinct_decision_lineages >= 2 {
        return (
            S1c4NaturalCensusVerdictV1::Pass,
            "natural_grounded_decision_surface_observed",
        );
    }
    if satisfied_episodes > 0 {
        return (
            S1c4NaturalCensusVerdictV1::InsufficientLineages,
            "fewer_than_two_independent_decision_lineages",
        );
    }
    (
        S1c4NaturalCensusVerdictV1::Veto,
        "heterogeneous_unresolved_surface",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn cursor() -> S1c4NaturalCensusCursorV1 {
        S1c4NaturalCensusCursorV1::seal(
            root('1'),
            root('2'),
            root('3'),
            10,
            20,
            12,
            300,
            4,
            root('4'),
            5,
            root('5'),
            6,
            root('6'),
            7,
            root('7'),
            0,
            0,
            0,
            0,
            0,
            0,
            1_700_000_000,
        )
        .expect("cursor")
    }

    #[test]
    fn frozen_verdict_matrix_is_ordered_and_fail_closed() {
        assert_eq!(
            s1c4_terminal_verdict_v1(8, 8, 0, 0, 0, 0, 8, false).0,
            S1c4NaturalCensusVerdictV1::EmptyGoalSurface
        );
        assert_eq!(
            s1c4_terminal_verdict_v1(8, 8, 2, 0, 0, 0, 6, false).0,
            S1c4NaturalCensusVerdictV1::EmptyAlternativeSurface
        );
        assert_eq!(
            s1c4_terminal_verdict_v1(8, 8, 2, 2, 1, 1, 6, false).0,
            S1c4NaturalCensusVerdictV1::InsufficientLineages
        );
        assert_eq!(
            s1c4_terminal_verdict_v1(8, 8, 2, 2, 2, 2, 6, false).0,
            S1c4NaturalCensusVerdictV1::Pass
        );
        assert_eq!(
            s1c4_terminal_verdict_v1(8, 7, 2, 2, 2, 2, 5, false).0,
            S1c4NaturalCensusVerdictV1::Veto
        );
    }

    #[test]
    fn cursor_binds_counter_epoch_and_all_three_journal_prefixes() {
        let original = cursor();
        original.validate().expect("valid cursor");

        let mut changed_epoch = original.clone();
        changed_epoch.opportunity_counter_started_after_sequence = 11;
        assert_eq!(
            changed_epoch.validate(),
            Err("s1c4_natural_census_cursor_invalid")
        );

        let mut changed_prefix = original;
        changed_prefix.selected_action_prefix_root_sha256 = root('8');
        assert_eq!(
            changed_prefix.validate(),
            Err("s1c4_natural_census_cursor_invalid")
        );
    }
}
