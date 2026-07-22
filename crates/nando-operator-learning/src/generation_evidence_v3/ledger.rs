use std::collections::BTreeSet;

use nando_operator_kernel::{
    OperatorGenerationManifestV3, canonical_json_sha256, valid_nonzero_sha256,
};

use super::{
    GENERATION_EVIDENCE_LEDGER_SCHEMA_V3, GENERATION_EVIDENCE_MAX_ROWS_PER_PARTITION_V3,
    GenerationEvidenceAccountingV3, GenerationEvidenceErrorV3, GenerationEvidenceObservationV3,
    GenerationEvidencePartitionV3, GenerationEvidenceRecordV3, GenerationLearningOutcomeV3,
    GenerationSupportFreezeV3,
};

pub struct GenerationEvidenceLedgerV3 {
    pub(super) generation_id_sha256: String,
    pub(super) support: Vec<GenerationEvidenceRecordV3>,
    pub(super) future: Vec<GenerationEvidenceRecordV3>,
    pub(super) freeze: Option<GenerationSupportFreezeV3>,
    support_lineages: BTreeSet<String>,
    future_lineages: BTreeSet<String>,
    event_roots: BTreeSet<String>,
    request_roots: BTreeSet<String>,
    receipt_roots: BTreeSet<String>,
    last_capture_sequence: u64,
}

impl GenerationEvidenceLedgerV3 {
    #[must_use]
    pub fn new(manifest: &OperatorGenerationManifestV3) -> Self {
        Self {
            generation_id_sha256: manifest.generation_id_sha256().to_owned(),
            support: Vec::new(),
            future: Vec::new(),
            freeze: None,
            support_lineages: BTreeSet::new(),
            future_lineages: BTreeSet::new(),
            event_roots: BTreeSet::new(),
            request_roots: BTreeSet::new(),
            receipt_roots: BTreeSet::new(),
            last_capture_sequence: 0,
        }
    }

    pub fn append_support(
        &mut self,
        observation: GenerationEvidenceObservationV3,
    ) -> Result<&GenerationEvidenceRecordV3, GenerationEvidenceErrorV3> {
        if self.freeze.is_some() {
            return Err(GenerationEvidenceErrorV3::SupportClosed);
        }
        if observation.capture_sequence >= observation.support_watermark_next_sequence
            || observation.support_freeze_sha256.is_some()
            || self.support.first().is_some_and(|record| {
                record.observation.support_watermark_next_sequence
                    != observation.support_watermark_next_sequence
            })
        {
            return Err(GenerationEvidenceErrorV3::InvalidPartitionBinding);
        }
        self.append(GenerationEvidencePartitionV3::Support, observation)
    }

