#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportDispatchV1 {
    pub schema: String,
    pub dispatch_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub probe_ordinal: u64,
    pub probe_root_sha256: String,
    pub support_world_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub action_id_sha256: String,
    pub hidden_operation_plan_root_sha256: String,
    pub request_root_sha256: String,
    pub worker_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub deterministic_seed_sha256: String,
}

impl K2SupportDispatchV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        freeze: &K2LearnedCapabilityFreezeV1,
        plan: &K2SupportProbePlanV1,
        probe: &K2SupportProbeV1,
        world: &K2SupportWorldV1,
        mapping: &K2HiddenActionMappingV1,
        request: &LawLabSandboxRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        world.validate()?;
        request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let entry =
            mapping
                .entry(&probe.action_id_sha256)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_dispatch_action_missing",
                ))?;
        if plan.plan_root_sha256 != freeze.support_probe_plan_root_sha256
            || probe.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || plan.probe(probe.probe_ordinal) != Some(probe)
            || probe.support_world_root_sha256 != world.world_root_sha256
            || request.source_tree_root_sha256 != world.source_manifest.tree_root_sha256
            || request.operations != [entry.effect.operation_v1()]
            || request.purpose != LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            || request.domain != LawLabProbeDomainV1::Filesystem
            || request.executor_manifest_root_sha256 != freeze.sandbox_executor_manifest_root_sha256
            || request.worker_sha256 != freeze.sandbox_worker_sha256
            || request.probe_root_sha256 != probe.probe_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_dispatch_binding_invalid",
            ));
        }
        let mut dispatch = Self {
            schema: K2_SUPPORT_DISPATCH_SCHEMA_V1.to_owned(),
            dispatch_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            probe_ordinal: probe.probe_ordinal,
            probe_root_sha256: probe.probe_root_sha256.clone(),
            support_world_root_sha256: world.world_root_sha256.clone(),
            source_tree_root_sha256: world.source_manifest.tree_root_sha256.clone(),
            action_id_sha256: probe.action_id_sha256.clone(),
            hidden_operation_plan_root_sha256: entry.operation_plan_root_sha256.clone(),
            request_root_sha256: request.request_root_sha256.clone(),
            worker_sha256: request.worker_sha256.clone(),
            executor_manifest_root_sha256: request.executor_manifest_root_sha256.clone(),
            deterministic_seed_sha256: probe.deterministic_seed_sha256.clone(),
        };
        dispatch.dispatch_root_sha256 = dispatch.expected_root_v1()?;
        dispatch.validate_persisted_v1()?;
        Ok(dispatch)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.dispatch_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.probe_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_support_dispatch_root_invalid")?;
        }
        if self.schema != K2_SUPPORT_DISPATCH_SCHEMA_V1
            || self.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.dispatch_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_dispatch_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_DISPATCH_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.probe_ordinal,
            self.probe_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportObservationV1 {
    pub schema: String,
    pub observation_root_sha256: String,
    pub public_context_root_sha256: String,
    pub dispatch_root_sha256: String,
    pub probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub source_manifest_root_sha256: String,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
    pub sandbox_receipt_root_sha256: String,
}

impl K2SupportObservationV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        world: &K2SupportWorldV1,
        dispatch: &K2SupportDispatchV1,
        request: &LawLabSandboxRequestV1,
        execution: &LawLabSandboxExecutionV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        world.validate()?;
        dispatch.validate_persisted_v1()?;
        execution
            .receipt
            .validate(request, &execution.worker_outcome)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let worker = &execution.worker_outcome;
        if dispatch.request_root_sha256 != request.request_root_sha256
            || dispatch.support_world_root_sha256 != world.world_root_sha256
            || dispatch.source_tree_root_sha256 != world.source_manifest.tree_root_sha256
            || worker.source_manifest != world.source_manifest
            || worker.pre_work_manifest != world.source_manifest
            || worker.source_manifest.tree_root_sha256 != dispatch.source_tree_root_sha256
            || worker.request_root_sha256 != dispatch.request_root_sha256
            || execution.receipt.request_root_sha256 != dispatch.request_root_sha256
            || execution.receipt.source_tree_root_sha256 != dispatch.source_tree_root_sha256
            || execution.receipt.post_tree_root_sha256 != worker.post_work_manifest.tree_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        let mut observation = Self {
            schema: K2_SUPPORT_OBSERVATION_SCHEMA_V1.to_owned(),
            observation_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            dispatch_root_sha256: dispatch.dispatch_root_sha256.clone(),
            probe_ordinal: dispatch.probe_ordinal,
            support_world_root_sha256: world.world_root_sha256.clone(),
            action_id_sha256: dispatch.action_id_sha256.clone(),
            source_manifest_root_sha256: world.source_manifest.tree_root_sha256.clone(),
            pre_work_manifest: worker.pre_work_manifest.clone(),
            post_work_manifest: worker.post_work_manifest.clone(),
            sandbox_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
        };
        observation.observation_root_sha256 = observation.expected_root_v1()?;
        observation.validate_persisted_v1()?;
        Ok(observation)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.observation_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.dispatch_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.source_manifest_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_support_observation_root_invalid")?;
        }
        if self.schema != K2_SUPPORT_OBSERVATION_SCHEMA_V1
            || self.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.source_manifest_root_sha256 != self.pre_work_manifest.tree_root_sha256
            || self.observation_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_OBSERVATION_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            self.dispatch_root_sha256.as_str(),
            self.probe_ordinal,
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.source_manifest_root_sha256.as_str(),
            &self.pre_work_manifest,
            &self.post_work_manifest,
            self.sandbox_receipt_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportObservationSetV1 {
    pub schema: String,
    pub observation_set_root_sha256: String,
    pub public_context_root_sha256: String,
    pub observations: Vec<K2SupportObservationV1>,
}

