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
    // PROVEN is receipt-backed controlled execution, never evidence that live
    // provider traffic crossed the missing F8-A capture boundary.
    let controlled_state = if proof.verified {
        RouteState::Proven
    } else {
        RouteState::Locked
    };
    let capture_ready = live.capture_phase == "ready_hash_only";
    let capture_live = capture_ready && live.capture_captured > 0;
    let capture_state = if capture_live {
        RouteState::Live
    } else if capture_ready {
        RouteState::Wait
    } else {
        RouteState::Block
    };
    let shadow_live = live.shadow_phase == "ready_shadow" && live.shadow_submitted > 0;
    let evaluated_live = shadow_live && live.shadow_evaluated > 0;
    let proof_commit = compact(&proof.f5_commit);
    let proof_boundary = if proof.verified {
        format!(
            "F5/F6/F7 controlled proof {} · F7 receipt {}",
            proof_commit, proof.f7_receipt_date
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
    } else if capture_live {
        "F8-B DURABLE GENERATION SHADOW LEDGER"
    } else {
        "F8-A LIVE PROVIDER CAPTURE OWNER"
    };
    let current_reason = if !proof.verified {
        "Proof-backed состояние F5-F8 не прошло fail-closed проверку, поэтому downstream-модули не могут показываться как доказанные.".to_owned()
    } else if capture_live {
        format!(
            "F8-A durable capture работает: {} записей. Следующая граница — сохранить generation-owned F5/F6 receipt; текущая evaluation пока только telemetry.",
            live.capture_records
        )
    } else {
        format!(
            "Hash-only capture ещё не доказан живым трафиком: phase {}, captured {}, censored {}, error {}.",
            live.capture_phase,
            live.capture_captured,
            live.capture_censored,
            if live.capture_last_error.is_empty() {
                "none"
            } else {
                live.capture_last_error
            }
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
        "TARGET F8 OPERATOR BRANCH",
        "целевая Nando Machine",
        "proven",
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-A",
            title: "Live provider capture owner",
            metric: format!(
                "captured {} · records {} · publish {}",
                live.capture_captured, live.capture_records, live.capture_publish_sequence
            ),
            module: "nando-transition-serving · provider_capture.v3".into(),
            signal: if capture_live {
                "HASH-ONLY SIGNAL LIVE"
            } else if capture_ready {
                "READY / NO DURABLE TRAFFIC"
            } else {
                "SIGNAL STOPS HERE"
            },
            state: capture_state,
        },
    ));
    if !capture_live {
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
            signal: if shadow_live {
                "PINNED LIVE INPUT"
            } else {
                "CONTROLLED PROOF ONLY"
            },
            state: if shadow_live {
                RouteState::Live
            } else {
                controlled_state
            },
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F5",
            title: "Role grounding + Operator VM",
            metric: "bounded actor execution".into(),
            module: "nando-operator-runtime · traffic-shadow.v3".into(),
            signal: if evaluated_live {
                "LIVE ACTOR EVALUATED"
            } else {
                "CONTROLLED PROOF ONLY"
            },
            state: if evaluated_live {
                RouteState::Live
            } else {
                controlled_state
            },
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F6",
            title: "Independent verifier",
            metric: if evaluated_live {
                format!(
                    "evaluated {} · verified {} · parity {}",
                    live.shadow_evaluated, live.shadow_verified, live.shadow_parity_mismatches
                )
            } else if proof.verified {
                "independent controlled receipt path PASS".into()
            } else {
                "proof unavailable".into()
            },
            module: "nando-operator-proof · independent-verifier.v3".into(),
            signal: if evaluated_live {
                "LIVE VERIFIER RECEIPT"
            } else {
                "CONTROLLED PROOF ONLY"
            },
            state: if evaluated_live {
                RouteState::Live
            } else {
                controlled_state
            },
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-B",
            title: "Generation shadow receipt ledger",
            metric: format!(
                "in-memory evaluated {} · durable receipts 0",
                live.shadow_evaluated
            ),
            module: "nando-operator-learning · receipt-ledger.v3".into(),
            signal: if capture_live {
                "NEXT BLOCK"
            } else {
                "LOCKED BY F8-A"
            },
            state: if capture_live {
                RouteState::Block
            } else {
                RouteState::Locked
            },
        },
    ));
    if capture_live {
        target_branch.push_str(&blocked_edge(
            "capture receipt -> F5/F6 telemetry exists, but GenerationShadowReceiptV3 is not durable yet",
        ));
    }
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-C",
            title: "External admission reconstruction",
            metric: "immutable bytes -> candidate".into(),
            module: "nando-operator-admission · external-admission.v3".into(),
            signal: "NOT STARTED",
            state: RouteState::Locked,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-D",
            title: "Causal controls + frozen latency",
            metric: if proof.verified {
                format!(
                    "no-match max {} ns · protocol not frozen",
                    proof.f8_no_match_p99_max_ns
                )
            } else {
                "proof unavailable".into()
            },
            module: "nando-live-transition-gate · composite-gate.v2".into(),
            signal: "WAITING FOR F8-A..C",
            state: RouteState::Wait,
        },
    ));
    target_branch.push_str(&stage(
        "   ├─",
        &RouteStage {
            id: "F8-E",
            title: "Live shadow proof",
            metric: "real traffic · authority=false".into(),
            module: "nando-transition-serving · live-shadow.v3".into(),
            signal: "NOT STARTED",
            state: RouteState::Locked,
        },
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
            "F8-0 RSS {} / {} B PASS ({} runs) · latency WATCH",
            proof.f8_rss_bytes, proof.f8_rss_target_bytes, proof.f8_resource_observations
        )
    } else {
        "F8-0 proof unavailable".into()
    };
    let current_stage = if capture_live { "f8-b" } else { "f8-a" };
    let (current_chip_class, current_chip) = if capture_live {
        ("block", "F8-B BLOCK")
    } else if capture_ready {
        ("wait", "F8-A READY")
    } else {
        ("block", "F8-A BLOCK")
    };

    format!(
        r#"<section class="architecture unified-map" data-current-stage="{}" data-proof-verified="{}">
<div class="architecture-head">
<div class="architecture-title"><h2>NANDO MACHINE · SIGNAL MAP</h2><p>одна карта: где живой сигнал идёт сейчас и где начинается доказанный, но ещё не подключённый F8-контур</p></div>
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
            f8_no_match_p99_max_ns: 284_901,
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
    fn map_places_the_live_signal_and_f8_block_on_different_routes() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("CURRENT LIVE OBSERVER BRANCH"));
        assert!(html.contains("data-stage=\"L1\" data-signal=\"SIGNAL PRESENT\""));
        assert!(html.contains("data-stage=\"F8-A\" data-signal=\"SIGNAL STOPS HERE\""));
        assert!(html.contains("data-stage=\"F7\" data-signal=\"CONTROLLED PROOF ONLY\""));
        assert!(html.contains("CURRENT BLOCKER"));
        assert!(html.contains("model gpt-test"));
        assert!(html.contains("future 11/32"));
        assert!(!html.contains("production end-to-end confirmed"));
    }

    #[test]
    fn durable_capture_moves_the_visible_blocker_to_f8_b() {
        let mut live = live();
        live.capture_phase = "ready_hash_only";
        live.capture_records = 3;
        live.capture_captured = 3;
        live.capture_publish_sequence = 2;
        live.shadow_phase = "ready_shadow";
        live.shadow_submitted = 3;
        live.shadow_evaluated = 2;
        live.shadow_verified = 1;
        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"f8-b\""));
        assert!(html.contains("data-stage=\"F8-A\" data-signal=\"HASH-ONLY SIGNAL LIVE\""));
        assert!(html.contains("data-stage=\"F8-B\" data-signal=\"NEXT BLOCK\""));
        assert!(html.contains("F8-B DURABLE GENERATION SHADOW LEDGER"));
        assert!(html.contains("durable receipts 0"));
    }

    #[test]
    fn invalid_proof_receipt_removes_controlled_pass_claims() {
        let html = render(&live(), &proof(false), &manifest(), "gpt-test");

        assert!(html.contains("data-proof-verified=\"false\""));
        assert!(html.contains("PROOF RECEIPT VALIDATION"));
        assert!(!html.contains("F5/F6/F7 controlled proof 1234567890ab..."));
    }
}
