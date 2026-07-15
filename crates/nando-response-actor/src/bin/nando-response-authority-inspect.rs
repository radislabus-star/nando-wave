use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use nando_response_actor::{
    RESPONSE_AUTHORITY_SCHEMA_V2, ResponseExecutor, ResponsePackageAuthorityBindingV2,
    ResponseRegistry, response_actor_program_digest, response_execution_payload_digest,
    response_independent_verifier_program_digest, response_package_digest,
    response_proof_receipts_digest, response_registry_digest, sha256_bytes,
};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--registry-digest" {
        let result = fs::read(&args[1])
            .map_err(|error| format!("read:{}:{error}", args[1]))
            .and_then(|bytes| {
                serde_json::from_slice::<ResponseRegistry>(&bytes)
                    .map_err(|error| format!("parse:{}:{error}", args[1]))
            })
            .and_then(|registry| response_registry_digest(&registry).map_err(str::to_owned));
        match result {
            Ok(digest) => println!("{digest}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.len() == 3 && args[0] == "--refresh-candidate" {
        match refresh_candidate(Path::new(&args[1]), Path::new(&args[2])) {
            Ok(candidate) => println!("{candidate}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.len() != 5 {
        eprintln!(
            "usage: nando-response-authority-inspect REGISTRY ADMISSION PROJECT_ID GATE_BUILD RUNTIME_BUILD"
        );
        std::process::exit(2);
    }
    let read =
        |path: &str| fs::read(Path::new(path)).map_err(|error| format!("read:{path}:{error}"));
    let result = (|| {
        let registry = read(&args[0])?;
        let admission = read(&args[1])?;
        let gate = read(&args[3])?;
        let runtime = read(&args[4])?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("clock:{error}"))?
            .as_secs();
        ResponseExecutor::from_authorized_json(
            &registry,
            &admission,
            &args[2],
            &sha256_bytes(&gate),
            &sha256_bytes(&runtime),
            now,
            30,
        )
        .map(|executor| {
            serde_json::json!({
                "verdict":"PASS",
                "registry_schema":executor.registry_schema(),
                "revision":executor.revision(),
                "active_packages":executor.active_package_count(),
                "admission_sha256":executor.admission_sha256(),
            })
        })
    })();
    match result {
        Ok(report) => println!("{report}"),
        Err(error) => {
            println!("{}", serde_json::json!({"verdict":"VETO","reason":error}));
            std::process::exit(1);
        }
    }
}

fn refresh_candidate(
    registry_path: &Path,
    candidate_path: &Path,
) -> Result<serde_json::Value, String> {
    let registry: ResponseRegistry = serde_json::from_slice(
        &fs::read(registry_path).map_err(|error| format!("registry_read:{error}"))?,
    )
    .map_err(|error| format!("registry_parse:{error}"))?;
    let candidate: serde_json::Value = serde_json::from_slice(
        &fs::read(candidate_path).map_err(|error| format!("candidate_read:{error}"))?,
    )
    .map_err(|error| format!("candidate_parse:{error}"))?;
    if candidate.get("schema").and_then(serde_json::Value::as_str)
        != Some("nando.response-authority-candidate.v1")
        || candidate
            .get("execution_authority")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
    {
        return Err("candidate_contract_mismatch".to_owned());
    }
    let old_bindings = serde_json::from_value::<Vec<ResponsePackageAuthorityBindingV2>>(
        candidate
            .get("packages")
            .cloned()
            .ok_or_else(|| "candidate_packages_missing".to_owned())?,
    )
    .map_err(|error| format!("candidate_packages_parse:{error}"))?;
    let old_by_id = old_bindings
        .into_iter()
        .map(|binding| (binding.package_id.clone(), binding))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut bindings = Vec::new();
    for package in registry
        .packages
        .iter()
        .filter(|package| package.eligible_for_admission_candidate())
    {
        let mut binding = old_by_id
            .get(&package.package_id)
            .cloned()
            .ok_or_else(|| format!("candidate_binding_missing:{}", package.package_id))?;
        let verifier = package
            .verifier
            .as_ref()
            .ok_or_else(|| format!("candidate_verifier_missing:{}", package.package_id))?;
        binding.registry_revision = registry.revision;
        binding.package_sha256 = response_package_digest(package).map_err(str::to_owned)?;
        binding.execution_payload_sha256 =
            response_execution_payload_digest(package).map_err(str::to_owned)?;
        binding.actor_program_sha256 =
            response_actor_program_digest(&package.program).map_err(str::to_owned)?;
        binding.independent_verifier_program_sha256 =
            response_independent_verifier_program_digest(verifier).map_err(str::to_owned)?;
        binding.verifier_schema = package.proof.verifier_schema.clone();
        binding.proof_receipts_sha256 = String::new();
        binding.proof_receipts_sha256 =
            response_proof_receipts_digest(&binding).map_err(str::to_owned)?;
        bindings.push(binding);
    }
    bindings.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    if bindings.len() != old_by_id.len() {
        return Err("candidate_active_package_set_mismatch".to_owned());
    }
    Ok(serde_json::json!({
        "schema": "nando.response-authority-candidate.v1",
        "authority_schema": RESPONSE_AUTHORITY_SCHEMA_V2,
        "registry_schema": registry.schema,
        "registry_revision": registry.revision,
        "registry_sha256": response_registry_digest(&registry).map_err(str::to_owned)?,
        "packages": bindings,
        "required_gate_fields": [
            "gate_build_sha256",
            "runtime_build_sha256",
            "generated_at_unix",
            "expires_at_unix"
        ],
        "execution_authority": false
    }))
}
