use nando_operator_kernel::{
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceTypeClassV1,
    canonical_json_sha256,
};
use serde_json::Value;

pub(super) struct ContainerRoleV1 {
    pub type_class: MultiSourceTypeClassV1,
    pub container_class: MultiSourceContainerClassV1,
    pub cardinality_class: MultiSourceCardinalityClassV1,
    pub value_sha256: String,
}

pub(super) fn from_output(output: &Value) -> Result<Option<ContainerRoleV1>, &'static str> {
    let (type_class, container_class, len) = match output {
        Value::Array(values) => (
            MultiSourceTypeClassV1::Array,
            MultiSourceContainerClassV1::Sequence,
            values.len(),
        ),
        Value::Object(values) => (
            MultiSourceTypeClassV1::Object,
            MultiSourceContainerClassV1::Mapping,
            values.len(),
        ),
        _ => return Ok(None),
    };
    let cardinality_class = match len {
        0 => MultiSourceCardinalityClassV1::Zero,
        1 => MultiSourceCardinalityClassV1::One,
        _ => MultiSourceCardinalityClassV1::Many,
    };
    Ok(Some(ContainerRoleV1 {
        type_class,
        container_class,
        cardinality_class,
        value_sha256: canonical_json_sha256(output)?,
    }))
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceTypeClassV1,
    };
    use serde_json::json;

    use super::from_output;

    #[test]
    fn classifies_sequence_and_mapping_without_retaining_raw_values() {
        let sequence = from_output(&json!([{"secret": 1}, {"secret": 2}]))
            .expect("sequence")
            .expect("container");
        assert_eq!(sequence.type_class, MultiSourceTypeClassV1::Array);
        assert_eq!(
            sequence.container_class,
            MultiSourceContainerClassV1::Sequence
        );
        assert_eq!(
            sequence.cardinality_class,
            MultiSourceCardinalityClassV1::Many
        );
        assert!(!sequence.value_sha256.contains("secret"));

        let mapping = from_output(&json!({"items": [1, 2]}))
            .expect("mapping")
            .expect("container");
        assert_eq!(mapping.type_class, MultiSourceTypeClassV1::Object);
        assert_eq!(
            mapping.container_class,
            MultiSourceContainerClassV1::Mapping
        );
    }
}
