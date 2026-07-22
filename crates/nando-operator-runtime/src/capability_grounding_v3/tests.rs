use nando_operator_kernel::BoundProtocolValueV3;
use serde_json::{Value, json};

use super::*;
use crate::mode_to_role_v3::tests::fixtures::{
    artifact, mentioned_string_selector, request_payload, runtime_context,
};
use crate::{bind_structural_modes_v3, compile_structural_dispatch_index_v3};

fn ground<'a>(
    artifacts: &[nando_operator_kernel::ExecutableProtocolModeArtifactV3],
    request: &crate::CanonicalRuntimeRequestV3<'a>,
) -> BoundProtocolActionOutcomeV3 {
    let index = compile_structural_dispatch_index_v3(artifacts).expect("dispatch index");
    let dispatch = index.dispatch(request);
    let binding = bind_structural_modes_v3(&index, request, &dispatch)
        .into_complete()
        .expect("complete structural binding");
    ground_protocol_actions_v3(&index, request, &binding)
}

fn rename_capability(payload: &mut Value, index: usize, name: &str) {
    payload["tools"][index]["name"] = Value::String(name.to_owned());
}

#[test]
fn renamed_capability_preserves_semantic_action_and_rebinds_physical_action() {
    let artifacts = [artifact(101, mentioned_string_selector())];
    let first_payload = request_payload(json!({"handle": "CellA17"}));
    let first_request = runtime_context("continue CellA17", &first_payload);
    let first = ground(&artifacts, &first_request)
        .into_complete()
        .expect("first action");

    let mut renamed_payload = first_payload.clone();
    rename_capability(&mut renamed_payload, 0, "continue_session");
    let renamed_request = runtime_context("continue CellA17", &renamed_payload);
    let renamed = ground(&artifacts, &renamed_request)
        .into_complete()
        .expect("renamed action");

    assert_eq!(
        first.action().semantic_action_sha256(),
        renamed.action().semantic_action_sha256()
    );
    assert_ne!(
        first.action().physical_action_sha256(),
        renamed.action().physical_action_sha256()
    );
    assert_eq!(renamed.action().physical_symbol(), "continue_session");
    assert_eq!(renamed.action().arguments().len(), 1);
    assert_eq!(
        renamed.action().arguments()[0].value(),
        &BoundProtocolValueV3::String("CellA17".to_owned())
    );
    assert!(!renamed.execution_authority());
}

#[test]
fn multiple_structural_modes_that_derive_one_action_collapse_deterministically() {
    let artifacts = [
        artifact(102, mentioned_string_selector()),
        artifact(103, mentioned_string_selector()),
    ];
    let payload = request_payload(json!({"handle": "CellA17"}));
    let request = runtime_context("continue CellA17", &payload);
    let outcome = ground(&artifacts, &request);

    assert_eq!(outcome.verdict(), CapabilityGroundingVerdictV3::Complete);
    assert!(outcome.structural_mappings() >= 2);
    assert_eq!(outcome.semantic_action_classes(), 1);
    assert_eq!(outcome.physical_action_classes(), 1);
    assert_eq!(outcome.actions().len(), 1);
    assert!(outcome.attempts().iter().all(|attempt| {
        attempt.verdict() == ActionDerivationVerdictV3::Bound
            && attempt.semantic_action_sha256().is_some()
            && attempt.physical_action_sha256().is_some()
    }));
}

#[test]
fn different_values_in_complete_structural_version_space_abstain() {
    let artifacts = [artifact(104, mentioned_string_selector())];
    let payload = request_payload(json!({"first": "CellA17", "second": "CellB18"}));
    let request = runtime_context("continue CellA17 then CellB18", &payload);
    let outcome = ground(&artifacts, &request);

    assert_eq!(
        outcome.verdict(),
        CapabilityGroundingVerdictV3::AbstainAmbiguousAction
    );
    assert!(outcome.semantic_action_classes() >= 2);
    assert!(outcome.into_complete().is_none());
}

#[test]
fn duplicate_identical_declarations_collapse_but_distinct_symbols_abstain() {
    let artifacts = [artifact(105, mentioned_string_selector())];
    let mut duplicate_payload = request_payload(json!({"handle": "CellA17"}));
    let declaration = duplicate_payload["tools"][0].clone();
    duplicate_payload["tools"]
        .as_array_mut()
        .expect("tools")
        .push(declaration);
    let duplicate_request = runtime_context("continue CellA17", &duplicate_payload);
    let duplicate = ground(&artifacts, &duplicate_request);
    assert_eq!(duplicate.verdict(), CapabilityGroundingVerdictV3::Complete);
    assert_eq!(duplicate.physical_action_classes(), 1);

    let mut ambiguous_payload = duplicate_payload;
    rename_capability(&mut ambiguous_payload, 1, "other_continuation");
    let ambiguous_request = runtime_context("continue CellA17", &ambiguous_payload);
    let ambiguous = ground(&artifacts, &ambiguous_request);
    assert_eq!(
        ambiguous.verdict(),
        CapabilityGroundingVerdictV3::AbstainAmbiguousCapability
    );
    assert_eq!(ambiguous.semantic_action_classes(), 1);
    assert_eq!(ambiguous.physical_action_classes(), 2);
}

#[test]
fn missing_or_incompatible_current_capability_never_reuses_stale_binding() {
    let artifacts = [artifact(106, mentioned_string_selector())];
    let mut missing_payload = request_payload(json!({"handle": "CellA17"}));
    missing_payload["tools"] = json!([]);
    let missing_request = runtime_context("continue CellA17", &missing_payload);
    let missing = ground(&artifacts, &missing_request);
    assert_eq!(
        missing.verdict(),
        CapabilityGroundingVerdictV3::AbstainNoStructuralMapping
    );
    assert!(missing.actions().is_empty());

    let mut incompatible_payload = request_payload(json!({"handle": "CellA17"}));
    incompatible_payload["tools"][0]["parameters"]["properties"]["handle"]["type"] =
        json!("integer");
    let incompatible_request = runtime_context("continue CellA17", &incompatible_payload);
    let incompatible = ground(&artifacts, &incompatible_request);
    assert_eq!(
        incompatible.verdict(),
        CapabilityGroundingVerdictV3::AbstainNoStructuralMapping
    );
    assert!(incompatible.actions().is_empty());
}

#[test]
fn binding_report_is_request_root_owned_and_cannot_cross_surfaces() {
    let artifacts = [artifact(107, mentioned_string_selector())];
    let index = compile_structural_dispatch_index_v3(&artifacts).expect("dispatch index");
    let first_payload = request_payload(json!({"handle": "CellA17"}));
    let first_request = runtime_context("continue CellA17", &first_payload);
    let binding = bind_structural_modes_v3(&index, &first_request, &index.dispatch(&first_request))
        .into_complete()
        .expect("first binding");

    let second_payload = request_payload(json!({"handle": "CellB18"}));
    let second_request = runtime_context("continue CellB18", &second_payload);
    let outcome = ground_protocol_actions_v3(&index, &second_request, &binding);
    assert_eq!(
        outcome.verdict(),
        CapabilityGroundingVerdictV3::RejectIndexMismatch
    );
    assert!(outcome.actions().is_empty());
}
