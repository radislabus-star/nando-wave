#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedEffectV1 {
    CopyFile,
    RemoveFile,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2LearnedEffectLawBodyV1 {
    CopyFile {
        source_path: String,
        target_path: String,
    },
    RemoveFile {
        path: String,
    },
}

impl K2LearnedEffectLawBodyV1 {
    fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        let valid = match self {
            Self::CopyFile {
                source_path,
                target_path,
            } => {
                validate_fixture_path_v1(source_path)
                    && validate_fixture_path_v1(target_path)
                    && source_path != target_path
            }
            Self::RemoveFile { path } => validate_fixture_path_v1(path),
        };
        if valid {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_path_invalid",
            ))
        }
    }

    #[must_use]
    pub fn operation_v1(&self) -> LawLabSandboxOperationV1 {
        match self {
            Self::CopyFile {
                source_path,
                target_path,
            } => LawLabSandboxOperationV1::CopySourceFile {
                source_path: source_path.clone(),
                work_path: target_path.clone(),
            },
            Self::RemoveFile { path } => LawLabSandboxOperationV1::RemoveWorkPath {
                work_path: path.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityBudgetV1 {
    pub schema: String,
    pub support_worlds: u64,
    pub opaque_actions: u64,
    pub support_probes: u64,
    pub target_probes: u64,
    pub maximum_tree_entries: u64,
    pub maximum_tree_bytes: u64,
    pub maximum_candidates_per_action: u64,
    pub maximum_learning_request_bytes: u64,
    pub maximum_learner_outcome_bytes: u64,
    pub learner_wall_ms: u64,
    pub learner_cpu_seconds: u64,
    pub learner_address_space_bytes: u64,
    pub learner_process_count: u64,
}

impl K2LearnedCapabilityBudgetV1 {
    #[must_use]
    pub fn preregistered_v1() -> Self {
        Self {
            schema: K2_LEARNED_CAPABILITY_BUDGET_SCHEMA_V1.to_owned(),
            support_worlds: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            opaque_actions: K2_LEARNED_ACTION_COUNT_V1 as u64,
            support_probes: K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64,
            target_probes: 1,
            maximum_tree_entries: K2_LEARNED_MAX_TREE_ENTRIES_V1 as u64,
            maximum_tree_bytes: K2_LEARNED_MAX_TREE_BYTES_V1,
            maximum_candidates_per_action: K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 as u64,
            maximum_learning_request_bytes: K2_LEARNER_MAX_REQUEST_BYTES_V1 as u64,
            maximum_learner_outcome_bytes: K2_LEARNER_MAX_OUTCOME_BYTES_V1 as u64,
            learner_wall_ms: K2_LEARNER_WALL_MS_V1,
            learner_cpu_seconds: K2_LEARNER_CPU_SECONDS_V1,
            learner_address_space_bytes: K2_LEARNER_ADDRESS_SPACE_BYTES_V1,
            learner_process_count: K2_LEARNER_PROCESS_COUNT_V1,
        }
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self == &Self::preregistered_v1() {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_budget_invalid",
            ))
        }
    }

    pub fn root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        self.validate()?;
        learned_root_v1(&(K2_LEARNED_CAPABILITY_BUDGET_SCHEMA_V1, self))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearnerManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub executable_sha256: String,
    pub protocol_schema: String,
    pub effect_language_root_sha256: String,
}

impl K2EffectLearnerManifestV1 {
    pub fn seal(executable_sha256: String) -> K2GoalEnvironmentResultV1<Self> {
        require_learned_root_v1(
            &executable_sha256,
            "k2_effect_learner_executable_sha_invalid",
        )?;
        let effect_language_root_sha256 = bounded_effect_language_root_v1()?;
        let mut value = Self {
            schema: K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            executable_sha256,
            protocol_schema: K2_EFFECT_LEARNER_PROTOCOL_SCHEMA_V1.to_owned(),
            effect_language_root_sha256,
        };
        value.manifest_root_sha256 = value.expected_root_v1()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_learned_root_v1(
            &self.executable_sha256,
            "k2_effect_learner_executable_sha_invalid",
        )?;
        if self.schema != K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1
            || self.protocol_schema != K2_EFFECT_LEARNER_PROTOCOL_SCHEMA_V1
            || self.effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.manifest_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_manifest_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1,
            self.executable_sha256.as_str(),
            self.protocol_schema.as_str(),
            self.effect_language_root_sha256.as_str(),
        ))
    }
}

