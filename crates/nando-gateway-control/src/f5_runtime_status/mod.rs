mod f7_receipt;
mod f8_final_receipt;
mod f8_resource_receipt;
mod receipt;
mod render;

use std::sync::OnceLock;

const CONVERGENCE_RECEIPT: &str =
    include_str!("../../../../plans/effect-law-unification-v1/STOP_F5_RUNTIME_CONVERGENCE.json");
const TRAFFIC_RECEIPT: &str =
    include_str!("../../../../plans/effect-law-unification-v1/f5g/STOP_F5_G_TRAFFIC_SHADOW.json");
const F6_RECEIPT: &str = include_str!(
    "../../../../plans/effect-law-unification-v1/STOP_F6_INDEPENDENT_VERIFIER_CONVERGENCE.json"
);
const F7_RECEIPT: &str = include_str!(
    "../../../../plans/effect-law-unification-v1/STOP_F7_E_CONTROLLED_GENERATION_SHADOW.json"
);
const F8_RESOURCE_RECEIPT: &str =
    include_str!("../../../../plans/effect-law-unification-v1/STOP_F8_0_RESOURCE_TRUTH.json");
const F8_FINAL_RECEIPT: &str =
    include_str!("../../../../plans/effect-law-unification-v1/STOP_F8.json");
const F8_PHASE_CONTROL_RECEIPT: &str = include_str!(
    "../../../../plans/effect-law-unification-v1/STOP_F8_D_PHASE_CONTROL_RECEIPT.json"
);
const F8_EXTERNAL_CANDIDATE: &str = include_str!(
    "../../../../plans/effect-law-unification-v1/STOP_F8_E_EXTERNAL_ADMISSION_CANDIDATE.json"
);

static PANEL_HTML: OnceLock<String> = OnceLock::new();

#[derive(Clone, Copy)]
struct ProofReceiptSources<'a> {
    convergence: &'a str,
    traffic: &'a str,
    f6: &'a str,
    f7: &'a str,
    f8_resource: &'a str,
    f8_final: &'a str,
    f8_phase_control: &'a str,
    f8_external_candidate: &'a str,
}

fn embedded_sources() -> ProofReceiptSources<'static> {
    ProofReceiptSources {
        convergence: CONVERGENCE_RECEIPT,
        traffic: TRAFFIC_RECEIPT,
        f6: F6_RECEIPT,
        f7: F7_RECEIPT,
        f8_resource: F8_RESOURCE_RECEIPT,
        f8_final: F8_FINAL_RECEIPT,
        f8_phase_control: F8_PHASE_CONTROL_RECEIPT,
        f8_external_candidate: F8_EXTERNAL_CANDIDATE,
    }
}

pub(crate) struct ProofSummary {
    pub(crate) verified: bool,
    pub(crate) failure: Option<String>,
    pub(crate) f5_commit: String,
    pub(crate) f7_receipt_date: String,
    pub(crate) f7_queue_max: u64,
    pub(crate) f7_no_match_p99_ns: u64,
    pub(crate) f7_matched_p99_ns: u64,
    pub(crate) f8_rss_bytes: u64,
    pub(crate) f8_rss_target_bytes: u64,
    pub(crate) f8_resource_observations: u64,
    pub(crate) f8_no_match_p99_max_ns: u64,
    pub(crate) f8_provider_records: u64,
    pub(crate) f8_verified_receipts: u64,
    pub(crate) f8_full_phase_gain: u64,
    pub(crate) f8_search_gain: u64,
    pub(crate) f8_external_verdict: String,
    pub(crate) f8_commitments_sha256: String,
    pub(crate) f8_matched_p99_max_ns: u64,
    pub(crate) f8_hard_max_ns: u64,
    pub(crate) f8_hot_rss_bytes: u64,
}

pub(crate) fn panel_html() -> &'static str {
    // This panel is proof-backed UI. A missing or incompatible receipt must
    // remove the PASS claim instead of falling back to duplicated constants.
    PANEL_HTML
        .get_or_init(|| render_from_sources(embedded_sources()))
        .as_str()
}

pub(crate) fn proof_summary() -> ProofSummary {
    match (
        receipt::parse_and_validate(CONVERGENCE_RECEIPT, TRAFFIC_RECEIPT, F6_RECEIPT, F7_RECEIPT),
        f8_resource_receipt::parse_and_validate(F8_RESOURCE_RECEIPT),
        f8_final_receipt::parse_and_validate(
            F8_FINAL_RECEIPT,
            F8_PHASE_CONTROL_RECEIPT,
            F8_EXTERNAL_CANDIDATE,
        ),
    ) {
        (Ok(status), Ok(resource), Ok(final_status)) => ProofSummary {
            verified: true,
            failure: None,
            f5_commit: status.f5_implementation_commit,
            f7_receipt_date: status.f7_receipt_date,
            f7_queue_max: status.f7_queue_max,
            f7_no_match_p99_ns: status.f7_no_match_p99_ns,
            f7_matched_p99_ns: status.f7_matched_p99_ns,
            f8_rss_bytes: resource.max_peak_rss_delta_bytes,
            f8_rss_target_bytes: resource.rss_target_bytes,
            f8_resource_observations: resource.resource_observations,
            f8_no_match_p99_max_ns: final_status.no_match_p99_max_ns,
            f8_provider_records: final_status.provider_records,
            f8_verified_receipts: final_status.verified_receipts,
            f8_full_phase_gain: final_status.full_phase_gain,
            f8_search_gain: final_status.search_gain,
            f8_external_verdict: final_status.external_verdict,
            f8_commitments_sha256: final_status.commitments_sha256,
            f8_matched_p99_max_ns: final_status.matched_p99_max_ns,
            f8_hard_max_ns: final_status.hard_max_ns,
            f8_hot_rss_bytes: final_status.hot_rss_bytes,
        },
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => ProofSummary {
            verified: false,
            failure: Some(error),
            f5_commit: String::new(),
            f7_receipt_date: String::new(),
            f7_queue_max: 0,
            f7_no_match_p99_ns: 0,
            f7_matched_p99_ns: 0,
            f8_rss_bytes: 0,
            f8_rss_target_bytes: 0,
            f8_resource_observations: 0,
            f8_no_match_p99_max_ns: 0,
            f8_provider_records: 0,
            f8_verified_receipts: 0,
            f8_full_phase_gain: 0,
            f8_search_gain: 0,
            f8_external_verdict: String::new(),
            f8_commitments_sha256: String::new(),
            f8_matched_p99_max_ns: 0,
            f8_hard_max_ns: 0,
            f8_hot_rss_bytes: 0,
        },
    }
}

