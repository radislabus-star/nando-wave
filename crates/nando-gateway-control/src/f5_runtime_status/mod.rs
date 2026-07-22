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

static PANEL_HTML: OnceLock<String> = OnceLock::new();

pub(crate) fn panel_html() -> &'static str {
    // This panel is proof-backed UI. A missing or incompatible receipt must
    // remove the PASS claim instead of falling back to duplicated constants.
    PANEL_HTML
        .get_or_init(|| render_from_sources(CONVERGENCE_RECEIPT, TRAFFIC_RECEIPT, F6_RECEIPT))
        .as_str()
}

fn render_from_sources(convergence_source: &str, traffic_source: &str, f6_source: &str) -> String {
    match receipt::parse_and_validate(convergence_source, traffic_source, f6_source) {
        Ok(status) => render::verified_panel(&status),
        Err(error) => render::unavailable_panel(&error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_receipts_render_the_full_f6_boundary() {
        let html = render_from_sources(CONVERGENCE_RECEIPT, TRAFFIC_RECEIPT, F6_RECEIPT);

        assert!(html.contains("data-research-status=\"f6-complete-f7-not-started\""));
        for stage in [
            "f5-a", "f5-b", "f5-c", "f5-d", "f5-e", "f5-f", "f5-g", "f6", "f7",
        ] {
            assert!(html.contains(&format!("data-rd-stage=\"{stage}\"")));
        }
        assert!(html.contains("FULL CONTROLLED F5 TO F6 PROOF PATH CONFIRMED"));
        assert!(html.contains("F6 COMPLETE"));
        assert!(html.contains("NOT STARTED"));
        assert!(html.contains("authority=false · ACTIVE=0"));
        assert!(html.contains("WATCH_NO_SEARCH_GAIN"));
        assert!(!html.contains("production end-to-end confirmed"));
    }

    #[test]
    fn incompatible_receipt_removes_the_pass_claim() {
        let invalid =
            CONVERGENCE_RECEIPT.replace("F5_COMPLETE_F6_UNLOCKED_NOT_STARTED", "F6_COMPLETE");
        let html = render_from_sources(&invalid, TRAFFIC_RECEIPT, F6_RECEIPT);

        assert!(html.contains("data-research-status=\"unavailable\""));
        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(html.contains("F7 LOCKED"));
        assert!(!html.contains("FULL CONTROLLED F5 TO F6 PROOF PATH CONFIRMED"));
    }

    #[test]
    fn authority_claim_in_receipt_fails_closed() {
        let invalid =
            CONVERGENCE_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(&invalid, TRAFFIC_RECEIPT, F6_RECEIPT);

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F5 COMPLETE"));
    }

    #[test]
    fn f6_authority_claim_removes_the_pass_claim() {
        let invalid = F6_RECEIPT.replacen("\"authority\": false", "\"authority\": true", 1);
        let html = render_from_sources(CONVERGENCE_RECEIPT, TRAFFIC_RECEIPT, &invalid);

        assert!(html.contains("R&amp;D STATUS UNAVAILABLE"));
        assert!(!html.contains("F6 COMPLETE"));
    }
}
