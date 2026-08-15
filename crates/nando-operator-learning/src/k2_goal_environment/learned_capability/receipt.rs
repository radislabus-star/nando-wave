#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectVerificationReceiptV1 {
    pub schema: String,
    pub verification_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub verifier_contract_root_sha256: String,
    pub public_context_root_sha256: String,
    pub support_observation_set_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub verified_support_laws: u64,
    pub verified_target_predictions: u64,
    pub wrong_laws: u64,
    pub wrong_predictions: u64,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedEffectVerificationReceiptV1 {
    pub fn verify(
        freeze: &K2LearnedCapabilityFreezeV1,
        learning_request: &K2EffectLearningRequestV1,
        laws: &K2LearnedEffectLawSetV1,
        prediction_request: &K2TargetPredictionRequestV1,
        predictions: &K2LearnedTargetPredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        learning_request.validate()?;
        laws.validate()?;
        prediction_request.validate()?;
        predictions.validate()?;
        if laws.learning_request_root_sha256 != learning_request.request_root_sha256
            || prediction_request.learned_law_set.law_set_root_sha256 != laws.law_set_root_sha256
            || predictions.target_prediction_request_root_sha256
                != prediction_request.request_root_sha256
            || freeze.independent_verifier_contract_root_sha256
                != learned_root_v1(&K2_INDEPENDENT_EFFECT_VERIFIER_CONTRACT_V1)?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_independent_verifier_binding_invalid",
            ));
        }
        let mut verified_support_laws = 0_u64;
        let mut wrong_laws = 0_u64;
        for law in &laws.laws {
            let observations = learning_request
                .support_observations
                .observations
                .iter()
                .filter(|observation| observation.action_id_sha256 == law.action_id_sha256)
                .collect::<Vec<_>>();
            let candidates = observations.first().map_or_else(
                || Ok(Vec::new()),
                |observation| independent_effect_candidates_v1(observation),
            )?;
            let survivors = candidates
                .iter()
                .filter(|candidate| {
                    observations.iter().all(|observation| {
                        independent_apply_effect_v1(&observation.pre_work_manifest, candidate)
                            .is_ok_and(|manifest| manifest == observation.post_work_manifest)
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let candidate_roots = candidates
                .iter()
                .map(|candidate| independent_candidate_root_v1(&law.action_id_sha256, candidate))
                .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
            let valid = observations.len() == K2_LEARNED_SUPPORT_WORLD_COUNT_V1
                && survivors.as_slice() == [law.effect.clone()]
                && law.enumerated_candidate_count == candidates.len() as u64
                && law.enumerated_candidate_roots_sha256 == candidate_roots
                && law.rejected_candidate_count + law.version_space_size
                    == law.enumerated_candidate_count
                && law.version_space_size == 1;
            if valid {
                verified_support_laws += 1;
            } else {
                wrong_laws += 1;
            }
        }
        let mut verified_target_predictions = 0_u64;
        let mut wrong_predictions = 0_u64;
        for prediction in &predictions.predictions {
            let law = laws.law(&prediction.action_id_sha256);
            let valid = law.is_some_and(|law| {
                prediction.learned_law_root_sha256 == law.law_root_sha256
                    && independent_apply_effect_v1(
                        &prediction_request.target_pre_manifest,
                        &law.effect,
                    )
                    .is_ok_and(|manifest| manifest == prediction.predicted_terminal_manifest)
            });
            if valid {
                verified_target_predictions += 1;
            } else {
                wrong_predictions += 1;
            }
        }
        let mut receipt = Self {
            schema: K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1.to_owned(),
            verification_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            verifier_contract_root_sha256: freeze.independent_verifier_contract_root_sha256.clone(),
            public_context_root_sha256: learning_request
                .public_context
                .public_context_root_sha256
                .clone(),
            support_observation_set_root_sha256: learning_request
                .support_observations
                .observation_set_root_sha256
                .clone(),
            learned_law_set_root_sha256: laws.law_set_root_sha256.clone(),
            target_prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            verified_support_laws,
            verified_target_predictions,
            wrong_laws,
            wrong_predictions,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.verification_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.verification_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.verifier_contract_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_effect_verification_root_invalid")?;
        }
        if self.schema != K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1
            || self.verified_support_laws != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.verified_target_predictions != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.wrong_laws != 0
            || self.wrong_predictions != 0
            || self.verification_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_independent_effect_verification_failed",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.verifier_contract_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.verified_support_laws,
            self.verified_target_predictions,
            self.wrong_laws,
            self.wrong_predictions,
            &self.authority,
        ))
    }
}

fn independent_candidate_root_v1(
    action_id_sha256: &str,
    candidate: &K2LearnedEffectLawBodyV1,
) -> K2GoalEnvironmentResultV1<String> {
    learned_root_v1(&(
        K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
        "candidate",
        action_id_sha256,
        candidate,
    ))
}

fn independent_effect_candidates_v1(
    observation: &K2SupportObservationV1,
) -> K2GoalEnvironmentResultV1<Vec<K2LearnedEffectLawBodyV1>> {
    observation.validate_persisted_v1()?;
    let pre = observation
        .pre_work_manifest
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let post = observation
        .post_work_manifest
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let added = post
        .iter()
        .filter_map(|(path, entry)| (!pre.contains_key(path)).then_some(*entry))
        .collect::<Vec<_>>();
    let removed = pre
        .iter()
        .filter_map(|(path, entry)| (!post.contains_key(path)).then_some(*entry))
        .collect::<Vec<_>>();
    let changed = pre.iter().any(|(path, entry)| {
        post.get(path)
            .is_some_and(|post_entry| **post_entry != **entry)
    });
    let mut candidates = Vec::new();
    if added.len() == 1 && removed.is_empty() && !changed {
        let target = added[0];
        if target.kind == LawLabTreeEntryKindV1::File {
            candidates.extend(
                pre.values()
                    .filter(|source| {
                        source.kind == LawLabTreeEntryKindV1::File
                            && source.byte_length == target.byte_length
                            && source.content_sha256 == target.content_sha256
                            && source.executable == target.executable
                    })
                    .map(|source| K2LearnedEffectLawBodyV1::CopyFile {
                        source_path: source.relative_path.clone(),
                        target_path: target.relative_path.clone(),
                    }),
            );
        }
    }
    if removed.len() == 1
        && added.is_empty()
        && !changed
        && removed[0].kind == LawLabTreeEntryKindV1::File
    {
        candidates.push(K2LearnedEffectLawBodyV1::RemoveFile {
            path: removed[0].relative_path.clone(),
        });
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn independent_apply_effect_v1(
    pre: &LawLabTreeManifestV1,
    effect: &K2LearnedEffectLawBodyV1,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    pre.validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    effect.validate()?;
    let mut entries = pre.entries.clone();
    match effect {
        K2LearnedEffectLawBodyV1::CopyFile {
            source_path,
            target_path,
        } => {
            if entries
                .iter()
                .any(|entry| entry.relative_path == *target_path)
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_target_exists",
                ));
            }
            let mut copied = entries
                .iter()
                .find(|entry| {
                    entry.relative_path == *source_path && entry.kind == LawLabTreeEntryKindV1::File
                })
                .cloned()
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_copy_source_missing",
                ))?;
            copied.relative_path.clone_from(target_path);
            entries.push(copied);
        }
        K2LearnedEffectLawBodyV1::RemoveFile { path } => {
            let previous_len = entries.len();
            entries.retain(|entry| entry.relative_path != *path);
            if entries.len() + 1 != previous_len {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_remove_path_missing",
                ));
            }
        }
    }
    independent_seal_manifest_v1(entries)
}

