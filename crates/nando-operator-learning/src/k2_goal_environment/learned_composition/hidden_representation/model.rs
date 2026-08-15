use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionExactGoalV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
    require_composition_root_v1,
};

pub const K2_REPRESENTATION_ACTIONS_V1: usize = 7;
pub const K2_REPRESENTATION_MAX_DEPTH_V1: u64 = 6;
pub const K2_REPRESENTATION_COMPLETE_PROGRAMS_V1: u64 = 8_659;
pub const K2_REPRESENTATION_FEATURES_V1: usize = 14;
pub const K2_REPRESENTATION_HIDDEN_UNITS_V1: usize = 8;
pub const K2_REPRESENTATION_BEAM_WIDTH_V1: u64 = 3;
pub const K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1: u64 = 67;
pub const K2_REPRESENTATION_TRAIN_TASKS_V1: usize = 6;
pub const K2_REPRESENTATION_CONFIRM_TASKS_V1: usize = 2;
pub const K2_REPRESENTATION_TRAIN_EPOCHS_V1: u64 = 256;
pub const K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1: usize = 32 * 1024 * 1024;
pub const K2_REPRESENTATION_FEATURE_SCALE_V1: i64 = 1_000;
pub const K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_PASS_V1: &str =
    "K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_PASS";

pub const K2_REPRESENTATION_TASK_SCHEMA_V1: &str = "nando.k2-representation-task.v1";
pub const K2_REPRESENTATION_ACTION_LAW_SCHEMA_V1: &str = "nando.k2-representation-action-law.v1";
pub const K2_REPRESENTATION_PROGRAM_SCHEMA_V1: &str = "nando.k2-representation-program.v1";
pub const K2_REPRESENTATION_BASELINE_REQUEST_SCHEMA_V1: &str =
    "nando.k2-representation-baseline-request.v1";
pub const K2_REPRESENTATION_BASELINE_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-representation-baseline-outcome.v1";
pub const K2_REPRESENTATION_FEATURE_SCHEMA_V1: &str = "nando.k2-representation-feature-vector.v1";
pub const K2_REPRESENTATION_TRAINING_CORPUS_SCHEMA_V1: &str =
    "nando.k2-representation-training-corpus.v1";
pub const K2_REPRESENTATION_TRAINER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-representation-trainer-request.v1";
pub const K2_REPRESENTATION_MODEL_SCHEMA_V1: &str =
    "nando.k2-representation-meaning-policy-snapshot.v1";
pub const K2_REPRESENTATION_POLICY_REQUEST_SCHEMA_V1: &str =
    "nando.k2-representation-policy-request.v1";
pub const K2_REPRESENTATION_POLICY_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-representation-policy-outcome.v1";
pub const K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1: &str =
    "nando.k2-representation-verifier-request.v1";
pub const K2_REPRESENTATION_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-representation-verification.v1";
pub const K2_REPRESENTATION_CONFIRM_SEAL_SCHEMA_V1: &str =
    "nando.k2-representation-confirm-seal.v1";
pub const K2_REPRESENTATION_PROCESS_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-representation-process-receipt.v1";

