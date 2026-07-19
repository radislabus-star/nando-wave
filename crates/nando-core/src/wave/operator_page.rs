use super::TernaryRelationState;

// Canonical hot Rich Operator page: the phase profile, structural roles,
// ternary relation cube, and executable programs total 4032 bytes. Evidence,
// verifier receipts, and learner reservoirs are cold state and never belong
// to this per-operator execution budget.
pub const OPERATOR_PAGE32_BYTES: usize = 4_032;
pub const OPERATOR_PAGE32_HEADER_BYTES: usize = 64;
pub const OPERATOR_PAGE32_PHASE_BYTES: usize = 1_024;
pub const OPERATOR_PAGE32_ROLES_BYTES: usize = 512;
pub const OPERATOR_PAGE32_CUBE_BYTES: usize = 2_048;
pub const OPERATOR_PAGE32_TRANSFORM_BYTES: usize = 128;
pub const OPERATOR_PAGE32_COMPOSITION_BYTES: usize = 128;
pub const OPERATOR_PAGE32_RENDERER_BYTES: usize = 128;
pub const OPERATOR_PAGE32_MAX_ROLES: usize = 32;
pub const OPERATOR_PAGE32_MAX_PLANES: usize = 8;
pub const OPERATOR_PAGE32_MAX_TRANSFORMS: usize = 16;
pub const OPERATOR_PAGE32_MAGIC: [u8; 8] = *b"NWOP3201";
pub const OPERATOR_PAGE32_SCHEMA_VERSION: u16 = 1;

const PHASE_OFFSET: usize = OPERATOR_PAGE32_HEADER_BYTES;
const ROLES_OFFSET: usize = PHASE_OFFSET + OPERATOR_PAGE32_PHASE_BYTES;
const CUBE_OFFSET: usize = ROLES_OFFSET + OPERATOR_PAGE32_ROLES_BYTES;
const TRANSFORM_OFFSET: usize = CUBE_OFFSET + OPERATOR_PAGE32_CUBE_BYTES;
const COMPOSITION_OFFSET: usize = TRANSFORM_OFFSET + OPERATOR_PAGE32_TRANSFORM_BYTES;
const RENDERER_OFFSET: usize = COMPOSITION_OFFSET + OPERATOR_PAGE32_COMPOSITION_BYTES;
const TERNARY_CELLS_PER_PLANE: usize = OPERATOR_PAGE32_MAX_ROLES * OPERATOR_PAGE32_MAX_ROLES;
const TERNARY_BYTES_PER_PLANE: usize = TERNARY_CELLS_PER_PLANE / 4;

