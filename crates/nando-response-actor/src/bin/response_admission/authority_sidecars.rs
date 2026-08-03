use std::path::Path;

use nando_operator_kernel::canonical_json_sha256;
use serde::{Deserialize, Serialize};

const GENERATION_SCHEMA_V2: &str = "nando.response-authority-sidecar-generation.v2";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthoritySidecarGenerationV2 {
    schema: &'static str,
    generation_root_sha256: String,
    registry_root_sha256: String,
    controller_root_sha256: String,
    candidate_root_sha256: String,
    marker_root_sha256: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn persist(
    registry_path: &Path,
    controller_admission_path: &Path,
    authority_candidate_path: &Path,
    marker_path: &Path,
    registry: &impl Serialize,
    controller_admission: &impl Serialize,
    authority_candidate: &impl Serialize,
    marker: &impl Serialize,
) -> Result<(), String> {
    let (parent, generation) = persist_generation(
        registry_path,
        controller_admission_path,
        authority_candidate_path,
        marker_path,
        registry,
        controller_admission,
        authority_candidate,
        marker,
    )?;
    super::write_json_atomic(registry_path, registry, "response-registry-legacy")?;
    super::write_json_atomic(
        controller_admission_path,
        controller_admission,
        "response-controller-admission",
    )?;
    super::write_json_atomic(marker_path, marker, "response-admission-marker")?;
    super::write_json_atomic(
        authority_candidate_path,
        authority_candidate,
        "response-authority-candidate",
    )?;
    super::write_json_atomic(
        &parent.join("response-authority-sidecar-current-v2.json"),
        &generation,
        "response-authority-generation-current",
    )
}

#[allow(clippy::too_many_arguments)]
fn persist_generation(
    registry_path: &Path,
    controller_path: &Path,
    candidate_path: &Path,
    marker_path: &Path,
    registry: &impl Serialize,
    controller: &impl Serialize,
    candidate: &impl Serialize,
    marker: &impl Serialize,
) -> Result<(std::path::PathBuf, AuthoritySidecarGenerationV2), String> {
    let parent = controller_path
        .parent()
        .ok_or_else(|| "response_authority_generation_parent_missing".to_owned())?;
    if registry_path.parent() != Some(parent)
        || candidate_path.parent() != Some(parent)
        || marker_path.parent() != Some(parent)
    {
        return Err("response_authority_generation_parent_mismatch".to_owned());
    }
    let registry_root_sha256 = canonical_json_sha256(registry)
        .map_err(|error| format!("response_authority_registry_root:{error}"))?;
    let controller_root_sha256 = canonical_json_sha256(controller)
        .map_err(|error| format!("response_authority_controller_root:{error}"))?;
    let candidate_root_sha256 = canonical_json_sha256(candidate)
        .map_err(|error| format!("response_authority_candidate_root:{error}"))?;
    let marker_root_sha256 = canonical_json_sha256(marker)
        .map_err(|error| format!("response_authority_marker_root:{error}"))?;
    let generation_root_sha256 = canonical_json_sha256(&(
        GENERATION_SCHEMA_V2,
        &registry_root_sha256,
        &controller_root_sha256,
        &candidate_root_sha256,
        &marker_root_sha256,
    ))
    .map_err(|error| format!("response_authority_generation_root:{error}"))?;
    let generation = AuthoritySidecarGenerationV2 {
        schema: GENERATION_SCHEMA_V2,
        generation_root_sha256: generation_root_sha256.clone(),
        registry_root_sha256,
        controller_root_sha256,
        candidate_root_sha256,
        marker_root_sha256,
    };
    let directory = parent
        .join("response-authority-sidecar-generations-v2")
        .join(&generation_root_sha256);
    super::write_json_atomic(
        &directory.join("registry.json"),
        registry,
        "response-authority-generation-registry",
    )?;
    super::write_json_atomic(
        &directory.join("controller.json"),
        controller,
        "response-authority-generation-controller",
    )?;
    super::write_json_atomic(
        &directory.join("candidate.json"),
        candidate,
        "response-authority-generation-candidate",
    )?;
    super::write_json_atomic(
        &directory.join("marker.json"),
        marker,
        "response-authority-generation-marker",
    )?;
    super::write_json_atomic(
        &directory.join("manifest.json"),
        &generation,
        "response-authority-generation-manifest",
    )?;
    Ok((parent.to_path_buf(), generation))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::persist;

    #[test]
    fn immutable_registry_refresh_replaces_all_authority_sidecars() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-authority-sidecars-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root");
        let controller = root.join("controller.json");
        let candidate = root.join("candidate.json");
        let marker = root.join("marker.json");
        for path in [&controller, &candidate, &marker] {
            fs::write(path, br#"{"runtime":"old"}"#).expect("write stale sidecar");
        }

        persist(
            &root.join("registry.json"),
            &controller,
            &candidate,
            &marker,
            &json!({"runtime": "new", "kind": "registry"}),
            &json!({"runtime": "new", "kind": "controller"}),
            &json!({"runtime": "new", "kind": "candidate"}),
            &json!({"runtime": "new", "kind": "marker"}),
        )
        .expect("refresh immutable generation sidecars");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(&controller).expect("read refreshed controller"),
            )
            .expect("decode refreshed controller"),
            json!({"runtime": "new", "kind": "controller"})
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(&candidate).expect("read refreshed candidate"),
            )
            .expect("decode refreshed candidate"),
            json!({"runtime": "new", "kind": "candidate"})
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(&marker).expect("read refreshed marker"),
            )
            .expect("decode refreshed marker"),
            json!({"runtime": "new", "kind": "marker"})
        );
        let current: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("response-authority-sidecar-current-v2.json"))
                .expect("read generation pointer"),
        )
        .expect("decode generation pointer");
        let generation_root = current["generation_root_sha256"]
            .as_str()
            .expect("generation root");
        let generation = root
            .join("response-authority-sidecar-generations-v2")
            .join(generation_root);
        for name in [
            "registry.json",
            "controller.json",
            "candidate.json",
            "marker.json",
            "manifest.json",
        ] {
            assert!(generation.join(name).is_file(), "missing {name}");
        }
        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn pointer_is_not_published_before_legacy_diagnostics_finish() {
        let root = std::env::temp_dir().join(format!(
            "nando-response-authority-sidecars-failure-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("candidate.json")).expect("blocking candidate directory");
        let result = persist(
            &root.join("registry.json"),
            &root.join("controller.json"),
            &root.join("candidate.json"),
            &root.join("marker.json"),
            &json!({"kind": "registry"}),
            &json!({"kind": "controller"}),
            &json!({"kind": "candidate"}),
            &json!({"kind": "marker"}),
        );
        assert!(result.is_err());
        assert!(
            !root
                .join("response-authority-sidecar-current-v2.json")
                .exists()
        );
        fs::remove_dir_all(root).expect("remove temp root");
    }
}
