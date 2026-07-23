use nando_operator_kernel::{Sha256CommitmentV3, sha256_bytes, valid_nonzero_sha256};
use nando_operator_proof::independent_verifier_v3::F6_MAX_RAW_REQUEST_BYTES_V3;
use serde::{Deserialize, Serialize};

use crate::{PROVIDER_REQUEST_CAPTURE_RECEIPT_MAX_BYTES_V3, ProviderRequestCaptureReceiptV3};

pub const LEARNING_EVIDENCE_ENVELOPE_SCHEMA_V1: &str = "nando.learning-evidence-envelope.v1";
pub const LEARNING_REQUEST_STRUCTURE_SCHEMA_V1: &str = "nando.learning-request-structure.v1";
pub const LEARNING_EVIDENCE_ENVELOPE_MAX_BYTES_V1: usize =
    F6_MAX_RAW_REQUEST_BYTES_V3 + PROVIDER_REQUEST_CAPTURE_RECEIPT_MAX_BYTES_V3 + 16 * 1024;
pub const LEARNING_REQUEST_MAX_SESSION_IDENTITIES_V1: usize = 4;
pub const LEARNING_REQUEST_MAX_PHASE_ATOMS_V1: usize = 256;
pub const LEARNING_REQUEST_MAX_CONTEXT_ATOMS_V1: usize = 256;
pub const LEARNING_REQUEST_MAX_CAPABILITY_ATOMS_V1: usize = 64;

