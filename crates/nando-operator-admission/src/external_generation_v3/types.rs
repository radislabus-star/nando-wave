#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalGenerationAdmissionErrorV3 {
    MissingInput,
    InvalidGenerationCheckpoint,
    InvalidGenerationCaptureIndex,
    InvalidProviderCaptureIndex,
    InvalidShadowLedger,
    InvalidPhaseControlReceipt,
    InvalidResourceReceipt,
    GenerationDrift,
    CaptureJoinMismatch,
    ControlTrafficMismatch,
    ControlEvidenceMismatch,
    RuntimeParityMismatch,
    CommitmentDrift,
    UnknownSchema,
    Serialization,
}

pub struct ExternalGenerationAdmissionInputV3<'a> {
    pub generation_checkpoint_bytes: &'a [u8],
    pub generation_capture_index_bytes: &'a [u8],
    pub provider_capture_index_bytes: &'a [u8],
    pub shadow_ledger_bytes: &'a [u8],
    pub phase_control_receipt_bytes: &'a [u8],
    pub resource_receipt_bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalGenerationAdmissionVerdictV3 {
    ShadowReady,
    WatchNoCausalGain,
}
