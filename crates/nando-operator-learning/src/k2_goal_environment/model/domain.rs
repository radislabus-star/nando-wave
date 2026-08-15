#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2EvidenceProvenanceV1 {
    GeneratedCapabilitySelfTest,
    CertificateBoundK1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2GoalHorizonV1 {
    SingleSandboxTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AuthorityBoundaryV1 {
    pub schema: String,
    pub law_certificate_issued: bool,
    pub package_activated: bool,
    pub execution_authority_granted: bool,
    pub k1_registry_mutated: bool,
    pub k2_claim_granted: bool,
    pub phase_memory_mutated: bool,
    pub product_economics_credited: bool,
    pub natural_holdout_satisfied: bool,
}

impl K2AuthorityBoundaryV1 {
    #[must_use]
    pub fn authority_free_v1() -> Self {
        Self {
            schema: K2_AUTHORITY_BOUNDARY_SCHEMA_V1.to_owned(),
            law_certificate_issued: false,
            package_activated: false,
            execution_authority_granted: false,
            k1_registry_mutated: false,
            k2_claim_granted: false,
            phase_memory_mutated: false,
            product_economics_credited: false,
            natural_holdout_satisfied: false,
        }
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self == &Self::authority_free_v1() {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_authority_boundary_violated",
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GoalEnvelopeV1 {
    pub schema: String,
    pub goal_envelope_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub environment_root_sha256: String,
    pub goal_predicate_root_sha256: String,
    pub expected_terminal_tree_root_sha256: String,
    pub expected_goal_store_snapshot_root_sha256: String,
    pub constraints_root_sha256: String,
    pub oracle_contract_root_sha256: String,
    pub horizon: K2GoalHorizonV1,
    pub created_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2GoalEnvelopeDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    environment_root_sha256: &'a str,
    goal_predicate_root_sha256: &'a str,
    expected_terminal_tree_root_sha256: &'a str,
    expected_goal_store_snapshot_root_sha256: &'a str,
    constraints_root_sha256: &'a str,
    oracle_contract_root_sha256: &'a str,
    horizon: K2GoalHorizonV1,
    created_at_unix_ms: u64,
}

impl K2GoalEnvelopeV1 {
    pub fn seal(
        provenance: K2EvidenceProvenanceV1,
        environment_root_sha256: String,
        expected_terminal_tree_root_sha256: String,
        expected_goal_store_snapshot_root_sha256: String,
        constraints_root_sha256: String,
        oracle_contract_root_sha256: String,
        created_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let goal_predicate_root_sha256 = canonical_root(&(
            K2_GOAL_PREDICATE_SCHEMA_V1,
            "workspace_tree_root_equals",
            expected_terminal_tree_root_sha256.as_str(),
        ))?;
        let mut goal = Self {
            schema: K2_GOAL_ENVELOPE_SCHEMA_V1.to_owned(),
            goal_envelope_root_sha256: String::new(),
            provenance,
            environment_root_sha256,
            goal_predicate_root_sha256,
            expected_terminal_tree_root_sha256,
            expected_goal_store_snapshot_root_sha256,
            constraints_root_sha256,
            oracle_contract_root_sha256,
            horizon: K2GoalHorizonV1::SingleSandboxTerminal,
            created_at_unix_ms,
        };
        goal.goal_envelope_root_sha256 = goal.expected_root()?;
        goal.validate()?;
        Ok(goal)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for (root, reason) in [
            (
                self.goal_envelope_root_sha256.as_str(),
                "k2_goal_root_invalid",
            ),
            (
                self.environment_root_sha256.as_str(),
                "k2_goal_environment_root_invalid",
            ),
            (
                self.expected_terminal_tree_root_sha256.as_str(),
                "k2_goal_expected_tree_root_invalid",
            ),
            (
                self.expected_goal_store_snapshot_root_sha256.as_str(),
                "k2_goal_store_snapshot_root_invalid",
            ),
            (
                self.constraints_root_sha256.as_str(),
                "k2_goal_constraints_root_invalid",
            ),
            (
                self.oracle_contract_root_sha256.as_str(),
                "k2_goal_oracle_contract_root_invalid",
            ),
        ] {
            require_root(root, reason)?;
        }
        let expected_predicate_root = canonical_root(&(
            K2_GOAL_PREDICATE_SCHEMA_V1,
            "workspace_tree_root_equals",
            self.expected_terminal_tree_root_sha256.as_str(),
        ))?;
        if self.schema != K2_GOAL_ENVELOPE_SCHEMA_V1
            || self.horizon != K2GoalHorizonV1::SingleSandboxTerminal
            || self.goal_predicate_root_sha256 != expected_predicate_root
            || self.goal_envelope_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_goal_envelope_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2GoalEnvelopeDigestV1 {
            schema: K2_GOAL_ENVELOPE_SCHEMA_V1,
            provenance: self.provenance,
            environment_root_sha256: &self.environment_root_sha256,
            goal_predicate_root_sha256: &self.goal_predicate_root_sha256,
            expected_terminal_tree_root_sha256: &self.expected_terminal_tree_root_sha256,
            expected_goal_store_snapshot_root_sha256: &self
                .expected_goal_store_snapshot_root_sha256,
            constraints_root_sha256: &self.constraints_root_sha256,
            oracle_contract_root_sha256: &self.oracle_contract_root_sha256,
            horizon: self.horizon,
            created_at_unix_ms: self.created_at_unix_ms,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GoalEnvironmentBudgetV1 {
    pub maximum_alternatives: u64,
    pub maximum_probes: u64,
    pub maximum_events_per_episode: u64,
    pub maximum_event_bytes: u64,
    pub maximum_episode_bytes: u64,
    pub maximum_retained_capability_episodes: u64,
}

impl K2GoalEnvironmentBudgetV1 {
    #[must_use]
    pub const fn preregistered_v1() -> Self {
        Self {
            maximum_alternatives: K2_MAX_ALTERNATIVES_V1 as u64,
            maximum_probes: 1,
            maximum_events_per_episode: K2_MAX_EVENTS_PER_EPISODE_V1,
            maximum_event_bytes: K2_MAX_EVENT_BYTES_V1,
            maximum_episode_bytes: K2_MAX_EPISODE_BYTES_V1,
            maximum_retained_capability_episodes: K2_MAX_RETAINED_CAPABILITY_EPISODES_V1,
        }
    }

    pub fn root(&self) -> K2GoalEnvironmentResultV1<String> {
        if self != &Self::preregistered_v1() {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_budget_invalid"));
        }
        canonical_root(&(K2_BUDGET_SCHEMA_V1, self))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2K1ActionRefInputV1 {
    pub provenance: K2EvidenceProvenanceV1,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
    pub fixture_effect_root_sha256: Option<String>,
    pub law_certificate_root_sha256: Option<String>,
    pub epistemic_registry_member_root_sha256: Option<String>,
    pub bundle_v4_root_sha256: Option<String>,
    pub execution_certificate_root_sha256: Option<String>,
    pub applicability_guard_root_sha256: Option<String>,
    pub effect_contract_root_sha256: Option<String>,
    pub semantic_class_root_sha256: Option<String>,
    pub role_topology_root_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2K1ActionRefV1 {
    pub schema: String,
    pub action_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
    pub fixture_effect_root_sha256: Option<String>,
    pub law_certificate_root_sha256: Option<String>,
    pub epistemic_registry_member_root_sha256: Option<String>,
    pub bundle_v4_root_sha256: Option<String>,
    pub execution_certificate_root_sha256: Option<String>,
    pub applicability_guard_root_sha256: Option<String>,
    pub effect_contract_root_sha256: Option<String>,
    pub semantic_class_root_sha256: Option<String>,
    pub role_topology_root_sha256: Option<String>,
}

#[derive(Serialize)]
struct K2K1ActionRefDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    applicability_environment_root_sha256: &'a str,
    applicability_receipt_root_sha256: &'a str,
    operation_plan_root_sha256: &'a str,
    predicted_consequence_root_sha256: &'a str,
    fixture_effect_root_sha256: Option<&'a str>,
    law_certificate_root_sha256: Option<&'a str>,
    epistemic_registry_member_root_sha256: Option<&'a str>,
    bundle_v4_root_sha256: Option<&'a str>,
    execution_certificate_root_sha256: Option<&'a str>,
    applicability_guard_root_sha256: Option<&'a str>,
    effect_contract_root_sha256: Option<&'a str>,
    semantic_class_root_sha256: Option<&'a str>,
    role_topology_root_sha256: Option<&'a str>,
}

impl K2K1ActionRefV1 {
    pub fn seal(input: K2K1ActionRefInputV1) -> K2GoalEnvironmentResultV1<Self> {
        let mut action = Self {
            schema: K2_ACTION_REF_SCHEMA_V1.to_owned(),
            action_root_sha256: String::new(),
            provenance: input.provenance,
            applicability_environment_root_sha256: input.applicability_environment_root_sha256,
            applicability_receipt_root_sha256: input.applicability_receipt_root_sha256,
            operation_plan_root_sha256: input.operation_plan_root_sha256,
            predicted_consequence_root_sha256: input.predicted_consequence_root_sha256,
            fixture_effect_root_sha256: input.fixture_effect_root_sha256,
            law_certificate_root_sha256: input.law_certificate_root_sha256,
            epistemic_registry_member_root_sha256: input.epistemic_registry_member_root_sha256,
            bundle_v4_root_sha256: input.bundle_v4_root_sha256,
            execution_certificate_root_sha256: input.execution_certificate_root_sha256,
            applicability_guard_root_sha256: input.applicability_guard_root_sha256,
            effect_contract_root_sha256: input.effect_contract_root_sha256,
            semantic_class_root_sha256: input.semantic_class_root_sha256,
            role_topology_root_sha256: input.role_topology_root_sha256,
        };
        action.action_root_sha256 = action.expected_root()?;
        action.validate()?;
        Ok(action)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.action_root_sha256.as_str(),
            self.applicability_environment_root_sha256.as_str(),
            self.applicability_receipt_root_sha256.as_str(),
            self.operation_plan_root_sha256.as_str(),
            self.predicted_consequence_root_sha256.as_str(),
        ] {
            require_root(root, "k2_action_required_root_invalid")?;
        }
        let certificate_roots = [
            self.law_certificate_root_sha256.as_deref(),
            self.epistemic_registry_member_root_sha256.as_deref(),
            self.bundle_v4_root_sha256.as_deref(),
            self.execution_certificate_root_sha256.as_deref(),
            self.applicability_guard_root_sha256.as_deref(),
            self.effect_contract_root_sha256.as_deref(),
            self.semantic_class_root_sha256.as_deref(),
            self.role_topology_root_sha256.as_deref(),
        ];
        if certificate_roots
            .iter()
            .flatten()
            .any(|root| !valid_nonzero_sha256(root))
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_action_certificate_root_invalid",
            ));
        }
        let provenance_valid = match self.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                self.fixture_effect_root_sha256
                    .as_deref()
                    .is_some_and(valid_nonzero_sha256)
                    && certificate_roots.iter().all(Option::is_none)
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                self.fixture_effect_root_sha256.is_none()
                    && certificate_roots.iter().all(Option::is_some)
            }
        };
        if self.schema != K2_ACTION_REF_SCHEMA_V1
            || !provenance_valid
            || self.action_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_action_ref_invalid"));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2K1ActionRefDigestV1 {
            schema: K2_ACTION_REF_SCHEMA_V1,
            provenance: self.provenance,
            applicability_environment_root_sha256: &self.applicability_environment_root_sha256,
            applicability_receipt_root_sha256: &self.applicability_receipt_root_sha256,
            operation_plan_root_sha256: &self.operation_plan_root_sha256,
            predicted_consequence_root_sha256: &self.predicted_consequence_root_sha256,
            fixture_effect_root_sha256: self.fixture_effect_root_sha256.as_deref(),
            law_certificate_root_sha256: self.law_certificate_root_sha256.as_deref(),
            epistemic_registry_member_root_sha256: self
                .epistemic_registry_member_root_sha256
                .as_deref(),
            bundle_v4_root_sha256: self.bundle_v4_root_sha256.as_deref(),
            execution_certificate_root_sha256: self.execution_certificate_root_sha256.as_deref(),
            applicability_guard_root_sha256: self.applicability_guard_root_sha256.as_deref(),
            effect_contract_root_sha256: self.effect_contract_root_sha256.as_deref(),
            semantic_class_root_sha256: self.semantic_class_root_sha256.as_deref(),
            role_topology_root_sha256: self.role_topology_root_sha256.as_deref(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2K1VocabularySnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub epistemic_registry_revision: Option<u64>,
    pub epistemic_registry_root_sha256: Option<String>,
    pub actions: Vec<K2K1ActionRefV1>,
    pub captured_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2K1VocabularySnapshotDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    epistemic_registry_revision: Option<u64>,
    epistemic_registry_root_sha256: Option<&'a str>,
    actions: &'a [K2K1ActionRefV1],
    captured_at_unix_ms: u64,
}

impl K2K1VocabularySnapshotV1 {
    pub fn seal(
        provenance: K2EvidenceProvenanceV1,
        epistemic_registry_revision: Option<u64>,
        epistemic_registry_root_sha256: Option<String>,
        mut actions: Vec<K2K1ActionRefV1>,
        captured_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<Self> {
        actions.sort_by(|left, right| left.action_root_sha256.cmp(&right.action_root_sha256));
        let mut snapshot = Self {
            schema: K2_VOCABULARY_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256: String::new(),
            provenance,
            epistemic_registry_revision,
            epistemic_registry_root_sha256,
            actions,
            captured_at_unix_ms,
        };
        snapshot.snapshot_root_sha256 = snapshot.expected_root()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_root(
            &self.snapshot_root_sha256,
            "k2_vocabulary_snapshot_root_invalid",
        )?;
        if self.actions.len() < 2 || self.actions.len() > K2_MAX_ALTERNATIVES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_size_invalid",
            ));
        }
        for action in &self.actions {
            action.validate()?;
            if action.provenance != self.provenance {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_vocabulary_provenance_mismatch",
                ));
            }
        }
        if !self
            .actions
            .windows(2)
            .all(|pair| pair[0].action_root_sha256 < pair[1].action_root_sha256)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_action_order_invalid",
            ));
        }
        let registry_valid = match self.provenance {
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest => {
                self.epistemic_registry_revision.is_none()
                    && self.epistemic_registry_root_sha256.is_none()
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.fixture_effect_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .map(|action| action.predicted_consequence_root_sha256.as_str()),
                    )
            }
            K2EvidenceProvenanceV1::CertificateBoundK1 => {
                self.epistemic_registry_revision
                    .is_some_and(|revision| revision > 0)
                    && self
                        .epistemic_registry_root_sha256
                        .as_deref()
                        .is_some_and(valid_nonzero_sha256)
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.law_certificate_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.semantic_class_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .filter_map(|action| action.effect_contract_root_sha256.as_deref()),
                    )
                    && roots_are_unique(
                        self.actions
                            .iter()
                            .map(|action| action.predicted_consequence_root_sha256.as_str()),
                    )
            }
        };
        if self.schema != K2_VOCABULARY_SNAPSHOT_SCHEMA_V1
            || !registry_valid
            || self.snapshot_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_vocabulary_snapshot_invalid",
            ));
        }
        Ok(())
    }

    pub fn action(&self, action_root_sha256: &str) -> Option<&K2K1ActionRefV1> {
        self.actions
            .iter()
            .find(|action| action.action_root_sha256 == action_root_sha256)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2K1VocabularySnapshotDigestV1 {
            schema: K2_VOCABULARY_SNAPSHOT_SCHEMA_V1,
            provenance: self.provenance,
            epistemic_registry_revision: self.epistemic_registry_revision,
            epistemic_registry_root_sha256: self.epistemic_registry_root_sha256.as_deref(),
            actions: &self.actions,
            captured_at_unix_ms: self.captured_at_unix_ms,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativeV1 {
    pub action_root_sha256: String,
    pub applicability_environment_root_sha256: String,
    pub applicability_receipt_root_sha256: String,
    pub operation_plan_root_sha256: String,
    pub predicted_consequence_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2AlternativeSetV1 {
    pub schema: String,
    pub alternative_set_root_sha256: String,
    pub provenance: K2EvidenceProvenanceV1,
    pub vocabulary_snapshot_root_sha256: String,
    pub environment_root_sha256: String,
    pub alternatives: Vec<K2AlternativeV1>,
}

#[derive(Serialize)]
struct K2AlternativeSetDigestV1<'a> {
    schema: &'static str,
    provenance: K2EvidenceProvenanceV1,
    vocabulary_snapshot_root_sha256: &'a str,
    environment_root_sha256: &'a str,
    alternatives: &'a [K2AlternativeV1],
}

impl K2AlternativeSetV1 {
    pub fn seal(
        vocabulary: &K2K1VocabularySnapshotV1,
        environment_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        vocabulary.validate()?;
        let alternatives = vocabulary
            .actions
            .iter()
            .map(|action| K2AlternativeV1 {
                action_root_sha256: action.action_root_sha256.clone(),
                applicability_environment_root_sha256: action
                    .applicability_environment_root_sha256
                    .clone(),
                applicability_receipt_root_sha256: action.applicability_receipt_root_sha256.clone(),
                operation_plan_root_sha256: action.operation_plan_root_sha256.clone(),
                predicted_consequence_root_sha256: action.predicted_consequence_root_sha256.clone(),
            })
            .collect();
        let mut set = Self {
            schema: K2_ALTERNATIVE_SET_SCHEMA_V1.to_owned(),
            alternative_set_root_sha256: String::new(),
            provenance: vocabulary.provenance,
            vocabulary_snapshot_root_sha256: vocabulary.snapshot_root_sha256.clone(),
            environment_root_sha256,
            alternatives,
        };
        set.alternative_set_root_sha256 = set.expected_root()?;
        set.validate(vocabulary)?;
        Ok(set)
    }

    pub fn validate(&self, vocabulary: &K2K1VocabularySnapshotV1) -> K2GoalEnvironmentResultV1<()> {
        vocabulary.validate()?;
        require_root(
            &self.alternative_set_root_sha256,
            "k2_alternative_set_root_invalid",
        )?;
        require_root(
            &self.environment_root_sha256,
            "k2_alternative_environment_root_invalid",
        )?;
        if self.schema != K2_ALTERNATIVE_SET_SCHEMA_V1
            || self.provenance != vocabulary.provenance
            || self.vocabulary_snapshot_root_sha256 != vocabulary.snapshot_root_sha256
            || self.alternatives.len() != vocabulary.actions.len()
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_alternative_set_binding_invalid",
            ));
        }
        for (alternative, action) in self.alternatives.iter().zip(&vocabulary.actions) {
            if alternative.action_root_sha256 != action.action_root_sha256
                || alternative.applicability_environment_root_sha256 != self.environment_root_sha256
                || alternative.applicability_environment_root_sha256
                    != action.applicability_environment_root_sha256
                || alternative.applicability_receipt_root_sha256
                    != action.applicability_receipt_root_sha256
                || alternative.operation_plan_root_sha256 != action.operation_plan_root_sha256
                || alternative.predicted_consequence_root_sha256
                    != action.predicted_consequence_root_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_alternative_binding_invalid",
                ));
            }
        }
        if self.alternative_set_root_sha256 != self.expected_root()? {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_alternative_set_invalid",
            ));
        }
        Ok(())
    }

    pub fn alternative(&self, action_root_sha256: &str) -> Option<&K2AlternativeV1> {
        self.alternatives
            .iter()
            .find(|alternative| alternative.action_root_sha256 == action_root_sha256)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_root(&K2AlternativeSetDigestV1 {
            schema: K2_ALTERNATIVE_SET_SCHEMA_V1,
            provenance: self.provenance,
            vocabulary_snapshot_root_sha256: &self.vocabulary_snapshot_root_sha256,
            environment_root_sha256: &self.environment_root_sha256,
            alternatives: &self.alternatives,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2ExactOracleManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub executable_sha256: String,
}

impl K2ExactOracleManifestV1 {
    pub fn seal(executable_sha256: String) -> K2GoalEnvironmentResultV1<Self> {
        require_root(&executable_sha256, "k2_oracle_executable_sha_invalid")?;
        let mut manifest = Self {
            schema: K2_ORACLE_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            executable_sha256,
        };
        manifest.manifest_root_sha256 = canonical_root(&(
            K2_ORACLE_MANIFEST_SCHEMA_V1,
            manifest.executable_sha256.as_str(),
        ))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_root(&self.executable_sha256, "k2_oracle_executable_sha_invalid")?;
        if self.schema != K2_ORACLE_MANIFEST_SCHEMA_V1
            || self.manifest_root_sha256
                != canonical_root(&(
                    K2_ORACLE_MANIFEST_SCHEMA_V1,
                    self.executable_sha256.as_str(),
                ))?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_oracle_manifest_invalid",
            ));
        }
        Ok(())
    }
}