pub struct LearningEvidenceEnvelopeV1 {
    capture_receipt: ProviderRequestCaptureReceiptV3,
    structure: LearningRequestStructureV1,
    raw_provider_payload: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningRequestStructureV1 {
    client_intent_id_sha256: String,
    session_identity_sha256s: Vec<String>,
    request_phase_atom_ids: Vec<u64>,
    pre_action_context_atom_ids: Vec<u64>,
    capability_atom_ids: Vec<u64>,
    provider_bound_turn_identity: bool,
    estimated_input_tokens: u64,
    provider_payload_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningRequestStructureInputV1 {
    pub client_intent_id_sha256: String,
    pub session_identity_sha256s: Vec<String>,
    pub request_phase_atom_ids: Vec<u64>,
    pub pre_action_context_atom_ids: Vec<u64>,
    pub capability_atom_ids: Vec<u64>,
    pub provider_bound_turn_identity: bool,
    pub estimated_input_tokens: u64,
    pub provider_payload_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningEvidenceEnvelopeErrorV1 {
    InvalidReceipt,
    InvalidStructure,
    InvalidPayload,
    DigestMismatch,
    BudgetExhausted,
    Serialization,
}

#[derive(Deserialize, Serialize)]
struct LearningEvidenceEnvelopeWireV1(
    String,
    #[serde(with = "serde_bytes")] Vec<u8>,
    #[serde(with = "serde_bytes")] Vec<u8>,
    Option<serde_bytes::ByteBuf>,
);

#[derive(Deserialize, Serialize)]
struct LearningRequestStructureWireV1(
    String,
    String,
    Vec<String>,
    Vec<u64>,
    Vec<u64>,
    Vec<u64>,
    bool,
    u64,
    u64,
);

impl LearningRequestStructureV1 {
    pub fn new(
        mut input: LearningRequestStructureInputV1,
    ) -> Result<Self, LearningEvidenceEnvelopeErrorV1> {
        input.session_identity_sha256s.sort();
        input.session_identity_sha256s.dedup();
        input.request_phase_atom_ids.sort_unstable();
        input.request_phase_atom_ids.dedup();
        input.pre_action_context_atom_ids.sort_unstable();
        input.pre_action_context_atom_ids.dedup();
        input.capability_atom_ids.sort_unstable();
        input.capability_atom_ids.dedup();
        let structure = Self {
            client_intent_id_sha256: input.client_intent_id_sha256,
            session_identity_sha256s: input.session_identity_sha256s,
            request_phase_atom_ids: input.request_phase_atom_ids,
            pre_action_context_atom_ids: input.pre_action_context_atom_ids,
            capability_atom_ids: input.capability_atom_ids,
            provider_bound_turn_identity: input.provider_bound_turn_identity,
            estimated_input_tokens: input.estimated_input_tokens,
            provider_payload_bytes: input.provider_payload_bytes,
        };
        structure.validate()?;
        Ok(structure)
    }

    fn validate(&self) -> Result<(), LearningEvidenceEnvelopeErrorV1> {
        if !valid_nonzero_sha256(&self.client_intent_id_sha256)
            || self.provider_payload_bytes == 0
            || self.session_identity_sha256s.len() > LEARNING_REQUEST_MAX_SESSION_IDENTITIES_V1
            || self.request_phase_atom_ids.len() > LEARNING_REQUEST_MAX_PHASE_ATOMS_V1
            || self.pre_action_context_atom_ids.len() > LEARNING_REQUEST_MAX_CONTEXT_ATOMS_V1
            || self.capability_atom_ids.len() > LEARNING_REQUEST_MAX_CAPABILITY_ATOMS_V1
            || self
                .session_identity_sha256s
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !strictly_ordered(&self.session_identity_sha256s)
            || !strictly_ordered(&self.request_phase_atom_ids)
            || !strictly_ordered(&self.pre_action_context_atom_ids)
            || !strictly_ordered(&self.capability_atom_ids)
        {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidStructure);
        }
        Ok(())
    }

    pub fn canonical_cbor(&self) -> Result<Vec<u8>, LearningEvidenceEnvelopeErrorV1> {
        self.validate()?;
        serde_cbor::to_vec(&LearningRequestStructureWireV1(
            LEARNING_REQUEST_STRUCTURE_SCHEMA_V1.to_owned(),
            self.client_intent_id_sha256.clone(),
            self.session_identity_sha256s.clone(),
            self.request_phase_atom_ids.clone(),
            self.pre_action_context_atom_ids.clone(),
            self.capability_atom_ids.clone(),
            self.provider_bound_turn_identity,
            self.estimated_input_tokens,
            self.provider_payload_bytes,
        ))
        .map_err(|_| LearningEvidenceEnvelopeErrorV1::Serialization)
    }

    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, LearningEvidenceEnvelopeErrorV1> {
        let wire: LearningRequestStructureWireV1 = serde_cbor::from_slice(bytes)
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::Serialization)?;
        if wire.0 != LEARNING_REQUEST_STRUCTURE_SCHEMA_V1 {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidStructure);
        }
        let structure = Self {
            client_intent_id_sha256: wire.1,
            session_identity_sha256s: wire.2,
            request_phase_atom_ids: wire.3,
            pre_action_context_atom_ids: wire.4,
            capability_atom_ids: wire.5,
            provider_bound_turn_identity: wire.6,
            estimated_input_tokens: wire.7,
            provider_payload_bytes: wire.8,
        };
        structure.validate()?;
        if structure.canonical_cbor()?.as_slice() != bytes {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidStructure);
        }
        Ok(structure)
    }

    #[must_use]
    pub fn client_intent_id_sha256(&self) -> &str {
        &self.client_intent_id_sha256
    }

    #[must_use]
    pub fn session_identity_sha256s(&self) -> &[String] {
        &self.session_identity_sha256s
    }

    #[must_use]
    pub fn request_phase_atom_ids(&self) -> &[u64] {
        &self.request_phase_atom_ids
    }

    #[must_use]
    pub fn pre_action_context_atom_ids(&self) -> &[u64] {
        &self.pre_action_context_atom_ids
    }

    #[must_use]
    pub fn capability_atom_ids(&self) -> &[u64] {
        &self.capability_atom_ids
    }

    #[must_use]
    pub const fn provider_bound_turn_identity(&self) -> bool {
        self.provider_bound_turn_identity
    }

    #[must_use]
    pub const fn estimated_input_tokens(&self) -> u64 {
        self.estimated_input_tokens
    }

    #[must_use]
    pub const fn provider_payload_bytes(&self) -> u64 {
        self.provider_payload_bytes
    }
}