impl K2SupportObservationSetV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        plan: &K2SupportProbePlanV1,
        mut observations: Vec<K2SupportObservationV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        observations.sort_by_key(|observation| observation.probe_ordinal);
        if observations.len() != plan.ordered_probes.len() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_count_invalid",
            ));
        }
        for (observation, probe) in observations.iter().zip(&plan.ordered_probes) {
            observation.validate_persisted_v1()?;
            if observation.public_context_root_sha256 != public_context.public_context_root_sha256
                || observation.probe_ordinal != probe.probe_ordinal
                || observation.support_world_root_sha256 != probe.support_world_root_sha256
                || observation.action_id_sha256 != probe.action_id_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_observation_schedule_invalid",
                ));
            }
        }
        let mut set = Self {
            schema: K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1.to_owned(),
            observation_set_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            observations,
        };
        set.observation_set_root_sha256 = set.expected_root_v1()?;
        set.validate_persisted_v1()?;
        Ok(set)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1
            || self.observations.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self
                .observations
                .iter()
                .enumerate()
                .any(|(ordinal, observation)| observation.probe_ordinal != ordinal as u64)
            || self.observations.iter().any(|observation| {
                observation.public_context_root_sha256 != self.public_context_root_sha256
            })
            || self.observation_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_set_invalid",
            ));
        }
        for observation in &self.observations {
            observation.validate_persisted_v1()?;
        }
        require_unique_roots_v1(
            self.observations
                .iter()
                .map(|observation| observation.observation_root_sha256.as_str()),
            "k2_support_observation_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.observations
                .iter()
                .map(|observation| observation.sandbox_receipt_root_sha256.as_str()),
            "k2_support_receipt_roots_not_unique",
        )?;
        let per_action = self.observations.iter().fold(
            BTreeMap::<&str, BTreeSet<&str>>::new(),
            |mut groups, observation| {
                groups
                    .entry(&observation.action_id_sha256)
                    .or_default()
                    .insert(&observation.support_world_root_sha256);
                groups
            },
        );
        if per_action.len() != K2_LEARNED_ACTION_COUNT_V1
            || per_action
                .values()
                .any(|worlds| worlds.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_denominator_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            &self.observations,
        ))
    }
}

impl K2LearnerPublicContextV1 {
    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.public_context_root_sha256.as_str(),
            self.public_experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_schedule_public_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learner_budget_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_public_context_root_invalid")?;
        }
        if self.schema != K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.public_context_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_public_context_persisted_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearningRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub public_context: K2LearnerPublicContextV1,
    pub catalog: K2OpaqueActionCatalogV1,
    pub support_observations: K2SupportObservationSetV1,
    pub minimum_support_worlds_per_action: u64,
    pub allowed_effect_language_root_sha256: String,
}

impl K2EffectLearningRequestV1 {
    pub fn seal(
        public_context: K2LearnerPublicContextV1,
        catalog: K2OpaqueActionCatalogV1,
        support_observations: K2SupportObservationSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        public_context.validate_persisted_v1()?;
        catalog.validate()?;
        support_observations.validate_persisted_v1()?;
        let mut request = Self {
            schema: K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            public_context,
            catalog,
            support_observations,
            minimum_support_worlds_per_action: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            allowed_effect_language_root_sha256: bounded_effect_language_root_v1()?,
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.public_context.validate_persisted_v1()?;
        self.catalog.validate()?;
        self.support_observations.validate_persisted_v1()?;
        if self.schema != K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1
            || self.public_context.catalog_root_sha256 != self.catalog.catalog_root_sha256
            || self.public_context.public_context_root_sha256
                != self.support_observations.public_context_root_sha256
            || self.minimum_support_worlds_per_action != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.public_context.allowed_effect_language_root_sha256
                != self.allowed_effect_language_root_sha256
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learning_request_invalid",
            ));
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learning_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_effect_learning_request_protocol_invalid",
        )?;
        request.validate()?;
        Ok(request)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1,
            &self.public_context,
            &self.catalog,
            &self.support_observations,
            self.minimum_support_worlds_per_action,
            self.allowed_effect_language_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectLawV1 {
    pub schema: String,
    pub law_root_sha256: String,
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
    pub supporting_world_roots_sha256: Vec<String>,
    pub supporting_observation_roots_sha256: Vec<String>,
    pub enumerated_candidate_count: u64,
    pub enumerated_candidate_roots_sha256: Vec<String>,
    pub rejected_candidate_count: u64,
    pub rejection_counts_by_reason: BTreeMap<String, u64>,
    pub version_space_size: u64,
}

