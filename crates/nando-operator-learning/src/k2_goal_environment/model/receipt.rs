pub struct K2LawLabBindingInputV1<'a> {
    pub freeze: &'a K2DecisionFreezeV1,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub request: &'a LawLabSandboxRequestV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LawLabBindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub episode_id_sha256: String,
    pub decision_freeze_root_sha256: String,
    pub goal_envelope_root_sha256: String,
    pub vocabulary_snapshot_root_sha256: String,
    pub alternative_set_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub law_lab_request_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub worker_sha256: String,
    pub deterministic_seed_sha256: String,
    pub budget_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LawLabBindingV1 {
    pub fn seal(input: K2LawLabBindingInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input
            .request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        input.selection.validate(input.freeze, input.predictions)?;
        input.predictions.validate(
            input.freeze,
            input.goal,
            input.vocabulary,
            input.alternatives,
        )?;
        let selected = input
            .alternatives
            .alternative(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selected_alternative_missing",
            ))?;
        let operation_plan_root_sha256 = canonical_root(&input.request.operations)?;
        let request_binding_valid = input.request.purpose
            == LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            && input.request.candidate_root_sha256 == input.freeze.episode_id_sha256
            && input.request.version_space_root_sha256
                == input.alternatives.alternative_set_root_sha256
            && input.request.durable_prediction_ledger_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.request.probe_root_sha256 == input.selection.selection_root_sha256
            && input.request.deterministic_seed_sha256 == input.freeze.deterministic_seed_sha256
            && input.request.worker_sha256 == input.freeze.sandbox_worker_sha256
            && input.request.surviving_hypothesis_count
                == input.alternatives.alternatives.len() as u64
            && input.request.precommitted_prediction_count
                == input.predictions.predictions.len() as u64
            && operation_plan_root_sha256 == selected.operation_plan_root_sha256;
        if !request_binding_valid {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_law_lab_request_binding_invalid",
            ));
        }
        let mut binding = Self {
            schema: K2_LAW_LAB_BINDING_SCHEMA_V1.to_owned(),
            binding_root_sha256: String::new(),
            episode_id_sha256: input.freeze.episode_id_sha256.clone(),
            decision_freeze_root_sha256: input.freeze.decision_freeze_root_sha256.clone(),
            goal_envelope_root_sha256: input.goal.goal_envelope_root_sha256.clone(),
            vocabulary_snapshot_root_sha256: input.vocabulary.snapshot_root_sha256.clone(),
            alternative_set_root_sha256: input.alternatives.alternative_set_root_sha256.clone(),
            prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            selected_action_root_sha256: input.selection.selected_action_root_sha256.clone(),
            law_lab_request_root_sha256: input.request.request_root_sha256.clone(),
            source_tree_root_sha256: input.request.source_tree_root_sha256.clone(),
            executor_manifest_root_sha256: input.request.executor_manifest_root_sha256.clone(),
            worker_sha256: input.request.worker_sha256.clone(),
            deterministic_seed_sha256: input.request.deterministic_seed_sha256.clone(),
            budget_root_sha256: input.freeze.budget_root_sha256.clone(),
            operation_plan_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        binding.binding_root_sha256 = binding.expected_root()?;
        binding.validate(input)?;
        Ok(binding)
    }

    pub fn validate(&self, input: K2LawLabBindingInputV1<'_>) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        input
            .request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let selected = input
            .alternatives
            .alternative(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_selected_alternative_missing",
            ))?;
        let operation_plan_root = canonical_root(&input.request.operations)?;
        if self.schema != K2_LAW_LAB_BINDING_SCHEMA_V1
            || self.episode_id_sha256 != input.freeze.episode_id_sha256
            || self.decision_freeze_root_sha256 != input.freeze.decision_freeze_root_sha256
            || self.goal_envelope_root_sha256 != input.goal.goal_envelope_root_sha256
            || self.vocabulary_snapshot_root_sha256 != input.vocabulary.snapshot_root_sha256
            || self.alternative_set_root_sha256 != input.alternatives.alternative_set_root_sha256
            || self.prediction_set_root_sha256 != input.predictions.prediction_set_root_sha256
            || self.selected_action_root_sha256 != input.selection.selected_action_root_sha256
            || self.law_lab_request_root_sha256 != input.request.request_root_sha256
            || self.source_tree_root_sha256 != input.request.source_tree_root_sha256
            || self.executor_manifest_root_sha256 != input.request.executor_manifest_root_sha256
            || self.worker_sha256 != input.request.worker_sha256
            || self.deterministic_seed_sha256 != input.request.deterministic_seed_sha256
            || self.budget_root_sha256 != input.freeze.budget_root_sha256
            || self.operation_plan_root_sha256 != selected.operation_plan_root_sha256
            || self.operation_plan_root_sha256 != operation_plan_root
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_law_lab_binding_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.binding_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.law_lab_request_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_law_lab_binding_root_invalid")?;
        }
        if self.schema != K2_LAW_LAB_BINDING_SCHEMA_V1
            || self.binding_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_law_lab_binding_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_LAW_LAB_BINDING_SCHEMA_V1,
            self.episode_id_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.goal_envelope_root_sha256.as_str(),
            self.vocabulary_snapshot_root_sha256.as_str(),
            self.alternative_set_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.selected_action_root_sha256.as_str(),
            self.law_lab_request_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub goal_predicate_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub law_lab_receipt_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
}

