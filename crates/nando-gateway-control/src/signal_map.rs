use crate::f5_runtime_status::ProofSummary;
use serde_json::Value;

pub(crate) struct LiveSignalView<'a> {
    pub(crate) partition: u64,
    pub(crate) generation: u64,
    pub(crate) transitions: u64,
    pub(crate) support: u64,
    pub(crate) matching: u64,
    pub(crate) matching_sessions: u64,
    pub(crate) after_watermark: u64,
    pub(crate) independent: u64,
    pub(crate) consistent: u64,
    pub(crate) routed: u64,
    pub(crate) future: u64,
    pub(crate) blocker: &'a str,
    pub(crate) admission_verdict: &'a str,
    pub(crate) admission_blocker: &'a str,
    pub(crate) admission_blocker_stage: &'a str,
    pub(crate) admission_age_seconds: u64,
    pub(crate) admission_relation_candidates: u64,
    pub(crate) admission_future_rows: u64,
    pub(crate) admission_runtime_parity_cases: u64,
    pub(crate) active_packages: u64,
    pub(crate) online_ready: bool,
    pub(crate) capture_phase: &'a str,
    pub(crate) capture_records: u64,
    pub(crate) capture_captured: u64,
    pub(crate) capture_censored: u64,
    pub(crate) capture_publish_sequence: u64,
    pub(crate) capture_last_error: &'a str,
    pub(crate) shadow_phase: &'a str,
    pub(crate) shadow_submitted: u64,
    pub(crate) shadow_evaluated: u64,
    pub(crate) shadow_verified: u64,
    pub(crate) shadow_parity_mismatches: u64,
}

#[derive(Clone, Copy)]
enum RouteState {
    Live,
    Proven,
    Wait,
    Block,
    Locked,
}

impl RouteState {
    const fn class(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Proven => "proven",
            Self::Wait => "wait",
            Self::Block => "block",
            Self::Locked => "locked",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Live => "LIVE FLOW",
            Self::Proven => "PROOF ONLY",
            Self::Wait => "WAIT",
            Self::Block => "BLOCK",
            Self::Locked => "LOCKED",
        }
    }

    const fn live_flow(self) -> bool {
        matches!(self, Self::Live)
    }
}

struct PipelineStage {
    id: &'static str,
    title: &'static str,
    owner: String,
    input: String,
    live: String,
    proof: String,
    diagnostic: String,
    output: String,
    state: RouteState,
}