impl K2LearnedEffectLawV1 {
    fn seal(
        action_id_sha256: String,
        effect: K2LearnedEffectLawBodyV1,
        observations: &[&K2SupportObservationV1],
        mut enumerated_candidates: Vec<K2LearnedEffectLawBodyV1>,
        rejected_candidate_count: u64,
        rejection_counts_by_reason: BTreeMap<String, u64>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        effect.validate()?;
        enumerated_candidates.sort();
        enumerated_candidates.dedup();
        let mut enumerated_candidate_roots_sha256 = enumerated_candidates
            .iter()
            .map(|candidate| {
                learned_root_v1(&(
                    K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
                    "candidate",
                    action_id_sha256.as_str(),
                    candidate,
                ))
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        enumerated_candidate_roots_sha256.sort();
        let mut supporting_world_roots_sha256 = observations
            .iter()
            .map(|observation| observation.support_world_root_sha256.clone())
            .collect::<Vec<_>>();
        let mut supporting_observation_roots_sha256 = observations
            .iter()
            .map(|observation| observation.observation_root_sha256.clone())
            .collect::<Vec<_>>();
        supporting_world_roots_sha256.sort();
        supporting_observation_roots_sha256.sort();
        let mut law = Self {
            schema: K2_LEARNED_EFFECT_LAW_SCHEMA_V1.to_owned(),
            law_root_sha256: String::new(),
            action_id_sha256,
            effect,
            supporting_world_roots_sha256,
            supporting_observation_roots_sha256,
            enumerated_candidate_count: enumerated_candidate_roots_sha256.len() as u64,
            enumerated_candidate_roots_sha256,
            rejected_candidate_count,
            rejection_counts_by_reason,
            version_space_size: 1,
        };
        law.law_root_sha256 = law.expected_root_v1()?;
        law.validate()?;
        Ok(law)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.effect.validate()?;
        require_learned_root_v1(&self.action_id_sha256, "k2_learned_law_action_invalid")?;
        require_unique_roots_v1(
            self.supporting_world_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_law_worlds_invalid",
        )?;
        require_unique_roots_v1(
            self.supporting_observation_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_law_observations_invalid",
        )?;
        require_unique_roots_v1(
            self.enumerated_candidate_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_candidate_roots_invalid",
        )?;
        let rejected_total = self
            .rejection_counts_by_reason
            .values()
            .try_fold(0_u64, |total, count| total.checked_add(*count))
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_rejection_count_overflow",
            ))?;
        if self.schema != K2_LEARNED_EFFECT_LAW_SCHEMA_V1
            || self.supporting_world_roots_sha256.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self.supporting_observation_roots_sha256.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self.enumerated_candidate_count
                != self.enumerated_candidate_roots_sha256.len() as u64
            || self.enumerated_candidate_count == 0
            || self.enumerated_candidate_count > K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 as u64
            || self.rejected_candidate_count != rejected_total
            || self
                .rejected_candidate_count
                .checked_add(self.version_space_size)
                != Some(self.enumerated_candidate_count)
            || self.version_space_size != 1
            || self.law_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_law_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
            self.action_id_sha256.as_str(),
            &self.effect,
            &self.supporting_world_roots_sha256,
            &self.supporting_observation_roots_sha256,
            self.enumerated_candidate_count,
            &self.enumerated_candidate_roots_sha256,
            self.rejected_candidate_count,
            &self.rejection_counts_by_reason,
            self.version_space_size,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectLawSetV1 {
    pub schema: String,
    pub law_set_root_sha256: String,
    pub learning_request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub support_observation_set_root_sha256: String,
    pub allowed_effect_language_root_sha256: String,
    pub laws: Vec<K2LearnedEffectLawV1>,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedEffectLawSetV1 {
    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.learning_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_law_set_root_invalid")?;
        }
        if self.schema != K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.laws.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .laws
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || !self.learned
            || self.law_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_law_set_invalid",
            ));
        }
        for law in &self.laws {
            law.validate()?;
        }
        Ok(())
    }

    pub fn law(&self, action_id_sha256: &str) -> Option<&K2LearnedEffectLawV1> {
        self.laws
            .iter()
            .find(|law| law.action_id_sha256 == action_id_sha256)
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let set = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_learned_law_set_protocol_invalid",
        )?;
        set.validate()?;
        Ok(set)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1,
            self.learning_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            &self.laws,
            self.learned,
            &self.authority,
        ))
    }
}