impl K2ExactOracleRequestV1 {
    pub fn seal(
        goal: &K2GoalEnvelopeV1,
        freeze: &K2DecisionFreezeV1,
        binding: &K2LawLabBindingV1,
        execution: &LawLabSandboxExecutionV1,
        oracle_manifest: &K2ExactOracleManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        goal.validate()?;
        oracle_manifest.validate()?;
        if goal.goal_envelope_root_sha256 != freeze.goal_envelope_root_sha256
            || binding.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || execution.receipt.request_root_sha256 != binding.law_lab_request_root_sha256
            || oracle_manifest.manifest_root_sha256 != freeze.oracle_manifest_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_binding_invalid",
            ));
        }
        let mut request = Self {
            schema: K2_EXACT_ORACLE_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            oracle_manifest_root_sha256: oracle_manifest.manifest_root_sha256.clone(),
            goal_predicate_root_sha256: goal.goal_predicate_root_sha256.clone(),
            law_lab_binding_root_sha256: binding.binding_root_sha256.clone(),
            law_lab_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
            expected_terminal_tree_root_sha256: goal.expected_terminal_tree_root_sha256.clone(),
            observed_terminal_tree_root_sha256: execution.receipt.post_tree_root_sha256.clone(),
        };
        request.request_root_sha256 = request.expected_root()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.request_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.goal_predicate_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ] {
            require_root(root, "k2_exact_oracle_request_root_invalid")?;
        }
        if self.schema != K2_EXACT_ORACLE_REQUEST_SCHEMA_V1
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request: Self = serde_json::from_slice(bytes).map_err(|_| {
            K2GoalEnvironmentErrorV1::Invalid("k2_exact_oracle_request_decode_failed")
        })?;
        request.validate()?;
        if request.canonical_bytes()? != bytes {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_request_not_canonical",
            ));
        }
        Ok(request)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_ORACLE_REQUEST_SCHEMA_V1,
            self.oracle_manifest_root_sha256.as_str(),
            self.goal_predicate_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub request_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
    pub goal_satisfied: bool,
}

impl K2ExactOracleOutcomeV1 {
    pub fn evaluate(request: &K2ExactOracleRequestV1) -> K2GoalEnvironmentResultV1<Self> {
        request.validate()?;
        let mut outcome = Self {
            schema: K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            request_root_sha256: request.request_root_sha256.clone(),
            expected_terminal_tree_root_sha256: request.expected_terminal_tree_root_sha256.clone(),
            observed_terminal_tree_root_sha256: request.observed_terminal_tree_root_sha256.clone(),
            goal_satisfied: request.expected_terminal_tree_root_sha256
                == request.observed_terminal_tree_root_sha256,
        };
        outcome.outcome_root_sha256 = outcome.expected_root()?;
        outcome.validate(request)?;
        Ok(outcome)
    }

