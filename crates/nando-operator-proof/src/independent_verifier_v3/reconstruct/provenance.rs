use std::collections::BTreeSet;

use nando_operator_kernel::canonical_json_sha256;

use super::super::IndependentVerifierErrorV3;
use super::super::capability::IndependentCapabilityV3;

pub(super) fn duplicate_capability_paths_v3(
    capabilities: &[IndependentCapabilityV3],
) -> Result<bool, IndependentVerifierErrorV3> {
    let mut signatures = BTreeSet::new();
    for capability in capabilities {
        let signature = canonical_json_sha256(&(
            capability.kind,
            capability.physical_symbol.as_str(),
            capability
                .arguments
                .iter()
                .map(|argument| {
                    (
                        argument.ordinal,
                        argument.physical_name.as_str(),
                        argument.value_type,
                        argument.required,
                    )
                })
                .collect::<Vec<_>>(),
        ))
        .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
        if !signatures.insert(signature) {
            return Ok(true);
        }
    }
    Ok(false)
}