fn independent_seal_manifest_v1(
    mut entries: Vec<LawLabTreeEntryV1>,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    entries.sort();
    let total_file_bytes = entries
        .iter()
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_length))
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_independent_manifest_bytes_overflow",
        ))?;
    #[derive(Serialize)]
    struct IndependentManifestDigestV1<'a> {
        schema: &'static str,
        total_file_bytes: u64,
        entries: &'a [LawLabTreeEntryV1],
    }
    let tree_root_sha256 = learned_root_v1(&IndependentManifestDigestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1,
        total_file_bytes,
        entries: &entries,
    })?;
    let manifest = LawLabTreeManifestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1.to_owned(),
        tree_root_sha256,
        total_file_bytes,
        entries,
    };
    manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    Ok(manifest)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedToV1BindingEntryV1 {
    pub schema: String,
    pub entry_root_sha256: String,
    pub opaque_action_id_sha256: String,
    pub learned_law_root_sha256: String,
    pub predicted_terminal_tree_root_sha256: String,
    pub v1_fixture_action_root_sha256: String,
    pub hidden_operation_plan_root_sha256: String,
    pub v1_predicted_consequence_root_sha256: String,
}

impl K2LearnedToV1BindingEntryV1 {
    fn seal(
        action_id_sha256: String,
        law_root_sha256: String,
        prediction_root_sha256: String,
        operation_plan_root_sha256: String,
        action: &K2K1ActionRefV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        action.validate()?;
        let mut entry = Self {
            schema: K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1.to_owned(),
            entry_root_sha256: String::new(),
            opaque_action_id_sha256: action_id_sha256,
            learned_law_root_sha256: law_root_sha256,
            predicted_terminal_tree_root_sha256: prediction_root_sha256,
            v1_fixture_action_root_sha256: action.action_root_sha256.clone(),
            hidden_operation_plan_root_sha256: operation_plan_root_sha256,
            v1_predicted_consequence_root_sha256: action.predicted_consequence_root_sha256.clone(),
        };
        entry.entry_root_sha256 = entry.expected_root_v1()?;
        entry.validate_persisted_v1()?;
        Ok(entry)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.entry_root_sha256.as_str(),
            self.opaque_action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            self.predicted_terminal_tree_root_sha256.as_str(),
            self.v1_fixture_action_root_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.v1_predicted_consequence_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_to_v1_entry_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1
            || self.predicted_terminal_tree_root_sha256 != self.v1_predicted_consequence_root_sha256
            || self.entry_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_root_mismatch",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1,
            self.opaque_action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            self.predicted_terminal_tree_root_sha256.as_str(),
            self.v1_fixture_action_root_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.v1_predicted_consequence_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedToV1BindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub independent_verification_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub entries: Vec<K2LearnedToV1BindingEntryV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedToV1BindingV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        freeze: &K2LearnedCapabilityFreezeV1,
        catalog: &K2OpaqueActionCatalogV1,
        mapping: &K2HiddenActionMappingV1,
        laws: &K2LearnedEffectLawSetV1,
        predictions: &K2LearnedTargetPredictionSetV1,
        verification: &K2LearnedEffectVerificationReceiptV1,
    ) -> K2GoalEnvironmentResultV1<(Self, Vec<K2K1ActionRefV1>)> {
        freeze.validate_persisted_v1()?;
        mapping.validate(catalog)?;
        laws.validate()?;
        predictions.validate()?;
        verification.validate_persisted_v1()?;
        if mapping.mapping_root_sha256 != freeze.hidden_mapping_root_sha256
            || laws.law_set_root_sha256 != verification.learned_law_set_root_sha256
            || predictions.prediction_set_root_sha256
                != verification.target_prediction_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_to_v1_input_binding_invalid",
            ));
        }
        let mut actions = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
        let mut entries = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
        for prediction in &predictions.predictions {
            let law =
                laws.law(&prediction.action_id_sha256)
                    .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_learned_to_v1_law_missing",
                    ))?;
            let hidden = mapping.entry(&prediction.action_id_sha256).ok_or(
                K2GoalEnvironmentErrorV1::Invalid("k2_learned_to_v1_mapping_missing"),
            )?;
            let action = K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
                provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
                applicability_environment_root_sha256: predictions
                    .target_pre_tree_root_sha256
                    .clone(),
                applicability_receipt_root_sha256: learned_root_v1(&(
                    K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
                    "applicability",
                    freeze.freeze_root_sha256.as_str(),
                    prediction.action_id_sha256.as_str(),
                ))?,
                operation_plan_root_sha256: hidden.operation_plan_root_sha256.clone(),
                predicted_consequence_root_sha256: prediction
                    .predicted_terminal_manifest
                    .tree_root_sha256
                    .clone(),
                fixture_effect_root_sha256: Some(law.law_root_sha256.clone()),
                law_certificate_root_sha256: None,
                epistemic_registry_member_root_sha256: None,
                bundle_v4_root_sha256: None,
                execution_certificate_root_sha256: None,
                applicability_guard_root_sha256: None,
                effect_contract_root_sha256: None,
                semantic_class_root_sha256: None,
                role_topology_root_sha256: None,
            })?;
            entries.push(K2LearnedToV1BindingEntryV1::seal(
                prediction.action_id_sha256.clone(),
                law.law_root_sha256.clone(),
                prediction
                    .predicted_terminal_manifest
                    .tree_root_sha256
                    .clone(),
                hidden.operation_plan_root_sha256.clone(),
                &action,
            )?);
            actions.push(action);
        }
        entries.sort_by(|left, right| {
            left.opaque_action_id_sha256
                .cmp(&right.opaque_action_id_sha256)
        });
        let mut binding = Self {
            schema: K2_LEARNED_TO_V1_BINDING_SCHEMA_V1.to_owned(),
            binding_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            hidden_mapping_root_sha256: mapping.mapping_root_sha256.clone(),
            learned_law_set_root_sha256: laws.law_set_root_sha256.clone(),
            target_prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            independent_verification_root_sha256: verification.verification_root_sha256.clone(),
            target_pre_tree_root_sha256: predictions.target_pre_tree_root_sha256.clone(),
            entries,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        binding.binding_root_sha256 = binding.expected_root_v1()?;
        binding.validate_persisted_v1()?;
        Ok((binding, actions))
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.binding_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.independent_verification_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_to_v1_binding_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TO_V1_BINDING_SCHEMA_V1
            || self.entries.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].opaque_action_id_sha256 >= pair[1].opaque_action_id_sha256)
            || self.binding_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_to_v1_binding_invalid",
            ));
        }
        for entry in &self.entries {
            entry.validate_persisted_v1()?;
        }
        require_unique_roots_v1(
            self.entries
                .iter()
                .map(|entry| entry.v1_fixture_action_root_sha256.as_str()),
            "k2_learned_to_v1_action_roots_not_unique",
        )
    }

    pub fn entry_for_v1_action(
        &self,
        action_root_sha256: &str,
    ) -> Option<&K2LearnedToV1BindingEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.v1_fixture_action_root_sha256 == action_root_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.independent_verification_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            &self.entries,
            &self.authority,
        ))
    }
}