    pub fn validate(&self, request: &K2ExactOracleRequestV1) -> K2GoalEnvironmentResultV1<()> {
        request.validate()?;
        if self.schema != K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1
            || self.request_root_sha256 != request.request_root_sha256
            || self.expected_terminal_tree_root_sha256 != request.expected_terminal_tree_root_sha256
            || self.observed_terminal_tree_root_sha256 != request.observed_terminal_tree_root_sha256
            || self.goal_satisfied
                != (self.expected_terminal_tree_root_sha256
                    == self.observed_terminal_tree_root_sha256)
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_outcome_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        request: &K2ExactOracleRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let outcome: Self = serde_json::from_slice(bytes).map_err(|_| {
            K2GoalEnvironmentErrorV1::Invalid("k2_exact_oracle_outcome_decode_failed")
        })?;
        outcome.validate(request)?;
        if outcome.canonical_bytes()? != bytes {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_outcome_not_canonical",
            ));
        }
        Ok(outcome)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_ORACLE_OUTCOME_SCHEMA_V1,
            self.request_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
            self.goal_satisfied,
        ))
    }
}

pub struct K2ExactGoalEvaluationInputV1<'a> {
    pub freeze: &'a K2DecisionFreezeV1,
    pub goal: &'a K2GoalEnvelopeV1,
    pub vocabulary: &'a K2K1VocabularySnapshotV1,
    pub alternatives: &'a K2AlternativeSetV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub binding: &'a K2LawLabBindingV1,
    pub request: &'a LawLabSandboxRequestV1,
    pub execution: &'a LawLabSandboxExecutionV1,
    pub oracle_manifest: &'a K2ExactOracleManifestV1,
    pub oracle_request: &'a K2ExactOracleRequestV1,
    pub oracle_outcome: &'a K2ExactOracleOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactGoalReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub decision_freeze_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub law_lab_receipt_root_sha256: String,
    pub oracle_manifest_root_sha256: String,
    pub oracle_request_root_sha256: String,
    pub oracle_outcome_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub observed_terminal_tree_root_sha256: String,
    pub goal_satisfied: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2ExactGoalReceiptV1 {
    pub fn evaluate(input: K2ExactGoalEvaluationInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.binding.validate(K2LawLabBindingInputV1 {
            freeze: input.freeze,
            goal: input.goal,
            vocabulary: input.vocabulary,
            alternatives: input.alternatives,
            predictions: input.predictions,
            selection: input.selection,
            request: input.request,
        })?;
        input
            .execution
            .receipt
            .validate(input.request, &input.execution.worker_outcome)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        input.oracle_manifest.validate()?;
        input.oracle_request.validate()?;
        input.oracle_outcome.validate(input.oracle_request)?;
        if input.oracle_manifest.manifest_root_sha256 != input.freeze.oracle_manifest_root_sha256
            || input.oracle_manifest.executable_sha256 != input.freeze.oracle_executable_sha256
            || input.execution.receipt.request_root_sha256
                != input.binding.law_lab_request_root_sha256
            || input.oracle_request.oracle_manifest_root_sha256
                != input.oracle_manifest.manifest_root_sha256
            || input.oracle_request.goal_predicate_root_sha256
                != input.goal.goal_predicate_root_sha256
            || input.oracle_request.law_lab_binding_root_sha256 != input.binding.binding_root_sha256
            || input.oracle_request.law_lab_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.oracle_request.expected_terminal_tree_root_sha256
                != input.goal.expected_terminal_tree_root_sha256
            || input.oracle_request.observed_terminal_tree_root_sha256
                != input.execution.receipt.post_tree_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_oracle_binding_invalid",
            ));
        }
        let observed_terminal_tree_root_sha256 = input
            .oracle_outcome
            .observed_terminal_tree_root_sha256
            .clone();
        let goal_satisfied = input.oracle_outcome.goal_satisfied;
        let mut receipt = Self {
            schema: K2_EXACT_GOAL_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            provenance: input.freeze.provenance,
            decision_freeze_root_sha256: input.freeze.decision_freeze_root_sha256.clone(),
            law_lab_binding_root_sha256: input.binding.binding_root_sha256.clone(),
            law_lab_receipt_root_sha256: input.execution.receipt.receipt_root_sha256.clone(),
            oracle_manifest_root_sha256: input.oracle_manifest.manifest_root_sha256.clone(),
            oracle_request_root_sha256: input.oracle_request.request_root_sha256.clone(),
            oracle_outcome_root_sha256: input.oracle_outcome.outcome_root_sha256.clone(),
            expected_terminal_tree_root_sha256: input
                .goal
                .expected_terminal_tree_root_sha256
                .clone(),
            observed_terminal_tree_root_sha256,
            goal_satisfied,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate(input)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        input: K2ExactGoalEvaluationInputV1<'_>,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        input.oracle_outcome.validate(input.oracle_request)?;
        let expected_satisfaction =
            self.expected_terminal_tree_root_sha256 == self.observed_terminal_tree_root_sha256;
        if self.schema != K2_EXACT_GOAL_RECEIPT_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.provenance != input.freeze.provenance
            || self.decision_freeze_root_sha256 != input.freeze.decision_freeze_root_sha256
            || self.law_lab_binding_root_sha256 != input.binding.binding_root_sha256
            || self.law_lab_receipt_root_sha256 != input.execution.receipt.receipt_root_sha256
            || self.oracle_manifest_root_sha256 != input.oracle_manifest.manifest_root_sha256
            || self.oracle_request_root_sha256 != input.oracle_request.request_root_sha256
            || self.oracle_outcome_root_sha256 != input.oracle_outcome.outcome_root_sha256
            || self.expected_terminal_tree_root_sha256
                != input.goal.expected_terminal_tree_root_sha256
            || self.observed_terminal_tree_root_sha256
                != input.execution.receipt.post_tree_root_sha256
            || self.goal_satisfied != expected_satisfaction
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_exact_goal_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_request_root_sha256.as_str(),
            self.oracle_outcome_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
        ] {
            require_root(root, "k2_persisted_exact_goal_root_invalid")?;
        }
        if self.schema != K2_EXACT_GOAL_RECEIPT_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.goal_satisfied
                != (self.expected_terminal_tree_root_sha256
                    == self.observed_terminal_tree_root_sha256)
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_exact_goal_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EXACT_GOAL_RECEIPT_SCHEMA_V1,
            self.provenance,
            self.decision_freeze_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.law_lab_receipt_root_sha256.as_str(),
            self.oracle_manifest_root_sha256.as_str(),
            self.oracle_request_root_sha256.as_str(),
            self.oracle_outcome_root_sha256.as_str(),
            self.expected_terminal_tree_root_sha256.as_str(),
            self.observed_terminal_tree_root_sha256.as_str(),
            self.goal_satisfied,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2DecisionTerminalVerdictV1 {
    CapabilityPass,
    LabGoalSatisfied,
    LabGoalNotSatisfied,
    InsufficientK1Vocabulary,
    CertificateBoundRuntimeClosed,
    StaleBeforeFreeze,
    NoMeaningfulAlternatives,
    NoUniqueSelection,
    SandboxVerificationFail,
    OracleMismatch,
    BudgetExhausted,
    SafetyVeto,
    IndeterminateAfterCrash,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionOutcomeReceiptV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub decision_freeze_root_sha256: String,
    pub prediction_set_root_sha256: String,
    pub law_lab_binding_root_sha256: String,
    pub sandbox_receipt_root_sha256: String,
    pub exact_goal_receipt_root_sha256: String,
    pub terminal_verdict: K2DecisionTerminalVerdictV1,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2DecisionOutcomeReceiptV1 {
    pub fn capability_pass(
        freeze: &K2DecisionFreezeV1,
        predictions: &K2AlternativePredictionSetV1,
        binding: &K2LawLabBindingV1,
        execution: &LawLabSandboxExecutionV1,
        exact_goal: &K2ExactGoalReceiptV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if freeze.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || predictions.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || binding.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || exact_goal.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
            || exact_goal.law_lab_binding_root_sha256 != binding.binding_root_sha256
            || exact_goal.law_lab_receipt_root_sha256 != execution.receipt.receipt_root_sha256
            || !exact_goal.goal_satisfied
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_capability_outcome_inputs_invalid",
            ));
        }
        let mut outcome = Self {
            schema: K2_DECISION_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            provenance: freeze.provenance,
            decision_freeze_root_sha256: freeze.decision_freeze_root_sha256.clone(),
            prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            law_lab_binding_root_sha256: binding.binding_root_sha256.clone(),
            sandbox_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
            exact_goal_receipt_root_sha256: exact_goal.receipt_root_sha256.clone(),
            terminal_verdict: K2DecisionTerminalVerdictV1::CapabilityPass,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.outcome_root_sha256 = outcome.expected_root()?;
        outcome.validate()?;
        Ok(outcome)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.outcome_root_sha256.as_str(),
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
            self.exact_goal_receipt_root_sha256.as_str(),
        ] {
            require_root(root, "k2_outcome_root_invalid")?;
        }
        if self.schema != K2_DECISION_OUTCOME_SCHEMA_V1
            || self.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
            || self.terminal_verdict != K2DecisionTerminalVerdictV1::CapabilityPass
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_decision_outcome_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_DECISION_OUTCOME_SCHEMA_V1,
            self.provenance,
            self.decision_freeze_root_sha256.as_str(),
            self.prediction_set_root_sha256.as_str(),
            self.law_lab_binding_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
            self.exact_goal_receipt_root_sha256.as_str(),
            self.terminal_verdict,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2DecisionEpisodeSealV1 {
    pub schema: String,
    pub seal_root_sha256: String,
    pub episode_id_sha256: String,
    pub outcome_root_sha256: String,
    pub terminal_event_root_sha256: String,
    pub final_projection_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2DecisionEpisodeSealV1 {
    pub fn derive(
        episode_id_sha256: String,
        outcome_root_sha256: String,
        terminal_event_root_sha256: String,
        final_projection_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        for root in [
            episode_id_sha256.as_str(),
            outcome_root_sha256.as_str(),
            terminal_event_root_sha256.as_str(),
            final_projection_root_sha256.as_str(),
        ] {
            require_root(root, "k2_episode_seal_input_root_invalid")?;
        }
        let mut seal = Self {
            schema: K2_EPISODE_SEAL_SCHEMA_V1.to_owned(),
            seal_root_sha256: String::new(),
            episode_id_sha256,
            outcome_root_sha256,
            terminal_event_root_sha256,
            final_projection_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        seal.seal_root_sha256 = seal.expected_root()?;
        seal.validate()?;
        Ok(seal)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.seal_root_sha256.as_str(),
            self.episode_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
        ] {
            require_root(root, "k2_episode_seal_root_invalid")?;
        }
        if self.schema != K2_EPISODE_SEAL_SCHEMA_V1
            || self.seal_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_episode_seal_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&(
            K2_EPISODE_SEAL_SCHEMA_V1,
            self.episode_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2CertificateBoundRuntimeStatusV1 {
    InsufficientK1Vocabulary,
    CertificateBoundRuntimeClosed,
}

#[must_use]
pub const fn k2_certificate_bound_runtime_status_v1(
    genuine_k1_action_count: usize,
) -> K2CertificateBoundRuntimeStatusV1 {
    if genuine_k1_action_count < 2 {
        K2CertificateBoundRuntimeStatusV1::InsufficientK1Vocabulary
    } else {
        K2CertificateBoundRuntimeStatusV1::CertificateBoundRuntimeClosed
    }
}
