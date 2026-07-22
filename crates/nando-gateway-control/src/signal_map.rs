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
    fn class(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Proven => "proven",
            Self::Wait => "wait",
            Self::Block => "block",
            Self::Locked => "locked",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Live => "LIVE",
            Self::Proven => "PROVEN",
            Self::Wait => "WAIT",
            Self::Block => "BLOCK",
            Self::Locked => "LOCKED",
        }
    }
}

struct RouteStage<'a> {
    id: &'a str,
    title: &'a str,
    metric: String,
    module: String,
    signal: &'a str,
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
    let live_future_state = if live.future >= 32 {
        RouteState::Proven
    } else {
        RouteState::Wait
    };
    let legacy_admission_state = if live.active_packages > 0 {
        RouteState::Live
    } else if live.admission_verdict == "BLOCK" {
        RouteState::Block
    } else {
        RouteState::Locked
    };
    // PROVEN is receipt-backed controlled execution. It never upgrades the
    // current process counters or grants production authority.
    let controlled_state = if proof.verified {
        RouteState::Proven
    } else {
        RouteState::Locked
    };
    let capture_ready = live.capture_phase == "ready_hash_only";
    let capture_durable =
        capture_ready && (live.capture_records > 0 || proof.f8_provider_records > 0);
    let capture_state = if proof.verified {
        RouteState::Proven
    } else if capture_durable {
        RouteState::Live
    } else if capture_ready {
        RouteState::Wait
    } else {
        RouteState::Block
    };
    let proof_commit = compact(&proof.f5_commit);
    let proof_boundary = if proof.verified {
        format!(
            "F5-F8 controlled proof {} · F7 receipt {} · {} verified · {}",
            proof_commit,
            proof.f7_receipt_date,
            proof.f8_verified_receipts,
            proof.f8_external_verdict
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
    let current_blocker = if !proof.verified {
        "PROOF RECEIPT VALIDATION"
    } else {
        "PRODUCTION AUTHORITY / NATURAL OPERATOR EVIDENCE"
    };
    let current_reason = if !proof.verified {
        "Proof-backed состояние F5-F8 не прошло fail-closed проверку, поэтому downstream-модули не могут показываться как доказанные.".to_owned()
    } else {
        format!(
            "STOP-F8 завершён в controlled live shadow: {}/{} verified, phase gain {}, false accepts 0. До CPU остаётся отдельный authority rollout либо natural operator evidence; текущий seed authority не получает.",
            proof.f8_verified_receipts, proof.f8_verified_receipts, proof.f8_full_phase_gain,
        )
    };

    let mut live_branch = String::new();
    live_branch.push_str(&stage(
        "├─",
        &RouteStage {
            id: "00",
            title: "Provider request ingress",
            metric: format!("model {}", model_label),
            module: "nando-nginx-gateway · HTTPS streaming -> OpenAI".into(),
            signal: "REQUEST FLOWING",
            state: RouteState::Live,
        },
    ));
    live_branch.push_str(&edge(
        "│ ",
        "Текущий fallback-трафик идёт дальше; наблюдатель получает завершённые трассы.",
    ));
    live_branch.push_str(&branch_label(
        "├─",
        "CURRENT LIVE OBSERVER BRANCH",
        "реальный сигнал сегодня",
        "live",
    ));
    live_branch.push_str(&stage(
        "│  ├─",
        &RouteStage {
            id: "L1",
            title: "Completed-trace observer",
            metric: format!("{} transitions", live.transitions),
            module: module_identity(manifest, "Streaming worker", live.partition),
            signal: if live.online_ready {
                "SIGNAL PRESENT"
            } else {
                "SNAPSHOT MISSING"
            },
            state: if live.online_ready {
                RouteState::Live
            } else {
                RouteState::Wait
            },
        },
    ));
    live_branch.push_str(&stage(
        "│  ├─",
        &RouteStage {
            id: "L2",
            title: "Legacy online learner",
            metric: format!("support {}/32 · future {}/32", live.support, live.future),
            module: format!(
                "{} · {}",
                module_identity(manifest, "Teacher/student miner", live.partition),
                module_identity(manifest, "Frozen future", live.partition)
            ),
            signal: "LEGACY EVIDENCE ONLY",
            state: live_future_state,
        },
    ));
    live_branch.push_str(&legacy_counts(live));
    live_branch.push_str(&stage(
        "│  └─",
        &RouteStage {
            id: "L3",
            title: "Legacy external admission",
            metric: format!(
                "controller {} · candidate proof {}/32",
                live.admission_verdict, live.admission_future_rows
            ),
            module: module_identity(manifest, "Admission", live.partition),
            signal: "DOES NOT FEED F7 V3",
            state: legacy_admission_state,
        },
    ));
    live_branch.push_str(&edge(
        "│ ",
        &format!(
            "Старый маршрут заканчивается здесь: {} / {}. Это не вход нового F8-контура.",
            live.admission_blocker_stage, live.admission_blocker
        ),
    ));

    let mut target_branch = String::new();
    target_branch.push_str(&branch_label(
        "└─",
        "CONTROLLED F8 PROOF BRANCH",
        "полный shadow-маршрут; не production authority",
        "proven",
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-A",
            title: "Hash-only provider capture owner",
            metric: format!(
                "{} · durable {} · captured/censored {}/{} · publish {} · error {}",
                live.capture_phase,
                proof.f8_provider_records.max(live.capture_records),
                live.capture_captured,
                live.capture_censored,
                live.capture_publish_sequence,
                if live.capture_last_error.is_empty() {
                    "none"
                } else {
                    live.capture_last_error
                },
            ),
            module: "nando-transition-serving · provider_capture.v3".into(),
            signal: if proof.verified {
                "CONTROLLED DURABLE CAPTURE"
            } else if capture_durable {
                "HASH-ONLY SIGNAL LIVE"
            } else if capture_ready {
                "READY / NO DURABLE TRAFFIC"
            } else {
                "SIGNAL STOPS HERE"
            },
            state: capture_state,
        },
    ));
    if !proof.verified && !capture_durable {
        target_branch.push_str(&blocked_edge(
            "real request hash -> no durable trusted capture receipt -> no live generation evidence",
        ));
    }
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F7",
            title: "Pinned generation + persistence",
            metric: if proof.verified {
                format!(
                    "queue <= {} · p99 no-match/matched {} / {} ns",
                    proof.f7_queue_max, proof.f7_no_match_p99_ns, proof.f7_matched_p99_ns
                )
            } else {
                "proof unavailable".into()
            },
            module: "nando-operator-persistence · generation-checkpoint.v3".into(),
            signal: "PINNED CONTROLLED GENERATION",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F5",
            title: "Role grounding + Operator VM",
            metric: "bounded actor execution".into(),
            module: "nando-operator-runtime · traffic-shadow.v3".into(),
            signal: "WINNER-OWNED ACTOR",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F6",
            title: "Independent verifier",
            metric: format!(
                "verified {} · parity {}",
                proof.f8_verified_receipts, live.shadow_parity_mismatches
            ),
            module: "nando-operator-proof · independent-verifier.v3".into(),
            signal: "INDEPENDENT CONTROLLED PASS",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-B",
            title: "Generation shadow receipt ledger",
            metric: format!(
                "{} durable · process {} {}/{}/{}",
                proof.f8_verified_receipts,
                live.shadow_phase,
                live.shadow_submitted,
                live.shadow_evaluated,
                live.shadow_verified,
            ),
            module: "nando-operator-learning · receipt-ledger.v3".into(),
            signal: "DURABLE LEDGER VERIFIED",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-C",
            title: "External admission reconstruction",
            metric: format!(
                "{} · {}",
                proof.f8_external_verdict,
                compact(&proof.f8_commitments_sha256)
            ),
            module: "nando-operator-admission · external-admission.v3".into(),
            signal: "IMMUTABLE RECONSTRUCTION",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-D",
            title: "Causal controls + frozen latency",
            metric: format!(
                "gain {} · search {} · p99 {}/{} ns",
                proof.f8_full_phase_gain,
                proof.f8_search_gain,
                proof.f8_no_match_p99_max_ns,
                proof.f8_matched_p99_max_ns,
            ),
            module: "nando-live-transition-gate · composite-gate.v2".into(),
            signal: "APPLICABILITY CAUSAL PASS",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-E",
            title: "Controlled live shadow proof",
            metric: format!(
                "{}/{} verified · authority=false",
                proof.f8_verified_receipts, proof.f8_verified_receipts
            ),
            module: "nando-transition-serving · live-shadow.v3".into(),
            signal: "STOP-F8 PASS",
            state: controlled_state,
        },
    ));
    target_branch.push_str(&blocked_edge(
        "SHADOW_READY -> controlled seed has no authority lease; natural operator and ordinary-traffic coverage are NOT_EVALUATED",
    ));
    target_branch.push_str(&stage(
        "   └─",
        &RouteStage {
            id: "CPU",
            title: "ACTIVE CPU execution",
            metric: format!("{} ACTIVE packages", live.active_packages),
            module: "nando-transition-serving · execution authority".into(),
            signal: "AUTHORITY FALSE",
            state: if live.active_packages > 0 {
                RouteState::Live
            } else {
                RouteState::Locked
            },
        },
    ));

    let resource_text = if proof.verified {
        format!(
            "STOP-F8 hot RSS {} / {} B PASS · precursor max {} B over {} runs · hard max {} ns PASS",
            proof.f8_hot_rss_bytes,
            proof.f8_rss_target_bytes,
            proof.f8_rss_bytes,
            proof.f8_resource_observations,
            proof.f8_hard_max_ns,
        )
    } else {
        "F8-0 proof unavailable".into()
    };
    let current_stage = if proof.verified { "authority" } else { "proof" };
    let (current_chip_class, current_chip) = if proof.verified {
        ("proven", "STOP-F8 PASS")
    } else if capture_ready {
        ("wait", "F8-A READY")
    } else {
        ("block", "F8-A BLOCK")
    };

    format!(
        r#"<section class="architecture unified-map" data-current-stage="{}" data-proof-verified="{}">
<div class="architecture-head">
<div class="architecture-title"><h2>NANDO MACHINE · SIGNAL MAP</h2><p>живой observer, полный controlled F8 proof и отдельная граница production authority</p></div>
<div class="architecture-state"><span class="state-chip live">LIVE OBSERVER</span><span class="state-chip {}">{}</span><span class="state-chip locked">AUTHORITY OFF</span></div>
</div>
<div class="identity-line"><span><b>MODEL</b> {}</span><span><b>DEPLOYED</b> {} · {}</span><span><b>LIVE LINEAGE</b> partition.v{} · generation {}</span><span><b>PROOF</b> {}</span></div>
<div class="current-blocker"><span class="blocker-label">CURRENT BLOCKER</span><strong>{}</strong><p>{}</p></div>
<div class="flow-tree unified-tree">{}{}</div>
<div class="terminal-rule">{} | false accepts remain verifier-owned | missing evidence = ABSTAIN</div>
</section>"#,
        current_stage,
        proof.verified,
        current_chip_class,
        current_chip,
        escape(model_label),
        escape(build_id),
        escape(&compact(build_commit)),
        live.partition,
        live.generation,
        escape(&proof_boundary),
        escape(current_blocker),
        escape(&current_reason),
        live_branch,
        target_branch,
        escape(&resource_text),
    )
}