pub fn learn_effects_v1(
    request: &K2EffectLearningRequestV1,
) -> K2GoalEnvironmentResultV1<K2LearnedEffectLawSetV1> {
    request.validate()?;
    let observation_views = request
        .support_observations
        .observations
        .iter()
        .map(|observation| K2EffectObservationViewV1 {
            action_id_sha256: &observation.action_id_sha256,
            pre_work_manifest: &observation.pre_work_manifest,
            post_work_manifest: &observation.post_work_manifest,
        })
        .collect::<Vec<_>>();
    let inferred = infer_effects_v1(&request.catalog.action_ids_sha256, &observation_views)?;
    let mut laws = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
    for inference in inferred {
        let observations = request
            .support_observations
            .observations
            .iter()
            .filter(|observation| observation.action_id_sha256 == inference.action_id_sha256)
            .collect::<Vec<_>>();
        laws.push(K2LearnedEffectLawV1::seal(
            inference.action_id_sha256,
            inference.effect,
            &observations,
            inference.enumerated_candidates,
            inference.rejected_candidate_count,
            inference.rejection_counts_by_reason,
        )?);
    }
    laws.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
    let mut set = K2LearnedEffectLawSetV1 {
        schema: K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1.to_owned(),
        law_set_root_sha256: String::new(),
        learning_request_root_sha256: request.request_root_sha256.clone(),
        public_context_root_sha256: request.public_context.public_context_root_sha256.clone(),
        learner_manifest_root_sha256: request.public_context.learner_manifest_root_sha256.clone(),
        learner_executable_sha256: request.public_context.learner_executable_sha256.clone(),
        support_observation_set_root_sha256: request
            .support_observations
            .observation_set_root_sha256
            .clone(),
        allowed_effect_language_root_sha256: request.allowed_effect_language_root_sha256.clone(),
        laws,
        learned: true,
        authority: K2AuthorityBoundaryV1::authority_free_v1(),
    };
    set.law_set_root_sha256 = set.expected_root_v1()?;
    set.validate()?;
    if set.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_learned_law_set_budget_exhausted",
        ));
    }
    Ok(set)
}

struct K2EffectObservationViewV1<'a> {
    action_id_sha256: &'a str,
    pre_work_manifest: &'a LawLabTreeManifestV1,
    post_work_manifest: &'a LawLabTreeManifestV1,
}

struct K2InferredEffectV1 {
    action_id_sha256: String,
    effect: K2LearnedEffectLawBodyV1,
    enumerated_candidates: Vec<K2LearnedEffectLawBodyV1>,
    rejected_candidate_count: u64,
    rejection_counts_by_reason: BTreeMap<String, u64>,
}

fn infer_effects_v1(
    action_ids_sha256: &[String],
    observations: &[K2EffectObservationViewV1<'_>],
) -> K2GoalEnvironmentResultV1<Vec<K2InferredEffectV1>> {
    let mut inferred = Vec::with_capacity(action_ids_sha256.len());
    for action_id_sha256 in action_ids_sha256 {
        let matching = observations
            .iter()
            .filter(|observation| observation.action_id_sha256 == action_id_sha256)
            .collect::<Vec<_>>();
        if matching.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_insufficient_support"));
        }
        let candidates = enumerate_effect_candidates_from_manifests_v1(
            matching[0].pre_work_manifest,
            matching[0].post_work_manifest,
        )?;
        if candidates.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_non_transferable_delta",
            ));
        }
        if candidates.len() > K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_version_space_budget_exhausted",
            ));
        }
        let surviving = candidates
            .iter()
            .filter(|candidate| {
                matching.iter().all(|observation| {
                    apply_learned_effect_v1(observation.pre_work_manifest, candidate)
                        .is_ok_and(|predicted| predicted == *observation.post_work_manifest)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if surviving.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_non_transferable_delta",
            ));
        }
        if surviving.len() > 1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ambiguous_source_match",
            ));
        }
        validate_effect_variation_views_v1(&surviving[0], &matching)?;
        let rejected_candidate_count = (candidates.len() - surviving.len()) as u64;
        let rejection_counts_by_reason = if rejected_candidate_count == 0 {
            BTreeMap::new()
        } else {
            BTreeMap::from([("support_mismatch".to_owned(), rejected_candidate_count)])
        };
        inferred.push(K2InferredEffectV1 {
            action_id_sha256: action_id_sha256.clone(),
            effect: surviving[0].clone(),
            enumerated_candidates: candidates,
            rejected_candidate_count,
            rejection_counts_by_reason,
        });
    }
    Ok(inferred)
}

fn enumerate_effect_candidates_from_manifests_v1(
    pre_work_manifest: &LawLabTreeManifestV1,
    post_work_manifest: &LawLabTreeManifestV1,
) -> K2GoalEnvironmentResultV1<Vec<K2LearnedEffectLawBodyV1>> {
    pre_work_manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    post_work_manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    let (added, removed, changed) = manifest_delta_v1(pre_work_manifest, post_work_manifest);
    let mut candidates = Vec::new();
    if added.len() == 1
        && removed.is_empty()
        && changed.is_empty()
        && added[0].kind == LawLabTreeEntryKindV1::File
    {
        let added_entry = added[0];
        for source in pre_work_manifest.entries.iter().filter(|entry| {
            entry.kind == LawLabTreeEntryKindV1::File
                && entry.byte_length == added_entry.byte_length
                && entry.content_sha256 == added_entry.content_sha256
                && entry.executable == added_entry.executable
                && entry.relative_path != added_entry.relative_path
        }) {
            candidates.push(K2LearnedEffectLawBodyV1::CopyFile {
                source_path: source.relative_path.clone(),
                target_path: added_entry.relative_path.clone(),
            });
        }
    }
    if removed.len() == 1
        && added.is_empty()
        && changed.is_empty()
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

fn manifest_delta_v1<'a>(
    pre: &'a LawLabTreeManifestV1,
    post: &'a LawLabTreeManifestV1,
) -> (
    Vec<&'a LawLabTreeEntryV1>,
    Vec<&'a LawLabTreeEntryV1>,
    Vec<(&'a LawLabTreeEntryV1, &'a LawLabTreeEntryV1)>,
) {
    let pre_by_path = pre
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let post_by_path = post
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let added = post_by_path
        .iter()
        .filter_map(|(path, entry)| (!pre_by_path.contains_key(path)).then_some(*entry))
        .collect();
    let removed = pre_by_path
        .iter()
        .filter_map(|(path, entry)| (!post_by_path.contains_key(path)).then_some(*entry))
        .collect();
    let changed = pre_by_path
        .iter()
        .filter_map(|(path, pre_entry)| {
            post_by_path
                .get(path)
                .filter(|post_entry| **post_entry != *pre_entry)
                .map(|post_entry| (*pre_entry, *post_entry))
        })
        .collect();
    (added, removed, changed)
}

