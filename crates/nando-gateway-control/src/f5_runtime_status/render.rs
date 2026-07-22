use super::f8_resource_receipt::F8ResourceStatus;
use super::receipt::PipelineStatus;

pub(super) fn verified_panel(status: &PipelineStatus, resource: &F8ResourceStatus) -> String {
    let commit = status
        .f5_implementation_commit
        .get(..12)
        .unwrap_or(&status.f5_implementation_commit);
    let mut stages = String::new();
    stages.push_str(&stage(
        "f5-a",
        "5A",
        "Executable operator artifact",
        "OperatorArtifactV1 owns the immutable effect law and compiled program.",
        "artifact",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("typed artifact + proof roots"));
    stages.push_str(&stage(
        "f5-b",
        "5B",
        "Canonical runtime context",
        "Normalizes the incoming surface without granting execution authority.",
        "context",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("canonical structural surface"));
    stages.push_str(&stage(
        "f5-c",
        "5C",
        "Structural dispatch + role binding",
        "Selects bounded structural candidates and resolves runtime roles fail-closed.",
        "binding",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("unique role environment"));
    stages.push_str(&stage(
        "f5-d",
        "5D",
        "Capability and action grounding",
        "Binds the semantic mode to an advertised physical capability and action.",
        "grounding",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("winner-owned physical action"));
    stages.push_str(&stage(
        "f5-e",
        "5E",
        "Actor + Operator VM shadow",
        "Executes the compiled mode in shadow and proves actor/VM parity.",
        "VM shadow",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("phase-ranked bounded candidates"));
    stages.push_str(&stage(
        "f5-f",
        "5F",
        "Phase integration",
        "Phase ranking preserves safety; this corpus did not yet prove search reduction.",
        &status.phase_search_gain,
        "WATCH",
        "wait",
    ));
    stages.push_str(&edge("traffic projection + pinned generations"));
    stages.push_str(&stage(
        "f5-g",
        "5G",
        "Incoming traffic shadow",
        "Accounts the ordinary window and enforces the hard traffic ceiling without local accepts.",
        &format!(
            "{} / {} ordinary",
            status.accounted_rows, status.ordinary_rows
        ),
        "WATCH",
        "wait",
    ));
    stages.push_str(&facts(status));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f5-to-f6"><span class="tree-glyph">│</span><strong>FULL CONTROLLED F5 SIGNAL TO F6 INPUT CONFIRMED</strong><span>not a production end-to-end claim</span></div>"#,
    );
    stages.push_str(&stage(
        "f6",
        "F6",
        "Independent verifier",
        "Independently rebuilds the request scene, roles, capability, action and expected output.",
        &format!(
            "{} adversarial · p99 {} ns",
            status.f6_integration_pass, status.f6_matched_p99_ns
        ),
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("opaque verifier receipt · authority=false"));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f6-to-f7"><span class="tree-glyph">│</span><strong>FULL CONTROLLED F5 TO F6 PROOF PATH CONFIRMED</strong><span>generation authority remains false</span></div>"#,
    );
    stages.push_str(&stage(
        "f7-a",
        "7A",
        "Generation identity + restart bundle",
        "One canonical generation ID binds immutable artifacts and reconstructed dispatch.",
        "canonical restart",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("generation-owned partitions"));
    stages.push_str(&stage(
        "f7-b",
        "7B",
        "Support / frozen-future ledger",
        "Separates support, future, censored outcomes and the post-freeze watermark.",
        "cross-partition reuse blocked",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("generation + lineage + request root"));
    stages.push_str(&stage(
        "f7-c",
        "7C",
        "Generation-bound verifier receipt",
        "Binds the independent F6 verdict to one generation and exact capture identity.",
        "tamper / relabel blocked",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("self-validating checkpoint"));
    stages.push_str(&stage(
        "f7-d",
        "7D",
        "Atomic persistence + recovery",
        "Publishes alternating generation slots and restores only monotonic byte-identical state.",
        "fsync + rename + recovery",
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("exact provider-request capture join"));
    stages.push_str(&stage(
        "f7-e",
        "7E",
        "Controlled generation shadow",
        "Loads after HTTP bind, pins the generation before enqueue, runs F5 and independently verifies through F6.",
        &format!(
            "queue <= {} · p99 {} ns",
            status.f7_queue_max, status.f7_matched_p99_ns
        ),
        "PASS",
        "pass",
    ));
    stages.push_str(&f7_facts(status));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f7-to-f8"><span class="tree-glyph">│</span><strong>FULL CONTROLLED F5 TO F7 PROOF PATH CONFIRMED</strong><span>live producer, admission and authority are not claimed</span></div>"#,
    );
    stages.push_str(&stage(
        "f8-0",
        "8-0",
        "Production allocator resource truth",
        "Separates compiler retention from the retained hot registry under the already deployed allocator policy.",
        &format!(
            "production-policy RSS {} / target {} B · {} runs",
            resource.max_peak_rss_delta_bytes,
            resource.rss_target_bytes,
            resource.resource_observations,
        ),
        "PASS",
        "pass",
    ));
    stages.push_str(&edge("latency WATCH · capture owner absent"));
    stages.push_str(&stage(
        "f8-a",
        "8-A",
        "Live provider capture owner",
        "Must emit bounded hash-only request provenance before external admission can consume real traffic.",
        "authority=false · ACTIVE=0",
        "READY",
        "wait",
    ));

    format!(
        r#"<section class="architecture research-architecture" data-research-status="f8-0-pass-f8-a-ready">
<div class="architecture-head">
<div class="architecture-title"><h2>R&amp;D OPERATOR PIPELINE</h2><p>artifact -&gt; grounding -&gt; VM -&gt; verifier -&gt; generation -&gt; F8 admission</p></div>
<div class="architecture-state"><span class="state-chip pass">F5 COMPLETE</span><span class="state-chip pass">F6 COMPLETE</span><span class="state-chip pass">F7 COMPLETE</span><span class="state-chip pass">F8-0 PASS</span><span class="state-chip wait">F8-A READY</span><span class="architecture-meta">F5 {} · F7 receipt {}</span></div>
</div>
<div class="flow-tree">{}</div>
<div class="terminal-rule">controlled F5 -&gt; F7 proof confirmed | F8-0 resource PASS | F8-D latency WATCH ({}) | live capture producer missing | authority false</div>
</section>"#,
        escape(commit),
        escape(&status.f7_receipt_date),
        stages,
        resource.no_match_p99_max_ns,
    )
}

fn facts(status: &PipelineStatus) -> String {
    format!(
        r#"<div class="research-facts">
<span>projection {}/{}</span><span>organic replay {}</span><span>F5 no-match p99 {} / target {} ns</span><span>F5 matched p99 {} / target {} ns</span><span>F5 hard ceiling {} ns PASS</span><span>RSS {} / target {} B</span><span>F6 no-match p99 {} ns</span><span>F6 matched p99 {} ns</span><span>F6 max {} ns</span>
</div>"#,
        status.projection_controls_passed,
        status.projection_controls_total,
        escape(&status.organic_runtime_replay),
        status.no_match_p99_ns,
        status.no_match_target_ns,
        status.matched_shadow_p99_ns,
        status.matched_target_ns,
        status.hard_ceiling_ns,
        status.rss_delta_bytes,
        status.rss_target_bytes,
        status.f6_no_match_p99_ns,
        status.f6_matched_p99_ns,
        status.f6_hard_max_ns,
    )
}

fn f7_facts(status: &PipelineStatus) -> String {
    format!(
        r#"<div class="research-facts">
<span>capture join exact</span><span>request generation pinned</span><span>raw persisted 0 B</span><span>local accepts 0</span><span>F7 no-match p99 {} ns</span><span>F7 matched p99 {} ns</span><span>F7 max {} ns</span><span>F5 conservative RSS {} / target {} B WATCH</span><span>live capture producer missing</span>
</div>"#,
        status.f7_no_match_p99_ns,
        status.f7_matched_p99_ns,
        status.f7_hard_max_ns,
        status.f7_rss_delta_bytes,
        status.f7_rss_target_bytes,
    )
}

fn stage(
    id: &str,
    step: &str,
    title: &str,
    logic: &str,
    metric: &str,
    label: &str,
    class: &str,
) -> String {
    let branch = if id == "f8-a" { "└─" } else { "├─" };
    format!(
        r#"<div class="terminal-stage {}" data-rd-stage="{}" title="{}">
<div class="terminal-line"><span class="tree-glyph">{}</span><span class="stage-index">[{}]</span><strong class="stage-title">{}</strong><span class="stage-metric">{}</span><span class="state-chip {}">{}</span></div>
</div>"#,
        escape(class),
        escape(id),
        escape(logic),
        branch,
        escape(step),
        escape(title),
        escape(metric),
        escape(class),
        escape(label),
    )
}

fn edge(label: &str) -> String {
    format!(
        "<div class=\"terminal-edge\"><span class=\"tree-glyph\">│</span>{}</div>",
        escape(label)
    )
}

pub(super) fn unavailable_panel(error: &str) -> String {
    format!(
        r#"<section class="architecture research-architecture" data-research-status="unavailable">
<div class="architecture-head">
<div class="architecture-title"><h2>R&amp;D OPERATOR PIPELINE</h2><p>receipt-backed status unavailable</p></div>
<div class="architecture-state"><span class="state-chip block">R&amp;D STATUS UNAVAILABLE</span><span class="state-chip locked">F8 LOCKED</span></div>
</div>
<div class="flow-tree"><div class="terminal-line terminal-failure"><span class="tree-glyph">└─</span><strong>FAIL-CLOSED</strong><span>{}</span></div></div>
<div class="terminal-rule">no receipt = no PASS claim | authority remains false</div>
</section>"#,
        escape(error)
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
