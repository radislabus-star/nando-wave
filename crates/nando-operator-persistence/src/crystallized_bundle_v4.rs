use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CRYSTALLIZED_OPERATOR_BUNDLE_V4_SCHEMA: &str = "nando.crystallized-operator-bundle.v4";
pub const CRYSTALLIZED_OPERATOR_COMPILER_V1: &str = "nando.canonical-operator-compiler.v1";
pub const CRYSTALLIZED_OPERATOR_VM_ABI_V1: &str = "nando.operator-vm.v1";
pub const CRYSTALLIZED_OPERATOR_BUNDLE_V4_MAX_BYTES: usize = 256 * 1024;
pub const CRYSTALLIZED_OPERATOR_IMAGE_V4_MAX_BYTES: usize = 128 * 1024;

pub type ContentIdV4 = [u8; 32];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrystallizedOperatorManifestV4 {
    schema: String,
    compiler_version: String,
    vm_abi: String,
    law_id: ContentIdV4,
    routing_id: ContentIdV4,
    artifact_id: ContentIdV4,
    verifier_id: ContentIdV4,
    proof_id: ContentIdV4,
    bundle_id: ContentIdV4,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CrystallizedOperatorBundleV4 {
    manifest: CrystallizedOperatorManifestV4,
    #[serde(with = "serde_bytes")]
    routing_image: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    execution_image: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    verifier_image: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    proof_envelope: Box<[u8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrystallizedOperatorBundleV4Error {
    EmptyImage,
    OversizedImage,
    InvalidLawId,
    DigestMismatch,
    Encode,
    Decode,
    NonCanonicalEncoding,
}

impl CrystallizedOperatorBundleV4 {
    pub fn seal(
        law_id: ContentIdV4,
        routing_image: Box<[u8]>,
        execution_image: Box<[u8]>,
        verifier_image: Box<[u8]>,
        proof_envelope: Box<[u8]>,
    ) -> Result<Self, CrystallizedOperatorBundleV4Error> {
        validate_images([
            routing_image.as_ref(),
            execution_image.as_ref(),
            verifier_image.as_ref(),
            proof_envelope.as_ref(),
        ])?;
        if law_id == [0; 32] {
            return Err(CrystallizedOperatorBundleV4Error::InvalidLawId);
        }
        let routing_id = image_id(b"nando.bundle-v4.routing", &routing_image);
        let artifact_id = digest_parts(&[
            b"nando.bundle-v4.artifact",
            CRYSTALLIZED_OPERATOR_COMPILER_V1.as_bytes(),
            CRYSTALLIZED_OPERATOR_VM_ABI_V1.as_bytes(),
            &law_id,
            &execution_image,
        ]);
        let verifier_id = digest_parts(&[
            b"nando.bundle-v4.verifier",
            CRYSTALLIZED_OPERATOR_VM_ABI_V1.as_bytes(),
            &verifier_image,
        ]);
        let proof_id = image_id(b"nando.bundle-v4.proof", &proof_envelope);
        let bundle_id = digest_parts(&[
            b"nando.bundle-v4.bundle",
            &law_id,
            &routing_id,
            &artifact_id,
            &verifier_id,
            &proof_id,
        ]);
        Ok(Self {
            manifest: CrystallizedOperatorManifestV4 {
                schema: CRYSTALLIZED_OPERATOR_BUNDLE_V4_SCHEMA.to_owned(),
                compiler_version: CRYSTALLIZED_OPERATOR_COMPILER_V1.to_owned(),
                vm_abi: CRYSTALLIZED_OPERATOR_VM_ABI_V1.to_owned(),
                law_id,
                routing_id,
                artifact_id,
                verifier_id,
                proof_id,
                bundle_id,
            },
            routing_image,
            execution_image,
            verifier_image,
            proof_envelope,
        })
    }

    pub fn validate(&self) -> Result<(), CrystallizedOperatorBundleV4Error> {
        validate_images([
            self.routing_image.as_ref(),
            self.execution_image.as_ref(),
            self.verifier_image.as_ref(),
            self.proof_envelope.as_ref(),
        ])?;
        if self.manifest.schema != CRYSTALLIZED_OPERATOR_BUNDLE_V4_SCHEMA
            || self.manifest.compiler_version != CRYSTALLIZED_OPERATOR_COMPILER_V1
            || self.manifest.vm_abi != CRYSTALLIZED_OPERATOR_VM_ABI_V1
            || self.manifest.law_id == [0; 32]
        {
            return Err(CrystallizedOperatorBundleV4Error::DigestMismatch);
        }
        let rebuilt = Self::seal(
            self.manifest.law_id,
            self.routing_image.clone(),
            self.execution_image.clone(),
            self.verifier_image.clone(),
            self.proof_envelope.clone(),
        )?;
        if rebuilt.manifest != self.manifest {
            return Err(CrystallizedOperatorBundleV4Error::DigestMismatch);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, CrystallizedOperatorBundleV4Error> {
        self.validate()?;
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| CrystallizedOperatorBundleV4Error::Encode)?;
        if bytes.len() > CRYSTALLIZED_OPERATOR_BUNDLE_V4_MAX_BYTES {
            return Err(CrystallizedOperatorBundleV4Error::Encode);
        }
        Ok(bytes.into_boxed_slice())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CrystallizedOperatorBundleV4Error> {
        if bytes.len() > CRYSTALLIZED_OPERATOR_BUNDLE_V4_MAX_BYTES {
            return Err(CrystallizedOperatorBundleV4Error::Decode);
        }
        let bundle: Self =
            serde_cbor::from_slice(bytes).map_err(|_| CrystallizedOperatorBundleV4Error::Decode)?;
        bundle.validate()?;
        if bundle.canonical_bytes()?.as_ref() != bytes {
            return Err(CrystallizedOperatorBundleV4Error::NonCanonicalEncoding);
        }
        Ok(bundle)
    }

    #[must_use]
    pub const fn manifest(&self) -> &CrystallizedOperatorManifestV4 {
        &self.manifest
    }

    #[must_use]
    pub fn routing_image(&self) -> &[u8] {
        &self.routing_image
    }

    #[must_use]
    pub fn execution_image(&self) -> &[u8] {
        &self.execution_image
    }

    #[must_use]
    pub fn verifier_image(&self) -> &[u8] {
        &self.verifier_image
    }

    #[must_use]
    pub fn proof_envelope(&self) -> &[u8] {
        &self.proof_envelope
    }
}

impl CrystallizedOperatorManifestV4 {
    #[must_use]
    pub const fn law_id(&self) -> &ContentIdV4 {
        &self.law_id
    }

    #[must_use]
    pub const fn artifact_id(&self) -> &ContentIdV4 {
        &self.artifact_id
    }

    #[must_use]
    pub const fn verifier_id(&self) -> &ContentIdV4 {
        &self.verifier_id
    }

    #[must_use]
    pub const fn proof_id(&self) -> &ContentIdV4 {
        &self.proof_id
    }

    #[must_use]
    pub const fn bundle_id(&self) -> &ContentIdV4 {
        &self.bundle_id
    }
}

fn validate_images(images: [&[u8]; 4]) -> Result<(), CrystallizedOperatorBundleV4Error> {
    if images.iter().any(|image| image.is_empty()) {
        return Err(CrystallizedOperatorBundleV4Error::EmptyImage);
    }
    if images
        .iter()
        .any(|image| image.len() > CRYSTALLIZED_OPERATOR_IMAGE_V4_MAX_BYTES)
    {
        return Err(CrystallizedOperatorBundleV4Error::OversizedImage);
    }
    Ok(())
}

fn image_id(domain: &[u8], image: &[u8]) -> ContentIdV4 {
    digest_parts(&[domain, image])
}

fn digest_parts(parts: &[&[u8]]) -> ContentIdV4 {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> CrystallizedOperatorBundleV4 {
        CrystallizedOperatorBundleV4::seal(
            [7; 32],
            b"routing".to_vec().into_boxed_slice(),
            b"execution".to_vec().into_boxed_slice(),
            b"verifier".to_vec().into_boxed_slice(),
            b"proof".to_vec().into_boxed_slice(),
        )
        .expect("bundle")
    }

    #[test]
    fn bundle_is_content_addressed_and_restart_stable() {
        let bundle = bundle();
        let bytes = bundle.canonical_bytes().expect("bytes");
        let restored =
            CrystallizedOperatorBundleV4::from_canonical_bytes(&bytes).expect("restored");
        assert_eq!(restored, bundle);
        assert_eq!(restored.canonical_bytes().expect("bytes"), bytes);
    }

    #[test]
    fn image_tamper_is_rejected_and_authority_is_not_serialized() {
        let bundle = bundle();
        let mut tampered = bundle.clone();
        tampered.execution_image[0] ^= 1;
        assert_eq!(
            tampered.validate(),
            Err(CrystallizedOperatorBundleV4Error::DigestMismatch)
        );
        let encoded = bundle.canonical_bytes().expect("bytes");
        assert!(!encoded.windows(5).any(|window| window == b"lease"));
        assert!(!encoded.windows(9).any(|window| window == b"authority"));
    }
}