fn validate_effect_variation_views_v1(
    effect: &K2LearnedEffectLawBodyV1,
    observations: &[&K2EffectObservationViewV1<'_>],
) -> K2GoalEnvironmentResultV1<()> {
    let path = match effect {
        K2LearnedEffectLawBodyV1::CopyFile { source_path, .. } => source_path,
        K2LearnedEffectLawBodyV1::RemoveFile { path } => path,
    };
    let entries = observations
        .iter()
        .map(|observation| {
            observation
                .pre_work_manifest
                .entry(path)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_effect_variation_source_missing",
                ))
        })
        .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
    let hashes = entries
        .iter()
        .filter_map(|entry| entry.content_sha256.as_deref())
        .collect::<BTreeSet<_>>();
    let lengths = entries
        .iter()
        .map(|entry| entry.byte_length)
        .collect::<BTreeSet<_>>();
    if hashes.len() != observations.len() || lengths.len() != observations.len() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_values_not_transferable",
        ));
    }
    Ok(())
}

fn apply_learned_effect_v1(
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
                    "k2_prediction_target_already_exists",
                ));
            }
            let source = entries
                .iter()
                .find(|entry| {
                    entry.relative_path == *source_path && entry.kind == LawLabTreeEntryKindV1::File
                })
                .cloned()
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_prediction_copy_source_missing",
                ))?;
            let mut target = source;
            target.relative_path = target_path.clone();
            entries.push(target);
        }
        K2LearnedEffectLawBodyV1::RemoveFile { path } => {
            let before = entries.len();
            entries.retain(|entry| entry.relative_path != *path);
            if entries.len() + 1 != before {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_prediction_remove_path_missing",
                ));
            }
        }
    }
    seal_manifest_entries_v1(entries)
}

fn seal_manifest_entries_v1(
    mut entries: Vec<LawLabTreeEntryV1>,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    entries.sort();
    let total_file_bytes = entries
        .iter()
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_length))
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_predicted_manifest_bytes_overflow",
        ))?;
    #[derive(Serialize)]
    struct ManifestDigestV1<'a> {
        schema: &'static str,
        total_file_bytes: u64,
        entries: &'a [LawLabTreeEntryV1],
    }
    let tree_root_sha256 = learned_root_v1(&ManifestDigestV1 {
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
pub struct K2TargetPredictionRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub catalog: K2OpaqueActionCatalogV1,
    pub learned_law_set: K2LearnedEffectLawSetV1,
    pub target_pre_manifest: LawLabTreeManifestV1,
}

impl K2TargetPredictionRequestV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        catalog: K2OpaqueActionCatalogV1,
        learned_law_set: K2LearnedEffectLawSetV1,
        target_pre_manifest: LawLabTreeManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        public_context.validate_persisted_v1()?;
        catalog.validate()?;
        learned_law_set.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut request = Self {
            schema: K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            catalog,
            learned_law_set,
            target_pre_manifest,
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.catalog.validate()?;
        self.learned_law_set.validate()?;
        self.target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        if self.schema != K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1
            || self.public_context_root_sha256 != self.learned_law_set.public_context_root_sha256
            || self
                .catalog
                .action_ids_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != self
                    .learned_law_set
                    .laws
                    .iter()
                    .map(|law| law.action_id_sha256.as_str())
                    .collect::<Vec<_>>()
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_request_invalid",
            ));
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_target_prediction_request_protocol_invalid",
        )?;
        request.validate()?;
        Ok(request)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            &self.catalog,
            &self.learned_law_set,
            &self.target_pre_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedTargetPredictionV1 {
    pub schema: String,
    pub prediction_root_sha256: String,
    pub action_id_sha256: String,
    pub learned_law_root_sha256: String,
    pub predicted_terminal_manifest: LawLabTreeManifestV1,
}

