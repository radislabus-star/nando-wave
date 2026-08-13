use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    PreActionMultiSourceTopologyV1, ResponseProgram, canonical_json_sha256,
    response_program_version_root_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use super::{MultiSourceT1IdentificationV3, SourceNeutralTopologyMotifV1};
use crate::multi_source::bind_pre_action_t1_program_to_motif_v1;

pub const PROGRAM_DISPOSITION_SCHEMA_V1: &str = "nando.k1-program-disposition.v1";
pub const PROGRAM_DISPOSITION_SET_SCHEMA_V1: &str = "nando.k1-program-disposition-set.v1";
pub const IDENTIFIER_RESULT_SCHEMA_V1: &str = "nando.k1-identifier-result.v1";
pub const TERMINAL_DIAGNOSTIC_SCHEMA_V1: &str = "nando.k1-terminal-diagnostic.v1";

#[must_use]
pub fn deterministic_initial_blocker_v1(blocker: &str) -> bool {
    matches!(
        blocker,
        "motif_program_candidates_empty"
            | "natural_collection_candidate_artifact_missing"
            | "natural_collection_candidate_generation_empty"
            | "all_supported_t1_protocol_modes_already_active"
    )
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramRejectionCodeV1 {
    ProgramInvalid,
    TopologyInvalid,
    MotifInvalid,
    ConsumedRoleInvalid,
    ConsumedRolesOutsideFrozenMotif,
    BindingInvalid,
    InternalUnclassified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramDispositionV1 {
    pub schema: String,
    pub disposition_root_sha256: String,
    pub seed_program_root_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_binding_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_code: Option<ProgramRejectionCodeV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramDispositionSetV1 {
    pub schema: String,
    pub disposition_set_root_sha256: String,
    pub seed_count: u64,
    pub accepted_count: u64,
    pub rejected_count: u64,
    pub accepted_set_root_sha256: String,
    pub rejection_histogram: BTreeMap<ProgramRejectionCodeV1, u64>,
    pub dispositions: Vec<ProgramDispositionV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentifierResultV1 {
    pub schema: String,
    pub identifier_result_root_sha256: String,
    pub opportunity_root_sha256: String,
    pub accepted_set_root_sha256: String,
    pub disposition_set_root_sha256: String,
    pub identifier_report_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalDiagnosticV1 {
    pub schema: String,
    pub terminal_diagnostic_root_sha256: String,
    pub opportunity_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub identifier_result_root_sha256: String,
    pub seed_count: u64,
    pub accepted_count: u64,
    pub rejected_count: u64,
    pub rejection_histogram: BTreeMap<ProgramRejectionCodeV1, u64>,
    pub deterministic: bool,
}

impl ProgramDispositionV1 {
    fn accepted(
        seed_program_root_sha256: String,
        binding_root_sha256: String,
    ) -> Result<Self, &'static str> {
        Self::seal(seed_program_root_sha256, Some(binding_root_sha256), None)
    }

    fn rejected(
        seed_program_root_sha256: String,
        code: ProgramRejectionCodeV1,
    ) -> Result<Self, &'static str> {
        Self::seal(seed_program_root_sha256, None, Some(code))
    }

    fn seal(
        seed_program_root_sha256: String,
        accepted_binding_root_sha256: Option<String>,
        rejected_code: Option<ProgramRejectionCodeV1>,
    ) -> Result<Self, &'static str> {
        let mut value = Self {
            schema: PROGRAM_DISPOSITION_SCHEMA_V1.to_owned(),
            disposition_root_sha256: String::new(),
            seed_program_root_sha256,
            accepted_binding_root_sha256,
            rejected_code,
        };
        value.disposition_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let accepted = self.accepted_binding_root_sha256.as_deref();
        if self.schema != PROGRAM_DISPOSITION_SCHEMA_V1
            || !valid_nonzero_sha256(&self.seed_program_root_sha256)
            || !valid_nonzero_sha256(&self.disposition_root_sha256)
            || !matches!(
                (accepted, self.rejected_code),
                (Some(_), None) | (None, Some(_))
            )
            || accepted.is_some_and(|root| !valid_nonzero_sha256(root))
            || self.disposition_root_sha256 != self.expected_root()?
        {
            return Err("program_disposition_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            PROGRAM_DISPOSITION_SCHEMA_V1,
            self.seed_program_root_sha256.as_str(),
            self.accepted_binding_root_sha256.as_deref(),
            self.rejected_code,
        ))
    }
}

pub fn evaluate_program_dispositions_v1(
    programs: &BTreeMap<String, ResponseProgram>,
    topology: &PreActionMultiSourceTopologyV1,
    motif: &SourceNeutralTopologyMotifV1,
) -> Result<(ProgramDispositionSetV1, BTreeMap<String, ResponseProgram>), &'static str> {
    let mut dispositions = Vec::with_capacity(programs.len());
    let mut accepted = BTreeMap::new();
    for (program_root, program) in programs {
        if response_program_version_root_sha256(program).as_deref() != Ok(program_root.as_str()) {
            dispositions.push(ProgramDispositionV1::rejected(
                program_root.clone(),
                ProgramRejectionCodeV1::ProgramInvalid,
            )?);
            continue;
        }
        match bind_pre_action_t1_program_to_motif_v1(program, topology, motif) {
            Ok(binding) => {
                dispositions.push(ProgramDispositionV1::accepted(
                    program_root.clone(),
                    binding.binding_root_sha256,
                )?);
                accepted.insert(program_root.clone(), program.clone());
            }
            Err(error) => dispositions.push(ProgramDispositionV1::rejected(
                program_root.clone(),
                rejection_code(error),
            )?),
        }
    }
    let disposition_set = ProgramDispositionSetV1::seal(dispositions)?;
    Ok((disposition_set, accepted))
}

fn rejection_code(error: &str) -> ProgramRejectionCodeV1 {
    match error {
        "unsupported_program_schema" | "invalid_output_budget" => {
            ProgramRejectionCodeV1::ProgramInvalid
        }
        "multi_source_topology_invalid" => ProgramRejectionCodeV1::TopologyInvalid,
        "source_neutral_topology_motif_invalid" => ProgramRejectionCodeV1::MotifInvalid,
        "program_consumed_roles_outside_frozen_motif" => {
            ProgramRejectionCodeV1::ConsumedRolesOutsideFrozenMotif
        }
        "pre_action_t1_motif_binding_invalid" => ProgramRejectionCodeV1::BindingInvalid,
        error if error.contains("role") => ProgramRejectionCodeV1::ConsumedRoleInvalid,
        _ => ProgramRejectionCodeV1::InternalUnclassified,
    }
}

impl ProgramDispositionSetV1 {
    pub fn seal(mut dispositions: Vec<ProgramDispositionV1>) -> Result<Self, &'static str> {
        dispositions.sort_by(|left, right| {
            left.seed_program_root_sha256
                .cmp(&right.seed_program_root_sha256)
        });
        let accepted_roots = dispositions
            .iter()
            .filter_map(|value| {
                value
                    .accepted_binding_root_sha256
                    .as_ref()
                    .map(|_| value.seed_program_root_sha256.clone())
            })
            .collect::<Vec<_>>();
        let mut rejection_histogram = BTreeMap::new();
        for code in dispositions.iter().filter_map(|value| value.rejected_code) {
            *rejection_histogram.entry(code).or_default() += 1;
        }
        let seed_count =
            u64::try_from(dispositions.len()).map_err(|_| "program_disposition_count")?;
        let accepted_count =
            u64::try_from(accepted_roots.len()).map_err(|_| "program_disposition_count")?;
        let rejected_count = seed_count.saturating_sub(accepted_count);
        let accepted_set_root_sha256 =
            canonical_json_sha256(&("nando.k1-accepted-program-set.v1", &accepted_roots))?;
        let disposition_set_root_sha256 = canonical_json_sha256(&(
            PROGRAM_DISPOSITION_SET_SCHEMA_V1,
            seed_count,
            accepted_count,
            rejected_count,
            accepted_set_root_sha256.as_str(),
            &rejection_histogram,
            dispositions
                .iter()
                .map(|value| value.disposition_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))?;
        let value = Self {
            schema: PROGRAM_DISPOSITION_SET_SCHEMA_V1.to_owned(),
            disposition_set_root_sha256,
            seed_count,
            accepted_count,
            rejected_count,
            accepted_set_root_sha256,
            rejection_histogram,
            dispositions,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let seed_roots = self
            .dispositions
            .iter()
            .map(|value| value.seed_program_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let accepted_roots = self
            .dispositions
            .iter()
            .filter_map(|value| {
                value
                    .accepted_binding_root_sha256
                    .as_ref()
                    .map(|_| value.seed_program_root_sha256.clone())
            })
            .collect::<Vec<_>>();
        let mut rejection_histogram = BTreeMap::new();
        for code in self
            .dispositions
            .iter()
            .filter_map(|value| value.rejected_code)
        {
            *rejection_histogram.entry(code).or_default() += 1;
        }
        let accepted_set_root_sha256 =
            canonical_json_sha256(&("nando.k1-accepted-program-set.v1", &accepted_roots))?;
        let disposition_set_root_sha256 = canonical_json_sha256(&(
            PROGRAM_DISPOSITION_SET_SCHEMA_V1,
            self.seed_count,
            self.accepted_count,
            self.rejected_count,
            accepted_set_root_sha256.as_str(),
            &rejection_histogram,
            self.dispositions
                .iter()
                .map(|value| value.disposition_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))?;
        if self.schema != PROGRAM_DISPOSITION_SET_SCHEMA_V1
            || self
                .dispositions
                .iter()
                .any(|value| value.validate().is_err())
            || seed_roots.len() != self.dispositions.len()
            || self
                .dispositions
                .windows(2)
                .any(|pair| pair[0].seed_program_root_sha256 >= pair[1].seed_program_root_sha256)
            || self.seed_count != self.accepted_count.saturating_add(self.rejected_count)
            || self.rejection_histogram.values().copied().sum::<u64>() != self.rejected_count
            || self.rejection_histogram != rejection_histogram
            || self.accepted_set_root_sha256 != accepted_set_root_sha256
            || self.disposition_set_root_sha256 != disposition_set_root_sha256
        {
            return Err("program_disposition_set_invalid");
        }
        Ok(())
    }
}

impl IdentifierResultV1 {
    pub fn seal(
        opportunity_root_sha256: String,
        disposition: &ProgramDispositionSetV1,
        identifier: &MultiSourceT1IdentificationV3,
    ) -> Result<Self, &'static str> {
        disposition.validate()?;
        if !identifier.validate() || !valid_nonzero_sha256(&opportunity_root_sha256) {
            return Err("identifier_result_input_invalid");
        }
        let identifier_result_root_sha256 = canonical_json_sha256(&(
            IDENTIFIER_RESULT_SCHEMA_V1,
            opportunity_root_sha256.as_str(),
            disposition.accepted_set_root_sha256.as_str(),
            disposition.disposition_set_root_sha256.as_str(),
            identifier.report_root_sha256.as_str(),
        ))?;
        let value = Self {
            schema: IDENTIFIER_RESULT_SCHEMA_V1.to_owned(),
            identifier_result_root_sha256,
            opportunity_root_sha256,
            accepted_set_root_sha256: disposition.accepted_set_root_sha256.clone(),
            disposition_set_root_sha256: disposition.disposition_set_root_sha256.clone(),
            identifier_report_root_sha256: identifier.report_root_sha256.clone(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != IDENTIFIER_RESULT_SCHEMA_V1
            || [
                self.identifier_result_root_sha256.as_str(),
                self.opportunity_root_sha256.as_str(),
                self.accepted_set_root_sha256.as_str(),
                self.disposition_set_root_sha256.as_str(),
                self.identifier_report_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.identifier_result_root_sha256
                != canonical_json_sha256(&(
                    IDENTIFIER_RESULT_SCHEMA_V1,
                    self.opportunity_root_sha256.as_str(),
                    self.accepted_set_root_sha256.as_str(),
                    self.disposition_set_root_sha256.as_str(),
                    self.identifier_report_root_sha256.as_str(),
                ))?
        {
            return Err("identifier_result_invalid");
        }
        Ok(())
    }
}

impl TerminalDiagnosticV1 {
    pub fn seal(
        candidate_freeze_root_sha256: String,
        result: &IdentifierResultV1,
        disposition: &ProgramDispositionSetV1,
    ) -> Result<Self, &'static str> {
        disposition.validate()?;
        if !valid_nonzero_sha256(&candidate_freeze_root_sha256) {
            return Err("terminal_diagnostic_input_invalid");
        }
        let deterministic = !disposition
            .rejection_histogram
            .contains_key(&ProgramRejectionCodeV1::InternalUnclassified);
        let terminal_diagnostic_root_sha256 = canonical_json_sha256(&(
            TERMINAL_DIAGNOSTIC_SCHEMA_V1,
            result.opportunity_root_sha256.as_str(),
            candidate_freeze_root_sha256.as_str(),
            result.identifier_result_root_sha256.as_str(),
            disposition.seed_count,
            disposition.accepted_count,
            disposition.rejected_count,
            &disposition.rejection_histogram,
            deterministic,
        ))?;
        let value = Self {
            schema: TERMINAL_DIAGNOSTIC_SCHEMA_V1.to_owned(),
            terminal_diagnostic_root_sha256,
            opportunity_root_sha256: result.opportunity_root_sha256.clone(),
            candidate_freeze_root_sha256,
            identifier_result_root_sha256: result.identifier_result_root_sha256.clone(),
            seed_count: disposition.seed_count,
            accepted_count: disposition.accepted_count,
            rejected_count: disposition.rejected_count,
            rejection_histogram: disposition.rejection_histogram.clone(),
            deterministic,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != TERMINAL_DIAGNOSTIC_SCHEMA_V1
            || [
                self.terminal_diagnostic_root_sha256.as_str(),
                self.opportunity_root_sha256.as_str(),
                self.candidate_freeze_root_sha256.as_str(),
                self.identifier_result_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.seed_count != self.accepted_count.saturating_add(self.rejected_count)
            || self.rejection_histogram.values().copied().sum::<u64>() != self.rejected_count
            || self.deterministic
                == self
                    .rejection_histogram
                    .contains_key(&ProgramRejectionCodeV1::InternalUnclassified)
            || self.terminal_diagnostic_root_sha256
                != canonical_json_sha256(&(
                    TERMINAL_DIAGNOSTIC_SCHEMA_V1,
                    self.opportunity_root_sha256.as_str(),
                    self.candidate_freeze_root_sha256.as_str(),
                    self.identifier_result_root_sha256.as_str(),
                    self.seed_count,
                    self.accepted_count,
                    self.rejected_count,
                    &self.rejection_histogram,
                    self.deterministic,
                ))?
        {
            return Err("terminal_diagnostic_invalid");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        CollectionProgramStep, CollectionScalarType, MultiSourceCardinalityClassV1,
        MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1,
        MultiSourceRelationKindV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
        MultiSourceTemporalClassV1, MultiSourceTypeClassV1, ResponseValueSelector,
        ValueProjectionFormat, response_program_version_root_sha256, sha256_bytes,
    };

    use super::*;
    use crate::multi_source::source_neutral_topology_motifs_v1;

    fn root(value: u64) -> String {
        format!("{value:064x}")
    }

    fn role(
        local_role_id: u16,
        source_ordinal: u16,
        type_class: MultiSourceTypeClassV1,
        container_class: MultiSourceContainerClassV1,
    ) -> MultiSourceRoleNodeV1 {
        MultiSourceRoleNodeV1 {
            local_role_id,
            source_ordinal,
            value_ordinal: 0,
            type_class,
            container_class,
            cardinality_class: MultiSourceCardinalityClassV1::One,
            temporal_class: MultiSourceTemporalClassV1::Latest,
            depth_bucket: 1,
            structural_flags: 0,
        }
    }

    fn topology() -> PreActionMultiSourceTopologyV1 {
        PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 1,
            roles: vec![
                role(
                    1,
                    0,
                    MultiSourceTypeClassV1::Array,
                    MultiSourceContainerClassV1::Sequence,
                ),
                role(
                    2,
                    2,
                    MultiSourceTypeClassV1::String,
                    MultiSourceContainerClassV1::Scalar,
                ),
            ],
            role_witnesses: vec![
                MultiSourceRoleWitnessV1 {
                    local_role_id: 1,
                    value_sha256: sha256_bytes(b"private-collection-value"),
                    request_reference_ordinal: None,
                    request_reference_ordinal_candidates: Vec::new(),
                },
                MultiSourceRoleWitnessV1 {
                    local_role_id: 2,
                    value_sha256: sha256_bytes(b"private-selector-value"),
                    request_reference_ordinal: None,
                    request_reference_ordinal_candidates: Vec::new(),
                },
            ],
            relations: vec![MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Contains,
                source_role_id: 1,
                target_role_id: 2,
            }],
        }
    }

    fn collection_only() -> ResponseProgram {
        ResponseProgram::compose_collection(
            vec![CollectionProgramStep::SelectOnlyArrayField],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        )
    }

    fn filter_count() -> ResponseProgram {
        ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 },
                CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                    selector: ResponseValueSelector::UniqueScalar {
                        value_type: nando_operator_kernel::AtomValueType::String,
                    },
                    value_type: CollectionScalarType::String,
                },
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        )
    }

    fn programs() -> BTreeMap<String, ResponseProgram> {
        [collection_only(), filter_count()]
            .into_iter()
            .map(|program| {
                (
                    response_program_version_root_sha256(&program).expect("program root"),
                    program,
                )
            })
            .collect()
    }

    fn motif(role_count: u8, required_role: Option<u16>) -> SourceNeutralTopologyMotifV1 {
        source_neutral_topology_motifs_v1(&topology())
            .expect("motifs")
            .into_iter()
            .find(|motif| {
                motif.role_count == role_count
                    && required_role.is_none_or(|role| {
                        motif
                            .embeddings
                            .iter()
                            .any(|embedding| embedding.local_role_ids.contains(&role))
                    })
            })
            .expect("motif")
    }

    #[test]
    fn evaluator_conserves_mixed_all_and_empty_seed_sets() {
        let topology = topology();
        let programs = programs();
        let (mixed, accepted) =
            evaluate_program_dispositions_v1(&programs, &topology, &motif(1, Some(1)))
                .expect("mixed");
        assert_eq!(
            (mixed.seed_count, mixed.accepted_count, mixed.rejected_count),
            (2, 1, 1)
        );
        assert_eq!(accepted.len(), 1);
        assert_eq!(
            mixed
                .rejection_histogram
                .get(&ProgramRejectionCodeV1::ConsumedRolesOutsideFrozenMotif),
            Some(&1)
        );

        let (all, accepted) =
            evaluate_program_dispositions_v1(&programs, &topology, &motif(2, None))
                .expect("all accepted");
        assert_eq!(
            (all.seed_count, all.accepted_count, all.rejected_count),
            (2, 2, 0)
        );
        assert_eq!(accepted.len(), 2);

        let (none, accepted) =
            evaluate_program_dispositions_v1(&programs, &topology, &motif(1, Some(2)))
                .expect("all rejected");
        assert_eq!(
            (none.seed_count, none.accepted_count, none.rejected_count),
            (2, 0, 2)
        );
        assert!(accepted.is_empty());

        let empty_programs = BTreeMap::new();
        let (empty, accepted) =
            evaluate_program_dispositions_v1(&empty_programs, &topology, &motif(2, None))
                .expect("empty");
        assert_eq!(
            (empty.seed_count, empty.accepted_count, empty.rejected_count),
            (0, 0, 0)
        );
        assert!(accepted.is_empty());
        empty.validate().expect("valid empty disposition");
    }

    #[test]
    fn evaluator_is_permutation_stable_and_rejects_duplicate_or_forged_dispositions() {
        let topology = topology();
        let programs = programs();
        let reversed = programs
            .iter()
            .rev()
            .map(|(root, value)| (root.clone(), value.clone()))
            .collect();
        let first = evaluate_program_dispositions_v1(&programs, &topology, &motif(1, Some(1)))
            .expect("first")
            .0;
        let second = evaluate_program_dispositions_v1(&reversed, &topology, &motif(1, Some(1)))
            .expect("second")
            .0;
        assert_eq!(first, second);

        assert_eq!(
            ProgramDispositionSetV1::seal(vec![
                first.dispositions[0].clone(),
                first.dispositions[0].clone(),
            ]),
            Err("program_disposition_set_invalid")
        );
        let mut forged = first;
        forged.accepted_count = forged.accepted_count.saturating_add(1);
        assert_eq!(forged.validate(), Err("program_disposition_set_invalid"));
    }

    #[test]
    fn diagnostic_bytes_are_hash_only_and_tamper_evident() {
        let topology = topology();
        let disposition =
            evaluate_program_dispositions_v1(&programs(), &topology, &motif(1, Some(1)))
                .expect("disposition")
                .0;
        let identifier = super::super::terminal_report(
            root(1),
            super::super::MultiSourceT1IdentificationStateV1::NoEligibleCohort,
            "motif_program_candidates_empty",
        );
        let result = IdentifierResultV1::seal(root(2), &disposition, &identifier)
            .expect("identifier result");
        let diagnostic = TerminalDiagnosticV1::seal(root(3), &result, &disposition)
            .expect("terminal diagnostic");
        result.validate().expect("valid result");
        diagnostic.validate().expect("valid diagnostic");

        let bytes = serde_json::to_vec(&diagnostic).expect("diagnostic bytes");
        let text = std::str::from_utf8(&bytes).expect("utf8");
        for secret in [
            "private-collection-value",
            "private-selector-value",
            "expected response",
            "rendered answer",
        ] {
            assert!(!text.contains(secret));
        }

        let mut forged = diagnostic;
        forged.identifier_result_root_sha256 = root(99);
        assert_eq!(forged.validate(), Err("terminal_diagnostic_invalid"));
    }
}