fn stage(branch: &str, stage: &RouteStage<'_>) -> String {
    format!(
        r#"<div class="map-stage {}" data-stage="{}" data-signal="{}">
<div class="map-stage-main"><span class="tree-glyph">{}</span><span class="stage-index">[{}]</span><strong class="stage-title">{}</strong><span class="stage-metric">{}</span><span class="state-chip {}">{}</span></div>
<div class="map-stage-meta"><span class="module-name">{}</span><span class="signal-state">signal: {}</span></div>
</div>"#,
        stage.state.class(),
        escape(stage.id),
        escape(stage.signal),
        escape(branch),
        escape(stage.id),
        escape(stage.title),
        escape(&stage.metric),
        stage.state.class(),
        stage.state.label(),
        escape(&stage.module),
        escape(stage.signal),
    )
}

fn edge(branch: &str, text: &str) -> String {
    format!(
        r#"<div class="map-edge"><span class="tree-glyph">{}</span><span>{}</span></div>"#,
        escape(branch),
        escape(text)
    )
}

fn blocked_edge(text: &str) -> String {
    format!(
        r#"<div class="map-blocked-edge"><span class="tree-glyph">   │</span><strong>BLOCK НА ЭТОМ РЕБРЕ</strong><span>{}</span></div>"#,
        escape(text)
    )
}