impl K2LearnedTargetPredictionV1 {
    fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.predicted_terminal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_prediction_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1
            || self.prediction_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1,
            self.action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            &self.predicted_terminal_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedTargetPredictionSetV1 {
    pub schema: String,
    pub prediction_set_root_sha256: String,
    pub target_prediction_request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub predictions: Vec<K2LearnedTargetPredictionV1>,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedTargetPredictionSetV1 {
    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.target_prediction_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_prediction_set_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1
            || self.predictions.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .predictions
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || !self.learned
            || self.prediction_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_set_invalid",
            ));
        }
        for prediction in &self.predictions {
            prediction.validate()?;
        }
        Ok(())
    }

    pub fn prediction(&self, action_id_sha256: &str) -> Option<&K2LearnedTargetPredictionV1> {
        self.predictions
            .iter()
            .find(|prediction| prediction.action_id_sha256 == action_id_sha256)
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let set = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_target_prediction_set_protocol_invalid",
        )?;
        set.validate()?;
        Ok(set)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1,
            self.target_prediction_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            &self.predictions,
            self.learned,
            &self.authority,
        ))
    }
}

pub fn verify_target_prediction_replay_v1(
    frozen: &K2LearnedTargetPredictionSetV1,
    replayed: &K2LearnedTargetPredictionSetV1,
) -> K2GoalEnvironmentResultV1<()> {
    frozen.validate()?;
    if replayed != frozen {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_target_prediction_root_mismatch",
        ));
    }
    replayed.validate()
}

pub fn require_exact_goal_for_learned_capability_v1(
    exact_goal: &K2ExactGoalReceiptV1,
) -> K2GoalEnvironmentResultV1<()> {
    exact_goal.validate_persisted_v1()?;
    if !exact_goal.goal_satisfied {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_exact_goal_unsatisfied",
        ));
    }
    Ok(())
}

