use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1: &str =
    "nando.durable-runtime-parity-receipt.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DurableRuntimeParityReceipt {
    pub schema: String,
    pub receipt_sha256: String,
    pub evidence_ref_sha256: String,
    pub program_sha256: String,
    pub verifier_sha256: String,
    pub input_sha256: String,
    pub teacher_response_sha256: String,
    pub actor_response_sha256: String,
    pub actor_executed: bool,
    pub teacher_authority_match: bool,
    pub independent_verifier_pass: bool,
    pub exact_teacher_match: bool,
}

#[derive(Serialize)]
struct DurableRuntimeParityReceiptMaterial<'a> {
    schema: &'a str,
    evidence_ref_sha256: &'a str,
    program_sha256: &'a str,
    verifier_sha256: &'a str,
    input_sha256: &'a str,
    teacher_response_sha256: &'a str,
    actor_response_sha256: &'a str,
    actor_executed: bool,
    teacher_authority_match: bool,
    independent_verifier_pass: bool,
    exact_teacher_match: bool,
}

impl DurableRuntimeParityReceipt {
    pub fn validate_sealed(&self) -> Result<(), &'static str> {
        if self.schema != DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.receipt_sha256)
            || !valid_nonzero_sha256(&self.evidence_ref_sha256)
            || !valid_nonzero_sha256(&self.program_sha256)
            || !valid_nonzero_sha256(&self.verifier_sha256)
            || !valid_nonzero_sha256(&self.input_sha256)
            || !valid_nonzero_sha256(&self.teacher_response_sha256)
            || !valid_nonzero_sha256(&self.actor_response_sha256)
            || !self.actor_executed
            || !self.teacher_authority_match
            || !self.independent_verifier_pass
            || !self.exact_teacher_match
        {
            return Err("durable_runtime_parity_receipt_invalid");
        }
        if durable_runtime_parity_receipt_digest(self)? != self.receipt_sha256 {
            return Err("durable_runtime_parity_receipt_digest_mismatch");
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn seal_digest(&mut self) -> Result<(), &'static str> {
        self.receipt_sha256 = durable_runtime_parity_receipt_digest(self)?;
        Ok(())
    }
}

#[doc(hidden)]
pub fn durable_runtime_parity_receipt_digest(
    receipt: &DurableRuntimeParityReceipt,
) -> Result<String, &'static str> {
    canonical_json_sha256(&DurableRuntimeParityReceiptMaterial {
        schema: &receipt.schema,
        evidence_ref_sha256: &receipt.evidence_ref_sha256,
        program_sha256: &receipt.program_sha256,
        verifier_sha256: &receipt.verifier_sha256,
        input_sha256: &receipt.input_sha256,
        teacher_response_sha256: &receipt.teacher_response_sha256,
        actor_response_sha256: &receipt.actor_response_sha256,
        actor_executed: receipt.actor_executed,
        teacher_authority_match: receipt.teacher_authority_match,
        independent_verifier_pass: receipt.independent_verifier_pass,
        exact_teacher_match: receipt.exact_teacher_match,
    })
}