pub(crate) fn render(
    live: &LiveSignalView<'_>,
    proof: &ProofSummary,
    manifest: &Value,
    model_label: &str,
) -> String {
    let build_id = manifest
        .get("build_id")
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    let build_commit = manifest
        .get("git_commit")
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    let natural_operator_live = live.active_packages > 0;
    let proof_state = if proof.verified {
        RouteState::Proven
    } else {
        RouteState::Locked
    };
    let downstream_state = if natural_operator_live {
        RouteState::Live
    } else {
        proof_state
    };
    let capture_error = if live.capture_last_error.is_empty() {
        "none"
    } else {
        live.capture_last_error
    };

    let proof_boundary = if proof.verified {
        format!(
            "STOP-F8 {} / {} verified / {} / commit {}",
            proof.f8_external_verdict,
            proof.f8_verified_receipts,
            compact(&proof.f5_commit),
            proof.f7_receipt_date,
        )
    } else {
        format!(
            "proof receipts unavailable: {}",
            proof
                .failure
                .as_deref()
                .unwrap_or("unknown validation failure")
        )
    };
    let (current_stage, current_blocker, current_reason) = if !proof.verified {
        (
            "PROOF",
            "PROOF RECEIPT VALIDATION",
            "Controlled capability receipts failed closed validation. Downstream modules cannot display PROOF until their canonical bytes validate.".to_owned(),
        )
    } else if !natural_operator_live {
        (
            "L3",
            "L3 NATURAL OPERATOR DISCOVERY",
            format!(
                "Ordinary traces reach learning, but no natural OperatorPackage exists: support {}/32, future {}/32, routed {}, blocker {}. STOP-F8 proves downstream plumbing only; its controlled seed is not natural evidence.",
                live.support, live.future, live.routed, live.blocker,
            ),
        )
    } else {
        (
            "L11",
            "L11 PRODUCTION AUTHORITY",
            "A natural package exists, but admission still requires an independent authority lease before CPU execution.".to_owned(),
        )
    };

    let stages = vec![
        PipelineStage {
            id: "L1",
            title: "Provider request ingress",
            owner: "nando-nginx-gateway".into(),
            input: format!("Codex request / model {model_label}"),
            live: "HTTPS streaming is active; provider fallback remains available".into(),
            proof: "transport health is observed independently from operator authority".into(),
            diagnostic: format!("build {build_id} / mode SHADOW / local CPU disabled"),
            output: "request envelope + eventual completed trace".into(),
            state: RouteState::Live,
        },
        PipelineStage {
            id: "L2",
            title: "Trace and hash evidence capture",
            owner: "nando-transition-serving".into(),
            input: "provider boundary + completed session trace".into(),
            live: format!(
                "observer {} / {} transitions; capture {} / current captured {} / censored {}",
                if live.online_ready { "READY" } else { "WAIT" },
                live.transitions,
                live.capture_phase,
                live.capture_captured,
                live.capture_censored,
            ),
            proof: if proof.verified {
                format!(
                    "hash-only capture PASS / {} durable controlled records / raw payload 0 B",
                    proof.f8_provider_records.max(live.capture_records)
                )
            } else {
                "hash-only capture proof unavailable".into()
            },
            diagnostic: format!(
                "publish {} / error {} / process counters may reset, durable index does not",
                live.capture_publish_sequence, capture_error,
            ),
            output: "relation fragments + immutable request receipt".into(),
            state: if live.online_ready {
                RouteState::Live
            } else {
                RouteState::Wait
            },
        },
        PipelineStage {
            id: "L3",
            title: "Natural operator discovery",
            owner: "nando-operator-learning".into(),
            input: "relation fragments + completed-trace teacher evidence".into(),
            live: format!(
                "support {}/32 / future {}/32 / matching {} in {} sessions / independent {}",
                live.support, live.future, live.matching, live.matching_sessions, live.independent,
            ),
            proof: "NOT_EVALUATED by STOP-F8; controlled seed is injected after this boundary"
                .into(),
            diagnostic: format!(
                "watermark {} / consistent {} / routed {} / blocker {}",
                live.after_watermark, live.consistent, live.routed, live.blocker,
            ),
            output: "natural circuit-attractor + typed OperatorPackage".into(),
            state: if natural_operator_live {
                RouteState::Live
            } else {
                RouteState::Block
            },
        },
        PipelineStage {
            id: "L4",
            title: "Operator crystallizer",
            owner: "nando-operator-learning".into(),
            input: "phase-coherent circuit-attractor".into(),
            live: live_downstream_text(natural_operator_live, "natural circuit"),
            proof: if proof.verified {
                "controlled circuit compiled into bounded operator data".into()
            } else {
                "controlled crystallization proof unavailable".into()
            },
            diagnostic:
                "whole-circuit coherence required; selected fragments alone are insufficient".into(),
            output: "versioned immutable OperatorPackage".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L5",
            title: "Generation persistence",
            owner: "nando-operator-persistence".into(),
            input: "immutable OperatorPackage generation".into(),
            live: live_downstream_text(natural_operator_live, "natural generation"),
            proof: if proof.verified {
                format!(
                    "pinned generation PASS / queue <= {} / restart parity PASS",
                    proof.f7_queue_max
                )
            } else {
                "generation proof unavailable".into()
            },
            diagnostic: format!(
                "live lineage partition.v{} / generation {}",
                live.partition, live.generation
            ),
            output: "restart-stable dispatch generation".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L6",
            title: "Phase Router",
            owner: "nando-operator-runtime".into(),
            input: "runtime relation state + candidate generation".into(),
            live: live_downstream_text(natural_operator_live, "ordinary request routing"),
            proof: if proof.verified {
                format!(
                    "full phase selected {0}/{0}; ablations selected 0; applicability gain {1}",
                    proof.f8_verified_receipts, proof.f8_full_phase_gain,
                )
            } else {
                "phase-control proof unavailable".into()
            },
            diagnostic: format!(
                "search gain {} / F7 no-match p99 {} ns / F8 no-match p99 max {} ns",
                proof.f8_search_gain, proof.f7_no_match_p99_ns, proof.f8_no_match_p99_max_ns,
            ),
            output: "one operator candidate with coherence margin | ABSTAIN".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L7",
            title: "Runtime Role Grounder",
            owner: "nando-operator-runtime".into(),
            input: "selected operator + current structural surface".into(),
            live: live_downstream_text(natural_operator_live, "ordinary role binding"),
            proof: if proof.verified {
                "controlled winner-owned role binding PASS".into()
            } else {
                "role-grounding proof unavailable".into()
            },
            diagnostic: "ambiguous or missing structural role always returns ABSTAIN".into(),
            output: "unique BoundRoleEnvironment | ABSTAIN".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L8",
            title: "Operator VM",
            owner: "nando-operator-runtime".into(),
            input: "bound roles + versioned operator program".into(),
            live: live_downstream_text(natural_operator_live, "ordinary VM execution"),
            proof: if proof.verified {
                "controlled actor execution PASS / execution authority false".into()
            } else {
                "VM proof unavailable".into()
            },
            diagnostic: format!(
                "F7 matched p99 {} ns / F8 matched p99 max {} ns / hard max {} ns",
                proof.f7_matched_p99_ns, proof.f8_matched_p99_max_ns, proof.f8_hard_max_ns,
            ),
            output: "candidate result + actor receipt".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L9",
            title: "Independent Verifier",
            owner: "nando-operator-proof".into(),
            input: "candidate result + immutable expected contract".into(),
            live: live_downstream_text(natural_operator_live, "ordinary candidate verification"),
            proof: if proof.verified {
                format!(
                    "{0}/{0} controlled receipts verified",
                    proof.f8_verified_receipts
                )
            } else {
                "verifier proof unavailable".into()
            },
            diagnostic: format!(
                "parity mismatches {} / false accepts 0",
                live.shadow_parity_mismatches,
            ),
            output: "VerifiedDeltaReceipt | verifier reject".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L10",
            title: "Generation-owned receipt ledger",
            owner: "nando-operator-learning".into(),
            input: "actor receipt + independent verifier receipt".into(),
            live: format!(
                "process {} / submitted {} / evaluated {} / verified {}",
                live.shadow_phase,
                live.shadow_submitted,
                live.shadow_evaluated,
                live.shadow_verified,
            ),
            proof: if proof.verified {
                format!(
                    "{} durable controlled receipts / restart append PASS",
                    proof.f8_verified_receipts
                )
            } else {
                "durable ledger proof unavailable".into()
            },
            diagnostic: format!(
                "hot RSS {} / {} B / precursor max {} B over {} observations / raw payload 0 B",
                proof.f8_hot_rss_bytes,
                proof.f8_rss_target_bytes,
                proof.f8_rss_bytes,
                proof.f8_resource_observations,
            ),
            output: "immutable evidence set for external admission".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L11",
            title: "External Admission",
            owner: "nando-operator-admission".into(),
            input: "generation + capture + ledger + causal-control commitments".into(),
            live: format!(
                "ordinary controller {} / candidate future {}/32 / parity cases {}",
                live.admission_verdict,
                live.admission_future_rows,
                live.admission_runtime_parity_cases,
            ),
            proof: if proof.verified {
                format!(
                    "{} / commitments {} / authority=false",
                    proof.f8_external_verdict,
                    compact(&proof.f8_commitments_sha256),
                )
            } else {
                "external reconstruction proof unavailable".into()
            },
            diagnostic: format!(
                "{} / {} / relation candidates {} / snapshot age {} s",
                live.admission_blocker_stage,
                live.admission_blocker,
                live.admission_relation_candidates,
                live.admission_age_seconds,
            ),
            output: "signed authority lease | SHADOW_READY | reject".into(),
            state: downstream_state,
        },
    ];

    let mut pipeline = String::new();
    for (index, stage) in stages.iter().enumerate() {
        pipeline.push_str(&render_stage(stage));
        if stage.id == "L3" && !natural_operator_live {
            pipeline.push_str(&live_signal_break(live));
        } else if index + 1 < stages.len() {
            let next = stages[index + 1].id;
            pipeline.push_str(&handoff(
                stage.id,
                next,
                if natural_operator_live || index < 2 {
                    "typed live handoff"
                } else {
                    "controlled proof handoff only"
                },
                natural_operator_live || index < 2,
            ));
        }
    }
    pipeline.push_str(&authority_boundary(proof));
    pipeline.push_str(&render_stage(&PipelineStage {
        id: "CPU",
        title: "ACTIVE CPU execution",
        owner: "nando-transition-serving".into(),
        input: "admission-authorized immutable operator generation".into(),
        live: format!("{} ACTIVE response packages", live.active_packages),
        proof: "STOP-F8 controlled candidate has no execution authority".into(),
        diagnostic: "local accept disabled / provider fallback remains active".into(),
        output: "verified local response | OpenAI fallback".into(),
        state: if natural_operator_live {
            RouteState::Live
        } else {
            RouteState::Locked
        },
    }));

    format!(
        r#"<section class="architecture signal-pipeline" data-pipeline-route="single" data-current-stage="{}" data-proof-verified="{}">
<div class="architecture-head">
<div class="architecture-title"><h2>NANDO MACHINE · SIGNAL PIPELINE</h2><p>один вход, один текущий owner на каждом этапе, один fail-closed handoff до CPU</p></div>
<div class="architecture-state"><span class="state-chip live">ORDINARY INPUT</span><span class="state-chip proven">CONTROLLED PROOF</span><span class="state-chip locked">AUTHORITY OFF</span></div>
</div>
<div class="identity-line"><span><b>MODEL</b> {}</span><span><b>DEPLOYED</b> {} · {}</span><span><b>LIVE LINEAGE</b> partition.v{} · generation {}</span><span><b>PROOF</b> {}</span></div>
<div class="pipeline-legend"><span><b>LIVE</b> ordinary traffic crosses the stage</span><span><b>PROOF ONLY</b> controlled evidence proves capability, not live coverage</span><span><b>BLOCK</b> ordinary signal stops</span><span><b>LOCKED</b> no authority</span></div>
<div class="current-blocker"><span class="blocker-label">CURRENT BLOCKER</span><strong>{}</strong><p>{}</p></div>
<div class="pipeline-stack">{}</div>
<div class="terminal-rule">one stage = one owner | ownership moves only through typed receipts | only External Admission may grant authority | missing evidence = ABSTAIN</div>
</section>"#,
        escape(current_stage),
        proof.verified,
        escape(model_label),
        escape(build_id),
        escape(&compact(build_commit)),
        live.partition,
        live.generation,
        escape(&proof_boundary),
        escape(current_blocker),
        escape(&current_reason),
        pipeline,
    )
}