pub fn bounded_effect_language_root_v1() -> K2GoalEnvironmentResultV1<String> {
    learned_root_v1(&(
        K2_EFFECT_LANGUAGE_SCHEMA_V1,
        ["copy_file{source_path,target_path}", "remove_file{path}"],
    ))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2OpaqueActionCatalogV1 {
    pub schema: String,
    pub catalog_root_sha256: String,
    pub action_ids_sha256: Vec<String>,
}

impl K2OpaqueActionCatalogV1 {
    pub fn from_harness_commitment_v1(
        harness_commitment_sha256: &str,
    ) -> K2GoalEnvironmentResultV1<Self> {
        require_learned_root_v1(harness_commitment_sha256, "k2_harness_commitment_invalid")?;
        let mut action_ids_sha256 = (0_u64..K2_LEARNED_ACTION_COUNT_V1 as u64)
            .map(|slot| {
                learned_root_v1(&(
                    K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1,
                    "opaque-action-id",
                    harness_commitment_sha256,
                    slot,
                ))
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        action_ids_sha256.sort();
        Self::seal(action_ids_sha256)
    }

    pub fn seal(mut action_ids_sha256: Vec<String>) -> K2GoalEnvironmentResultV1<Self> {
        action_ids_sha256.sort();
        require_unique_roots_v1(
            action_ids_sha256.iter().map(String::as_str),
            "k2_opaque_action_ids_invalid",
        )?;
        if action_ids_sha256.len() != K2_LEARNED_ACTION_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_opaque_action_count_invalid",
            ));
        }
        let mut catalog = Self {
            schema: K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1.to_owned(),
            catalog_root_sha256: String::new(),
            action_ids_sha256,
        };
        catalog.catalog_root_sha256 = catalog.expected_root_v1()?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_unique_roots_v1(
            self.action_ids_sha256.iter().map(String::as_str),
            "k2_opaque_action_ids_invalid",
        )?;
        if self.schema != K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1
            || self.action_ids_sha256.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .action_ids_sha256
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.catalog_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_opaque_action_catalog_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1, &self.action_ids_sha256))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2HiddenActionEntryV1 {
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
    pub operation_plan_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2HiddenActionMappingV1 {
    pub schema: String,
    pub mapping_root_sha256: String,
    pub catalog_root_sha256: String,
    pub entries: Vec<K2HiddenActionEntryV1>,
}

impl K2HiddenActionMappingV1 {
    pub fn seal_fixture_v1(
        catalog: &K2OpaqueActionCatalogV1,
        copy_action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        if !catalog.action_ids_sha256.contains(&copy_action_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_copy_action_missing",
            ));
        }
        let remove_action_id_sha256 = catalog
            .action_ids_sha256
            .iter()
            .find(|action_id| **action_id != copy_action_id_sha256)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_remove_action_missing",
            ))?;
        let mut entries = vec![
            K2HiddenActionEntryV1 {
                action_id_sha256: copy_action_id_sha256,
                effect: K2LearnedEffectLawBodyV1::CopyFile {
                    source_path: K2_COPY_SOURCE_PATH_V1.to_owned(),
                    target_path: K2_COPY_TARGET_PATH_V1.to_owned(),
                },
                operation_plan_root_sha256: String::new(),
            },
            K2HiddenActionEntryV1 {
                action_id_sha256: remove_action_id_sha256,
                effect: K2LearnedEffectLawBodyV1::RemoveFile {
                    path: K2_REMOVE_PATH_V1.to_owned(),
                },
                operation_plan_root_sha256: String::new(),
            },
        ];
        for entry in &mut entries {
            entry.operation_plan_root_sha256 = learned_root_v1(&vec![entry.effect.operation_v1()])?;
        }
        entries.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
        let mut mapping = Self {
            schema: K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1.to_owned(),
            mapping_root_sha256: String::new(),
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            entries,
        };
        mapping.mapping_root_sha256 = mapping.expected_root_v1()?;
        mapping.validate(catalog)?;
        Ok(mapping)
    }

    pub fn validate(&self, catalog: &K2OpaqueActionCatalogV1) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        if self.schema != K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.entries.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || self
                .entries
                .iter()
                .map(|entry| entry.action_id_sha256.as_str())
                .collect::<Vec<_>>()
                != catalog
                    .action_ids_sha256
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            || self.mapping_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_action_mapping_invalid",
            ));
        }
        let mut kinds = BTreeSet::new();
        for entry in &self.entries {
            entry.effect.validate()?;
            require_learned_root_v1(
                &entry.operation_plan_root_sha256,
                "k2_hidden_operation_root_invalid",
            )?;
            if entry.operation_plan_root_sha256
                != learned_root_v1(&vec![entry.effect.operation_v1()])?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_hidden_operation_binding_invalid",
                ));
            }
            kinds.insert(match &entry.effect {
                K2LearnedEffectLawBodyV1::CopyFile { .. } => K2LearnedEffectV1::CopyFile,
                K2LearnedEffectLawBodyV1::RemoveFile { .. } => K2LearnedEffectV1::RemoveFile,
            });
        }
        if kinds != BTreeSet::from([K2LearnedEffectV1::CopyFile, K2LearnedEffectV1::RemoveFile]) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_effect_diversity_invalid",
            ));
        }
        Ok(())
    }

    pub fn entry(&self, action_id_sha256: &str) -> Option<&K2HiddenActionEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.action_id_sha256 == action_id_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1,
            self.catalog_root_sha256.as_str(),
            &self.entries,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportWorldV1 {
    pub schema: String,
    pub world_root_sha256: String,
    pub world_ordinal: u64,
    pub source_manifest: LawLabTreeManifestV1,
    pub fixture_provenance_root_sha256: String,
}