    pub fn freeze_support(
        &mut self,
        next_capture_sequence: u64,
        watermark_root_sha256: String,
    ) -> Result<&GenerationSupportFreezeV3, GenerationEvidenceErrorV3> {
        if self.freeze.is_some() {
            return Err(GenerationEvidenceErrorV3::SupportClosed);
        }
        if self.support.is_empty() {
            return Err(GenerationEvidenceErrorV3::EmptySupport);
        }
        if next_capture_sequence <= self.last_capture_sequence {
            return Err(GenerationEvidenceErrorV3::InvalidSequence);
        }
        if !valid_nonzero_sha256(&watermark_root_sha256) {
            return Err(GenerationEvidenceErrorV3::InvalidRoot);
        }
        if self.support.iter().any(|record| {
            record.observation.support_watermark_next_sequence != next_capture_sequence
                || record.observation.support_freeze_sha256.is_some()
        }) {
            return Err(GenerationEvidenceErrorV3::InvalidPartitionBinding);
        }
        let support_partition_sha256 = self.support_partition_sha256()?;
        let support_lineages = u32::try_from(self.support_lineages.len())
            .map_err(|_| GenerationEvidenceErrorV3::PartitionBudgetExhausted)?;
        let freeze_sha256 = canonical_json_sha256(&(
            GENERATION_EVIDENCE_LEDGER_SCHEMA_V3,
            "support-freeze",
            self.generation_id_sha256.as_str(),
            next_capture_sequence,
            watermark_root_sha256.as_str(),
            support_partition_sha256.as_str(),
            support_lineages,
        ))
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)?;
        self.freeze = Some(GenerationSupportFreezeV3 {
            next_capture_sequence,
            watermark_root_sha256,
            support_partition_sha256,
            support_lineages,
            freeze_sha256,
        });
        self.freeze
            .as_ref()
            .ok_or(GenerationEvidenceErrorV3::InvalidFreeze)
    }

    pub fn append_future(
        &mut self,
        observation: GenerationEvidenceObservationV3,
    ) -> Result<&GenerationEvidenceRecordV3, GenerationEvidenceErrorV3> {
        let freeze = self
            .freeze
            .as_ref()
            .ok_or(GenerationEvidenceErrorV3::SupportNotFrozen)?;
        if observation.capture_sequence < freeze.next_capture_sequence {
            return Err(GenerationEvidenceErrorV3::BeforeWatermark);
        }
        if observation.support_watermark_next_sequence != freeze.next_capture_sequence
            || observation.support_freeze_sha256.as_deref() != Some(freeze.freeze_sha256())
        {
            return Err(GenerationEvidenceErrorV3::InvalidPartitionBinding);
        }
        if self
            .support_lineages
            .contains(&observation.lineage_root_sha256)
        {
            return Err(GenerationEvidenceErrorV3::CrossPartitionLineage);
        }
        self.append(GenerationEvidencePartitionV3::Future, observation)
    }

    fn append(
        &mut self,
        partition: GenerationEvidencePartitionV3,
        observation: GenerationEvidenceObservationV3,
    ) -> Result<&GenerationEvidenceRecordV3, GenerationEvidenceErrorV3> {
        observation.validate()?;
        if observation.generation_id_sha256 != self.generation_id_sha256 {
            return Err(GenerationEvidenceErrorV3::InvalidGeneration);
        }
        if observation.capture_sequence <= self.last_capture_sequence {
            return Err(GenerationEvidenceErrorV3::InvalidSequence);
        }
        let records = match partition {
            GenerationEvidencePartitionV3::Support => &self.support,
            GenerationEvidencePartitionV3::Future => &self.future,
        };
        if records.len() >= GENERATION_EVIDENCE_MAX_ROWS_PER_PARTITION_V3 {
            return Err(GenerationEvidenceErrorV3::PartitionBudgetExhausted);
        }
        self.ensure_unique(&observation)?;
        let ordinal = u32::try_from(records.len())
            .map_err(|_| GenerationEvidenceErrorV3::PartitionBudgetExhausted)?;
        let previous_record_sha256 = records
            .last()
            .map(|record| record.record_sha256.clone())
            .unwrap_or(self.partition_genesis_sha256(partition)?);
        let record_sha256 = GenerationEvidenceRecordV3::digest(
            &self.generation_id_sha256,
            partition,
            ordinal,
            &previous_record_sha256,
            &observation,
        )?;
        let lineage = observation.lineage_root_sha256.clone();
        self.event_roots
            .insert(observation.event_root_sha256.clone());
        self.request_roots
            .insert(observation.request_root_sha256.clone());
        self.receipt_roots
            .insert(observation.verifier_receipt_root_sha256.clone());
        self.last_capture_sequence = observation.capture_sequence;
        let record = GenerationEvidenceRecordV3 {
            partition,
            ordinal,
            previous_record_sha256,
            observation,
            record_sha256,
        };
        let records = match partition {
            GenerationEvidencePartitionV3::Support => {
                self.support_lineages.insert(lineage);
                &mut self.support
            }
            GenerationEvidencePartitionV3::Future => {
                self.future_lineages.insert(lineage);
                &mut self.future
            }
        };
        records.push(record);
        records
            .last()
            .ok_or(GenerationEvidenceErrorV3::InvalidRecord)
    }

    fn ensure_unique(
        &self,
        observation: &GenerationEvidenceObservationV3,
    ) -> Result<(), GenerationEvidenceErrorV3> {
        if self.event_roots.contains(&observation.event_root_sha256) {
            return Err(GenerationEvidenceErrorV3::DuplicateEvent);
        }
        if self
            .request_roots
            .contains(&observation.request_root_sha256)
        {
            return Err(GenerationEvidenceErrorV3::DuplicateRequest);
        }
        if self
            .receipt_roots
            .contains(&observation.verifier_receipt_root_sha256)
        {
            return Err(GenerationEvidenceErrorV3::DuplicateReceipt);
        }
        Ok(())
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub fn support(&self) -> &[GenerationEvidenceRecordV3] {
        &self.support
    }

    #[must_use]
    pub fn future(&self) -> &[GenerationEvidenceRecordV3] {
        &self.future
    }

    #[must_use]
    pub const fn freeze(&self) -> Option<&GenerationSupportFreezeV3> {
        self.freeze.as_ref()
    }

    #[must_use]
    pub fn accounting(&self) -> GenerationEvidenceAccountingV3 {
        let mut accounting = GenerationEvidenceAccountingV3 {
            support_rows: self.support.len(),
            support_lineages: self.support_lineages.len(),
            future_rows: self.future.len(),
            future_lineages: self.future_lineages.len(),
            ..GenerationEvidenceAccountingV3::default()
        };
        for outcome in self
            .support
            .iter()
            .chain(&self.future)
            .map(|record| record.observation.outcome)
        {
            match outcome {
                GenerationLearningOutcomeV3::VerifiedPass => accounting.positive_rows += 1,
                GenerationLearningOutcomeV3::ApplicabilityNegative => {
                    accounting.applicability_negative_rows += 1;
                }
                GenerationLearningOutcomeV3::HardContradiction => {
                    accounting.hard_contradiction_rows += 1;
                }
                GenerationLearningOutcomeV3::Censored(_) => accounting.censored_rows += 1,
            }
        }
        accounting
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
