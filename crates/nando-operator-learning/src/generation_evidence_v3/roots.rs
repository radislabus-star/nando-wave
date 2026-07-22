use nando_operator_kernel::canonical_json_sha256;

use super::{
    GENERATION_EVIDENCE_LEDGER_SCHEMA_V3, GenerationEvidenceErrorV3, GenerationEvidenceLedgerV3,
    GenerationEvidencePartitionV3, GenerationEvidenceRecordV3, GenerationSupportFreezeV3,
};

impl GenerationEvidenceLedgerV3 {
    pub(super) fn partition_genesis_sha256(
        &self,
        partition: GenerationEvidencePartitionV3,
    ) -> Result<String, GenerationEvidenceErrorV3> {
        let freeze = self
            .freeze
            .as_ref()
            .map(GenerationSupportFreezeV3::freeze_sha256);
        canonical_json_sha256(&(
            GENERATION_EVIDENCE_LEDGER_SCHEMA_V3,
            "partition-genesis",
            self.generation_id_sha256.as_str(),
            partition,
            freeze,
        ))
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)
    }

    pub(super) fn support_partition_sha256(&self) -> Result<String, GenerationEvidenceErrorV3> {
        self.partition_sha256(GenerationEvidencePartitionV3::Support, &self.support)
    }

    pub fn future_partition_sha256(&self) -> Result<String, GenerationEvidenceErrorV3> {
        self.partition_sha256(GenerationEvidencePartitionV3::Future, &self.future)
    }

    fn partition_sha256(
        &self,
        partition: GenerationEvidencePartitionV3,
        records: &[GenerationEvidenceRecordV3],
    ) -> Result<String, GenerationEvidenceErrorV3> {
        canonical_json_sha256(&(
            GENERATION_EVIDENCE_LEDGER_SCHEMA_V3,
            "partition-root",
            self.generation_id_sha256.as_str(),
            partition,
            records.len(),
            records
                .last()
                .map(GenerationEvidenceRecordV3::record_sha256),
        ))
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)
    }

    pub fn evidence_root_sha256(&self) -> Result<String, GenerationEvidenceErrorV3> {
        canonical_json_sha256(&(
            GENERATION_EVIDENCE_LEDGER_SCHEMA_V3,
            "ledger-root",
            self.generation_id_sha256.as_str(),
            self.support_partition_sha256()?,
            self.freeze
                .as_ref()
                .map(GenerationSupportFreezeV3::freeze_sha256),
            self.future_partition_sha256()?,
        ))
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)
    }
}