impl K2SupportWorldV1 {
    pub fn seal(
        world_ordinal: u64,
        source_manifest: LawLabTreeManifestV1,
        fixture_provenance_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        require_learned_root_v1(
            &fixture_provenance_root_sha256,
            "k2_support_fixture_provenance_invalid",
        )?;
        let mut world = Self {
            schema: K2_SUPPORT_WORLD_SCHEMA_V1.to_owned(),
            world_root_sha256: String::new(),
            world_ordinal,
            source_manifest,
            fixture_provenance_root_sha256,
        };
        world.world_root_sha256 = world.expected_root_v1()?;
        world.validate()?;
        Ok(world)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.source_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        require_learned_root_v1(
            &self.fixture_provenance_root_sha256,
            "k2_support_fixture_provenance_invalid",
        )?;
        validate_fixture_manifest_v1(&self.source_manifest)?;
        if self.schema != K2_SUPPORT_WORLD_SCHEMA_V1
            || self.world_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_world_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_WORLD_SCHEMA_V1,
            self.world_ordinal,
            &self.source_manifest,
            self.fixture_provenance_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportWorldSetV1 {
    pub schema: String,
    pub support_set_root_sha256: String,
    pub worlds: Vec<K2SupportWorldV1>,
}

impl K2SupportWorldSetV1 {
    pub fn seal(mut worlds: Vec<K2SupportWorldV1>) -> K2GoalEnvironmentResultV1<Self> {
        worlds.sort_by_key(|world| world.world_ordinal);
        let mut set = Self {
            schema: K2_SUPPORT_WORLD_SET_SCHEMA_V1.to_owned(),
            support_set_root_sha256: String::new(),
            worlds,
        };
        set.support_set_root_sha256 = set.expected_root_v1()?;
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_SUPPORT_WORLD_SET_SCHEMA_V1
            || self.worlds.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self
                .worlds
                .iter()
                .enumerate()
                .any(|(ordinal, world)| world.world_ordinal != ordinal as u64)
            || self.support_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_world_set_invalid",
            ));
        }
        for world in &self.worlds {
            world.validate()?;
        }
        require_unique_roots_v1(
            self.worlds
                .iter()
                .map(|world| world.world_root_sha256.as_str()),
            "k2_support_world_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.worlds
                .iter()
                .map(|world| world.source_manifest.tree_root_sha256.as_str()),
            "k2_support_tree_roots_not_unique",
        )?;
        require_distinct_fixture_file_values_v1(&self.worlds, K2_COPY_SOURCE_PATH_V1)?;
        require_distinct_fixture_file_values_v1(&self.worlds, K2_REMOVE_PATH_V1)?;
        let topology_roots = self
            .worlds
            .iter()
            .map(|world| distractor_topology_root_v1(&world.source_manifest))
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        require_unique_roots_v1(
            topology_roots.iter().map(String::as_str),
            "k2_support_distractor_topology_not_unique",
        )?;
        Ok(())
    }

    pub fn world(&self, world_root_sha256: &str) -> Option<&K2SupportWorldV1> {
        self.worlds
            .iter()
            .find(|world| world.world_root_sha256 == world_root_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(K2_SUPPORT_WORLD_SET_SCHEMA_V1, &self.worlds))
    }
}