pub struct K2V1EpisodeEvidenceInputV1<'a> {
    pub learned_binding: &'a K2LearnedToV1BindingV1,
    pub decision_freeze: &'a K2DecisionFreezeV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub law_lab_binding: &'a K2LawLabBindingV1,
    pub execution: &'a LawLabSandboxExecutionV1,
    pub exact_goal: &'a K2ExactGoalReceiptV1,
    pub outcome: &'a K2DecisionOutcomeReceiptV1,
    pub episode_seal: &'a K2DecisionEpisodeSealV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2V1EpisodeEvidenceV1 {
    pub schema: String,
    pub evidence_root_sha256: String,
    pub learned_to_v1_binding_root_sha256: String,
    pub v1_episode_id_sha256: String,
    pub v1_decision_freeze_root_sha256: String,
    pub v1_prediction_set_root_sha256: String,
    pub v1_selection_root_sha256: String,
    pub v1_selected_action_root_sha256: String,
    pub v1_law_lab_binding_root_sha256: String,
    pub v1_sandbox_receipt_root_sha256: String,
    pub v1_exact_goal_receipt_root_sha256: String,
    pub v1_terminal_outcome_root_sha256: String,
    pub v1_episode_seal_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2V1EpisodeEvidenceV1 {
    pub fn seal(input: K2V1EpisodeEvidenceInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.learned_binding.validate_persisted_v1()?;
        input.decision_freeze.validate_persisted_v1()?;
        input.predictions.validate_persisted_v1()?;
        input
            .selection
            .validate(input.decision_freeze, input.predictions)?;
        input.law_lab_binding.validate_persisted_v1()?;
        input.exact_goal.validate_persisted_v1()?;
        input.outcome.validate()?;
        input.episode_seal.validate()?;
        let selected_entry = input
            .learned_binding
            .entry_for_v1_action(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_selected_action_not_learned",
            ))?;
        let prediction = input
            .predictions
            .predictions
            .iter()
            .find(|prediction| {
                prediction.action_root_sha256 == input.selection.selected_action_root_sha256
            })
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_selected_prediction_missing",
            ))?;
        if selected_entry.predicted_terminal_tree_root_sha256
            != prediction.predicted_terminal_tree_root_sha256
            || input.law_lab_binding.selected_action_root_sha256
                != input.selection.selected_action_root_sha256
            || input.execution.receipt.request_root_sha256
                != input.law_lab_binding.law_lab_request_root_sha256
            || input.exact_goal.law_lab_binding_root_sha256
                != input.law_lab_binding.binding_root_sha256
            || input.exact_goal.law_lab_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.outcome.decision_freeze_root_sha256
                != input.decision_freeze.decision_freeze_root_sha256
            || input.outcome.prediction_set_root_sha256
                != input.predictions.prediction_set_root_sha256
            || input.outcome.law_lab_binding_root_sha256
                != input.law_lab_binding.binding_root_sha256
            || input.outcome.sandbox_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.outcome.exact_goal_receipt_root_sha256 != input.exact_goal.receipt_root_sha256
            || input.episode_seal.episode_id_sha256 != input.decision_freeze.episode_id_sha256
            || input.episode_seal.outcome_root_sha256 != input.outcome.outcome_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_episode_evidence_binding_invalid",
            ));
        }
        let mut evidence = Self {
            schema: K2_V1_EPISODE_EVIDENCE_SCHEMA_V1.to_owned(),
            evidence_root_sha256: String::new(),
            learned_to_v1_binding_root_sha256: input.learned_binding.binding_root_sha256.clone(),
            v1_episode_id_sha256: input.decision_freeze.episode_id_sha256.clone(),
            v1_decision_freeze_root_sha256: input
                .decision_freeze
                .decision_freeze_root_sha256
                .clone(),
            v1_prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            v1_selection_root_sha256: input.selection.selection_root_sha256.clone(),
            v1_selected_action_root_sha256: input.selection.selected_action_root_sha256.clone(),
            v1_law_lab_binding_root_sha256: input.law_lab_binding.binding_root_sha256.clone(),
            v1_sandbox_receipt_root_sha256: input.execution.receipt.receipt_root_sha256.clone(),
            v1_exact_goal_receipt_root_sha256: input.exact_goal.receipt_root_sha256.clone(),
            v1_terminal_outcome_root_sha256: input.outcome.outcome_root_sha256.clone(),
            v1_episode_seal_root_sha256: input.episode_seal.seal_root_sha256.clone(),
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        evidence.evidence_root_sha256 = evidence.expected_root_v1()?;
        evidence.validate_persisted_v1()?;
        Ok(evidence)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.evidence_root_sha256.as_str(),
            self.learned_to_v1_binding_root_sha256.as_str(),
            self.v1_episode_id_sha256.as_str(),
            self.v1_decision_freeze_root_sha256.as_str(),
            self.v1_prediction_set_root_sha256.as_str(),
            self.v1_selection_root_sha256.as_str(),
            self.v1_selected_action_root_sha256.as_str(),
            self.v1_law_lab_binding_root_sha256.as_str(),
            self.v1_sandbox_receipt_root_sha256.as_str(),
            self.v1_exact_goal_receipt_root_sha256.as_str(),
            self.v1_terminal_outcome_root_sha256.as_str(),
            self.v1_episode_seal_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_v1_episode_evidence_root_invalid")?;
        }
        if self.schema != K2_V1_EPISODE_EVIDENCE_SCHEMA_V1
            || self.evidence_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_episode_evidence_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_V1_EPISODE_EVIDENCE_SCHEMA_V1,
            (
                self.learned_to_v1_binding_root_sha256.as_str(),
                self.v1_episode_id_sha256.as_str(),
                self.v1_decision_freeze_root_sha256.as_str(),
                self.v1_prediction_set_root_sha256.as_str(),
                self.v1_selection_root_sha256.as_str(),
                self.v1_selected_action_root_sha256.as_str(),
            ),
            (
                self.v1_law_lab_binding_root_sha256.as_str(),
                self.v1_sandbox_receipt_root_sha256.as_str(),
                self.v1_exact_goal_receipt_root_sha256.as_str(),
                self.v1_terminal_outcome_root_sha256.as_str(),
                self.v1_episode_seal_root_sha256.as_str(),
                &self.authority,
            ),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PrivateExperimentArtifactReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub private_contract_root_sha256: String,
    pub artifact_root_sha256: String,
    pub artifact_bytes_sha256: String,
    pub artifact_bytes: u64,
    pub file_mode: u32,
    pub file_synced: bool,
    pub directory_synced: bool,
    pub no_replace_publication: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2PrivateExperimentArtifactReceiptV1 {
    fn seal(
        contract: &K2PrivateExperimentContractV1,
        bytes: &[u8],
    ) -> K2GoalEnvironmentResultV1<Self> {
        let artifact_root_sha256 = learned_root_v1(contract)?;
        let mut receipt = Self {
            schema: K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            private_contract_root_sha256: contract.private_contract_root_sha256.clone(),
            artifact_root_sha256: artifact_root_sha256.clone(),
            artifact_bytes_sha256: artifact_root_sha256,
            artifact_bytes: u64::try_from(bytes.len()).map_err(|_| {
                K2GoalEnvironmentErrorV1::Invalid("k2_private_artifact_size_overflow")
            })?,
            file_mode: 0o400,
            file_synced: true,
            directory_synced: true,
            no_replace_publication: true,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.private_contract_root_sha256.as_str(),
            self.artifact_root_sha256.as_str(),
            self.artifact_bytes_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_artifact_root_invalid")?;
        }
        if self.schema != K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1
            || self.artifact_root_sha256 != self.artifact_bytes_sha256
            || self.artifact_bytes == 0
            || self.file_mode != 0o400
            || !self.file_synced
            || !self.directory_synced
            || !self.no_replace_publication
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_private_artifact_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1,
            self.private_contract_root_sha256.as_str(),
            self.artifact_root_sha256.as_str(),
            self.artifact_bytes_sha256.as_str(),
            self.artifact_bytes,
            self.file_mode,
            self.file_synced,
            self.directory_synced,
            self.no_replace_publication,
            &self.authority,
        ))
    }
}

pub fn publish_private_experiment_contract_v1(
    path: &Path,
    contract: &K2PrivateExperimentContractV1,
) -> K2GoalEnvironmentResultV1<K2PrivateExperimentArtifactReceiptV1> {
    let parent = path.parent().ok_or(K2GoalEnvironmentErrorV1::Invalid(
        "k2_private_artifact_parent_missing",
    ))?;
    if !parent.is_dir() || path.file_name().is_none() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_path_invalid",
        ));
    }
    let bytes = contract.canonical_bytes_v1()?;
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or(
        K2GoalEnvironmentErrorV1::Invalid("k2_private_artifact_name_invalid"),
    )?;
    let temp_path = parent.join(format!(".{file_name}.tmp"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(io_error_learned_v1("create_private_artifact_temp"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_error_learned_v1("sync_private_artifact_temp"))?;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o400))
            .map_err(io_error_learned_v1("chmod_private_artifact_temp"))?;
        drop(file);
        fs::hard_link(&temp_path, path)
            .map_err(io_error_learned_v1("publish_private_artifact_no_replace"))?;
        sync_directory_learned_v1(parent)?;
        fs::remove_file(&temp_path).map_err(io_error_learned_v1("remove_private_artifact_temp"))?;
        sync_directory_learned_v1(parent)?;
        let receipt = K2PrivateExperimentArtifactReceiptV1::seal(contract, &bytes)?;
        verify_private_artifact_file_v1(path, &receipt)?;
        Ok(receipt)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn reopen_private_experiment_contract_v1(
    path: &Path,
    receipt: &K2PrivateExperimentArtifactReceiptV1,
    context: &K2LearnerPublicContextV1,
    catalog: &K2OpaqueActionCatalogV1,
    support: &K2SupportWorldSetV1,
) -> K2GoalEnvironmentResultV1<K2PrivateExperimentContractV1> {
    receipt.validate_persisted_v1()?;
    verify_private_artifact_file_v1(path, receipt)?;
    let bytes = fs::read(path).map_err(io_error_learned_v1("read_private_artifact"))?;
    if bytes.len() as u64 != receipt.artifact_bytes {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_size_mismatch",
        ));
    }
    let contract = parse_canonical_v1::<K2PrivateExperimentContractV1>(
        &bytes,
        K2_LEARNER_MAX_REQUEST_BYTES_V1,
        "k2_private_artifact_decode_invalid",
    )?;
    contract.validate(context, catalog, support)?;
    if contract.private_contract_root_sha256 != receipt.private_contract_root_sha256
        || learned_root_v1(&contract)? != receipt.artifact_root_sha256
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_reopen_mismatch",
        ));
    }
    Ok(contract)
}

