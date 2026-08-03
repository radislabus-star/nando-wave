use std::path::Path;

use serde::Serialize;

pub(super) fn persist(
    controller_admission_path: &Path,
    authority_candidate_path: &Path,
    marker_path: &Path,
    controller_admission: &impl Serialize,
    authority_candidate: &impl Serialize,
    marker: &impl Serialize,
) -> Result<(), String> {
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
    )
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
            &controller,
            &candidate,
            &marker,
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
        fs::remove_dir_all(root).expect("remove temp root");
    }
}