fn validate_fixture_manifest_v1(manifest: &LawLabTreeManifestV1) -> K2GoalEnvironmentResultV1<()> {
    if manifest.entries.len() > K2_LEARNED_MAX_TREE_ENTRIES_V1
        || manifest.total_file_bytes > K2_LEARNED_MAX_TREE_BYTES_V1
        || manifest.entry(K2_COPY_TARGET_PATH_V1).is_some()
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_manifest_budget_or_target_invalid",
        ));
    }
    let source = manifest
        .entry(K2_COPY_SOURCE_PATH_V1)
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_copy_source_missing",
        ))?;
    manifest
        .entry(K2_REMOVE_PATH_V1)
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_remove_path_missing",
        ))?;
    let duplicate_source = manifest.entries.iter().filter(|entry| {
        entry.kind == LawLabTreeEntryKindV1::File
            && entry.byte_length == source.byte_length
            && entry.content_sha256 == source.content_sha256
            && entry.executable == source.executable
    });
    if duplicate_source.count() != 1
        || manifest.entries.iter().all(|entry| {
            matches!(
                entry.relative_path.as_str(),
                K2_COPY_SOURCE_PATH_V1 | K2_REMOVE_PATH_V1
            )
        })
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_distractor_or_source_uniqueness_invalid",
        ));
    }
    Ok(())
}

fn require_distinct_fixture_file_values_v1(
    worlds: &[K2SupportWorldV1],
    path: &str,
) -> K2GoalEnvironmentResultV1<()> {
    let entries = worlds
        .iter()
        .map(|world| {
            world
                .source_manifest
                .entry(path)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_fixture_required_file_missing",
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
    if hashes.len() != worlds.len() || lengths.len() != worlds.len() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_file_values_not_distinct",
        ));
    }
    Ok(())
}