pub const K2_REPRESENTATION_FEATURE_NAMES_V1: [&str; K2_REPRESENTATION_FEATURES_V1] = [
    "bias",
    "action_applicable",
    "action_is_copy",
    "action_is_remove",
    "write_path_required_by_goal",
    "write_value_matches_goal",
    "write_path_currently_differs",
    "write_path_read_by_remaining_action",
    "read_path_missing_but_writable",
    "remove_path_absent_from_goal",
    "remove_path_required_by_goal",
    "remove_path_read_by_remaining_action",
    "write_path_nuisance",
    "remaining_depth_fraction",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationActionLawV1 {
    pub schema: String,
    pub action_id_sha256: String,
    pub effect: K2CompositionLearnedEffectV1,
    pub support_root_sha256: String,
    pub law_root_sha256: String,
}

impl K2RepresentationActionLawV1 {
    pub fn seal(
        action_id_sha256: String,
        effect: K2CompositionLearnedEffectV1,
        support_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let law_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_ACTION_LAW_SCHEMA_V1,
            &action_id_sha256,
            &effect,
            &support_root_sha256,
        ))?;
        let law = Self {
            schema: K2_REPRESENTATION_ACTION_LAW_SCHEMA_V1.to_owned(),
            action_id_sha256,
            effect,
            support_root_sha256,
            law_root_sha256,
        };
        law.validate()?;
        Ok(law)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.action_id_sha256)?;
        require_composition_root_v1(&self.support_root_sha256)?;
        self.effect.validate()?;
        let expected = composition_root_v1(&(
            K2_REPRESENTATION_ACTION_LAW_SCHEMA_V1,
            &self.action_id_sha256,
            &self.effect,
            &self.support_root_sha256,
        ))?;
        if self.schema != K2_REPRESENTATION_ACTION_LAW_SCHEMA_V1 || expected != self.law_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_action_law_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationTaskV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub laws: Vec<K2RepresentationActionLawV1>,
    pub initial: K2CompositionTreeManifestV1,
    pub goal: K2CompositionExactGoalV1,
    pub maximum_depth: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub task_root_sha256: String,
}

