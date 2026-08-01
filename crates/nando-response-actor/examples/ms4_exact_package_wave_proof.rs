use std::path::PathBuf;

use nando_response_actor::Ms4ExternalAdmissionCandidateV1;

fn main() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "usage: ms4_exact_package_wave_proof <candidate.cbor>".to_owned())?;
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("candidate_read:{}:{error}", path.display()))?;
    let candidate = Ms4ExternalAdmissionCandidateV1::from_canonical_bytes(&bytes)
        .map_err(|error| format!("candidate_restore:{error}"))?;
    let proof = candidate
        .exact_package_wave_proof()
        .map_err(|error| format!("exact_package_wave_proof:{error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&proof)
            .map_err(|error| format!("proof_json_encode:{error}"))?
    );
    Ok(())
}