fn render_from_sources(sources: ProofReceiptSources<'_>) -> String {
    match (
        receipt::parse_and_validate(sources.convergence, sources.traffic, sources.f6, sources.f7),
        f8_resource_receipt::parse_and_validate(sources.f8_resource),
        f8_final_receipt::parse_and_validate(
            sources.f8_final,
            sources.f8_phase_control,
            sources.f8_external_candidate,
        ),
    ) {
        (Ok(status), Ok(resource), Ok(final_status)) => {
            render::verified_panel(&status, &resource, &final_status)
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            render::unavailable_panel(&error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_receipts_render_the_full_f8_boundary() {
        let html = render_from_sources(embedded_sources());

        assert!(html.contains("data-research-status=\"stop-f8-pass-authority-false\""));
        for stage in [
            "f5-a", "f5-b", "f5-c", "f5-d", "f5-e", "f5-f", "f5-g", "f6", "f7-a", "f7-b", "f7-c",
            "f7-d", "f7-e", "f8-0", "f8-a", "f8-b", "f8-c", "f8-d", "f8-e", "cpu",
        ] {
            assert!(html.contains(&format!("data-rd-stage=\"{stage}\"")));
        }
        assert!(html.contains("FULL CONTROLLED F5 TO F7 PROOF PATH CONFIRMED"));
        assert!(html.contains("F6 COMPLETE"));
        assert!(html.contains("F7 COMPLETE"));
        assert!(html.contains("F8 COMPLETE"));
        assert!(html.contains("SHADOW_READY"));
        assert!(html.contains("ACTIVE=0 · authority=false"));
        assert!(html.contains("WATCH_NO_SEARCH_GAIN"));
        assert!(!html.contains("live capture producer missing"));
        assert!(html.contains("F5 conservative RSS 49631232 / target 16777216 B WATCH"));
        assert!(html.contains("production-policy RSS 10723328 / target 16777216 B"));
        assert!(html.contains("p99 no-match/matched 195340/648010 ns"));
        assert!(!html.contains("production end-to-end confirmed"));
    }

    #[test]
    fn incompatible_receipt_removes_the_pass_claim() {
        let invalid =
            CONVERGENCE_RECEIPT.replace("F5_COMPLETE_F6_UNLOCKED_NOT_STARTED", "F6_COMPLETE");
        let html = render_from_sources(ProofReceiptSources {
            convergence: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("data-research-status=\"unavailable\""));
        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(html.contains("F8 LOCKED"));
        assert!(!html.contains("FULL CONTROLLED F5 TO F7 PROOF PATH CONFIRMED"));
    }

    #[test]
    fn authority_claim_in_receipt_fails_closed() {
        let invalid =
            CONVERGENCE_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(ProofReceiptSources {
            convergence: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F5 COMPLETE"));
    }

    #[test]
    fn f6_authority_claim_removes_the_pass_claim() {
        let invalid = F6_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(ProofReceiptSources {
            f6: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F6 COMPLETE"));
    }

    #[test]
    fn f7_authority_claim_removes_the_pass_claim() {
        let invalid = F7_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(ProofReceiptSources {
            f7: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F7 COMPLETE"));
    }

    #[test]
    fn f8_resource_authority_claim_removes_the_pass_claim() {
        let invalid =
            F8_RESOURCE_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(ProofReceiptSources {
            f8_resource: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F8-0 PASS"));
    }

    #[test]
    fn f8_authority_claim_removes_the_pass_claim() {
        let invalid = F8_FINAL_RECEIPT.replacen(
            "\"execution_authority\": false",
            "\"execution_authority\": true",
            1,
        );
        let html = render_from_sources(ProofReceiptSources {
            f8_final: &invalid,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F8 COMPLETE"));
    }

    #[test]
    fn f8_canonical_artifact_drift_removes_the_pass_claim() {
        let drifted_candidate = format!("{F8_EXTERNAL_CANDIDATE}\n");
        let html = render_from_sources(ProofReceiptSources {
            f8_external_candidate: &drifted_candidate,
            ..embedded_sources()
        });

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("SHADOW_READY"));
    }
}