pub fn predict_target_v1(
    request: &K2TargetPredictionRequestV1,
) -> K2GoalEnvironmentResultV1<K2LearnedTargetPredictionSetV1> {
    request.validate()?;
    let mut predictions = request
        .learned_law_set
        .laws
        .iter()
        .map(|law| {
            let predicted_terminal_manifest =
                apply_learned_effect_v1(&request.target_pre_manifest, &law.effect)?;
            let mut prediction = K2LearnedTargetPredictionV1 {
                schema: K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1.to_owned(),
                prediction_root_sha256: String::new(),
                action_id_sha256: law.action_id_sha256.clone(),
                learned_law_root_sha256: law.law_root_sha256.clone(),
                predicted_terminal_manifest,
            };
            prediction.prediction_root_sha256 = prediction.expected_root_v1()?;
            prediction.validate()?;
            Ok(prediction)
        })
        .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
    predictions.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
    let mut set = K2LearnedTargetPredictionSetV1 {
        schema: K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1.to_owned(),
        prediction_set_root_sha256: String::new(),
        target_prediction_request_root_sha256: request.request_root_sha256.clone(),
        public_context_root_sha256: request.public_context_root_sha256.clone(),
        learner_manifest_root_sha256: request.learned_law_set.learner_manifest_root_sha256.clone(),
        learner_executable_sha256: request.learned_law_set.learner_executable_sha256.clone(),
        learned_law_set_root_sha256: request.learned_law_set.law_set_root_sha256.clone(),
        target_pre_tree_root_sha256: request.target_pre_manifest.tree_root_sha256.clone(),
        predictions,
        learned: true,
        authority: K2AuthorityBoundaryV1::authority_free_v1(),
    };
    set.prediction_set_root_sha256 = set.expected_root_v1()?;
    set.validate()?;
    if set.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_target_prediction_set_budget_exhausted",
        ));
    }
    Ok(set)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2GeneratedAblationProvenanceV1 {
    GeneratedCapabilityAblation,
    GeneratedCapabilitySelfTest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationObservationV1 {
    pub schema: String,
    pub observation_root_sha256: String,
    pub provenance: K2GeneratedAblationProvenanceV1,
    pub source_observation_root_sha256: String,
    pub source_probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
}

impl K2GeneratedAblationObservationV1 {
    pub fn unchanged_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            source.post_work_manifest.clone(),
        )
    }

    pub fn ambiguous_copy_source_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        let mut duplicate = source
            .pre_work_manifest
            .entry(K2_COPY_SOURCE_PATH_V1)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_copy_source_missing",
            ))?;
        duplicate.relative_path = "duplicate-input.bin".to_owned();
        let mut pre_entries = source.pre_work_manifest.entries.clone();
        pre_entries.push(duplicate.clone());
        let mut post_entries = source.post_work_manifest.entries.clone();
        post_entries.push(duplicate);
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            seal_manifest_entries_v1(pre_entries)?,
            seal_manifest_entries_v1(post_entries)?,
        )
    }

    pub fn constant_output_from_support_v1(
        source: &K2SupportObservationV1,
        donor: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        donor.validate_persisted_v1()?;
        let mut donor_entry = donor
            .post_work_manifest
            .entry(K2_COPY_TARGET_PATH_V1)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_constant_donor_missing",
            ))?;
        donor_entry.relative_path = K2_COPY_TARGET_PATH_V1.to_owned();
        let mut post_entries = source.post_work_manifest.entries.clone();
        let target = post_entries
            .iter_mut()
            .find(|entry| entry.relative_path == K2_COPY_TARGET_PATH_V1)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_constant_target_missing",
            ))?;
        *target = donor_entry;
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            seal_manifest_entries_v1(post_entries)?,
        )
    }

    pub fn outcome_equals_pre_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            source.pre_work_manifest.clone(),
        )
    }

    fn seal_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
        pre_work_manifest: LawLabTreeManifestV1,
        post_work_manifest: LawLabTreeManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        require_learned_root_v1(&action_id_sha256, "k2_ablation_action_id_invalid")?;
        pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut observation = Self {
            schema: K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1.to_owned(),
            observation_root_sha256: String::new(),
            provenance: K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation,
            source_observation_root_sha256: source.observation_root_sha256.clone(),
            source_probe_ordinal: source.probe_ordinal,
            support_world_root_sha256: source.support_world_root_sha256.clone(),
            action_id_sha256,
            pre_work_manifest,
            post_work_manifest,
        };
        observation.observation_root_sha256 = observation.expected_root_v1()?;
        observation.validate_persisted_v1()?;
        Ok(observation)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.observation_root_sha256.as_str(),
            self.source_observation_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_observation_root_invalid")?;
        }
        if self.schema != K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1
            || self.provenance != K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation
            || self.source_probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.observation_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1,
            self.provenance,
            self.source_observation_root_sha256.as_str(),
            self.source_probe_ordinal,
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            &self.pre_work_manifest,
            &self.post_work_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub provenance: K2GeneratedAblationProvenanceV1,
    pub source_learning_request: K2EffectLearningRequestV1,
    pub catalog: K2OpaqueActionCatalogV1,
    pub observations: Vec<K2GeneratedAblationObservationV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2GeneratedAblationRequestV1 {
    pub fn seal(
        source_learning_request: K2EffectLearningRequestV1,
        catalog: K2OpaqueActionCatalogV1,
        mut observations: Vec<K2GeneratedAblationObservationV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source_learning_request.validate()?;
        catalog.validate()?;
        observations.sort_by_key(|observation| observation.source_probe_ordinal);
        let mut request = Self {
            schema: K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            provenance: K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation,
            source_learning_request,
            catalog,
            observations,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        self.source_learning_request.validate()?;
        self.catalog.validate()?;
        require_learned_root_v1(
            &self.request_root_sha256,
            "k2_ablation_request_root_invalid",
        )?;
        if self.schema != K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1
            || self.provenance != K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation
            || !(4..=K2_LEARNED_SUPPORT_PROBE_COUNT_V1).contains(&self.observations.len())
            || self
                .observations
                .windows(2)
                .any(|pair| pair[0].source_probe_ordinal >= pair[1].source_probe_ordinal)
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        let catalog_ids = self
            .catalog
            .action_ids_sha256
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let observed_ids = self
            .observations
            .iter()
            .map(|observation| observation.action_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if observed_ids != catalog_ids {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        for observation in &self.observations {
            observation.validate_persisted_v1()?;
            let source = self
                .source_learning_request
                .support_observations
                .observations
                .iter()
                .find(|source| source.probe_ordinal == observation.source_probe_ordinal)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_evidence_invalid",
                ))?;
            if source.observation_root_sha256 != observation.source_observation_root_sha256
                || source.support_world_root_sha256 != observation.support_world_root_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_evidence_invalid",
                ));
            }
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1,
            self.provenance,
            &self.source_learning_request,
            &self.catalog,
            &self.observations,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command", content = "payload", rename_all = "snake_case")]
pub enum K2EffectLearnerProtocolRequestV1 {
    LearnEffects(K2EffectLearningRequestV1),
    PredictTarget(K2TargetPredictionRequestV1),
    EvaluateGeneratedAblation(K2GeneratedAblationRequestV1),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", content = "payload", rename_all = "snake_case")]
pub enum K2EffectLearnerProtocolOutcomeV1 {
    LearnedEffects(K2LearnedEffectLawSetV1),
    TargetPredictions(K2LearnedTargetPredictionSetV1),
    GeneratedAblation(K2GeneratedAblationOutcomeV1),
}

impl K2EffectLearnerProtocolRequestV1 {
    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_effect_learner_protocol_request_invalid",
        )?;
        match &request {
            Self::LearnEffects(value) => value.validate()?,
            Self::PredictTarget(value) => value.validate()?,
            Self::EvaluateGeneratedAblation(value) => value.validate()?,
        }
        Ok(request)
    }

    pub fn evaluate_v1(&self) -> K2GoalEnvironmentResultV1<K2EffectLearnerProtocolOutcomeV1> {
        match self {
            Self::LearnEffects(request) => Ok(K2EffectLearnerProtocolOutcomeV1::LearnedEffects(
                learn_effects_v1(request)?,
            )),
            Self::PredictTarget(request) => Ok(
                K2EffectLearnerProtocolOutcomeV1::TargetPredictions(predict_target_v1(request)?),
            ),
            Self::EvaluateGeneratedAblation(request) => {
                Ok(K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(
                    K2GeneratedAblationOutcomeV1::evaluate(request)?,
                ))
            }
        }
    }
}

