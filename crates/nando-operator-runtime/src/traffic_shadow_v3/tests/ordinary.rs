use std::collections::BTreeSet;
use std::sync::Arc;

use nando_operator_kernel::{RuntimeProjectionV3, canonical_json_sha256};
use serde::Deserialize;

use super::generation;
use crate::{
    RuntimeContextBudgetV3, TrafficShadowInputV3, TrafficShadowSourceV3, TrafficShadowVerdictV3,
    execute_traffic_shadow_v3,
};

const ORDINARY_WINDOW: &str = include_str!(
    "../../../../../plans/effect-law-unification-v1/f5g/FROZEN_ORDINARY_TRAFFIC_WINDOW_V1.json"
);

#[derive(Deserialize)]
struct FrozenWindow {
    schema: String,
    status: String,
    source: FrozenSource,
    privacy: FrozenPrivacy,
    rows: Vec<FrozenRow>,
}

#[derive(Deserialize)]
struct FrozenSource {
    path: String,
    sha256: String,
    total_rows: usize,
    ordinary_rows: usize,
}

#[derive(Deserialize)]
struct FrozenPrivacy {
    raw_request_text_present: bool,
    raw_provider_payload_present: bool,
    client_intent_id_present: bool,
    replay_authority: bool,
    execution_authority: bool,
}

#[derive(Deserialize)]
struct FrozenRow {
    ordinal: usize,
    timestamp_unix: u64,
    request_sha256: String,
    input_tokens: u64,
    terminal_state: String,
    verification_status: String,
    upstream_socket_opened: bool,
    intent_dedupe_eligible: bool,
    input_token_accounting: String,
}

#[test]
fn frozen_ordinary_denominator_is_fully_accounted_without_fake_replay() {
    let window: FrozenWindow = serde_json::from_str(ORDINARY_WINDOW).expect("frozen window");
    assert_eq!(window.schema, "nando.f5g-frozen-ordinary-traffic-window.v1");
    assert_eq!(window.status, "FROZEN_METADATA_ONLY");
    assert_eq!(
        window.source.path,
        "/var/lib/nando-wave/transition/economics-terminal.jsonl"
    );
    assert_eq!(
        window.source.sha256,
        "cf6de0789fc363957d79ffc207f93a7f5c542edd2892dd87a86c705f8af07e60"
    );
    assert_eq!(
        (window.source.total_rows, window.source.ordinary_rows),
        (222, 25)
    );
    assert_eq!(window.rows.len(), 25);
    assert!(!window.privacy.raw_request_text_present);
    assert!(!window.privacy.raw_provider_payload_present);
    assert!(!window.privacy.client_intent_id_present);
    assert!(!window.privacy.replay_authority);
    assert!(!window.privacy.execution_authority);

    let generation = Arc::new(generation(1, 721));
    let mut accounted = 0_usize;
    let mut request_roots = BTreeSet::new();
    for row in window.rows {
        assert_eq!(row.ordinal, accounted);
        assert!(row.timestamp_unix > 0);
        assert!(row.input_tokens > 0);
        assert_eq!(row.terminal_state, "delivered");
        assert_eq!(row.verification_status, "verified");
        assert!(!row.upstream_socket_opened);
        assert!(row.intent_dedupe_eligible);
        assert_eq!(row.input_token_accounting, "byte_estimate_v1");
        assert!(request_roots.insert(row.request_sha256.clone()));
        let row_sha256 = canonical_json_sha256(&(
            "nando.f5g-ordinary-window-row.v1",
            row.ordinal,
            row.request_sha256.as_str(),
        ))
        .expect("window row digest");
        let input = TrafficShadowInputV3::metadata_only(
            &row_sha256,
            &row.request_sha256,
            Some(RuntimeProjectionV3::Responses),
            None,
            TrafficShadowSourceV3::Ordinary,
        )
        .expect("metadata-only row");
        let receipt = execute_traffic_shadow_v3(
            Arc::clone(&generation),
            input,
            RuntimeContextBudgetV3::default(),
        );
        assert_eq!(
            receipt.verdict(),
            TrafficShadowVerdictV3::CensoredPayloadUnavailable
        );
        assert_eq!(receipt.raw_payloads_persisted(), 0);
        assert_eq!(receipt.local_accepts(), 0);
        assert!(!receipt.execution_authority());
        accounted += 1;
    }
    assert_eq!(accounted, window.source.ordinary_rows);
}