fn live_downstream_text(natural_operator_live: bool, operation: &str) -> String {
    if natural_operator_live {
        format!("{operation} is receiving ordinary traffic")
    } else {
        format!("no {operation}: live signal stopped at L3")
    }
}

fn render_stage(stage: &PipelineStage) -> String {
    format!(
        r#"<article class="pipeline-stage {}" data-stage="{}" data-owner="{}" data-live-flow="{}">
<div class="pipeline-stage-head"><span class="pipeline-index">{}</span><div><h3>{}</h3><p class="stage-owner"><b>OWNER</b> {}</p></div><span class="state-chip {}">{}</span></div>
<dl class="pipeline-diagnostics">
<div class="diagnostic-row input"><dt>IN</dt><dd>{}</dd></div>
<div class="diagnostic-row live"><dt>LIVE</dt><dd>{}</dd></div>
<div class="diagnostic-row proof"><dt>PROOF</dt><dd>{}</dd></div>
<div class="diagnostic-row diagnostic"><dt>DIAG</dt><dd>{}</dd></div>
<div class="diagnostic-row output"><dt>OUT</dt><dd>{}</dd></div>
</dl>
</article>"#,
        stage.state.class(),
        escape(stage.id),
        escape(&stage.owner),
        stage.state.live_flow(),
        escape(stage.id),
        escape(stage.title),
        escape(&stage.owner),
        stage.state.class(),
        stage.state.label(),
        escape(&stage.input),
        escape(&stage.live),
        escape(&stage.proof),
        escape(&stage.diagnostic),
        escape(&stage.output),
    )
}