fn distractor_topology_root_v1(
    manifest: &LawLabTreeManifestV1,
) -> K2GoalEnvironmentResultV1<String> {
    let topology = manifest
        .entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.relative_path.as_str(),
                K2_COPY_SOURCE_PATH_V1 | K2_COPY_TARGET_PATH_V1 | K2_REMOVE_PATH_V1
            )
        })
        .map(|entry| (&entry.relative_path, entry.kind, entry.executable))
        .collect::<Vec<_>>();
    learned_root_v1(&("nando.k2-distractor-topology.v1", topology))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportProbeV1 {
    pub probe_root_sha256: String,
    pub probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub deterministic_seed_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportProbePlanV1 {
    pub schema: String,
    pub plan_root_sha256: String,
    pub public_schedule_root_sha256: String,
    pub experiment_id_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub ordered_probes: Vec<K2SupportProbeV1>,
}

impl K2SupportProbePlanV1 {
    pub fn seal(
        experiment_id_sha256: String,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: &K2HiddenActionMappingV1,
        deterministic_seed_sha256: &str,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        support.validate()?;
        mapping.validate(catalog)?;
        for root in [&experiment_id_sha256, deterministic_seed_sha256] {
            require_learned_root_v1(root, "k2_support_probe_plan_root_invalid")?;
        }
        let mut ordered_probes = Vec::with_capacity(K2_LEARNED_SUPPORT_PROBE_COUNT_V1);
        for world in &support.worlds {
            for action_id_sha256 in &catalog.action_ids_sha256 {
                let probe_ordinal = ordered_probes.len() as u64;
                let probe_seed = learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    deterministic_seed_sha256,
                    world.world_root_sha256.as_str(),
                    action_id_sha256.as_str(),
                    probe_ordinal,
                ))?;
                let probe_root_sha256 = learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    "probe",
                    experiment_id_sha256.as_str(),
                    probe_ordinal,
                    world.world_root_sha256.as_str(),
                    action_id_sha256.as_str(),
                    probe_seed.as_str(),
                ))?;
                ordered_probes.push(K2SupportProbeV1 {
                    probe_root_sha256,
                    probe_ordinal,
                    support_world_root_sha256: world.world_root_sha256.clone(),
                    action_id_sha256: action_id_sha256.clone(),
                    deterministic_seed_sha256: probe_seed,
                });
            }
        }
        let public_schedule_root_sha256 = learned_root_v1(&(
            K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
            "public-schedule",
            catalog.catalog_root_sha256.as_str(),
            support.support_set_root_sha256.as_str(),
            &ordered_probes,
        ))?;
        let mut plan = Self {
            schema: K2_SUPPORT_PROBE_PLAN_SCHEMA_V1.to_owned(),
            plan_root_sha256: String::new(),
            public_schedule_root_sha256,
            experiment_id_sha256,
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            hidden_mapping_root_sha256: mapping.mapping_root_sha256.clone(),
            ordered_probes,
        };
        plan.plan_root_sha256 = plan.expected_root_v1()?;
        plan.validate(catalog, support, mapping)?;
        Ok(plan)
    }

    pub fn validate(
        &self,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: &K2HiddenActionMappingV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        support.validate()?;
        mapping.validate(catalog)?;
        require_learned_root_v1(&self.experiment_id_sha256, "k2_experiment_id_invalid")?;
        if self.schema != K2_SUPPORT_PROBE_PLAN_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.support_set_root_sha256 != support.support_set_root_sha256
            || self.hidden_mapping_root_sha256 != mapping.mapping_root_sha256
            || self.ordered_probes.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self
                .ordered_probes
                .iter()
                .enumerate()
                .any(|(ordinal, probe)| probe.probe_ordinal != ordinal as u64)
            || self.public_schedule_root_sha256
                != learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    "public-schedule",
                    catalog.catalog_root_sha256.as_str(),
                    support.support_set_root_sha256.as_str(),
                    &self.ordered_probes,
                ))?
            || self.plan_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_probe_plan_invalid",
            ));
        }
        let expected_pairs = support
            .worlds
            .iter()
            .flat_map(|world| {
                catalog
                    .action_ids_sha256
                    .iter()
                    .map(move |action_id| (world.world_root_sha256.as_str(), action_id.as_str()))
            })
            .collect::<Vec<_>>();
        for (probe, expected) in self.ordered_probes.iter().zip(expected_pairs) {
            for root in [
                probe.probe_root_sha256.as_str(),
                probe.deterministic_seed_sha256.as_str(),
            ] {
                require_learned_root_v1(root, "k2_support_probe_root_invalid")?;
            }
            if (
                probe.support_world_root_sha256.as_str(),
                probe.action_id_sha256.as_str(),
            ) != expected
                || probe.probe_root_sha256
                    != learned_root_v1(&(
                        K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                        "probe",
                        self.experiment_id_sha256.as_str(),
                        probe.probe_ordinal,
                        probe.support_world_root_sha256.as_str(),
                        probe.action_id_sha256.as_str(),
                        probe.deterministic_seed_sha256.as_str(),
                    ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_probe_binding_invalid",
                ));
            }
        }
        Ok(())
    }

    pub fn probe(&self, probe_ordinal: u64) -> Option<&K2SupportProbeV1> {
        self.ordered_probes.get(probe_ordinal as usize)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
            self.public_schedule_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            &self.ordered_probes,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnerPublicContextV1 {
    pub schema: String,
    pub public_context_root_sha256: String,
    pub public_experiment_id_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub support_probe_schedule_public_root_sha256: String,
    pub allowed_effect_language_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub learner_budget_root_sha256: String,
}

impl K2LearnerPublicContextV1 {
    pub fn seal(
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        plan: &K2SupportProbePlanV1,
        learner: &K2EffectLearnerManifestV1,
        budget: &K2LearnedCapabilityBudgetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        support.validate()?;
        learner.validate()?;
        budget.validate()?;
        if plan.catalog_root_sha256 != catalog.catalog_root_sha256
            || plan.support_set_root_sha256 != support.support_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_public_context_plan_binding_invalid",
            ));
        }
        let public_experiment_id_sha256 = learned_root_v1(&(
            K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1,
            "public-experiment",
            catalog.catalog_root_sha256.as_str(),
            support.support_set_root_sha256.as_str(),
            plan.public_schedule_root_sha256.as_str(),
            learner.manifest_root_sha256.as_str(),
        ))?;
        let mut context = Self {
            schema: K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1.to_owned(),
            public_context_root_sha256: String::new(),
            public_experiment_id_sha256,
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            support_probe_schedule_public_root_sha256: plan.public_schedule_root_sha256.clone(),
            allowed_effect_language_root_sha256: bounded_effect_language_root_v1()?,
            learner_manifest_root_sha256: learner.manifest_root_sha256.clone(),
            learner_executable_sha256: learner.executable_sha256.clone(),
            learner_budget_root_sha256: budget.root_v1()?,
        };
        context.public_context_root_sha256 = context.expected_root_v1()?;
        context.validate(catalog, support, plan, learner, budget)?;
        Ok(context)
    }

    pub fn validate(
        &self,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        plan: &K2SupportProbePlanV1,
        learner: &K2EffectLearnerManifestV1,
        budget: &K2LearnedCapabilityBudgetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        support.validate()?;
        learner.validate()?;
        budget.validate()?;
        if self.schema != K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.support_set_root_sha256 != support.support_set_root_sha256
            || self.support_probe_schedule_public_root_sha256 != plan.public_schedule_root_sha256
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.learner_manifest_root_sha256 != learner.manifest_root_sha256
            || self.learner_executable_sha256 != learner.executable_sha256
            || self.learner_budget_root_sha256 != budget.root_v1()?
            || self.public_context_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learner_public_context_invalid",
            ));
        }
        require_learned_root_v1(
            &self.public_experiment_id_sha256,
            "k2_public_experiment_id_invalid",
        )
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1,
            self.public_experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_schedule_public_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learner_budget_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PrivateExperimentContractV1 {
    pub schema: String,
    pub private_contract_root_sha256: String,
    pub experiment_id_sha256: String,
    pub harness_commitment_sha256: String,
    pub public_context_root_sha256: String,
    pub hidden_action_mapping: K2HiddenActionMappingV1,
    pub support_source_manifest_roots_sha256: Vec<String>,
    pub target_pre_manifest: LawLabTreeManifestV1,
    pub target_expected_goal_manifest: LawLabTreeManifestV1,
    pub target_goal_store_snapshot_root_sha256: String,
}

