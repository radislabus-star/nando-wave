pub struct K2DecisionFreezeInputV1<'a> {
    pub episode_id_sha256: String,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub budget: K2GoalEnvironmentBudgetV1,
    pub selector_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub oracle_manifest: &'a K2ExactOracleManifestV1,
    pub sandbox_worker_sha256: String,
    pub deterministic_seed_sha256: String,
    pub observed_registry_revision: Option<u64>,
    pub observed_registry_root_sha256: Option<String>,
    pub frozen_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionFreezeV1 {
    pub schema: String,
    pub decision_freeze_root_sha256: String,
    pub episode_id_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub initial_environment_root_sha256: String,
    pub selector_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub oracle_executable_sha256: String,
    pub sandbox_worker_sha256: String,
    pub budget_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub previous_journal_entry_root_sha256: Option<String>,
    pub frozen_at_unix_ms: u64,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2DecisionFreezeDigestV1<'a> {
    schema: &'static str,
    episode_id_sha256: &'a str,
    provenance: K2EvidenceProvenanceV1,
    goal_envelope_root_sha256: &'a str,
    vocabulary_snapshot_root_sha256: &'a str,
    alternative_set_root_sha256: &'a str,
    initial_environment_root_sha256: &'a str,
    selector_contract_root_sha256: &'a str,
    selector_executable_sha256: &'a str,
    oracle_manifest_root_sha256: &'a str,
    oracle_executable_sha256: &'a str,
    sandbox_worker_sha256: &'a str,
    budget_root_sha256: &'a str,
    deterministic_seed_sha256: &'a str,
    previous_journal_entry_root_sha256: Option<&'a str>,
    frozen_at_unix_ms: u64,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2DecisionFreezeV1 {
    pub fn seal(input: K2DecisionFreezeInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.goal.validate()?;
        input.vocabulary.validate()?;
        input.alternatives.validate(input.vocabulary)?;
        input.oracle_manifest.validate()?;
        let budget_root_sha256 = input.budget.root()?;
        if input.goal.provenance != input.vocabulary.provenance
            || input.goal.provenance != input.alternatives.provenance
            || input.goal.environment_root_sha256 != input.alternatives.environment_root_sha256
            || input.goal.oracle_contract_root_sha256 != input.oracle_manifest.manifest_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_input_binding_invalid",
            ));
        }
        match input.vocabulary.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                if input.observed_registry_revision.is_some()
                    || input.observed_registry_root_sha256.is_some()
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_fixture_registry_binding_forbidden",
                    ));
                }
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                if input.observed_registry_revision != input.vocabulary.epistemic_registry_revision
                    || input.observed_registry_root_sha256
                        != input.vocabulary.epistemic_registry_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_registry_stale_before_freeze",
                    ));
                }
            }
        }
        for root in [
            input.episode_id_sha256.as_str(),
            input.selector_contract_root_sha256.as_str(),
            input.selector_executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
            input.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_decision_input_root_invalid")?;
        }
        if !roots_are_unique([
            input.selector_executable_sha256.as_str(),
            input.oracle_manifest.executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
        ]) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_executable_identity_not_independent",
            ));
        }
        let mut freeze = Self {
            schema: K2_DECISION_FREEZE_SCHEMA_V1.to_owned(),
            decision_freeze_root_sha256: String::new(),
            episode_id_sha256: input.episode_id_sha256,
            provenance: input.goal.provenance,
            goal_envelope_root_sha256: input.goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: input.vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: input.alternatives.alternative_set_root_sha256.clone(),
            initial_environment_root_sha256: input.goal.environment_root_sha256.clone(),
            selector_contract_root_sha256: input.selector_contract_root_sha256,
            selector_executable_sha256: input.selector_executable_sha256,
            oracle_manifest_root_sha256: input.oracle_manifest.manifest_root_sha256.clone(),
            oracle_executable_sha256: input.oracle_manifest.executable_sha256.clone(),
            sandbox_worker_sha256: input.sandbox_worker_sha256,
            budget_root_sha256,
            deterministic_seed_sha256: input.deterministic_seed_sha256,
            previous_journal_entry_root_sha256: None,
            frozen_at_unix_ms: input.frozen_at_unix_ms,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        freeze.decision_freeze_root_sha256 = freeze.expected_root()?;
        freeze.validate(
            input.goal,
            input.vocabulary,
            input.alternatives,
            &input.budget,
            input.oracle_manifest,
        )?;
        Ok(freeze)
    }

    pub fn validate(
        &self,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
        budget: &K2GoalEnvironmentBudgetV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        goal.validate()?;
        vocabulary.validate()?;
        alternatives.validate(vocabulary)?;
        oracle_manifest.validate()?;
        self.authority.validate()?;
        for root in [
            self.decision_freeze_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.selector_contract_root_sha256.as_str(),
            self.selector_executable_sha256.as_str(),
            self.sandbox_worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_decision_root_invalid")?;
        }
        if self.schema != K2_DECISION_FREEZE_SCHEMA_V1
            || self.provenance != goal.provenance
            || self.provenance != vocabulary.provenance
            || self.goal_envelope_root_sha256 != goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != alternatives.alternative_set_root_sha256
            || self.initial_environment_root_sha256 != goal.environment_root_sha256
            || self.initial_environment_root_sha256 != alternatives.environment_root_sha256
            || self.oracle_manifest_root_sha256 != oracle_manifest.manifest_root_sha256
            || self.oracle_executable_sha256 != oracle_manifest.executable_sha256
            || goal.oracle_contract_root_sha256 != oracle_manifest.manifest_root_sha256
            || self.budget_root_sha256 != budget.root()?
            || self.previous_journal_entry_root_sha256.is_some()
            || !roots_are_unique([
                self.selector_executable_sha256.as_str(),
                self.oracle_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
            ])
            || self.decision_freeze_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_freeze_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.decision_freeze_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.initial_environment_root_sha256.as_str(),
            self.selector_contract_root_sha256.as_str(),
            self.selector_executable_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_executable_sha256.as_str(),
            self.sandbox_worker_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_decision_root_invalid")?;
        }
        if self.schema != K2_DECISION_FREEZE_SCHEMA_V1
            || self.previous_journal_entry_root_sha256.is_some()
            || !roots_are_unique([
                self.selector_executable_sha256.as_str(),
                self.oracle_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
            ])
            || self.decision_freeze_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_decision_freeze_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2DecisionFreezeDigestV1 {
            schema: K2_DECISION_FREEZE_SCHEMA_V1,
            episode_id_sha256: &self.episode_id_sha256,
            provenance: self.provenance,
            goal_envelope_root_sha256: &self.goal_envelope_root_sha256,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            alternative_set_root_sha256: &self.alternative_set_root_sha256,
            initial_environment_root_sha256: &self.initial_environment_root_sha256,
            selector_contract_root_sha256: &self.selector_contract_root_sha256,
            selector_executable_sha256: &self.selector_executable_sha256,
            oracle_manifest_root_sha256: &self.oracle_manifest_root_sha256,
            oracle_executable_sha256: &self.oracle_executable_sha256,
            sandbox_worker_sha256: &self.sandbox_worker_sha256,
            budget_root_sha256: &self.budget_root_sha256,
            deterministic_seed_sha256: &self.deterministic_seed_sha256,
            previous_journal_entry_root_sha256: self.previous_journal_entry_root_sha256.as_deref(),
            frozen_at_unix_ms: self.frozen_at_unix_ms,
            authority: &self.authority,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativePredictionV1 {
    pub action_root_sha256: String,
    pub predicted_terminal_tree_root_sha256: String,
    pub predicted_goal_satisfied: bool,
    pub prediction_evidence_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativePredictionSetV1 {
    pub schema: String,
    pub prediction_set_root_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub predictor_schema: String,
    pub predictor_executable_sha256: String,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub creation_sequence: u64,
    pub learned: bool,
    pub predictions: Vec<K2AlternativePredictionV1>,
}

#[derive(Serialize)]
struct K2AlternativePredictionSetDigestV1<'a> {
    schema: &'static str,
    decision_freeze_root_sha256: &'a str,
    provenance: K2EvidenceProvenanceV1,
    predictor_schema: &'a str,
    predictor_executable_sha256: &'a str,
    goal_envelope_root_sha256: &'a str,
    vocabulary_snapshot_root_sha256: &'a str,
    alternative_set_root_sha256: &'a str,
    creation_sequence: u64,
    learned: bool,
    predictions: &'a [K2AlternativePredictionV1],
}

impl K2AlternativePredictionSetV1 {
    pub fn prepared_capability_v1(
        freeze: &K2DecisionFreezeV1,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
        budget: &K2GoalEnvironmentBudgetV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate(goal, vocabulary, alternatives, budget, oracle_manifest)?;
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_certificate_bound_runtime_closed",
            ));
        }
        let predictions = alternatives
            .alternatives
            .iter()
            .map(|alternative| {
                let predicted_goal_satisfied = alternative.predicted_consequence_root_sha256
                    == goal.expected_terminal_tree_root_sha256;
                let prediction_evidence_root_sha256 = canonical_root(&(
                    K2_PREPARED_SELECTOR_SCHEMA_V1,
                    freeze.decision_freeze_root_sha256.as_str(),
                    alternative.action_root_sha256.as_str(),
                    alternative.predicted_consequence_root_sha256.as_str(),
                    predicted_goal_satisfied,
                ))?;
                Ok(K2AlternativePredictionV1 {
                    action_root_sha256: alternative.action_root_sha256.clone(),
                    predicted_terminal_tree_root_sha256: alternative
                        .predicted_consequence_root_sha256
                        .clone(),
                    predicted_goal_satisfied,
                    prediction_evidence_root_sha256,
                })
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let mut set = Self {
            schema: K2_PREDICTION_SET_SCHEMA_V1.to_owned(),
            prediction_set_root_sha256: String::new(),
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            provenance: freeze.provenance,
            predictor_schema: K2_PREPARED_SELECTOR_SCHEMA_V1.to_owned(),
            predictor_executable_sha256: freeze.selector_executable_sha256.clone(),
            goal_envelope_root_sha256: goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: alternatives.alternative_set_root_sha256.clone(),
            creation_sequence: 1,
            learned: false,
            predictions,
        };
        set.prediction_set_root_sha256 = set.expected_root()?;
        set.validate(freeze, goal, vocabulary, alternatives)?;
        Ok(set)
    }

    pub fn validate(
        &self,
        freeze: &K2DecisionFreezeV1,
        goal: &K2GoalEnvelopeV1,
        vocabulary: &K2K1VocabularySnapshotV1,
        alternatives: &K2AlternativeSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_PREDICTION_SET_SCHEMA_V1
            || self.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.provenance != freeze.provenance
            || self.predictor_schema != K2_PREPARED_SELECTOR_SCHEMA_V1
            || self.predictor_executable_sha256 != freeze.selector_executable_sha256
            || self.goal_envelope_root_sha256 != goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != alternatives.alternative_set_root_sha256
            || self.creation_sequence != 1
            || self.learned
            || self.predictions.len() != alternatives.alternatives.len()
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prediction_set_binding_invalid",
            ));
        }
        for (prediction, alternative) in self.predictions.iter().zip(&alternatives.alternatives) {
            require_root(
                &prediction.prediction_evidence_root_sha256,
                "k2_prediction_evidence_root_invalid",
            )?;
            let expected_satisfaction = alternative.predicted_consequence_root_sha256
                == goal.expected_terminal_tree_root_sha256;
            if prediction.action_root_sha256 != alternative.action_root_sha256
                || prediction.predicted_terminal_tree_root_sha256
                    != alternative.predicted_consequence_root_sha256
                || prediction.predicted_goal_satisfied != expected_satisfaction
                || prediction.prediction_evidence_root_sha256
                    != canonical_root(&(
                        K2_PREPARED_SELECTOR_SCHEMA_V1,
                        freeze.decision_freeze_root_sha256.as_str(),
                        alternative.action_root_sha256.as_str(),
                        alternative.predicted_consequence_root_sha256.as_str(),
                        expected_satisfaction,
                    ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid("k2_prediction_invalid"));
            }
        }
        if self.prediction_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prediction_set_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.prediction_set_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.predictor_executable_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_prediction_root_invalid")?;
        }
        if self.schema != K2_PREDICTION_SET_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.predictor_schema != K2_PREPARED_SELECTOR_SCHEMA_V1
            || self.creation_sequence != 1
            || self.learned
            || self.predictions.len() < 2
            || self.predictions.len() > K2_MAX_ALTERNATIVES_V1
            || !roots_are_unique(
                self.predictions
                    .iter()
                    .map(|prediction| prediction.action_root_sha256.as_str()),
            )
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_prediction_set_invalid",
            ));
        }
        for prediction in &self.predictions {
            for root in [
                prediction.action_root_sha256.as_str(),
                prediction.predicted_terminal_tree_root_sha256.as_str(),
                prediction.prediction_evidence_root_sha256.as_str(),
            ] {
                require_root(root, "k2_persisted_prediction_entry_root_invalid")?;
            }
            if prediction.prediction_evidence_root_sha256
                != canonical_root(&(
                    K2_PREPARED_SELECTOR_SCHEMA_V1,
                    self.decision_freeze_root_sha256.as_str(),
                    prediction.action_root_sha256.as_str(),
                    prediction.predicted_terminal_tree_root_sha256.as_str(),
                    prediction.predicted_goal_satisfied,
                ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_persisted_prediction_entry_invalid",
                ));
            }
        }
        if self.prediction_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_prediction_set_root_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2AlternativePredictionSetDigestV1 {
            schema: K2_PREDICTION_SET_SCHEMA_V1,
            decision_freeze_root_sha256: &self.decision_freeze_root_sha256,
            provenance: self.provenance,
            predictor_schema: &self.predictor_schema,
            predictor_executable_sha256: &self.predictor_executable_sha256,
            goal_envelope_root_sha256: &self.goal_envelope_root_sha256,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            alternative_set_root_sha256: &self.alternative_set_root_sha256,
            creation_sequence: self.creation_sequence,
            learned: self.learned,
            predictions: &self.predictions,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PreparedSelectionReceiptV1 {
    pub schema: String,
    pub selection_root_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2PreparedSelectionReceiptV1 {
    pub fn select(
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || predictions.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || predictions.learned
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_prepared_selector_scope_invalid",
            ));
        }
        let satisfying = predictions
            .predictions
            .iter()
            .filter(|prediction| prediction.predicted_goal_satisfied)
            .collect::<Vec<_>>();
        if satisfying.len() != 1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_no_unique_selection"));
        }
        let mut receipt = Self {
            schema: K2_SELECTION_RECEIPT_SCHEMA_V1.to_owned(),
            selection_root_sha256: String::new(),
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            selected_action_root_sha256: satisfying[0].action_root_sha256.clone(),
            learned: false,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.selection_root_sha256 = receipt.expected_root()?;
        receipt.validate(freeze, predictions)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        let satisfying = predictions
            .predictions
            .iter()
            .filter(|prediction| prediction.predicted_goal_satisfied)
            .collect::<Vec<_>>();
        if self.schema != K2_SELECTION_RECEIPT_SCHEMA_V1
            || self.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || self.prediction_set_root_sha256 != predictions.prediction_set_root_sha256
            || satisfying.len() != 1
            || self.selected_action_root_sha256 != satisfying[0].action_root_sha256
            || self.learned
            || self.selection_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selection_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_SELECTION_RECEIPT_SCHEMA_V1,
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.learned,
            &self.authority,
        ))
    }
}