fn handoff(from: &str, to: &str, label: &str, live: bool) -> String {
    format!(
        r#"<div class="pipeline-handoff {}" data-handoff="{}-{}"><span class="handoff-line"></span><span>{}</span></div>"#,
        if live { "live" } else { "proof" },
        escape(from),
        escape(to),
        escape(label),
    )
}

fn live_signal_break(live: &LiveSignalView<'_>) -> String {
    format!(
        r#"<div class="pipeline-break" data-blocker-stage="L3"><strong>LIVE SIGNAL STOPS HERE</strong><span>Natural OperatorPackage is missing: future {}/32, routed {}, blocker {}.</span><span>Below this edge the blue route is controlled STOP-F8 proof only.</span></div>"#,
        live.future,
        live.routed,
        escape(live.blocker),
    )
}

fn authority_boundary(proof: &ProofSummary) -> String {
    let detail = if proof.verified {
        "SHADOW_READY is a controlled candidate. It cannot authorize itself; natural evidence and a separate authority lease are required."
    } else {
        "Authority remains locked because controlled proof receipts are unavailable."
    };
    format!(
        r#"<div class="authority-boundary" data-authority="false"><strong>AUTHORITY BOUNDARY</strong><span>{}</span></div>"#,
        escape(detail),
    )
}

