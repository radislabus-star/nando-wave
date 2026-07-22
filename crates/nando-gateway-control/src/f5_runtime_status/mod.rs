mod f7_receipt;
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

static PANEL_HTML: OnceLock<String> = OnceLock::new();

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
}

pub(crate) fn panel_html() -> &'static str {
    // This panel is proof-backed UI. A missing or incompatible receipt must
    // remove the PASS claim instead of falling back to duplicated constants.
    PANEL_HTML
        .get_or_init(|| {
            render_from_sources(
                CONVERGENCE_RECEIPT,
                TRAFFIC_RECEIPT,
                F6_RECEIPT,
                F7_RECEIPT,
                F8_RESOURCE_RECEIPT,
            )
        })
        .as_str()
}

pub(crate) fn proof_summary() -> ProofSummary {
    match (
        receipt::parse_and_validate(CONVERGENCE_RECEIPT, TRAFFIC_RECEIPT, F6_RECEIPT, F7_RECEIPT),
        f8_resource_receipt::parse_and_validate(F8_RESOURCE_RECEIPT),
    ) {
        (Ok(status), Ok(resource)) => ProofSummary {
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
            f8_no_match_p99_max_ns: resource.no_match_p99_max_ns,
        },
        (Err(error), _) | (_, Err(error)) => ProofSummary {
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
        },
    }
}

fn render_from_sources(
    convergence_source: &str,
    traffic_source: &str,
    f6_source: &str,
    f7_source: &str,
    f8_resource_source: &str,
) -> String {
    match (
        receipt::parse_and_validate(convergence_source, traffic_source, f6_source, f7_source),
        f8_resource_receipt::parse_and_validate(f8_resource_source),
    ) {
        (Ok(status), Ok(resource)) => render::verified_panel(&status, &resource),
        (Err(error), _) | (_, Err(error)) => render::unavailable_panel(&error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_receipts_render_the_full_f7_boundary() {
        let html = render_from_sources(
            CONVERGENCE_RECEIPT,
            TRAFFIC_RECEIPT,
            F6_RECEIPT,
            F7_RECEIPT,
            F8_RESOURCE_RECEIPT,
        );

        assert!(html.contains("data-research-status=\"f8-0-pass-f8-a-ready\""));
        for stage in [
            "f5-a", "f5-b", "f5-c", "f5-d", "f5-e", "f5-f", "f5-g", "f6", "f7-a", "f7-b", "f7-c",
            "f7-d", "f7-e", "f8-0", "f8-a",
        ] {
            assert!(html.contains(&format!("data-rd-stage=\"{stage}\"")));
        }
        assert!(html.contains("FULL CONTROLLED F5 TO F7 PROOF PATH CONFIRMED"));
        assert!(html.contains("F6 COMPLETE"));
        assert!(html.contains("F7 COMPLETE"));
        assert!(html.contains("F8-0 PASS"));
        assert!(html.contains("F8-A CONTROLLED"));
        assert!(html.contains("authority=false · ACTIVE=0"));
        assert!(html.contains("WATCH_NO_SEARCH_GAIN"));
        assert!(html.contains("live capture producer missing"));
        assert!(html.contains("F5 conservative RSS 49631232 / target 16777216 B WATCH"));
        assert!(html.contains("production-policy RSS 10723328 / target 16777216 B"));
        assert!(html.contains("latency WATCH"));
        assert!(!html.contains("production end-to-end confirmed"));
    }

    #[test]
    fn incompatible_receipt_removes_the_pass_claim() {
        let invalid =
            CONVERGENCE_RECEIPT.replace("F5_COMPLETE_F6_UNLOCKED_NOT_STARTED", "F6_COMPLETE");
        let html = render_from_sources(
            &invalid,
            TRAFFIC_RECEIPT,
            F6_RECEIPT,
            F7_RECEIPT,
            F8_RESOURCE_RECEIPT,
        );

        assert!(html.contains("data-research-status=\"unavailable\""));
        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(html.contains("F8 LOCKED"));
        assert!(!html.contains("FULL CONTROLLED F5 TO F7 PROOF PATH CONFIRMED"));
    }

    #[test]
    fn authority_claim_in_receipt_fails_closed() {
        let invalid =
            CONVERGENCE_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(
            &invalid,
            TRAFFIC_RECEIPT,
            F6_RECEIPT,
            F7_RECEIPT,
            F8_RESOURCE_RECEIPT,
        );

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F5 COMPLETE"));
    }

    #[test]
    fn f6_authority_claim_removes_the_pass_claim() {
        let invalid = F6_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(
            CONVERGENCE_RECEIPT,
            TRAFFIC_RECEIPT,
            &invalid,
            F7_RECEIPT,
            F8_RESOURCE_RECEIPT,
        );

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F6 COMPLETE"));
    }

    #[test]
    fn f7_authority_claim_removes_the_pass_claim() {
        let invalid = F7_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(
            CONVERGENCE_RECEIPT,
            TRAFFIC_RECEIPT,
            F6_RECEIPT,
            &invalid,
            F8_RESOURCE_RECEIPT,
        );

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F7 COMPLETE"));
    }

    #[test]
    fn f8_resource_authority_claim_removes_the_pass_claim() {
        let invalid =
            F8_RESOURCE_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(
            CONVERGENCE_RECEIPT,
            TRAFFIC_RECEIPT,
            F6_RECEIPT,
            F7_RECEIPT,
            &invalid,
        );

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F8-0 PASS"));
    }
}
