use super::*;

mod future;
mod probe;

pub(super) use future::advance_independent_future;
pub(super) use probe::advance_probe;

pub(super) fn generation_expired(
    freeze: &K1NaturalCandidateFreezeV1,
    generated_at_unix: u64,
) -> bool {
    generated_at_unix
        >= freeze
            .selected_at_unix
            .saturating_add(freeze.budget.maximum_generation_seconds)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn terminal_verdict(
    candidate: &K1NaturalCandidateFreezeV1,
    identification: Option<&K1IdentificationFreezeV1>,
    classes: Vec<String>,
    evidence: Vec<String>,
    verdict: K1GenerationVerdictClassV1,
    blocker: &str,
    terminal_at_unix: u64,
    transfer_identification: Option<MultiSourceT1IdentificationV3>,
) -> Result<K1GenerationTerminalVerdictV1, String> {
    K1GenerationTerminalVerdictV1::seal(
        candidate.freeze_root_sha256.clone(),
        identification.map(|freeze| freeze.freeze_root_sha256.clone()),
        classes,
        evidence,
        verdict,
        blocker.to_owned(),
        terminal_at_unix,
        transfer_identification,
    )
    .map_err(str::to_owned)
}