fn verify_private_artifact_file_v1(
    path: &Path,
    receipt: &K2PrivateExperimentArtifactReceiptV1,
) -> K2GoalEnvironmentResultV1<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(io_error_learned_v1("stat_private_artifact"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o777 != receipt.file_mode
        || metadata.len() != receipt.artifact_bytes
        || law_lab_sha256_file_v1(path)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?
            != receipt.artifact_bytes_sha256
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_file_invalid",
        ));
    }
    Ok(())
}

fn sync_directory_learned_v1(path: &Path) -> K2GoalEnvironmentResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(io_error_learned_v1("sync_directory"))
}

fn io_error_learned_v1(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> K2GoalEnvironmentErrorV1 {
    move |error| K2GoalEnvironmentErrorV1::Io(format!("{operation}:{error}"))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearnerProcessReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub protocol_request_root_sha256: String,
    pub protocol_outcome_root_sha256: String,
    pub request_bytes: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub elapsed_ms: u64,
    pub wall_limit_ms: u64,
    pub cpu_limit_seconds: u64,
    pub address_space_limit_bytes: u64,
    pub process_limit: u64,
    pub environment_cleared: bool,
    pub network_enabled: bool,
    pub repository_mounted: bool,
    pub private_contract_mounted: bool,
    pub target_store_mounted: bool,
    pub bwrap_sha256: String,
    pub prlimit_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2EffectLearnerProcessReceiptV1 {
    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.protocol_request_root_sha256.as_str(),
            self.protocol_outcome_root_sha256.as_str(),
            self.bwrap_sha256.as_str(),
            self.prlimit_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learner_process_root_invalid")?;
        }
        if self.schema != K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1
            || self.request_bytes == 0
            || self.request_bytes > K2_LEARNER_MAX_REQUEST_BYTES_V1 as u64
            || self.stdout_bytes == 0
            || self.stdout_bytes > K2_LEARNER_MAX_OUTCOME_BYTES_V1 as u64
            || self.stderr_bytes != 0
            || self.elapsed_ms > K2_LEARNER_WALL_MS_V1
            || self.wall_limit_ms != K2_LEARNER_WALL_MS_V1
            || self.cpu_limit_seconds != K2_LEARNER_CPU_SECONDS_V1
            || self.address_space_limit_bytes != K2_LEARNER_ADDRESS_SPACE_BYTES_V1
            || self.process_limit != K2_LEARNER_PROCESS_COUNT_V1
            || !self.environment_cleared
            || self.network_enabled
            || self.repository_mounted
            || self.private_contract_mounted
            || self.target_store_mounted
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_process_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1,
            (
                self.learner_manifest_root_sha256.as_str(),
                self.learner_executable_sha256.as_str(),
                self.protocol_request_root_sha256.as_str(),
                self.protocol_outcome_root_sha256.as_str(),
                self.request_bytes,
                self.stdout_bytes,
                self.stderr_bytes,
                self.elapsed_ms,
            ),
            (
                self.wall_limit_ms,
                self.cpu_limit_seconds,
                self.address_space_limit_bytes,
                self.process_limit,
                self.environment_cleared,
                self.network_enabled,
                self.repository_mounted,
                self.private_contract_mounted,
                self.target_store_mounted,
                self.bwrap_sha256.as_str(),
                self.prlimit_sha256.as_str(),
                &self.authority,
            ),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2EffectLearnerRunnerV1 {
    learner_path: PathBuf,
    bwrap_path: PathBuf,
    prlimit_path: PathBuf,
}

impl K2EffectLearnerRunnerV1 {
    #[must_use]
    pub fn new(learner_path: PathBuf) -> Self {
        Self {
            learner_path,
            bwrap_path: PathBuf::from("/usr/bin/bwrap"),
            prlimit_path: PathBuf::from("/usr/bin/prlimit"),
        }
    }

    pub fn learner_manifest_v1(&self) -> K2GoalEnvironmentResultV1<K2EffectLearnerManifestV1> {
        validate_executable_learned_v1(&self.learner_path)?;
        K2EffectLearnerManifestV1::seal(
            law_lab_sha256_file_v1(&self.learner_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
        )
    }

    pub fn run_v1(
        &self,
        frozen_manifest: &K2EffectLearnerManifestV1,
        request: &K2EffectLearnerProtocolRequestV1,
    ) -> K2GoalEnvironmentResultV1<(
        K2EffectLearnerProtocolOutcomeV1,
        K2EffectLearnerProcessReceiptV1,
    )> {
        frozen_manifest.validate()?;
        validate_executable_learned_v1(&self.bwrap_path)?;
        validate_executable_learned_v1(&self.prlimit_path)?;
        validate_executable_learned_v1(&self.learner_path)?;
        let actual_learner_sha = law_lab_sha256_file_v1(&self.learner_path)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        if actual_learner_sha != frozen_manifest.executable_sha256 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_hash_mismatch",
            ));
        }
        let input = request.canonical_bytes_v1()?;
        if input.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_request_budget_exhausted",
            ));
        }
        let args = self.command_args_v1();
        let started = Instant::now();
        let (stdout, stderr) = run_bounded_learner_process_v1(
            &self.bwrap_path,
            &args,
            &input,
            Duration::from_millis(K2_LEARNER_WALL_MS_V1),
        )?;
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !stderr.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_stderr_not_empty",
            ));
        }
        let outcome = K2EffectLearnerProtocolOutcomeV1::from_canonical_bytes_v1(&stdout)?;
        validate_protocol_binding_v1(request, &outcome)?;
        let mut receipt = K2EffectLearnerProcessReceiptV1 {
            schema: K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            learner_manifest_root_sha256: frozen_manifest.manifest_root_sha256.clone(),
            learner_executable_sha256: actual_learner_sha,
            protocol_request_root_sha256: protocol_request_root_v1(request).to_owned(),
            protocol_outcome_root_sha256: protocol_outcome_root_v1(&outcome).to_owned(),
            request_bytes: input.len() as u64,
            stdout_bytes: stdout.len() as u64,
            stderr_bytes: stderr.len() as u64,
            elapsed_ms,
            wall_limit_ms: K2_LEARNER_WALL_MS_V1,
            cpu_limit_seconds: K2_LEARNER_CPU_SECONDS_V1,
            address_space_limit_bytes: K2_LEARNER_ADDRESS_SPACE_BYTES_V1,
            process_limit: K2_LEARNER_PROCESS_COUNT_V1,
            environment_cleared: true,
            network_enabled: false,
            repository_mounted: false,
            private_contract_mounted: false,
            target_store_mounted: false,
            bwrap_sha256: law_lab_sha256_file_v1(&self.bwrap_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
            prlimit_sha256: law_lab_sha256_file_v1(&self.prlimit_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok((outcome, receipt))
    }

    fn command_args_v1(&self) -> Vec<OsString> {
        const GUEST_LEARNER: &str = "/nando/bin/nando-k2-effect-learner";
        let mut args = vec![
            OsString::from("--unshare-all"),
            OsString::from("--die-with-parent"),
            OsString::from("--new-session"),
            OsString::from("--cap-drop"),
            OsString::from("ALL"),
            OsString::from("--clearenv"),
        ];
        for path in ["/usr", "/lib", "/lib64"]
            .into_iter()
            .filter(|path| Path::new(path).exists())
        {
            args.extend([
                OsString::from("--ro-bind"),
                OsString::from(path),
                OsString::from(path),
            ]);
        }
        args.extend([
            OsString::from("--proc"),
            OsString::from("/proc"),
            OsString::from("--dev"),
            OsString::from("/dev"),
            OsString::from("--tmpfs"),
            OsString::from("/tmp"),
            OsString::from("--dir"),
            OsString::from("/nando"),
            OsString::from("--dir"),
            OsString::from("/nando/bin"),
            OsString::from("--ro-bind"),
            self.learner_path.as_os_str().to_owned(),
            OsString::from(GUEST_LEARNER),
            OsString::from("--chdir"),
            OsString::from("/tmp"),
            OsString::from("--setenv"),
            OsString::from("LANG"),
            OsString::from("C"),
            OsString::from("--setenv"),
            OsString::from("LC_ALL"),
            OsString::from("C"),
            OsString::from("--setenv"),
            OsString::from("TZ"),
            OsString::from("UTC"),
            OsString::from("--"),
            self.prlimit_path.as_os_str().to_owned(),
            OsString::from(format!("--cpu={0}:{0}", K2_LEARNER_CPU_SECONDS_V1)),
            OsString::from(format!("--as={0}:{0}", K2_LEARNER_ADDRESS_SPACE_BYTES_V1)),
            OsString::from(format!("--nproc={0}:{0}", K2_LEARNER_PROCESS_COUNT_V1)),
            OsString::from(format!("--fsize={0}:{0}", K2_LEARNER_MAX_OUTCOME_BYTES_V1)),
            OsString::from("--"),
            OsString::from(GUEST_LEARNER),
        ]);
        args
    }
}

fn validate_executable_learned_v1(path: &Path) -> K2GoalEnvironmentResultV1<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(io_error_learned_v1("stat_learned_executable"))?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_executable_invalid",
        ));
    }
    Ok(())
}

