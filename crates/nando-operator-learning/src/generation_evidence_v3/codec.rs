use serde::{Deserialize, Serialize};

use nando_operator_kernel::{
    OperatorGenerationManifestV3, canonical_json_bytes, valid_nonzero_sha256,
};

use super::{
    GENERATION_EVIDENCE_LEDGER_SCHEMA_V3, GENERATION_EVIDENCE_MAX_BYTES_V3,
    GenerationEvidenceErrorV3, GenerationEvidenceLedgerV3, GenerationEvidenceRecordV3,
    GenerationSupportFreezeV3,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GenerationEvidenceLedgerWireV3 {
    schema: String,
    generation_id_sha256: String,
    support: Vec<GenerationEvidenceRecordV3>,
    freeze: Option<GenerationSupportFreezeV3>,
    future: Vec<GenerationEvidenceRecordV3>,
    evidence_root_sha256: String,
    execution_authority: bool,
}

impl GenerationEvidenceLedgerV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, GenerationEvidenceErrorV3> {
        let bytes = canonical_json_bytes(&GenerationEvidenceLedgerWireV3 {
            schema: GENERATION_EVIDENCE_LEDGER_SCHEMA_V3.to_owned(),
            generation_id_sha256: self.generation_id_sha256.clone(),
            support: self.support.clone(),
            freeze: self.freeze.clone(),
            future: self.future.clone(),
            evidence_root_sha256: self.evidence_root_sha256()?,
            execution_authority: false,
        })
        .map_err(|_| GenerationEvidenceErrorV3::Serialization)?;
        if bytes.len() > GENERATION_EVIDENCE_MAX_BYTES_V3 {
            return Err(GenerationEvidenceErrorV3::LedgerBudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        manifest: &OperatorGenerationManifestV3,
    ) -> Result<Self, GenerationEvidenceErrorV3> {
        if bytes.len() > GENERATION_EVIDENCE_MAX_BYTES_V3 {
            return Err(GenerationEvidenceErrorV3::LedgerBudgetExhausted);
        }
        let wire: GenerationEvidenceLedgerWireV3 =
            serde_json::from_slice(bytes).map_err(|_| GenerationEvidenceErrorV3::InvalidLedger)?;
        if wire.schema != GENERATION_EVIDENCE_LEDGER_SCHEMA_V3
            || wire.execution_authority
            || !valid_nonzero_sha256(&wire.evidence_root_sha256)
            || wire.generation_id_sha256 != manifest.generation_id_sha256()
        {
            return Err(GenerationEvidenceErrorV3::InvalidGeneration);
        }
        let mut ledger = Self::new(manifest);
        for expected in wire.support {
            let actual = ledger.append_support(expected.observation.clone())?;
            if actual != &expected {
                return Err(GenerationEvidenceErrorV3::InvalidRecord);
            }
        }
        match wire.freeze {
            Some(expected) => {
                let actual = ledger.freeze_support(
                    expected.next_capture_sequence,
                    expected.watermark_root_sha256.clone(),
                )?;
                if actual != &expected {
                    return Err(GenerationEvidenceErrorV3::InvalidFreeze);
                }
            }
            None if !wire.future.is_empty() => {
                return Err(GenerationEvidenceErrorV3::SupportNotFrozen);
            }
            None => {}
        }
        for expected in wire.future {
            let actual = ledger.append_future(expected.observation.clone())?;
            if actual != &expected {
                return Err(GenerationEvidenceErrorV3::InvalidRecord);
            }
        }
        if ledger.evidence_root_sha256()? != wire.evidence_root_sha256
            || ledger.canonical_bytes()? != bytes
        {
            return Err(GenerationEvidenceErrorV3::InvalidLedger);
        }
        Ok(ledger)
    }
}