impl K2EffectLearnerProtocolOutcomeV1 {
    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        let bytes = learned_bytes_v1(self)?;
        if bytes.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_protocol_outcome_too_large",
            ));
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let outcome = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_effect_learner_protocol_outcome_invalid",
        )?;
        match &outcome {
            Self::LearnedEffects(value) => value.validate()?,
            Self::TargetPredictions(value) => value.validate()?,
            Self::GeneratedAblation(value) => value.validate_persisted_v1()?,
        }
        Ok(outcome)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2TargetIndependenceReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub support_set_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub support_tree_roots_pairwise_distinct: bool,
    pub target_tree_root_novel: bool,
    pub target_input_hash_novel: bool,
    pub target_input_length_novel: bool,
    pub target_obsolete_hash_novel: bool,
    pub target_obsolete_length_novel: bool,
    pub target_distractor_topology_novel: bool,
    pub target_absent_from_learning_request: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2TargetIndependenceReceiptV1 {
    pub fn verify(
        support: &K2SupportWorldSetV1,
        target_pre_manifest: &LawLabTreeManifestV1,
        learning_request: &K2EffectLearningRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        support.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        learning_request.validate()?;
        if learning_request.public_context.support_set_root_sha256
            != support.support_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_holdout_support_binding_invalid",
            ));
        }
        let support_roots = support
            .worlds
            .iter()
            .map(|world| world.source_manifest.tree_root_sha256.as_str())
            .collect::<Vec<_>>();
        let support_tree_roots_pairwise_distinct =
            support_roots.iter().copied().collect::<BTreeSet<_>>().len() == support_roots.len();
        let target_tree_root_novel = !support_roots
            .iter()
            .any(|root| **root == target_pre_manifest.tree_root_sha256);
        let target_input = target_pre_manifest
            .entry(K2_COPY_SOURCE_PATH_V1)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid("k2_target_input_missing"))?;
        let target_obsolete = target_pre_manifest.entry(K2_REMOVE_PATH_V1).ok_or(
            K2GoalEnvironmentErrorV1::Invalid("k2_target_obsolete_missing"),
        )?;
        let support_input = support
            .worlds
            .iter()
            .map(|world| {
                world.source_manifest.entry(K2_COPY_SOURCE_PATH_V1).ok_or(
                    K2GoalEnvironmentErrorV1::Invalid("k2_support_input_missing"),
                )
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let support_obsolete = support
            .worlds
            .iter()
            .map(|world| {
                world.source_manifest.entry(K2_REMOVE_PATH_V1).ok_or(
                    K2GoalEnvironmentErrorV1::Invalid("k2_support_obsolete_missing"),
                )
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let target_input_hash_novel = target_input.content_sha256.is_some()
            && support_input
                .iter()
                .all(|entry| entry.content_sha256 != target_input.content_sha256);
        let target_input_length_novel = support_input
            .iter()
            .all(|entry| entry.byte_length != target_input.byte_length);
        let target_obsolete_hash_novel = target_obsolete.content_sha256.is_some()
            && support_obsolete
                .iter()
                .all(|entry| entry.content_sha256 != target_obsolete.content_sha256);
        let target_obsolete_length_novel = support_obsolete
            .iter()
            .all(|entry| entry.byte_length != target_obsolete.byte_length);
        let target_topology_root = distractor_topology_root_v1(target_pre_manifest)?;
        let target_distractor_topology_novel = support.worlds.iter().all(|world| {
            distractor_topology_root_v1(&world.source_manifest)
                .is_ok_and(|root| root != target_topology_root)
        });
        let learning_bytes = learning_request.canonical_bytes_v1()?;
        let target_manifest_bytes = learned_bytes_v1(target_pre_manifest)?;
        let target_absent_from_learning_request =
            !contains_bytes_v1(
                &learning_bytes,
                target_pre_manifest.tree_root_sha256.as_bytes(),
            ) && !contains_bytes_v1(&learning_bytes, &target_manifest_bytes);
        let mut receipt = Self {
            schema: K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            target_pre_tree_root_sha256: target_pre_manifest.tree_root_sha256.clone(),
            support_tree_roots_pairwise_distinct,
            target_tree_root_novel,
            target_input_hash_novel,
            target_input_length_novel,
            target_obsolete_hash_novel,
            target_obsolete_length_novel,
            target_distractor_topology_novel,
            target_absent_from_learning_request,
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
            self.support_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_independence_root_invalid")?;
        }
        let all_independent = self.support_tree_roots_pairwise_distinct
            && self.target_tree_root_novel
            && self.target_input_hash_novel
            && self.target_input_length_novel
            && self.target_obsolete_hash_novel
            && self.target_obsolete_length_novel
            && self.target_distractor_topology_novel
            && self.target_absent_from_learning_request;
        if self.schema != K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1
            || !all_independent
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_not_independent",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1,
            self.support_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            self.support_tree_roots_pairwise_distinct,
            self.target_tree_root_novel,
            self.target_input_hash_novel,
            self.target_input_length_novel,
            self.target_obsolete_hash_novel,
            self.target_obsolete_length_novel,
            self.target_distractor_topology_novel,
            self.target_absent_from_learning_request,
            &self.authority,
        ))
    }
}

fn contains_bytes_v1(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