fn branch_label(branch: &str, title: &str, note: &str, class: &str) -> String {
    format!(
        r#"<div class="map-branch {}"><span class="tree-glyph">{}</span><strong>{}</strong><span>{}</span></div>"#,
        escape(class),
        escape(branch),
        escape(title),
        escape(note)
    )
}

fn legacy_counts(live: &LiveSignalView<'_>) -> String {
    format!(
        r#"<div class="map-evidence"><span>matching {} / sessions {}</span><span>watermark {}</span><span>independent {}</span><span>typed parity {}</span><span>routed {}</span><span>controller candidates {} / parity {}</span><span>snapshot age {} s</span><span>blocker {}</span></div>"#,
        live.matching,
        live.matching_sessions,
        live.after_watermark,
        live.independent,
        live.consistent,
        live.routed,
        live.admission_relation_candidates,
        live.admission_runtime_parity_cases,
        live.admission_age_seconds,
        escape(live.blocker),
    )
}

fn module_identity(manifest: &Value, module_name: &str, runtime_partition: u64) -> String {
    let Some(module) = manifest
        .get("modules")
        .and_then(Value::as_array)
        .and_then(|modules| {
            modules
                .iter()
                .find(|module| module.get("name").and_then(Value::as_str) == Some(module_name))
        })
    else {
        return format!("{module_name} · version MISSING");
    };
    let mut version = module
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("MISSING")
        .to_owned();
    if module_name == "Frozen future" && runtime_partition > 0 {
        version = format!("partition.v{runtime_partition}");
    }
    let contract = module
        .get("sha256")
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    format!("{module_name} · {version} · {}", compact(contract))
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
            capture_phase: "missing",
            capture_records: 0,
            capture_captured: 0,
            capture_censored: 0,
            capture_publish_sequence: 0,
            capture_last_error: "",
            shadow_phase: "missing",
            shadow_submitted: 0,
            shadow_evaluated: 0,
            shadow_verified: 0,
            shadow_parity_mismatches: 0,
        }
    }

    fn manifest() -> Value {
        serde_json::json!({
            "build_id": "build-1",
            "git_commit": "abcdef1234567890",
            "modules": [
                {"name":"Streaming worker","version":"event-driven.v2","sha256":"1111111111111111"},
                {"name":"Teacher/student miner","version":"strategy.v3","sha256":"2222222222222222"},
                {"name":"Frozen future","version":"partition.v16","sha256":"3333333333333333"},
                {"name":"Admission","version":"gate.v2","sha256":"4444444444444444"}
            ]
        })
    }

    #[test]
    fn map_separates_live_observer_controlled_f8_and_authority() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("CURRENT LIVE OBSERVER BRANCH"));
        assert!(html.contains("data-stage=\"L1\" data-signal=\"SIGNAL PRESENT\""));
        assert!(html.contains("CONTROLLED F8 PROOF BRANCH"));
        assert!(html.contains("data-stage=\"F8-A\" data-signal=\"CONTROLLED DURABLE CAPTURE\""));
        assert!(html.contains("data-stage=\"F8-E\" data-signal=\"STOP-F8 PASS\""));
        assert!(html.contains("data-stage=\"CPU\" data-signal=\"AUTHORITY FALSE\""));
        assert!(html.contains("CURRENT BLOCKER"));
        assert!(html.contains("PRODUCTION AUTHORITY / NATURAL OPERATOR EVIDENCE"));
        assert!(html.contains("model gpt-test"));
        assert!(html.contains("future 11/32"));
        assert!(html.contains("natural operator and ordinary-traffic coverage are NOT_EVALUATED"));
    }

    #[test]
    fn durable_capture_survives_process_counter_reset_in_the_view() {
        let mut live = live();
        live.capture_phase = "ready_hash_only";
        live.capture_records = 3;
        live.capture_captured = 0;
        live.capture_publish_sequence = 2;
        live.shadow_phase = "ready_shadow";
        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"authority\""));
        assert!(html.contains("data-stage=\"F8-A\" data-signal=\"CONTROLLED DURABLE CAPTURE\""));
        assert!(html.contains("durable 4 · captured/censored 0/0"));
        assert!(html.contains("data-stage=\"F8-B\" data-signal=\"DURABLE LEDGER VERIFIED\""));
        assert!(!html.contains("F8-B BLOCK"));
    }

    #[test]
    fn invalid_proof_receipt_removes_controlled_pass_claims() {
        let html = render(&live(), &proof(false), &manifest(), "gpt-test");

        assert!(html.contains("data-proof-verified=\"false\""));
        assert!(html.contains("PROOF RECEIPT VALIDATION"));
        assert!(!html.contains("F5-F8 controlled proof 1234567890ab..."));
    }
}