fn protocol_request_root_v1(request: &K2EffectLearnerProtocolRequestV1) -> &str {
    match request {
        K2EffectLearnerProtocolRequestV1::LearnEffects(value) => &value.request_root_sha256,
        K2EffectLearnerProtocolRequestV1::PredictTarget(value) => &value.request_root_sha256,
        K2EffectLearnerProtocolRequestV1::EvaluateGeneratedAblation(value) => {
            &value.request_root_sha256
        }
    }
}

fn protocol_outcome_root_v1(outcome: &K2EffectLearnerProtocolOutcomeV1) -> &str {
    match outcome {
        K2EffectLearnerProtocolOutcomeV1::LearnedEffects(value) => &value.law_set_root_sha256,
        K2EffectLearnerProtocolOutcomeV1::TargetPredictions(value) => {
            &value.prediction_set_root_sha256
        }
        K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(value) => &value.outcome_root_sha256,
    }
}

fn validate_protocol_binding_v1(
    request: &K2EffectLearnerProtocolRequestV1,
    outcome: &K2EffectLearnerProtocolOutcomeV1,
) -> K2GoalEnvironmentResultV1<()> {
    let valid = match (request, outcome) {
        (
            K2EffectLearnerProtocolRequestV1::LearnEffects(request),
            K2EffectLearnerProtocolOutcomeV1::LearnedEffects(outcome),
        ) => outcome.learning_request_root_sha256 == request.request_root_sha256,
        (
            K2EffectLearnerProtocolRequestV1::PredictTarget(request),
            K2EffectLearnerProtocolOutcomeV1::TargetPredictions(outcome),
        ) => outcome.target_prediction_request_root_sha256 == request.request_root_sha256,
        (
            K2EffectLearnerProtocolRequestV1::EvaluateGeneratedAblation(request),
            K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(outcome),
        ) => outcome.request_root_sha256 == request.request_root_sha256,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_protocol_binding_invalid",
        ))
    }
}

fn run_bounded_learner_process_v1(
    program: &Path,
    args: &[OsString],
    input: &[u8],
    deadline: Duration,
) -> K2GoalEnvironmentResultV1<(Vec<u8>, Vec<u8>)> {
    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(io_error_learned_v1("spawn_effect_learner"))?;
    child
        .stdin
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stdin_missing",
        ))?
        .write_all(input)
        .map_err(io_error_learned_v1("write_effect_learner_stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stdout_missing",
        ))?;
    let stderr = child
        .stderr
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stderr_missing",
        ))?;
    let stdout_reader = thread::spawn(move || {
        read_limited_pipe_learned_v1(stdout, K2_LEARNER_MAX_OUTCOME_BYTES_V1)
    });
    let stderr_reader =
        thread::spawn(move || read_limited_pipe_learned_v1(stderr, K2_LEARNER_MAX_STDERR_BYTES_V1));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(io_error_learned_v1("poll_effect_learner"))?
        {
            break status;
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_timed_out",
            ));
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_effect_learner_stdout_join_failed"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_effect_learner_stderr_join_failed"))??;
    if !status.success() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_process_failed",
        ));
    }
    Ok((stdout, stderr))
}

fn read_limited_pipe_learned_v1(
    mut pipe: impl Read,
    maximum_bytes: usize,
) -> K2GoalEnvironmentResultV1<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut exceeded = false;
    loop {
        let read = pipe
            .read(&mut buffer)
            .map_err(io_error_learned_v1("read_effect_learner_pipe"))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) <= maximum_bytes {
            output.extend_from_slice(&buffer[..read]);
        } else {
            exceeded = true;
        }
    }
    if exceeded {
        Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_pipe_budget_exhausted",
        ))
    } else {
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedAblationKindV1 {
    SupportCount,
    ActionIdentityShuffle,
    AmbiguousCopySource,
    ConstantOutput,
    OutcomeDependence,
    DynamicId,
    HoldoutAlias,
    SupportProvenanceMismatch,
    TargetGoalLeakage,
    PredictionTamper,
    WrongActionExactOracle,
    CrossExperimentReplay,
    AuthorityTamper,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedAblationVerdictV1 {
    InsufficientSupport,
    NonTransferableDelta,
    AmbiguousSourceMatch,
    TransferableWithDynamicIds,
    TargetNotIndependent,
    SupportEvidenceInvalid,
    LearnerRequestPrivateFieldRejected,
    TargetPredictionRootMismatch,
    ExactGoalUnsatisfied,
    CrossExperimentReplay,
    AuthorityBoundaryViolated,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationLearnedEffectV1 {
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub request_root_sha256: String,
    pub observed_verdict: K2LearnedAblationVerdictV1,
    pub rejection_code: Option<String>,
    pub learned_effects: Vec<K2GeneratedAblationLearnedEffectV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2GeneratedAblationOutcomeV1 {
    pub fn evaluate(request: &K2GeneratedAblationRequestV1) -> K2GoalEnvironmentResultV1<Self> {
        request.validate()?;
        let observations = request
            .observations
            .iter()
            .map(|observation| K2EffectObservationViewV1 {
                action_id_sha256: &observation.action_id_sha256,
                pre_work_manifest: &observation.pre_work_manifest,
                post_work_manifest: &observation.post_work_manifest,
            })
            .collect::<Vec<_>>();
        let (observed_verdict, rejection_code, learned_effects) =
            match infer_effects_v1(&request.catalog.action_ids_sha256, &observations) {
                Ok(inferred) => (
                    K2LearnedAblationVerdictV1::TransferableWithDynamicIds,
                    None,
                    inferred
                        .into_iter()
                        .map(|value| K2GeneratedAblationLearnedEffectV1 {
                            action_id_sha256: value.action_id_sha256,
                            effect: value.effect,
                        })
                        .collect(),
                ),
                Err(K2GoalEnvironmentErrorV1::Invalid(reason)) => {
                    let verdict = generated_ablation_verdict_for_rejection_v1(reason)?;
                    (verdict, Some(reason.to_owned()), Vec::new())
                }
                Err(error) => return Err(error),
            };
        let mut outcome = Self {
            schema: K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            request_root_sha256: request.request_root_sha256.clone(),
            observed_verdict,
            rejection_code,
            learned_effects,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.learned_effects.sort();
        outcome.outcome_root_sha256 = outcome.expected_root_v1()?;
        outcome.validate_persisted_v1()?;
        Ok(outcome)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.outcome_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_generated_ablation_outcome_root_invalid")?;
        }
        let result_shape_valid = match self.observed_verdict {
            K2LearnedAblationVerdictV1::TransferableWithDynamicIds => {
                self.rejection_code.is_none()
                    && self.learned_effects.len() == K2_LEARNED_ACTION_COUNT_V1
                    && self
                        .learned_effects
                        .windows(2)
                        .all(|pair| pair[0].action_id_sha256 < pair[1].action_id_sha256)
                    && self.learned_effects.iter().all(|value| {
                        valid_nonzero_sha256(&value.action_id_sha256)
                            && value.effect.validate().is_ok()
                    })
            }
            K2LearnedAblationVerdictV1::InsufficientSupport
            | K2LearnedAblationVerdictV1::NonTransferableDelta
            | K2LearnedAblationVerdictV1::AmbiguousSourceMatch => {
                self.learned_effects.is_empty()
                    && self.rejection_code.as_deref()
                        == generated_ablation_rejection_code_v1(self.observed_verdict)
            }
            _ => false,
        };
        if self.schema != K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1
            || !result_shape_valid
            || self.outcome_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_generated_ablation_outcome_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate_persisted_v1()?;
        learned_bytes_v1(self)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1,
            self.request_root_sha256.as_str(),
            self.observed_verdict,
            self.rejection_code.as_deref(),
            &self.learned_effects,
            &self.authority,
        ))
    }
}

fn generated_ablation_verdict_for_rejection_v1(
    reason: &'static str,
) -> K2GoalEnvironmentResultV1<K2LearnedAblationVerdictV1> {
    match reason {
        "k2_insufficient_support" => Ok(K2LearnedAblationVerdictV1::InsufficientSupport),
        "k2_non_transferable_delta" | "k2_effect_values_not_transferable" => {
            Ok(K2LearnedAblationVerdictV1::NonTransferableDelta)
        }
        "k2_ambiguous_source_match" => Ok(K2LearnedAblationVerdictV1::AmbiguousSourceMatch),
        _ => Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_generated_ablation_unexpected_rejection",
        )),
    }
}