impl K2RepresentationTaskV1 {
    pub fn seal(
        experiment_id_sha256: String,
        mut laws: Vec<K2RepresentationActionLawV1>,
        initial: K2CompositionTreeManifestV1,
        goal: K2CompositionExactGoalV1,
    ) -> K2CompositionResultV1<Self> {
        laws.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
        let maximum_depth = K2_REPRESENTATION_MAX_DEPTH_V1;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let task_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_TASK_SCHEMA_V1,
            &experiment_id_sha256,
            &laws,
            &initial,
            &goal,
            maximum_depth,
            &authority,
        ))?;
        let task = Self {
            schema: K2_REPRESENTATION_TASK_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            laws,
            initial,
            goal,
            maximum_depth,
            authority,
            task_root_sha256,
        };
        task.validate()?;
        Ok(task)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        self.initial.validate()?;
        self.goal.validate()?;
        self.authority.validate()?;
        if self.schema != K2_REPRESENTATION_TASK_SCHEMA_V1
            || self.laws.len() != K2_REPRESENTATION_ACTIONS_V1
            || self.maximum_depth != K2_REPRESENTATION_MAX_DEPTH_V1
            || self
                .laws
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_task_shape_invalid",
            ));
        }
        for law in &self.laws {
            law.validate()?;
        }
        let expected = composition_root_v1(&(
            K2_REPRESENTATION_TASK_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.laws,
            &self.initial,
            &self.goal,
            self.maximum_depth,
            &self.authority,
        ))?;
        if expected != self.task_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_task_root_mismatch",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn law(&self, action_id: &str) -> Option<&K2RepresentationActionLawV1> {
        self.laws
            .iter()
            .find(|law| law.action_id_sha256 == action_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationProgramV1 {
    pub schema: String,
    pub action_ids_sha256: Vec<String>,
    pub program_root_sha256: String,
}

impl K2RepresentationProgramV1 {
    pub fn seal(action_ids_sha256: Vec<String>) -> K2CompositionResultV1<Self> {
        if action_ids_sha256.is_empty()
            || action_ids_sha256.len() > K2_REPRESENTATION_MAX_DEPTH_V1 as usize
            || action_ids_sha256.iter().collect::<BTreeSet<_>>().len() != action_ids_sha256.len()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_program_invalid",
            ));
        }
        for action_id in &action_ids_sha256 {
            require_composition_root_v1(action_id)?;
        }
        let program_root_sha256 =
            composition_root_v1(&(K2_REPRESENTATION_PROGRAM_SCHEMA_V1, &action_ids_sha256))?;
        Ok(Self {
            schema: K2_REPRESENTATION_PROGRAM_SCHEMA_V1.to_owned(),
            action_ids_sha256,
            program_root_sha256,
        })
    }

    #[must_use]
    pub fn depth(&self) -> u64 {
        self.action_ids_sha256.len() as u64
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationBaselineRequestV1 {
    pub schema: String,
    pub baseline_executable_sha256: String,
    pub task: K2RepresentationTaskV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2RepresentationBaselineRequestV1 {
    pub fn seal(
        baseline_executable_sha256: String,
        task: K2RepresentationTaskV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_BASELINE_REQUEST_SCHEMA_V1,
            &baseline_executable_sha256,
            &task,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_BASELINE_REQUEST_SCHEMA_V1.to_owned(),
            baseline_executable_sha256,
            task,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.task.validate()?;
        self.authority.validate()?;
        require_composition_root_v1(&self.baseline_executable_sha256)?;
        let expected = composition_root_v1(&(
            K2_REPRESENTATION_BASELINE_REQUEST_SCHEMA_V1,
            &self.baseline_executable_sha256,
            &self.task,
            &self.authority,
        ))?;
        if self.schema != K2_REPRESENTATION_BASELINE_REQUEST_SCHEMA_V1
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_baseline_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationBaselineOutcomeV1 {
    pub schema: String,
    pub task_root_sha256: String,
    pub request_root_sha256: String,
    pub complete_programs: u64,
    pub valid_programs: u64,
    pub inapplicable_programs: u64,
    pub candidate_set_root_sha256: String,
    pub minimum_satisfying_depth: u64,
    pub minimum_satisfying_programs: Vec<K2RepresentationProgramV1>,
    pub satisfying_strict_prefixes: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2RepresentationBaselineOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_BASELINE_OUTCOME_SCHEMA_V1,
            &self.task_root_sha256,
            &self.request_root_sha256,
            self.complete_programs,
            self.valid_programs,
            self.inapplicable_programs,
            &self.candidate_set_root_sha256,
            self.minimum_satisfying_depth,
            &self.minimum_satisfying_programs,
            self.satisfying_strict_prefixes,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationFeatureVectorV1 {
    pub schema: String,
    pub values: Vec<i64>,
    pub feature_root_sha256: String,
}

impl K2RepresentationFeatureVectorV1 {
    pub fn seal(values: Vec<i64>) -> K2CompositionResultV1<Self> {
        if values.len() != K2_REPRESENTATION_FEATURES_V1
            || values.iter().any(|value| value.abs() > 1_000)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_feature_shape_invalid",
            ));
        }
        let feature_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_FEATURE_SCHEMA_V1,
            K2_REPRESENTATION_FEATURE_NAMES_V1,
            &values,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_FEATURE_SCHEMA_V1.to_owned(),
            values,
            feature_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationTrainingRowV1 {
    pub features: K2RepresentationFeatureVectorV1,
    pub positive_continuation: bool,
    pub row_root_sha256: String,
}

impl K2RepresentationTrainingRowV1 {
    pub fn seal(
        features: K2RepresentationFeatureVectorV1,
        positive_continuation: bool,
    ) -> K2CompositionResultV1<Self> {
        let row_root_sha256 = composition_root_v1(&(
            "nando.k2-representation-training-row.v1",
            &features,
            positive_continuation,
        ))?;
        Ok(Self {
            features,
            positive_continuation,
            row_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationDecisionGroupV1 {
    pub rows: Vec<K2RepresentationTrainingRowV1>,
    pub group_root_sha256: String,
}

impl K2RepresentationDecisionGroupV1 {
    pub fn seal(mut rows: Vec<K2RepresentationTrainingRowV1>) -> K2CompositionResultV1<Self> {
        rows.sort_by(|left, right| left.row_root_sha256.cmp(&right.row_root_sha256));
        if rows.len() < 2
            || !rows.iter().any(|row| row.positive_continuation)
            || !rows.iter().any(|row| !row.positive_continuation)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_decision_group_invalid",
            ));
        }
        let group_root_sha256 =
            composition_root_v1(&("nando.k2-representation-decision-group.v1", &rows))?;
        Ok(Self {
            rows,
            group_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationTrainingCorpusV1 {
    pub schema: String,
    pub feature_language_root_sha256: String,
    pub train_baseline_roots_sha256: Vec<String>,
    pub groups: Vec<K2RepresentationDecisionGroupV1>,
    pub pair_count: u64,
    pub corpus_root_sha256: String,
}

impl K2RepresentationTrainingCorpusV1 {
    pub fn seal(
        mut train_baseline_roots_sha256: Vec<String>,
        mut groups: Vec<K2RepresentationDecisionGroupV1>,
    ) -> K2CompositionResultV1<Self> {
        train_baseline_roots_sha256.sort();
        groups.sort_by(|left, right| left.group_root_sha256.cmp(&right.group_root_sha256));
        let feature_language_root_sha256 = feature_language_root_v1()?;
        let pair_count = groups
            .iter()
            .map(|group| {
                let positives = group
                    .rows
                    .iter()
                    .filter(|row| row.positive_continuation)
                    .count() as u64;
                positives * (group.rows.len() as u64 - positives)
            })
            .sum();
        if train_baseline_roots_sha256.len() != K2_REPRESENTATION_TRAIN_TASKS_V1
            || groups.is_empty()
            || pair_count == 0
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_training_corpus_invalid",
            ));
        }
        let corpus_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_TRAINING_CORPUS_SCHEMA_V1,
            &feature_language_root_sha256,
            &train_baseline_roots_sha256,
            &groups,
            pair_count,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_TRAINING_CORPUS_SCHEMA_V1.to_owned(),
            feature_language_root_sha256,
            train_baseline_roots_sha256,
            groups,
            pair_count,
            corpus_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationTrainerRequestV1 {
    pub schema: String,
    pub trainer_executable_sha256: String,
    pub corpus: K2RepresentationTrainingCorpusV1,
    pub epochs: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2RepresentationTrainerRequestV1 {
    pub fn seal(
        trainer_executable_sha256: String,
        corpus: K2RepresentationTrainingCorpusV1,
    ) -> K2CompositionResultV1<Self> {
        let epochs = K2_REPRESENTATION_TRAIN_EPOCHS_V1;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_TRAINER_REQUEST_SCHEMA_V1,
            &trainer_executable_sha256,
            &corpus,
            epochs,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_TRAINER_REQUEST_SCHEMA_V1.to_owned(),
            trainer_executable_sha256,
            corpus,
            epochs,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.authority.validate()?;
        require_composition_root_v1(&self.trainer_executable_sha256)?;
        let expected = composition_root_v1(&(
            K2_REPRESENTATION_TRAINER_REQUEST_SCHEMA_V1,
            &self.trainer_executable_sha256,
            &self.corpus,
            self.epochs,
            &self.authority,
        ))?;
        if self.schema != K2_REPRESENTATION_TRAINER_REQUEST_SCHEMA_V1
            || self.epochs != K2_REPRESENTATION_TRAIN_EPOCHS_V1
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_trainer_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2MeaningPolicySnapshotV1 {
    pub schema: String,
    pub trainer_executable_sha256: String,
    pub trainer_request_root_sha256: String,
    pub corpus_root_sha256: String,
    pub feature_language_root_sha256: String,
    pub encoder_weights: Vec<Vec<i64>>,
    pub output_weights: Vec<i64>,
    pub epochs: u64,
    pub update_count: u64,
    pub training_pairs: u64,
    pub correctly_ranked_pairs: u64,
    pub nonzero_parameters: u64,
    pub parameter_l1: u64,
    pub control_variant: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub model_root_sha256: String,
}

impl K2MeaningPolicySnapshotV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.nonzero_parameters = self
            .encoder_weights
            .iter()
            .flatten()
            .chain(self.output_weights.iter())
            .filter(|weight| **weight != 0)
            .count() as u64;
        self.parameter_l1 = self
            .encoder_weights
            .iter()
            .flatten()
            .chain(self.output_weights.iter())
            .map(|weight| weight.unsigned_abs())
            .sum();
        self.model_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_MODEL_SCHEMA_V1,
            &self.trainer_executable_sha256,
            &self.trainer_request_root_sha256,
            &self.corpus_root_sha256,
            &self.feature_language_root_sha256,
            &self.encoder_weights,
            &self.output_weights,
            self.epochs,
            self.update_count,
            self.training_pairs,
            self.correctly_ranked_pairs,
            self.nonzero_parameters,
            self.parameter_l1,
            &self.control_variant,
            &self.authority,
        ))?;
        Ok(())
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.authority.validate()?;
        if self.schema != K2_REPRESENTATION_MODEL_SCHEMA_V1
            || self.encoder_weights.len() != K2_REPRESENTATION_HIDDEN_UNITS_V1
            || self
                .encoder_weights
                .iter()
                .any(|row| row.len() != K2_REPRESENTATION_FEATURES_V1)
            || self.output_weights.len() != K2_REPRESENTATION_HIDDEN_UNITS_V1
            || self.epochs != K2_REPRESENTATION_TRAIN_EPOCHS_V1
            || self.training_pairs == 0
            || self.correctly_ranked_pairs > self.training_pairs
            || self
                .encoder_weights
                .iter()
                .flatten()
                .chain(self.output_weights.iter())
                .any(|weight| weight.abs() > 1_000_000)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_model_shape_invalid",
            ));
        }
        let mut resealed = self.clone();
        resealed.reseal()?;
        if resealed.model_root_sha256 != self.model_root_sha256
            || resealed.nonzero_parameters != self.nonzero_parameters
            || resealed.parameter_l1 != self.parameter_l1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_model_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationPolicyRequestV1 {
    pub schema: String,
    pub policy_executable_sha256: String,
    pub model: K2MeaningPolicySnapshotV1,
    pub task: K2RepresentationTaskV1,
    pub beam_width: u64,
    pub maximum_action_evaluations: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2RepresentationPolicyRequestV1 {
    pub fn seal(
        policy_executable_sha256: String,
        model: K2MeaningPolicySnapshotV1,
        task: K2RepresentationTaskV1,
    ) -> K2CompositionResultV1<Self> {
        Self::seal_with_budget(
            policy_executable_sha256,
            model,
            task,
            K2_REPRESENTATION_BEAM_WIDTH_V1,
            K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1,
        )
    }

    pub fn seal_with_budget(
        policy_executable_sha256: String,
        model: K2MeaningPolicySnapshotV1,
        task: K2RepresentationTaskV1,
        beam_width: u64,
        maximum_action_evaluations: u64,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_POLICY_REQUEST_SCHEMA_V1,
            &policy_executable_sha256,
            &model,
            &task,
            beam_width,
            maximum_action_evaluations,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_POLICY_REQUEST_SCHEMA_V1.to_owned(),
            policy_executable_sha256,
            model,
            task,
            beam_width,
            maximum_action_evaluations,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.model.validate()?;
        self.task.validate()?;
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_REPRESENTATION_POLICY_REQUEST_SCHEMA_V1,
            &self.policy_executable_sha256,
            &self.model,
            &self.task,
            self.beam_width,
            self.maximum_action_evaluations,
            &self.authority,
        ))?;
        if self.schema != K2_REPRESENTATION_POLICY_REQUEST_SCHEMA_V1
            || self.beam_width == 0
            || self.beam_width > K2_REPRESENTATION_BEAM_WIDTH_V1
            || self.maximum_action_evaluations > K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_policy_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationPolicyTraceV1 {
    pub depth: u64,
    pub prefix_program_root_sha256: Option<String>,
    pub action_id_sha256: String,
    pub features: K2RepresentationFeatureVectorV1,
    pub hidden: Vec<i64>,
    pub action_score: i64,
    pub cumulative_score: i64,
    pub applicable: bool,
    pub resulting_program_root_sha256: String,
    pub resulting_tree_root_sha256: Option<String>,
    pub trace_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationBeamLayerV1 {
    pub depth: u64,
    pub retained_program_roots_sha256: Vec<String>,
    pub layer_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationPolicyOutcomeV1 {
    pub schema: String,
    pub task_root_sha256: String,
    pub request_root_sha256: String,
    pub selected_program: Option<K2RepresentationProgramV1>,
    pub selected_terminal: Option<K2CompositionTreeManifestV1>,
    pub exact_goal_satisfied: bool,
    pub action_evaluations: u64,
    pub exact_score_ties: u64,
    pub trace: Vec<K2RepresentationPolicyTraceV1>,
    pub layers: Vec<K2RepresentationBeamLayerV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2RepresentationPolicyOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_POLICY_OUTCOME_SCHEMA_V1,
            &self.task_root_sha256,
            &self.request_root_sha256,
            &self.selected_program,
            &self.selected_terminal,
            self.exact_goal_satisfied,
            self.action_evaluations,
            self.exact_score_ties,
            &self.trace,
            &self.layers,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationVerifierRequestV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub policy_request: K2RepresentationPolicyRequestV1,
    pub policy_outcome: K2RepresentationPolicyOutcomeV1,
    pub baseline_outcome: K2RepresentationBaselineOutcomeV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2RepresentationVerifierRequestV1 {
    pub fn seal(
        verifier_executable_sha256: String,
        policy_request: K2RepresentationPolicyRequestV1,
        policy_outcome: K2RepresentationPolicyOutcomeV1,
        baseline_outcome: K2RepresentationBaselineOutcomeV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1,
            &verifier_executable_sha256,
            &policy_request,
            &policy_outcome,
            &baseline_outcome,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_VERIFIER_REQUEST_SCHEMA_V1.to_owned(),
            verifier_executable_sha256,
            policy_request,
            policy_outcome,
            baseline_outcome,
            authority,
            request_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationVerificationReceiptV1 {
    pub schema: String,
    pub task_root_sha256: String,
    pub policy_outcome_root_sha256: String,
    pub baseline_outcome_root_sha256: String,
    pub independently_reconstructed_programs: u64,
    pub independently_reconstructed_evaluations: u64,
    pub selected_is_minimum_satisfying: bool,
    pub exact_goal_satisfied: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub verification_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationConfirmSealReceiptV1 {
    pub schema: String,
    pub model_root_sha256: String,
    pub train_task_roots_sha256: Vec<String>,
    pub confirm_task_roots_sha256: Vec<String>,
    pub root_intersections: u64,
    pub trainer_byte_leaks: u64,
    pub receipt_root_sha256: String,
}

impl K2RepresentationConfirmSealReceiptV1 {
    pub fn seal(
        model_root_sha256: String,
        train_tasks: &[K2RepresentationTaskV1],
        confirm_tasks: &[K2RepresentationTaskV1],
        trainer_bytes: &[u8],
    ) -> K2CompositionResultV1<Self> {
        let mut train_task_roots_sha256 = train_tasks
            .iter()
            .map(|task| task.task_root_sha256.clone())
            .collect::<Vec<_>>();
        let mut confirm_task_roots_sha256 = confirm_tasks
            .iter()
            .map(|task| task.task_root_sha256.clone())
            .collect::<Vec<_>>();
        train_task_roots_sha256.sort();
        confirm_task_roots_sha256.sort();
        let train_tokens = split_identity_tokens_v1(train_tasks)?;
        let confirm_tokens = split_identity_tokens_v1(confirm_tasks)?;
        let root_intersections = train_tokens.intersection(&confirm_tokens).count() as u64;
        let trainer_byte_leaks = confirm_tokens
            .iter()
            .filter(|token| contains_bytes_v1(trainer_bytes, token.as_bytes()))
            .count() as u64;
        if train_tasks.len() != K2_REPRESENTATION_TRAIN_TASKS_V1
            || confirm_tasks.len() != K2_REPRESENTATION_CONFIRM_TASKS_V1
            || root_intersections != 0
            || trainer_byte_leaks != 0
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_confirm_seal_invalid",
            ));
        }
        let receipt_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_CONFIRM_SEAL_SCHEMA_V1,
            &model_root_sha256,
            &train_task_roots_sha256,
            &confirm_task_roots_sha256,
            root_intersections,
            trainer_byte_leaks,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_CONFIRM_SEAL_SCHEMA_V1.to_owned(),
            model_root_sha256,
            train_task_roots_sha256,
            confirm_task_roots_sha256,
            root_intersections,
            trainer_byte_leaks,
            receipt_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationProcessReceiptV1 {
    pub schema: String,
    pub role: String,
    pub executable_sha256: String,
    pub request_root_sha256: String,
    pub outcome_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2RepresentationProcessReceiptV1 {
    pub fn seal(
        role: &str,
        executable_sha256: String,
        request_root_sha256: String,
        outcome_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let receipt_root_sha256 = composition_root_v1(&(
            K2_REPRESENTATION_PROCESS_RECEIPT_SCHEMA_V1,
            role,
            &executable_sha256,
            &request_root_sha256,
            &outcome_root_sha256,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_REPRESENTATION_PROCESS_RECEIPT_SCHEMA_V1.to_owned(),
            role: role.to_owned(),
            executable_sha256,
            request_root_sha256,
            outcome_root_sha256,
            authority,
            receipt_root_sha256,
        })
    }
}

pub fn feature_language_root_v1() -> K2CompositionResultV1<String> {
    composition_root_v1(&(
        K2_REPRESENTATION_FEATURE_SCHEMA_V1,
        K2_REPRESENTATION_FEATURE_NAMES_V1,
    ))
}

pub fn extract_policy_features_v1(
    task: &K2RepresentationTaskV1,
    current: &K2CompositionTreeManifestV1,
    used_action_ids: &[String],
    action_id: &str,
) -> K2CompositionResultV1<K2RepresentationFeatureVectorV1> {
    let law = task.law(action_id).ok_or(K2CompositionErrorV1::Invalid(
        "representation_feature_action_missing",
    ))?;
    let current_entries = manifest_entries_v1(current);
    let goal_entries = manifest_entries_v1(&task.goal.expected_terminal);
    let used = used_action_ids.iter().collect::<BTreeSet<_>>();
    let remaining = task
        .laws
        .iter()
        .filter(|other| {
            other.action_id_sha256 != action_id && !used.contains(&other.action_id_sha256)
        })
        .collect::<Vec<_>>();
    let mut values = vec![0_i64; K2_REPRESENTATION_FEATURES_V1];
    values[0] = K2_REPRESENTATION_FEATURE_SCALE_V1;
    values[13] = ((K2_REPRESENTATION_MAX_DEPTH_V1.saturating_sub(used.len() as u64))
        * K2_REPRESENTATION_FEATURE_SCALE_V1 as u64
        / K2_REPRESENTATION_MAX_DEPTH_V1) as i64;
    match &law.effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            values[2] = K2_REPRESENTATION_FEATURE_SCALE_V1;
            let source = current_entries.get(source_path);
            values[1] = bool_feature_v1(source.is_some());
            let goal_target = goal_entries.get(target_path);
            values[4] = bool_feature_v1(goal_target.is_some());
            values[5] = bool_feature_v1(same_file_value_v1(source, goal_target));
            values[6] = bool_feature_v1(
                source.is_some() && !same_file_value_v1(source, current_entries.get(target_path)),
            );
            let consumed = remaining
                .iter()
                .any(|other| other.effect.read_paths().contains(target_path));
            values[7] = bool_feature_v1(consumed);
            values[8] = bool_feature_v1(
                source.is_none()
                    && remaining
                        .iter()
                        .any(|other| other.effect.write_paths().contains(source_path)),
            );
            values[12] = bool_feature_v1(goal_target.is_none() && !consumed);
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            values[3] = K2_REPRESENTATION_FEATURE_SCALE_V1;
            values[1] = bool_feature_v1(current_entries.contains_key(path));
            values[9] = bool_feature_v1(!goal_entries.contains_key(path));
            values[10] = bool_feature_v1(goal_entries.contains_key(path));
            values[11] = bool_feature_v1(
                remaining
                    .iter()
                    .any(|other| other.effect.read_paths().contains(path)),
            );
        }
    }
    K2RepresentationFeatureVectorV1::seal(values)
}

pub fn apply_feature_transition_v1(
    current: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<K2CompositionTreeManifestV1> {
    let mut entries = current
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            let source = entries
                .get(source_path)
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "representation_copy_source_missing",
                ))?;
            entries.insert(
                target_path.clone(),
                super::super::K2CompositionFileEntryV1 {
                    path: target_path.clone(),
                    content_sha256: source.content_sha256,
                    byte_len: source.byte_len,
                },
            );
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_none() {
                return Err(K2CompositionErrorV1::Invalid(
                    "representation_remove_path_missing",
                ));
            }
        }
    }
    K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())
}

pub fn hidden_score_v1(
    model: &K2MeaningPolicySnapshotV1,
    features: &K2RepresentationFeatureVectorV1,
) -> K2CompositionResultV1<(Vec<i64>, i64)> {
    model.validate()?;
    if features.values.len() != K2_REPRESENTATION_FEATURES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_score_feature_invalid",
        ));
    }
    let hidden = model
        .encoder_weights
        .iter()
        .map(|row| {
            row.iter()
                .zip(&features.values)
                .map(|(weight, value)| weight.saturating_mul(*value))
                .sum::<i64>()
                .saturating_div(K2_REPRESENTATION_FEATURE_SCALE_V1)
                .clamp(0, 1_000_000)
        })
        .collect::<Vec<_>>();
    let score = model
        .output_weights
        .iter()
        .zip(&hidden)
        .map(|(weight, value)| weight.saturating_mul(*value))
        .sum::<i64>();
    Ok((hidden, score))
}

pub fn representation_bytes_v1<T: Serialize>(value: &T) -> K2CompositionResultV1<Vec<u8>> {
    composition_bytes_v1(value)
}

pub fn representation_decode_v1<T: for<'de> Deserialize<'de> + Serialize>(
    bytes: &[u8],
) -> K2CompositionResultV1<T> {
    composition_decode_v1(bytes)
}

pub fn representation_executable_matches_v1(expected_sha256: &str) -> K2CompositionResultV1<()> {
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_representation_executable"))?;
    if composition_sha256_file_v1(&executable)? != expected_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_executable_mismatch",
        ));
    }
    Ok(())
}

fn manifest_entries_v1(
    manifest: &K2CompositionTreeManifestV1,
) -> BTreeMap<String, super::super::K2CompositionFileEntryV1> {
    manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect()
}

fn same_file_value_v1(
    left: Option<&super::super::K2CompositionFileEntryV1>,
    right: Option<&super::super::K2CompositionFileEntryV1>,
) -> bool {
    matches!(
        (left, right),
        (Some(left), Some(right))
            if left.content_sha256 == right.content_sha256 && left.byte_len == right.byte_len
    )
}

const fn bool_feature_v1(value: bool) -> i64 {
    if value {
        K2_REPRESENTATION_FEATURE_SCALE_V1
    } else {
        0
    }
}

fn split_identity_tokens_v1(
    tasks: &[K2RepresentationTaskV1],
) -> K2CompositionResultV1<BTreeSet<String>> {
    let mut tokens = BTreeSet::new();
    for task in tasks {
        tokens.insert(task.experiment_id_sha256.clone());
        tokens.insert(task.task_root_sha256.clone());
        tokens.insert(task.goal.goal_root_sha256.clone());
        tokens.insert(task.initial.tree_root_sha256.clone());
        for law in &task.laws {
            tokens.insert(law.action_id_sha256.clone());
            tokens.insert(law.law_root_sha256.clone());
            tokens.insert(law.support_root_sha256.clone());
            match &law.effect {
                K2CompositionLearnedEffectV1::CopyFile {
                    source_path,
                    target_path,
                } => {
                    tokens.insert(source_path.clone());
                    tokens.insert(target_path.clone());
                }
                K2CompositionLearnedEffectV1::RemoveFile { path } => {
                    tokens.insert(path.clone());
                }
            }
        }
        for entry in task
            .initial
            .entries
            .iter()
            .chain(task.goal.expected_terminal.entries.iter())
        {
            tokens.insert(entry.path.clone());
            tokens.insert(entry.content_sha256.clone());
        }
    }
    if tokens.iter().any(|token| token.is_empty()) {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_split_token_empty",
        ));
    }
    Ok(tokens)
}

fn contains_bytes_v1(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