impl K2PrivateExperimentContractV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        harness_commitment_sha256: String,
        context: &K2LearnerPublicContextV1,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: K2HiddenActionMappingV1,
        target_pre_manifest: LawLabTreeManifestV1,
        target_expected_goal_manifest: LawLabTreeManifestV1,
        target_goal_store_snapshot_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        for root in [
            experiment_id_sha256.as_str(),
            harness_commitment_sha256.as_str(),
            target_goal_store_snapshot_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_contract_root_invalid")?;
        }
        mapping.validate(catalog)?;
        support.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        target_expected_goal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut contract = Self {
            schema: K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1.to_owned(),
            private_contract_root_sha256: String::new(),
            experiment_id_sha256,
            harness_commitment_sha256,
            public_context_root_sha256: context.public_context_root_sha256.clone(),
            hidden_action_mapping: mapping,
            support_source_manifest_roots_sha256: support
                .worlds
                .iter()
                .map(|world| world.source_manifest.tree_root_sha256.clone())
                .collect(),
            target_pre_manifest,
            target_expected_goal_manifest,
            target_goal_store_snapshot_root_sha256,
        };
        contract.private_contract_root_sha256 = contract.expected_root_v1()?;
        contract.validate(context, catalog, support)?;
        Ok(contract)
    }

    pub fn validate(
        &self,
        context: &K2LearnerPublicContextV1,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.hidden_action_mapping.validate(catalog)?;
        support.validate()?;
        self.target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.target_expected_goal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.experiment_id_sha256.as_str(),
            self.harness_commitment_sha256.as_str(),
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_contract_root_invalid")?;
        }
        let expected_support_roots = support
            .worlds
            .iter()
            .map(|world| world.source_manifest.tree_root_sha256.as_str())
            .collect::<Vec<_>>();
        if self.schema != K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1
            || self.public_context_root_sha256 != context.public_context_root_sha256
            || self.hidden_action_mapping.catalog_root_sha256 != catalog.catalog_root_sha256
            || self
                .support_source_manifest_roots_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_support_roots
            || self.private_contract_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_private_experiment_contract_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn artifact_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(self)
    }

    pub fn target_holdout_commitment_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1,
            "target-holdout",
            self.target_pre_manifest.tree_root_sha256.as_str(),
            self.target_expected_goal_manifest.tree_root_sha256.as_str(),
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ))
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1,
            self.experiment_id_sha256.as_str(),
            self.harness_commitment_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            &self.hidden_action_mapping,
            &self.support_source_manifest_roots_sha256,
            &self.target_pre_manifest,
            &self.target_expected_goal_manifest,
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ))
    }
}