fn generated_ablation_rejection_code_v1(
    verdict: K2LearnedAblationVerdictV1,
) -> Option<&'static str> {
    match verdict {
        K2LearnedAblationVerdictV1::InsufficientSupport => Some("k2_insufficient_support"),
        K2LearnedAblationVerdictV1::NonTransferableDelta => Some("k2_non_transferable_delta"),
        K2LearnedAblationVerdictV1::AmbiguousSourceMatch => Some("k2_ambiguous_source_match"),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedAblationControlV1 {
    pub schema: String,
    pub control_root_sha256: String,
    pub kind: K2LearnedAblationKindV1,
    pub input_root_sha256: String,
    pub expected_verdict: K2LearnedAblationVerdictV1,
    pub observed_verdict: K2LearnedAblationVerdictV1,
    pub learner_processes: u64,
    pub sandbox_probes: u64,
    pub oracle_invocations: u64,
    pub canonical_outcome_root_sha256: String,
    pub passed: bool,
}

impl K2LearnedAblationControlV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        kind: K2LearnedAblationKindV1,
        input_root_sha256: String,
        expected_verdict: K2LearnedAblationVerdictV1,
        observed_verdict: K2LearnedAblationVerdictV1,
        learner_processes: u64,
        sandbox_probes: u64,
        oracle_invocations: u64,
        canonical_outcome_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let mut control = Self {
            schema: K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1.to_owned(),
            control_root_sha256: String::new(),
            kind,
            input_root_sha256,
            expected_verdict,
            observed_verdict,
            learner_processes,
            sandbox_probes,
            oracle_invocations,
            canonical_outcome_root_sha256,
            passed: expected_verdict == observed_verdict,
        };
        control.control_root_sha256 = control.expected_root_v1()?;
        control.validate_persisted_v1()?;
        Ok(control)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.control_root_sha256.as_str(),
            self.input_root_sha256.as_str(),
            self.canonical_outcome_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_control_root_invalid")?;
        }
        if self.schema != K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1
            || self.expected_verdict != required_ablation_verdict_v1(self.kind)
            || self.observed_verdict != self.expected_verdict
            || !self.passed
            || self.learner_processes > 1
            || self.sandbox_probes > 1
            || self.oracle_invocations > 1
            || self.control_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_control_failed",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1,
            self.kind,
            self.input_root_sha256.as_str(),
            self.expected_verdict,
            self.observed_verdict,
            self.learner_processes,
            self.sandbox_probes,
            self.oracle_invocations,
            self.canonical_outcome_root_sha256.as_str(),
            self.passed,
        ))
    }
}

fn required_ablation_verdict_v1(kind: K2LearnedAblationKindV1) -> K2LearnedAblationVerdictV1 {
    match kind {
        K2LearnedAblationKindV1::SupportCount => K2LearnedAblationVerdictV1::InsufficientSupport,
        K2LearnedAblationKindV1::ActionIdentityShuffle
        | K2LearnedAblationKindV1::ConstantOutput
        | K2LearnedAblationKindV1::OutcomeDependence => {
            K2LearnedAblationVerdictV1::NonTransferableDelta
        }
        K2LearnedAblationKindV1::AmbiguousCopySource => {
            K2LearnedAblationVerdictV1::AmbiguousSourceMatch
        }
        K2LearnedAblationKindV1::DynamicId => {
            K2LearnedAblationVerdictV1::TransferableWithDynamicIds
        }
        K2LearnedAblationKindV1::HoldoutAlias => K2LearnedAblationVerdictV1::TargetNotIndependent,
        K2LearnedAblationKindV1::SupportProvenanceMismatch => {
            K2LearnedAblationVerdictV1::SupportEvidenceInvalid
        }
        K2LearnedAblationKindV1::TargetGoalLeakage => {
            K2LearnedAblationVerdictV1::LearnerRequestPrivateFieldRejected
        }
        K2LearnedAblationKindV1::PredictionTamper => {
            K2LearnedAblationVerdictV1::TargetPredictionRootMismatch
        }
        K2LearnedAblationKindV1::WrongActionExactOracle => {
            K2LearnedAblationVerdictV1::ExactGoalUnsatisfied
        }
        K2LearnedAblationKindV1::CrossExperimentReplay => {
            K2LearnedAblationVerdictV1::CrossExperimentReplay
        }
        K2LearnedAblationKindV1::AuthorityTamper => {
            K2LearnedAblationVerdictV1::AuthorityBoundaryViolated
        }
    }
}