const _: () = assert!(RENDERER_OFFSET + OPERATOR_PAGE32_RENDERER_BYTES == OPERATOR_PAGE32_BYTES);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralRole16 {
    pub type_class: u8,
    pub cardinality_class: u8,
    pub temporal_class: u8,
    pub relation_flags: u8,
    pub phase_center: u16,
    pub selector_center: u16,
    pub constraint_mask: u32,
    pub role_signature_hash: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransformOp8 {
    pub opcode: u8,
    pub output: u8,
    pub source_a: u8,
    pub source_b: u8,
    pub parameter: u16,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OperatorPage32Metadata {
    pub generation: u64,
    pub circuit_fingerprint64: u64,
    pub verifier_binding_fingerprint64: u64,
    pub proof_lineage_fingerprint64: u64,
    pub role_signature_fingerprint64: u64,
    pub relation_plane_count: u8,
    pub composition_node_count: u8,
    pub renderer_instruction_count: u8,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperatorPage32Header {
    pub schema_version: u16,
    pub role_count: u8,
    pub relation_plane_count: u8,
    pub transform_count: u8,
    pub composition_node_count: u8,
    pub renderer_instruction_count: u8,
    pub flags: u8,
    pub generation: u64,
    pub circuit_fingerprint64: u64,
    pub verifier_binding_fingerprint64: u64,
    pub proof_lineage_fingerprint64: u64,
    pub role_signature_fingerprint64: u64,
    pub payload_fingerprint64: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TernaryOperatorCube32 {
    bytes: Box<[u8; OPERATOR_PAGE32_CUBE_BYTES]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorPage32 {
    bytes: Box<[u8; OPERATOR_PAGE32_BYTES]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorPage32Error {
    InvalidPageLength,
    InvalidMagic,
    UnsupportedSchema,
    InvalidRoleCount,
    InvalidPlaneCount,
    InvalidTransformCount,
    InvalidCompositionCount,
    InvalidRendererCount,
    InvalidRoleIndex,
    InvalidPlaneIndex,
    ReservedTernaryEncoding,
    NonCanonicalInactivePlane,
    PayloadFingerprintMismatch,
}

impl StructuralRole16 {
    #[must_use]
    pub fn encode(self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0] = self.type_class;
        bytes[1] = self.cardinality_class;
        bytes[2] = self.temporal_class;
        bytes[3] = self.relation_flags;
        bytes[4..6].copy_from_slice(&self.phase_center.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.selector_center.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.constraint_mask.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.role_signature_hash.to_le_bytes());
        bytes
    }

    #[must_use]
    pub fn decode(bytes: [u8; 16]) -> Self {
        Self {
            type_class: bytes[0],
            cardinality_class: bytes[1],
            temporal_class: bytes[2],
            relation_flags: bytes[3],
            phase_center: u16::from_le_bytes([bytes[4], bytes[5]]),
            selector_center: u16::from_le_bytes([bytes[6], bytes[7]]),
            constraint_mask: u32::from_le_bytes(bytes[8..12].try_into().unwrap_or_default()),
            role_signature_hash: u32::from_le_bytes(bytes[12..16].try_into().unwrap_or_default()),
        }
    }
}

impl TransformOp8 {
    #[must_use]
    pub fn encode(self) -> [u8; 8] {
        let mut bytes = [0_u8; 8];
        bytes[0] = self.opcode;
        bytes[1] = self.output;
        bytes[2] = self.source_a;
        bytes[3] = self.source_b;
        bytes[4..6].copy_from_slice(&self.parameter.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.flags.to_le_bytes());
        bytes
    }

    #[must_use]
    pub fn decode(bytes: [u8; 8]) -> Self {
        Self {
            opcode: bytes[0],
            output: bytes[1],
            source_a: bytes[2],
            source_b: bytes[3],
            parameter: u16::from_le_bytes([bytes[4], bytes[5]]),
            flags: u16::from_le_bytes([bytes[6], bytes[7]]),
        }
    }
}

impl Default for TernaryOperatorCube32 {
    fn default() -> Self {
        Self {
            bytes: Box::new([0; OPERATOR_PAGE32_CUBE_BYTES]),
        }
    }
}

impl TernaryOperatorCube32 {
    pub fn set(
        &mut self,
        plane: u8,
        source_role: u8,
        target_role: u8,
        state: TernaryRelationState,
    ) -> Result<(), OperatorPage32Error> {
        let (byte_index, shift) = ternary_position(plane, source_role, target_role)?;
        let code = match state {
            TernaryRelationState::Unresolved => 0_u8,
            TernaryRelationState::Supported => 1_u8,
            TernaryRelationState::Opposed => 2_u8,
        };
        self.bytes[byte_index] &= !(0b11 << shift);
        self.bytes[byte_index] |= code << shift;
        Ok(())
    }

    pub fn get(
        &self,
        plane: u8,
        source_role: u8,
        target_role: u8,
    ) -> Result<TernaryRelationState, OperatorPage32Error> {
        let (byte_index, shift) = ternary_position(plane, source_role, target_role)?;
        match (self.bytes[byte_index] >> shift) & 0b11 {
            0 => Ok(TernaryRelationState::Unresolved),
            1 => Ok(TernaryRelationState::Supported),
            2 => Ok(TernaryRelationState::Opposed),
            _ => Err(OperatorPage32Error::ReservedTernaryEncoding),
        }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; OPERATOR_PAGE32_CUBE_BYTES] {
        &self.bytes
    }

    fn from_slice(bytes: &[u8]) -> Result<Self, OperatorPage32Error> {
        let array = bytes
            .try_into()
            .map_err(|_| OperatorPage32Error::InvalidPageLength)?;
        Ok(Self {
            bytes: Box::new(array),
        })
    }

    fn validate(&self, active_planes: u8) -> Result<(), OperatorPage32Error> {
        if active_planes == 0 || usize::from(active_planes) > OPERATOR_PAGE32_MAX_PLANES {
            return Err(OperatorPage32Error::InvalidPlaneCount);
        }
        let active_bytes = usize::from(active_planes) * TERNARY_BYTES_PER_PLANE;
        for byte in &self.bytes[..active_bytes] {
            for shift in [0, 2, 4, 6] {
                if (byte >> shift) & 0b11 == 0b11 {
                    return Err(OperatorPage32Error::ReservedTernaryEncoding);
                }
            }
        }
        if self.bytes[active_bytes..].iter().any(|byte| *byte != 0) {
            return Err(OperatorPage32Error::NonCanonicalInactivePlane);
        }
        Ok(())
    }
}

impl OperatorPage32 {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        metadata: OperatorPage32Metadata,
        phase_profile: &[u8; OPERATOR_PAGE32_PHASE_BYTES],
        roles: &[StructuralRole16],
        cube: &TernaryOperatorCube32,
        transforms: &[TransformOp8],
        composition: &[u8; OPERATOR_PAGE32_COMPOSITION_BYTES],
        renderer: &[u8; OPERATOR_PAGE32_RENDERER_BYTES],
    ) -> Result<Self, OperatorPage32Error> {
        if roles.is_empty() || roles.len() > OPERATOR_PAGE32_MAX_ROLES {
            return Err(OperatorPage32Error::InvalidRoleCount);
        }
        if metadata.relation_plane_count == 0
            || usize::from(metadata.relation_plane_count) > OPERATOR_PAGE32_MAX_PLANES
        {
            return Err(OperatorPage32Error::InvalidPlaneCount);
        }
        if transforms.len() > OPERATOR_PAGE32_MAX_TRANSFORMS {
            return Err(OperatorPage32Error::InvalidTransformCount);
        }
        cube.validate(metadata.relation_plane_count)?;

        let mut bytes = Box::new([0_u8; OPERATOR_PAGE32_BYTES]);
        bytes[..8].copy_from_slice(&OPERATOR_PAGE32_MAGIC);
        bytes[8..10].copy_from_slice(&OPERATOR_PAGE32_SCHEMA_VERSION.to_le_bytes());
        bytes[10] = roles.len() as u8;
        bytes[11] = metadata.relation_plane_count;
        bytes[12] = transforms.len() as u8;
        bytes[13] = metadata.composition_node_count;
        bytes[14] = metadata.renderer_instruction_count;
        bytes[15] = metadata.flags;
        bytes[16..24].copy_from_slice(&metadata.generation.to_le_bytes());
        bytes[24..32].copy_from_slice(&metadata.circuit_fingerprint64.to_le_bytes());
        bytes[32..40].copy_from_slice(&metadata.verifier_binding_fingerprint64.to_le_bytes());
        bytes[40..48].copy_from_slice(&metadata.proof_lineage_fingerprint64.to_le_bytes());
        bytes[48..56].copy_from_slice(&metadata.role_signature_fingerprint64.to_le_bytes());

        bytes[PHASE_OFFSET..ROLES_OFFSET].copy_from_slice(phase_profile);
        for (index, role) in roles.iter().enumerate() {
            let start = ROLES_OFFSET + index * 16;
            bytes[start..start + 16].copy_from_slice(&role.encode());
        }
        bytes[CUBE_OFFSET..TRANSFORM_OFFSET].copy_from_slice(cube.as_bytes());
        for (index, transform) in transforms.iter().enumerate() {
            let start = TRANSFORM_OFFSET + index * 8;
            bytes[start..start + 8].copy_from_slice(&transform.encode());
        }
        bytes[COMPOSITION_OFFSET..RENDERER_OFFSET].copy_from_slice(composition);
        bytes[RENDERER_OFFSET..].copy_from_slice(renderer);
        let payload_fingerprint = fingerprint64(&bytes[OPERATOR_PAGE32_HEADER_BYTES..]);
        bytes[56..64].copy_from_slice(&payload_fingerprint.to_le_bytes());

        Ok(Self { bytes })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, OperatorPage32Error> {
        let array: [u8; OPERATOR_PAGE32_BYTES] = bytes
            .try_into()
            .map_err(|_| OperatorPage32Error::InvalidPageLength)?;
        let page = Self {
            bytes: Box::new(array),
        };
        page.validate()?;
        Ok(page)
    }

    pub fn validate(&self) -> Result<(), OperatorPage32Error> {
        let header = self.header()?;
        if header.composition_node_count as usize > OPERATOR_PAGE32_COMPOSITION_BYTES {
            return Err(OperatorPage32Error::InvalidCompositionCount);
        }
        if header.renderer_instruction_count as usize > OPERATOR_PAGE32_RENDERER_BYTES {
            return Err(OperatorPage32Error::InvalidRendererCount);
        }
        let actual = fingerprint64(&self.bytes[OPERATOR_PAGE32_HEADER_BYTES..]);
        if actual != header.payload_fingerprint64 {
            return Err(OperatorPage32Error::PayloadFingerprintMismatch);
        }
        self.cube()?.validate(header.relation_plane_count)
    }

    pub fn header(&self) -> Result<OperatorPage32Header, OperatorPage32Error> {
        if self.bytes[..8] != OPERATOR_PAGE32_MAGIC {
            return Err(OperatorPage32Error::InvalidMagic);
        }
        let schema_version = read_u16(&self.bytes[8..10]);
        if schema_version != OPERATOR_PAGE32_SCHEMA_VERSION {
            return Err(OperatorPage32Error::UnsupportedSchema);
        }
        let role_count = self.bytes[10];
        let relation_plane_count = self.bytes[11];
        let transform_count = self.bytes[12];
        if role_count == 0 || usize::from(role_count) > OPERATOR_PAGE32_MAX_ROLES {
            return Err(OperatorPage32Error::InvalidRoleCount);
        }
        if relation_plane_count == 0
            || usize::from(relation_plane_count) > OPERATOR_PAGE32_MAX_PLANES
        {
            return Err(OperatorPage32Error::InvalidPlaneCount);
        }
        if usize::from(transform_count) > OPERATOR_PAGE32_MAX_TRANSFORMS {
            return Err(OperatorPage32Error::InvalidTransformCount);
        }
        Ok(OperatorPage32Header {
            schema_version,
            role_count,
            relation_plane_count,
            transform_count,
            composition_node_count: self.bytes[13],
            renderer_instruction_count: self.bytes[14],
            flags: self.bytes[15],
            generation: read_u64(&self.bytes[16..24]),
            circuit_fingerprint64: read_u64(&self.bytes[24..32]),
            verifier_binding_fingerprint64: read_u64(&self.bytes[32..40]),
            proof_lineage_fingerprint64: read_u64(&self.bytes[40..48]),
            role_signature_fingerprint64: read_u64(&self.bytes[48..56]),
            payload_fingerprint64: read_u64(&self.bytes[56..64]),
        })
    }

    pub fn cube(&self) -> Result<TernaryOperatorCube32, OperatorPage32Error> {
        TernaryOperatorCube32::from_slice(&self.bytes[CUBE_OFFSET..TRANSFORM_OFFSET])
    }

    #[must_use]
    pub fn role(&self, index: usize) -> Option<StructuralRole16> {
        let count = usize::from(self.bytes[10]);
        if index >= count {
            return None;
        }
        let start = ROLES_OFFSET + index * 16;
        let bytes = self.bytes[start..start + 16].try_into().ok()?;
        Some(StructuralRole16::decode(bytes))
    }

    #[must_use]
    pub fn transform(&self, index: usize) -> Option<TransformOp8> {
        let count = usize::from(self.bytes[12]);
        if index >= count {
            return None;
        }
        let start = TRANSFORM_OFFSET + index * 8;
        let bytes = self.bytes[start..start + 8].try_into().ok()?;
        Some(TransformOp8::decode(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; OPERATOR_PAGE32_BYTES] {
        &self.bytes
    }
}

fn ternary_position(
    plane: u8,
    source_role: u8,
    target_role: u8,
) -> Result<(usize, u8), OperatorPage32Error> {
    if usize::from(plane) >= OPERATOR_PAGE32_MAX_PLANES {
        return Err(OperatorPage32Error::InvalidPlaneIndex);
    }
    if usize::from(source_role) >= OPERATOR_PAGE32_MAX_ROLES
        || usize::from(target_role) >= OPERATOR_PAGE32_MAX_ROLES
    {
        return Err(OperatorPage32Error::InvalidRoleIndex);
    }
    let linear = usize::from(plane) * TERNARY_CELLS_PER_PLANE
        + usize::from(source_role) * OPERATOR_PAGE32_MAX_ROLES
        + usize::from(target_role);
    Ok((linear / 4, ((linear % 4) * 2) as u8))
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().unwrap_or_default())
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap_or_default())
}

fn fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(id: u8) -> StructuralRole16 {
        StructuralRole16 {
            type_class: id,
            cardinality_class: 1,
            temporal_class: 2,
            relation_flags: 3,
            phase_center: 10 + u16::from(id),
            selector_center: 20 + u16::from(id),
            constraint_mask: 0x0102_0304,
            role_signature_hash: 0x1122_3300 + u32::from(id),
        }
    }

    fn transform() -> TransformOp8 {
        TransformOp8 {
            opcode: 4,
            output: 2,
            source_a: 0,
            source_b: 1,
            parameter: 17,
            flags: 9,
        }
    }

    #[test]
    fn operator_page32_is_exactly_one_4032_byte_hot_page_and_roundtrips() {
        assert_eq!(std::mem::size_of::<StructuralRole16>(), 16);
        assert_eq!(std::mem::size_of::<TransformOp8>(), 8);

        let mut cube = TernaryOperatorCube32::default();
        cube.set(0, 0, 1, TernaryRelationState::Supported)
            .expect("supported relation");
        cube.set(1, 1, 2, TernaryRelationState::Opposed)
            .expect("opposed relation");
        let roles = [role(0), role(1), role(2)];
        let transforms = [transform()];
        let page = OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 8,
                circuit_fingerprint64: 11,
                verifier_binding_fingerprint64: 12,
                proof_lineage_fingerprint64: 13,
                role_signature_fingerprint64: 14,
                relation_plane_count: 2,
                composition_node_count: 1,
                renderer_instruction_count: 1,
                flags: 0,
            },
            &[7; OPERATOR_PAGE32_PHASE_BYTES],
            &roles,
            &cube,
            &transforms,
            &[3; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &[4; OPERATOR_PAGE32_RENDERER_BYTES],
        )
        .expect("valid operator page");

        assert_eq!(page.as_bytes().len(), OPERATOR_PAGE32_BYTES);
        let decoded = OperatorPage32::from_bytes(page.as_bytes()).expect("canonical page");
        assert_eq!(decoded.as_bytes(), page.as_bytes());
        assert_eq!(decoded.role(2), Some(role(2)));
        assert_eq!(decoded.transform(0), Some(transform()));
        assert_eq!(
            decoded.cube().expect("cube").get(0, 0, 1),
            Ok(TernaryRelationState::Supported)
        );
        assert_eq!(decoded.header().expect("header").generation, 8);
    }

    #[test]
    fn reserved_ternary_encoding_is_rejected() {
        let mut cube = TernaryOperatorCube32::default();
        cube.bytes[0] = 0b11;
        assert_eq!(
            cube.validate(1),
            Err(OperatorPage32Error::ReservedTernaryEncoding)
        );
    }

    #[test]
    fn inactive_planes_must_be_canonical_zeroes() {
        let mut cube = TernaryOperatorCube32::default();
        cube.set(1, 0, 1, TernaryRelationState::Supported)
            .expect("physical plane exists");
        assert_eq!(
            cube.validate(1),
            Err(OperatorPage32Error::NonCanonicalInactivePlane)
        );
    }
}