pub struct K2LearnedCapabilityFreezeInputV1<'a> {
    pub private_contract: &'a K2PrivateExperimentContractV1,
    pub public_context: &'a K2LearnerPublicContextV1,
    pub catalog: &'a K2OpaqueActionCatalogV1,
    pub support: &'a K2SupportWorldSetV1,
    pub plan: &'a K2SupportProbePlanV1,
    pub learner: &'a K2EffectLearnerManifestV1,
    pub budget: &'a K2LearnedCapabilityBudgetV1,
    pub independent_verifier_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub sandbox_executor_manifest_root_sha256: String,
    pub sandbox_worker_sha256: String,
    pub exact_oracle_manifest_root_sha256: String,
    pub exact_oracle_executable_sha256: String,
    pub deterministic_seed_sha256: String,
    pub frozen_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityFreezeV1 {
    pub schema: String,
    pub freeze_root_sha256: String,
    pub experiment_id_sha256: String,
    pub public_context_root_sha256: String,
    pub private_contract_artifact_root_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub support_probe_plan_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub target_holdout_commitment_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub independent_verifier_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub sandbox_executor_manifest_root_sha256: String,
    pub sandbox_worker_sha256: String,
    pub exact_oracle_manifest_root_sha256: String,
    pub exact_oracle_executable_sha256: String,
    pub budget_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub frozen_at_unix_ms: u64,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilityFreezeV1 {
    pub fn seal(input: K2LearnedCapabilityFreezeInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input
            .private_contract
            .validate(input.public_context, input.catalog, input.support)?;
        input.plan.validate(
            input.catalog,
            input.support,
            &input.private_contract.hidden_action_mapping,
        )?;
        input.learner.validate()?;
        input.budget.validate()?;
        let executable_roots = [
            input.learner.executable_sha256.as_str(),
            input.selector_executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
            input.exact_oracle_executable_sha256.as_str(),
        ];
        require_unique_roots_v1(
            executable_roots,
            "k2_learned_executable_identities_not_distinct",
        )?;
        for root in [
            input.independent_verifier_contract_root_sha256.as_str(),
            input.sandbox_executor_manifest_root_sha256.as_str(),
            input.exact_oracle_manifest_root_sha256.as_str(),
            input.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_freeze_root_invalid")?;
        }
        let mut freeze = Self {
            schema: K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1.to_owned(),
            freeze_root_sha256: String::new(),
            experiment_id_sha256: input.private_contract.experiment_id_sha256.clone(),
            public_context_root_sha256: input.public_context.public_context_root_sha256.clone(),
            private_contract_artifact_root_sha256: input.private_contract.artifact_root_v1()?,
            catalog_root_sha256: input.catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: input.support.support_set_root_sha256.clone(),
            support_probe_plan_root_sha256: input.plan.plan_root_sha256.clone(),
            hidden_mapping_root_sha256: input
                .private_contract
                .hidden_action_mapping
                .mapping_root_sha256
                .clone(),
            target_holdout_commitment_root_sha256: input
                .private_contract
                .target_holdout_commitment_root_v1()?,
            learner_manifest_root_sha256: input.learner.manifest_root_sha256.clone(),
            learner_executable_sha256: input.learner.executable_sha256.clone(),
            independent_verifier_contract_root_sha256: input
                .independent_verifier_contract_root_sha256,
            selector_executable_sha256: input.selector_executable_sha256,
            sandbox_executor_manifest_root_sha256: input.sandbox_executor_manifest_root_sha256,
            sandbox_worker_sha256: input.sandbox_worker_sha256,
            exact_oracle_manifest_root_sha256: input.exact_oracle_manifest_root_sha256,
            exact_oracle_executable_sha256: input.exact_oracle_executable_sha256,
            budget_root_sha256: input.budget.root_v1()?,
            deterministic_seed_sha256: input.deterministic_seed_sha256,
            frozen_at_unix_ms: input.frozen_at_unix_ms,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        freeze.freeze_root_sha256 = freeze.expected_root_v1()?;
        freeze.validate_persisted_v1()?;
        Ok(freeze)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.freeze_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.private_contract_artifact_root_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_plan_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.target_holdout_commitment_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.independent_verifier_contract_root_sha256.as_str(),
            self.sandbox_executor_manifest_root_sha256.as_str(),
            self.exact_oracle_manifest_root_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_freeze_root_invalid")?;
        }
        require_unique_roots_v1(
            [
                self.learner_executable_sha256.as_str(),
                self.selector_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
                self.exact_oracle_executable_sha256.as_str(),
            ],
            "k2_learned_executable_identities_not_distinct",
        )?;
        if self.schema != K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1
            || self.freeze_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_freeze_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1,
            (
                self.experiment_id_sha256.as_str(),
                self.public_context_root_sha256.as_str(),
                self.private_contract_artifact_root_sha256.as_str(),
                self.catalog_root_sha256.as_str(),
                self.support_set_root_sha256.as_str(),
                self.support_probe_plan_root_sha256.as_str(),
                self.hidden_mapping_root_sha256.as_str(),
                self.target_holdout_commitment_root_sha256.as_str(),
                self.learner_manifest_root_sha256.as_str(),
                self.learner_executable_sha256.as_str(),
            ),
            (
                self.independent_verifier_contract_root_sha256.as_str(),
                self.selector_executable_sha256.as_str(),
                self.sandbox_executor_manifest_root_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
                self.exact_oracle_manifest_root_sha256.as_str(),
                self.exact_oracle_executable_sha256.as_str(),
                self.budget_root_sha256.as_str(),
                self.deterministic_seed_sha256.as_str(),
                self.frozen_at_unix_ms,
                &self.authority,
            ),
        ))
    }
}