fn required_ablation_kinds_v1() -> [K2LearnedAblationKindV1; 13] {
    [
        K2LearnedAblationKindV1::SupportCount,
        K2LearnedAblationKindV1::ActionIdentityShuffle,
        K2LearnedAblationKindV1::AmbiguousCopySource,
        K2LearnedAblationKindV1::ConstantOutput,
        K2LearnedAblationKindV1::OutcomeDependence,
        K2LearnedAblationKindV1::DynamicId,
        K2LearnedAblationKindV1::HoldoutAlias,
        K2LearnedAblationKindV1::SupportProvenanceMismatch,
        K2LearnedAblationKindV1::TargetGoalLeakage,
        K2LearnedAblationKindV1::PredictionTamper,
        K2LearnedAblationKindV1::WrongActionExactOracle,
        K2LearnedAblationKindV1::CrossExperimentReplay,
        K2LearnedAblationKindV1::AuthorityTamper,
    ]
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedAblationReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub controls: Vec<K2LearnedAblationControlV1>,
    pub learner_processes: u64,
    pub sandbox_probes: u64,
    pub oracle_invocations: u64,
    pub canonical_bytes: u64,
    pub all_passed: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedAblationReceiptV1 {
    pub fn seal(
        freeze: &K2LearnedCapabilityFreezeV1,
        mut controls: Vec<K2LearnedAblationControlV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        controls.sort_by_key(|control| control.kind);
        for control in &controls {
            control.validate_persisted_v1()?;
        }
        let learner_processes = controls
            .iter()
            .map(|control| control.learner_processes)
            .sum();
        let sandbox_probes = controls.iter().map(|control| control.sandbox_probes).sum();
        let oracle_invocations = controls
            .iter()
            .map(|control| control.oracle_invocations)
            .sum();
        let canonical_bytes = controls.iter().try_fold(0_u64, |total, control| {
            let bytes = learned_bytes_v1(control)?.len() as u64;
            total
                .checked_add(bytes)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_ablation_bytes_overflow",
                ))
        })?;
        let mut receipt = Self {
            schema: K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            controls,
            learner_processes,
            sandbox_probes,
            oracle_invocations,
            canonical_bytes,
            all_passed: true,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_receipt_root_invalid")?;
        }
        for control in &self.controls {
            control.validate_persisted_v1()?;
        }
        let observed_kinds = self
            .controls
            .iter()
            .map(|control| control.kind)
            .collect::<Vec<_>>();
        let learner_processes = self
            .controls
            .iter()
            .map(|control| control.learner_processes)
            .sum::<u64>();
        let sandbox_probes = self
            .controls
            .iter()
            .map(|control| control.sandbox_probes)
            .sum::<u64>();
        let oracle_invocations = self
            .controls
            .iter()
            .map(|control| control.oracle_invocations)
            .sum::<u64>();
        let canonical_bytes = self.controls.iter().try_fold(0_u64, |total, control| {
            total
                .checked_add(learned_bytes_v1(control)?.len() as u64)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_ablation_bytes_overflow",
                ))
        })?;
        if self.schema != K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1
            || observed_kinds != required_ablation_kinds_v1()
            || self.learner_processes != learner_processes
            || self.learner_processes > 8
            || self.sandbox_probes != sandbox_probes
            || self.sandbox_probes != 1
            || self.oracle_invocations != oracle_invocations
            || self.oracle_invocations != 1
            || self.canonical_bytes != canonical_bytes
            || self.canonical_bytes > 2 * 1024 * 1024
            || !self.all_passed
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_ablation_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            &self.controls,
            self.learner_processes,
            self.sandbox_probes,
            self.oracle_invocations,
            self.canonical_bytes,
            self.all_passed,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedCapabilityEvidenceClassV1 {
    CapabilityPass,
    LearningNegative,
    InfrastructureFailure,
    IndeterminateAfterDispatch,
}