fn compact(value: &str) -> String {
    match value.get(..12) {
        Some(prefix) if value.len() > 12 => format!("{prefix}..."),
        _ => value.to_owned(),
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(verified: bool) -> ProofSummary {
        ProofSummary {
            verified,
            failure: (!verified).then(|| "tampered receipt".into()),
            f5_commit: "1234567890abcdef".into(),
            f7_receipt_date: "2026-07-22".into(),
            f7_queue_max: 48,
            f7_no_match_p99_ns: 100,
            f7_matched_p99_ns: 200,
            f8_rss_bytes: 10,
            f8_rss_target_bytes: 16,
            f8_resource_observations: 12,
            f8_no_match_p99_max_ns: 195_340,
            f8_provider_records: 4,
            f8_verified_receipts: 3,
            f8_full_phase_gain: 3,
            f8_search_gain: 0,
            f8_external_verdict: "SHADOW_READY".into(),
            f8_commitments_sha256:
                "84e0c8fc735d5c7397757fbbedd40bf8c2f49a17df9d33a754d6c8989fdb7c7f".into(),
            f8_matched_p99_max_ns: 648_010,
            f8_hard_max_ns: 690_741,
            f8_hot_rss_bytes: 10_493_952,
        }
    }

    fn live() -> LiveSignalView<'static> {
        LiveSignalView {
            partition: 16,
            generation: 7,
            transitions: 18_721,
            support: 32,
            matching: 19,
            matching_sessions: 4,
            after_watermark: 16,
            independent: 16,
            consistent: 16,
            routed: 16,
            future: 11,
            blocker: "future_rows_below_32",
            admission_verdict: "BLOCK",
            admission_blocker: "no_candidate",
            admission_blocker_stage: "runtime_parity",
            admission_age_seconds: 5,
            admission_relation_candidates: 1,
            admission_future_rows: 11,
            admission_runtime_parity_cases: 22,
            active_packages: 0,
            online_ready: true,
            capture_phase: "ready_hash_only",
            capture_records: 4,
            capture_captured: 0,
            capture_censored: 0,
            capture_publish_sequence: 13,
            capture_last_error: "",
            shadow_phase: "ready_shadow",
            shadow_submitted: 0,
            shadow_evaluated: 0,
            shadow_verified: 0,
            shadow_parity_mismatches: 0,
        }
    }

    fn manifest() -> Value {
        serde_json::json!({
            "build_id": "build-1",
            "git_commit": "abcdef1234567890"
        })
    }

    #[test]
    fn map_is_one_ordered_pipeline_with_one_owner_per_stage() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");
        let stages = [
            "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "L9", "L10", "L11", "CPU",
        ];
        let mut previous = 0;
        for id in stages {
            let marker = format!("data-stage=\"{id}\"");
            let position = html.find(&marker).expect("stage must exist");
            assert!(position >= previous, "{id} is out of order");
            previous = position;
        }
        assert!(html.contains("data-pipeline-route=\"single\""));
        assert_eq!(html.matches("data-owner=").count(), stages.len());
        assert!(!html.contains("CURRENT LIVE OBSERVER BRANCH"));
        assert!(!html.contains("CONTROLLED F8 PROOF BRANCH"));
    }

    #[test]
    fn ordinary_signal_stops_at_natural_discovery_only() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"L3\""));
        assert!(html.contains(
            "data-stage=\"L1\" data-owner=\"nando-nginx-gateway\" data-live-flow=\"true\""
        ));
        assert!(html.contains(
            "data-stage=\"L2\" data-owner=\"nando-transition-serving\" data-live-flow=\"true\""
        ));
        assert!(html.contains(
            "data-stage=\"L3\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(html.contains("data-blocker-stage=\"L3\""));
        assert_eq!(html.matches("LIVE SIGNAL STOPS HERE").count(), 1);
        assert!(html.contains("controlled STOP-F8 proof only"));
        assert!(html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(html.contains(
            "data-stage=\"CPU\" data-owner=\"nando-transition-serving\" data-live-flow=\"false\""
        ));
    }

    #[test]
    fn controlled_proof_does_not_upgrade_natural_or_authority_status() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("STOP-F8 SHADOW_READY"));
        assert!(html.contains("NOT_EVALUATED by STOP-F8"));
        assert!(html.contains("data-authority=\"false\""));
        assert!(html.contains("controlled candidate has no execution authority"));
        assert!(!html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"true\""
        ));
    }

    #[test]
    fn invalid_proof_receipt_locks_downstream_proof_claims() {
        let html = render(&live(), &proof(false), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"PROOF\""));
        assert!(html.contains("PROOF RECEIPT VALIDATION"));
        assert!(html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(!html.contains("STOP-F8 SHADOW_READY"));
    }

    #[test]
    fn diagnostics_escape_live_blocker_text() {
        let mut live = live();
        live.blocker = "<bad&blocker>";
        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("&lt;bad&amp;blocker&gt;"));
        assert!(!html.contains("<bad&blocker>"));
    }
}