impl LearningEvidenceEnvelopeV1 {
    pub fn new(
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure: LearningRequestStructureV1,
        provider_payload: &[u8],
    ) -> Result<Self, LearningEvidenceEnvelopeErrorV1> {
        if provider_payload.is_empty() {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidPayload);
        }
        if u64::try_from(provider_payload.len()).unwrap_or(u64::MAX)
            != structure.provider_payload_bytes()
        {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidStructure);
        }
        if Sha256CommitmentV3::digest_bytes(provider_payload)
            != capture_receipt.request_root_sha256()
        {
            return Err(LearningEvidenceEnvelopeErrorV1::DigestMismatch);
        }
        capture_receipt
            .canonical_bytes()
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::InvalidReceipt)?;
        structure.validate()?;
        let raw_provider_payload = (provider_payload.len() <= F6_MAX_RAW_REQUEST_BYTES_V3)
            .then(|| provider_payload.to_vec());
        Ok(Self {
            capture_receipt,
            structure,
            raw_provider_payload,
        })
    }

    pub fn canonical_cbor(&self) -> Result<Vec<u8>, LearningEvidenceEnvelopeErrorV1> {
        let receipt = self
            .capture_receipt
            .canonical_bytes()
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::InvalidReceipt)?;
        let structure = self.structure.canonical_cbor()?;
        let wire = LearningEvidenceEnvelopeWireV1(
            LEARNING_EVIDENCE_ENVELOPE_SCHEMA_V1.to_owned(),
            receipt.into_vec(),
            structure,
            self.raw_provider_payload.clone().map(Into::into),
        );
        let bytes = serde_cbor::to_vec(&wire)
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::Serialization)?;
        if bytes.len() > LEARNING_EVIDENCE_ENVELOPE_MAX_BYTES_V1 {
            return Err(LearningEvidenceEnvelopeErrorV1::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, LearningEvidenceEnvelopeErrorV1> {
        if bytes.is_empty() || bytes.len() > LEARNING_EVIDENCE_ENVELOPE_MAX_BYTES_V1 {
            return Err(LearningEvidenceEnvelopeErrorV1::BudgetExhausted);
        }
        let wire: LearningEvidenceEnvelopeWireV1 = serde_cbor::from_slice(bytes)
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::Serialization)?;
        if wire.0 != LEARNING_EVIDENCE_ENVELOPE_SCHEMA_V1 {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidPayload);
        }
        let capture_receipt = ProviderRequestCaptureReceiptV3::from_canonical_bytes(&wire.1)
            .map_err(|_| LearningEvidenceEnvelopeErrorV1::InvalidReceipt)?;
        let structure = LearningRequestStructureV1::from_canonical_cbor(&wire.2)?;
        let raw_provider_payload = wire.3.map(serde_bytes::ByteBuf::into_vec);
        if let Some(payload) = raw_provider_payload.as_deref() {
            if payload.is_empty() || payload.len() > F6_MAX_RAW_REQUEST_BYTES_V3 {
                return Err(LearningEvidenceEnvelopeErrorV1::InvalidPayload);
            }
            if sha256_bytes(payload) != capture_receipt.request_root_sha256().to_hex() {
                return Err(LearningEvidenceEnvelopeErrorV1::DigestMismatch);
            }
        } else if structure.provider_payload_bytes()
            <= u64::try_from(F6_MAX_RAW_REQUEST_BYTES_V3).unwrap_or(u64::MAX)
        {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidPayload);
        }
        let envelope = Self {
            capture_receipt,
            structure,
            raw_provider_payload,
        };
        if envelope.canonical_cbor()?.as_slice() != bytes {
            return Err(LearningEvidenceEnvelopeErrorV1::InvalidPayload);
        }
        Ok(envelope)
    }

    #[must_use]
    pub const fn capture_receipt(&self) -> &ProviderRequestCaptureReceiptV3 {
        &self.capture_receipt
    }

    #[must_use]
    pub const fn structure(&self) -> &LearningRequestStructureV1 {
        &self.structure
    }

    #[must_use]
    pub fn raw_provider_payload(&self) -> Option<&[u8]> {
        self.raw_provider_payload.as_deref()
    }

    #[must_use]
    pub const fn has_raw_provider_payload(&self) -> bool {
        self.raw_provider_payload.is_some()
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        ProviderRequestCaptureReceiptV3,
        LearningRequestStructureV1,
        Option<Vec<u8>>,
    ) {
        (
            self.capture_receipt,
            self.structure,
            self.raw_provider_payload,
        )
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn strictly_ordered<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};

    use super::*;
    use crate::{ProviderRequestCaptureInputV3, seal_provider_request_capture_v3};

    fn envelope(payload: &[u8]) -> LearningEvidenceEnvelopeV1 {
        let request_root_sha256 = Sha256CommitmentV3::digest_bytes(payload);
        let receipt = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: 9,
            capture_epoch_root: Sha256CommitmentV3::digest_bytes(b"epoch"),
            lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"lineage"),
            request_root_sha256,
            projection: RuntimeProjectionV3::Responses,
            streaming: true,
            observed_at_unix_ms: 1_750_000_000_000,
        })
        .expect("capture receipt");
        let structure = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
            client_intent_id_sha256: sha256_bytes(b"intent"),
            session_identity_sha256s: vec![sha256_bytes(b"session")],
            request_phase_atom_ids: vec![3, 1, 3],
            pre_action_context_atom_ids: vec![9, 7],
            capability_atom_ids: vec![13],
            provider_bound_turn_identity: true,
            estimated_input_tokens: 17,
            provider_payload_bytes: u64::try_from(payload.len()).unwrap_or(u64::MAX),
        })
        .expect("structure");
        LearningEvidenceEnvelopeV1::new(receipt, structure, payload).expect("envelope")
    }

    #[test]
    fn envelope_round_trip_preserves_structure_and_bounded_raw_payload() {
        let envelope = envelope(br#"{"input":"continue"}"#);
        let bytes = envelope.canonical_cbor().expect("canonical envelope");
        let restored =
            LearningEvidenceEnvelopeV1::from_canonical_cbor(&bytes).expect("restore envelope");
        assert_eq!(
            restored.capture_receipt().receipt_sha256(),
            envelope.capture_receipt().receipt_sha256()
        );
        assert_eq!(restored.structure(), envelope.structure());
        assert_eq!(
            restored.raw_provider_payload(),
            envelope.raw_provider_payload()
        );
        assert!(!restored.execution_authority());
    }

    #[test]
    fn oversized_raw_payload_is_omitted_but_structure_still_crosses() {
        let payload = vec![b'x'; F6_MAX_RAW_REQUEST_BYTES_V3 + 1];
        let envelope = envelope(&payload);
        assert!(!envelope.has_raw_provider_payload());
        let restored = LearningEvidenceEnvelopeV1::from_canonical_cbor(
            &envelope.canonical_cbor().expect("canonical envelope"),
        )
        .expect("restore envelope");
        assert_eq!(
            restored.structure().provider_payload_bytes(),
            u64::try_from(payload.len()).unwrap_or(u64::MAX)
        );
        assert!(restored.raw_provider_payload().is_none());
    }

    #[test]
    fn payload_digest_mismatch_is_rejected() {
        let envelope = envelope(br#"{"input":"continue"}"#);
        let receipt = envelope.capture_receipt().clone();
        let mut different = br#"{"input":"continue"}"#.to_vec();
        different[10] ^= 1;
        assert_eq!(
            LearningEvidenceEnvelopeV1::new(receipt, envelope.structure().clone(), &different)
                .err(),
            Some(LearningEvidenceEnvelopeErrorV1::DigestMismatch)
        );
    }
}