pub struct K2LearnedCapabilityOutcomeInputV1<'a> {
    pub freeze: &'a K2LearnedCapabilityFreezeV1,
    pub dispatches: &'a [K2SupportDispatchV1],
    pub observations: &'a K2SupportObservationSetV1,
    pub learning_request: &'a K2EffectLearningRequestV1,
    pub laws: &'a K2LearnedEffectLawSetV1,
    pub independence: &'a K2TargetIndependenceReceiptV1,
    pub prediction_request: &'a K2TargetPredictionRequestV1,
    pub predictions: &'a K2LearnedTargetPredictionSetV1,
    pub verification: &'a K2LearnedEffectVerificationReceiptV1,
    pub v1_binding: &'a K2LearnedToV1BindingV1,
    pub v1_episode: &'a K2V1EpisodeEvidenceV1,
    pub ablations: &'a K2LearnedAblationReceiptV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub support_dispatch_roots_sha256: Vec<String>,
    pub support_observation_roots_sha256: Vec<String>,
    pub support_evidence_set_root_sha256: String,
    pub learning_request_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_independence_receipt_root_sha256: String,
    pub target_prediction_request_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub independent_verification_root_sha256: String,
    pub learned_to_v1_binding_root_sha256: String,
    pub v1_decision_freeze_root_sha256: String,
    pub v1_prediction_set_root_sha256: String,
    pub v1_selection_root_sha256: String,
    pub v1_law_lab_binding_root_sha256: String,
    pub v1_sandbox_receipt_root_sha256: String,
    pub v1_exact_goal_receipt_root_sha256: String,
    pub v1_terminal_outcome_root_sha256: String,
    pub v1_episode_seal_root_sha256: String,
    pub ablation_receipt_root_sha256: String,
    pub support_worlds: u64,
    pub support_executions: u64,
    pub learned_laws: u64,
    pub target_predictions: u64,
    pub wrong_predictions: u64,
    pub verdict: String,
    pub evidence_class: K2LearnedCapabilityEvidenceClassV1,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2LearnedCapabilityOutcomeDigestV1<'a> {
    schema: &'static str,
    experiment_freeze_root_sha256: &'a str,
    support_dispatch_roots_sha256: &'a [String],
    support_observation_roots_sha256: &'a [String],
    support_evidence_set_root_sha256: &'a str,
    learning_request_root_sha256: &'a str,
    learned_law_set_root_sha256: &'a str,
    target_independence_receipt_root_sha256: &'a str,
    target_prediction_request_root_sha256: &'a str,
    target_prediction_set_root_sha256: &'a str,
    independent_verification_root_sha256: &'a str,
    learned_to_v1_binding_root_sha256: &'a str,
    v1_decision_freeze_root_sha256: &'a str,
    v1_prediction_set_root_sha256: &'a str,
    v1_selection_root_sha256: &'a str,
    v1_law_lab_binding_root_sha256: &'a str,
    v1_sandbox_receipt_root_sha256: &'a str,
    v1_exact_goal_receipt_root_sha256: &'a str,
    v1_terminal_outcome_root_sha256: &'a str,
    v1_episode_seal_root_sha256: &'a str,
    ablation_receipt_root_sha256: &'a str,
    support_worlds: u64,
    support_executions: u64,
    learned_laws: u64,
    target_predictions: u64,
    wrong_predictions: u64,
    verdict: &'a str,
    evidence_class: K2LearnedCapabilityEvidenceClassV1,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilityOutcomeV1 {
    pub fn capability_pass(
        input: K2LearnedCapabilityOutcomeInputV1<'_>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        input.freeze.validate_persisted_v1()?;
        input.observations.validate_persisted_v1()?;
        input.learning_request.validate()?;
        input.laws.validate()?;
        input.independence.validate_persisted_v1()?;
        input.prediction_request.validate()?;
        input.predictions.validate()?;
        input.verification.validate_persisted_v1()?;
        input.v1_binding.validate_persisted_v1()?;
        input.v1_episode.validate_persisted_v1()?;
        input.ablations.validate_persisted_v1()?;
        if input.dispatches.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_outcome_dispatch_count_invalid",
            ));
        }
        for (ordinal, dispatch) in input.dispatches.iter().enumerate() {
            dispatch.validate_persisted_v1()?;
            let observation = &input.observations.observations[ordinal];
            if dispatch.probe_ordinal != ordinal as u64
                || dispatch.experiment_freeze_root_sha256 != input.freeze.freeze_root_sha256
                || observation.dispatch_root_sha256 != dispatch.dispatch_root_sha256
                || observation.probe_ordinal != dispatch.probe_ordinal
                || observation.action_id_sha256 != dispatch.action_id_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_outcome_support_binding_invalid",
                ));
            }
        }
        let cross_roots_valid = input
            .learning_request
            .support_observations
            .observation_set_root_sha256
            == input.observations.observation_set_root_sha256
            && input.laws.learning_request_root_sha256
                == input.learning_request.request_root_sha256
            && input.laws.support_observation_set_root_sha256
                == input.observations.observation_set_root_sha256
            && input.independence.support_set_root_sha256 == input.freeze.support_set_root_sha256
            && input.independence.target_pre_tree_root_sha256
                == input
                    .prediction_request
                    .target_pre_manifest
                    .tree_root_sha256
            && input.predictions.target_prediction_request_root_sha256
                == input.prediction_request.request_root_sha256
            && input.predictions.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.verification.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.verification.target_prediction_set_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.v1_binding.experiment_freeze_root_sha256 == input.freeze.freeze_root_sha256
            && input.v1_binding.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.v1_binding.target_prediction_set_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.v1_binding.independent_verification_root_sha256
                == input.verification.verification_root_sha256
            && input.v1_episode.learned_to_v1_binding_root_sha256
                == input.v1_binding.binding_root_sha256
            && input.ablations.experiment_freeze_root_sha256 == input.freeze.freeze_root_sha256;
        if !cross_roots_valid {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_outcome_cross_root_invalid",
            ));
        }
        let mut outcome = Self {
            schema: K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            experiment_freeze_root_sha256: input.freeze.freeze_root_sha256.clone(),
            support_dispatch_roots_sha256: input
                .dispatches
                .iter()
                .map(|dispatch| dispatch.dispatch_root_sha256.clone())
                .collect(),
            support_observation_roots_sha256: input
                .observations
                .observations
                .iter()
                .map(|observation| observation.observation_root_sha256.clone())
                .collect(),
            support_evidence_set_root_sha256: input
                .observations
                .observation_set_root_sha256
                .clone(),
            learning_request_root_sha256: input.learning_request.request_root_sha256.clone(),
            learned_law_set_root_sha256: input.laws.law_set_root_sha256.clone(),
            target_independence_receipt_root_sha256: input.independence.receipt_root_sha256.clone(),
            target_prediction_request_root_sha256: input
                .prediction_request
                .request_root_sha256
                .clone(),
            target_prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            independent_verification_root_sha256: input
                .verification
                .verification_root_sha256
                .clone(),
            learned_to_v1_binding_root_sha256: input.v1_binding.binding_root_sha256.clone(),
            v1_decision_freeze_root_sha256: input.v1_episode.v1_decision_freeze_root_sha256.clone(),
            v1_prediction_set_root_sha256: input.v1_episode.v1_prediction_set_root_sha256.clone(),
            v1_selection_root_sha256: input.v1_episode.v1_selection_root_sha256.clone(),
            v1_law_lab_binding_root_sha256: input.v1_episode.v1_law_lab_binding_root_sha256.clone(),
            v1_sandbox_receipt_root_sha256: input.v1_episode.v1_sandbox_receipt_root_sha256.clone(),
            v1_exact_goal_receipt_root_sha256: input
                .v1_episode
                .v1_exact_goal_receipt_root_sha256
                .clone(),
            v1_terminal_outcome_root_sha256: input
                .v1_episode
                .v1_terminal_outcome_root_sha256
                .clone(),
            v1_episode_seal_root_sha256: input.v1_episode.v1_episode_seal_root_sha256.clone(),
            ablation_receipt_root_sha256: input.ablations.receipt_root_sha256.clone(),
            support_worlds: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            support_executions: K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64,
            learned_laws: input.laws.laws.len() as u64,
            target_predictions: input.predictions.predictions.len() as u64,
            wrong_predictions: input.verification.wrong_predictions,
            verdict: K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1.to_owned(),
            evidence_class: K2LearnedCapabilityEvidenceClassV1::CapabilityPass,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.outcome_root_sha256 = outcome.expected_root_v1()?;
        outcome.validate_persisted_v1()?;
        Ok(outcome)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in std::iter::once(self.outcome_root_sha256.as_str())
            .chain(std::iter::once(self.experiment_freeze_root_sha256.as_str()))
            .chain(
                self.support_dispatch_roots_sha256
                    .iter()
                    .map(String::as_str),
            )
            .chain(
                self.support_observation_roots_sha256
                    .iter()
                    .map(String::as_str),
            )
            .chain([
                self.support_evidence_set_root_sha256.as_str(),
                self.learning_request_root_sha256.as_str(),
                self.learned_law_set_root_sha256.as_str(),
                self.target_independence_receipt_root_sha256.as_str(),
                self.target_prediction_request_root_sha256.as_str(),
                self.target_prediction_set_root_sha256.as_str(),
                self.independent_verification_root_sha256.as_str(),
                self.learned_to_v1_binding_root_sha256.as_str(),
                self.v1_decision_freeze_root_sha256.as_str(),
                self.v1_prediction_set_root_sha256.as_str(),
                self.v1_selection_root_sha256.as_str(),
                self.v1_law_lab_binding_root_sha256.as_str(),
                self.v1_sandbox_receipt_root_sha256.as_str(),
                self.v1_exact_goal_receipt_root_sha256.as_str(),
                self.v1_terminal_outcome_root_sha256.as_str(),
                self.v1_episode_seal_root_sha256.as_str(),
                self.ablation_receipt_root_sha256.as_str(),
            ])
        {
            require_learned_root_v1(root, "k2_learned_outcome_root_invalid")?;
        }
        if self.schema != K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1
            || self.support_dispatch_roots_sha256.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_observation_roots_sha256.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_worlds != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64
            || self.support_executions != K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.learned_laws != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.target_predictions != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.wrong_predictions != 0
            || self.verdict != K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1
            || self.evidence_class != K2LearnedCapabilityEvidenceClassV1::CapabilityPass
            || self.outcome_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_outcome_invalid",
            ));
        }
        require_unique_roots_v1(
            self.support_dispatch_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_outcome_dispatch_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.support_observation_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_outcome_observation_roots_not_unique",
        )
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&K2LearnedCapabilityOutcomeDigestV1 {
            schema: K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1,
            experiment_freeze_root_sha256: &self.experiment_freeze_root_sha256,
            support_dispatch_roots_sha256: &self.support_dispatch_roots_sha256,
            support_observation_roots_sha256: &self.support_observation_roots_sha256,
            support_evidence_set_root_sha256: &self.support_evidence_set_root_sha256,
            learning_request_root_sha256: &self.learning_request_root_sha256,
            learned_law_set_root_sha256: &self.learned_law_set_root_sha256,
            target_independence_receipt_root_sha256: &self.target_independence_receipt_root_sha256,
            target_prediction_request_root_sha256: &self.target_prediction_request_root_sha256,
            target_prediction_set_root_sha256: &self.target_prediction_set_root_sha256,
            independent_verification_root_sha256: &self.independent_verification_root_sha256,
            learned_to_v1_binding_root_sha256: &self.learned_to_v1_binding_root_sha256,
            v1_decision_freeze_root_sha256: &self.v1_decision_freeze_root_sha256,
            v1_prediction_set_root_sha256: &self.v1_prediction_set_root_sha256,
            v1_selection_root_sha256: &self.v1_selection_root_sha256,
            v1_law_lab_binding_root_sha256: &self.v1_law_lab_binding_root_sha256,
            v1_sandbox_receipt_root_sha256: &self.v1_sandbox_receipt_root_sha256,
            v1_exact_goal_receipt_root_sha256: &self.v1_exact_goal_receipt_root_sha256,
            v1_terminal_outcome_root_sha256: &self.v1_terminal_outcome_root_sha256,
            v1_episode_seal_root_sha256: &self.v1_episode_seal_root_sha256,
            ablation_receipt_root_sha256: &self.ablation_receipt_root_sha256,
            support_worlds: self.support_worlds,
            support_executions: self.support_executions,
            learned_laws: self.learned_laws,
            target_predictions: self.target_predictions,
            wrong_predictions: self.wrong_predictions,
            verdict: &self.verdict,
            evidence_class: self.evidence_class,
            authority: &self.authority,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilitySealV1 {
    pub schema: String,
    pub seal_root_sha256: String,
    pub experiment_id_sha256: String,
    pub outcome_root_sha256: String,
    pub terminal_event_root_sha256: String,
    pub final_projection_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilitySealV1 {
    pub fn derive(
        experiment_id_sha256: String,
        outcome_root_sha256: String,
        terminal_event_root_sha256: String,
        final_projection_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let mut seal = Self {
            schema: K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1.to_owned(),
            seal_root_sha256: String::new(),
            experiment_id_sha256,
            outcome_root_sha256,
            terminal_event_root_sha256,
            final_projection_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        seal.seal_root_sha256 = seal.expected_root_v1()?;
        seal.validate_persisted_v1()?;
        Ok(seal)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.seal_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_capability_seal_root_invalid")?;
        }
        if self.schema != K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1
            || self.seal_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_seal_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1,
            self.experiment_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
            &self.authority,
        ))
    }
}
