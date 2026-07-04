pub(crate) fn print_help() {
    println!("nando-cli");
    println!();
    println!("Commands:");
    println!("  status    Print the current Nando Wave workspace status");
    println!("  organ128-plan");
    println!("            Print the T480 cache-aware Organ128 cell plan");
    println!("  organ128-train-generate [seed] [epochs] [prompt] [generate-len]");
    println!("            Train the first Organ128 byte readout and generate text");
    println!("  organ128-dialog-generate [seed] [prompt]");
    println!("            Answer from the bilingual prompt-wave dialog corpus");
    println!("  organ128-settle-dialog [seed] [prompt] [ticks]");
    println!("            Answer after Organ128 CarrierWave/WaveBus settle ticks with controls");
    println!("  organ128-wave-scorer-eval [seed] [epochs] [ticks]");
    println!("            Train/eval a tiny wave-state scorer over the dialog corpus");
    println!("  organ128-response-gate-eval [seed] [ticks]");
    println!("            Check answer/refuse gating on known prompts and noise prompts");
    println!("  organ128-thought-probe-eval [seed] [ticks] [epochs]");
    println!("            Train/eval a tiny dynamic-state probe over ThoughtState only");
    println!("  organ128-modadd-eval [seed] [modulus] [train-cases] [holdout-cases]");
    println!("            Run the GOAL v0 modular-addition evidence-package probe");
    println!("  organ128-modadd-seed-sweep [modulus] [train-cases] [holdout-cases]");
    println!("            Sweep the GOAL v0 modular-addition probe over fixed seeds");
    println!("  wave-tick <input-byte> [seed]");
    println!("            Run one deterministic Stage 2 wave tick");
    println!("  snapshot-save <input-byte> [seed] [path]");
    println!("            Save a Stage 2 .nws1 snapshot");
    println!("  snapshot-read <path>");
    println!("            Read a Stage 2 .nws1 snapshot");
    println!("  bench-stage2-tick [seed] [ticks]");
    println!("            Measure seed tick vs precomputed Stage2Organ tick speed");
    println!("  bench-link-tissue [seed] [ticks]");
    println!("            Measure Cell32 score vs pair/triple LinkTissue score loops");
    println!("  bench-symbol-l3 [seed] [ticks]");
    println!("            Measure SymbolL3 256/512/768/1024-cell tick speed");
    println!("  bench-wave-layers");
    println!("            Measure L1/L2/L3 layered Wave speed, size, heldout, and ablation");
    println!("  phase-package-v4 [corpus-jsonl] [package-path] [cells] [manifest-path]");
    println!(
        "            Build, save, load, inspect, bench, and manifest the v4 phase-center package"
    );
    println!("  phase-package-inspect [package-path] [manifest-path]");
    println!("            Inspect an existing v4 phase-center package and verify its manifest");
    println!(
        "  phase-package-score-v4 [package-path] [manifest-path] [corpus-jsonl] [score-report-json]"
    );
    println!(
        "            Score v4 JSONL through an existing package and manifest without compiling"
    );
    println!("  phase-eval-pack-v4 [package-path] [manifest-path] [corpus-jsonl] [eval-pack-path]");
    println!("            Precompile v4 heldout/action-ablation eval vectors into a binary pack");
    println!(
        "  phase-package-score-pack-v4 [package-path] [manifest-path] [eval-pack-path] [score-report-json]"
    );
    println!(
        "            Score an existing package through a binary eval pack without JSONL rebuild"
    );
    println!("  phase-package-verify [package-path] [manifest-path] [score-report-json]");
    println!(
        "            Verify package, manifest, and score report as one product proof artifact"
    );
    println!("  phase-action-boundary-v4 [corpus-jsonl]");
    println!(
        "            Audit whether v4 action text is safe to claim as a raw action-router gate"
    );
    println!("  phase-action-corpus-v1 [output-jsonl] [report-json]");
    println!("            Generate a deterministic clean action_contract_v1 corpus in Rust");
    println!("  phase-action-domain-corpus-v1 [output-jsonl] [report-json]");
    println!(
        "            Generate a deterministic workflow-shaped action_contract_v1 corpus in Rust"
    );
    println!("  phase-action-coverage-corpus-v1 [output-jsonl] [report-json]");
    println!(
        "            Generate a V5 action_tree coverage corpus with select/transform/write/condition/check diversity"
    );
    println!("  phase-action-contract-v1 [contract-jsonl] [report-json]");
    println!(
        "            Validate the clean action_tree corpus contract before action-router training"
    );
    println!("  phase-action-operator-coverage-v1 [contract-jsonl] [report-json]");
    println!(
        "            Audit action_tree select/transform/write/condition/check coverage without label authority"
    );
    println!("  phase-action-shortcut-v1 [contract-jsonl] [report-json]");
    println!(
        "            Check exact/token/length/bag/source-bigram shortcuts on action_tree corpus"
    );
    println!("  phase-action-runtime-v1 [contract-jsonl] [cells] [report-json]");
    println!(
        "            Compile clean action_tree rows into PhaseCenterFlatRuntime and score heldout"
    );
    println!("  phase-action-package-v1 [contract-jsonl] [package-path] [cells] [manifest-path]");
    println!(
        "            Save/load a clean action_tree PhaseCenterFlatRuntime package and manifest"
    );
    println!("  phase-action-package-inspect-v1 [package-path] [manifest-path]");
    println!("            Inspect a saved clean action_tree package and manifest");
    println!(
        "  phase-action-source-verify-v1 [package-path] [manifest-path] [source-verify-report-json]"
    );
    println!(
        "            Rebuild a saved action package from its source JSONL through Rust and verify exact bytes"
    );
    println!(
        "  phase-action-package-score-v1 [package-path] [manifest-path] [contract-jsonl] [score-report-json]"
    );
    println!("            Score a saved clean action_tree package without recompiling it");
    println!(
        "  phase-action-eval-pack-v1 [package-path] [manifest-path] [contract-jsonl] [eval-pack-path]"
    );
    println!("            Build a binary action eval pack once from JSONL");
    println!(
        "  phase-action-package-score-pack-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json]"
    );
    println!("            Score a saved clean action_tree package through a binary eval pack");
    println!(
        "  phase-action-package-bench-pack-v1 [package-path] [manifest-path] [eval-pack-path] [iterations] [bench-report-json]"
    );
    println!("            Benchmark a saved clean action_tree package through a binary eval pack");
    println!(
        "  phase-action-package-bench-verify-v1 [package-path] [manifest-path] [eval-pack-path] [bench-report-json]"
    );
    println!("            Verify a clean action_tree benchmark package report");
    println!(
        "  phase-action-product-proof-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json] [bench-report-json] [product-proof-json]"
    );
    println!("            Build a product-proof bundle over package, score, and benchmark reports");
    println!(
        "  phase-action-product-verify-v1 [package-path] [manifest-path] [eval-pack-path] [score-report-json] [bench-report-json] [product-proof-json]"
    );
    println!("            Verify a product-proof bundle for a clean action_tree package");
    println!("  role-binding-package-inspect-v1 [package-path] [report-json]");
    println!("            Inspect a serialized .nwrb role-binding package through public SDK APIs");
    println!("  role-binding-package-verify-v1 [package-path] [report-json]");
    println!("            Verify a saved .nwrb role-binding inspect report against package bytes");
    println!(
        "  role-binding-eval-pack-from-package-v1 [package-path] [eval-pack-json] [max-tasks]"
    );
    println!(
        "            Build a package-derived .nwrb scoring smoke eval-pack; not independent corpus proof"
    );
    println!(
        "  role-binding-eval-pack-binary-v1 [source-eval-pack-json] [binary-eval-pack] [report-json]"
    );
    println!("            Convert a role-binding eval-pack JSON into compact binary .nwreb format");
    println!(
        "  role-binding-binary-eval-pack-suite-v1 [root-dir] [suite-report-json] [margin-threshold]"
    );
    println!(
        "            Convert/score all current slot32 role-binding corpus eval-packs as .nwreb"
    );
    println!(
        "  role-binding-binary-eval-pack-suite-verify-v1 [root-dir] [suite-report-json] [margin-threshold]"
    );
    println!("            Rebuild and verify the all-seed slot32 .nwreb suite report");
    println!(
        "  role-binding-package-score-v1 [package-path] [eval-pack-json] [score-report-json] [margin-threshold]"
    );
    println!(
        "            Score a serialized .nwrb role-binding package through an explicit eval-pack"
    );
    println!(
        "  role-binding-package-score-verify-v1 [package-path] [eval-pack-json] [score-report-json] [margin-threshold]"
    );
    println!("            Rebuild and verify a saved .nwrb role-binding score report");
    println!(
        "  role-binding-release-suite-v1 [binary-suite-report-json] [release-suite-report-json]"
    );
    println!(
        "            Bundle .nwrb packages, .nwreb eval-packs, and score reports into one proof"
    );
    println!(
        "  role-binding-release-suite-verify-v1 [binary-suite-report-json] [release-suite-report-json]"
    );
    println!("            Rebuild and verify the role-binding release-suite proof bundle");
    println!(
        "  role-binding-operator-blueprint-gap-v1 [release-suite-report-json] [gap-report-json]"
    );
    println!(
        "            Audit current .nwrb/.nwreb role-binding proof against OPERATOR_BLUEPRINT"
    );
    println!(
        "  role-binding-operator-blueprint-gap-verify-v1 [release-suite-report-json] [gap-report-json]"
    );
    println!("            Rebuild and verify the OPERATOR_BLUEPRINT gap audit report");
    println!(
        "  role-binding-profile-registry-from-release-v1 [release-suite-report-json] [registry-config-json]"
    );
    println!("            Emit a serving-only .nwrb profile registry from a green release-suite");
    println!("  role-binding-profile-serve-v1 [registry-config-json] [bind-addr] [request-limit]");
    println!(
        "            Serve .nwrb role-binding profiles over /health /profiles /score /replay /metrics"
    );
    println!(
        "  role-binding-profile-runtime-smoke-v1 [registry-config-json] [runtime-smoke-report-json]"
    );
    println!("            Smoke-test the serving-only role-binding profile runtime endpoints");
    println!(
        "  role-binding-profile-replay-suite-v1 [registry-config-json] [binary-suite-report-json] [replay-suite-report-json] [max-unique-sequences-per-profile] [batch-unique-sequences]"
    );
    println!(
        "            Replay release-suite requests through the serving-only .nwrb HTTP profile runtime"
    );
    println!(
        "  role-binding-profile-fallback-smoke-v1 [registry-config-json] [fallback-smoke-report-json]"
    );
    println!(
        "            Verify local accept, missing-route fallback, and low-margin fallback over .nwrb HTTP profile runtime"
    );
    println!(
        "  role-binding-profile-worker-scaling-v1 [registry-config-json] [worker-scaling-report-json] [worker-count]"
    );
    println!("            Verify serving-only .nwrb profile shards across multiple local workers");
    println!(
        "  role-binding-profile-worker-replay-v1 [registry-config-json] [binary-suite-report-json] [worker-replay-report-json] [worker-count] [max-unique-sequences-per-profile] [batch-unique-sequences]"
    );
    println!(
        "            Replay release-suite requests through multiple serving-only .nwrb profile workers"
    );
    println!("  role-binding-profile-lb-serve-v1 <lb-config-json> [bind-addr] [request-limit]");
    println!(
        "            Serve a metadata-only profile load-balancer over /health /profiles /score /replay /metrics"
    );
    println!(
        "  role-binding-profile-lb-replay-v1 [registry-config-json] [binary-suite-report-json] [lb-replay-report-json] [worker-count] [max-unique-sequences-per-profile] [batch-unique-sequences]"
    );
    println!(
        "            Replay release-suite requests through a local load-balancer and serving-only .nwrb worker shards"
    );
    println!(
        "  role-binding-profile-lb-throughput-v1 [registry-config-json] [binary-suite-report-json] [throughput-report-json] [worker-count] [max-unique-sequences-per-profile] [client-threads] [sequence-repetitions]"
    );
    println!(
        "            Run a bounded concurrent /score pressure proof through local LB and serving-only .nwrb worker shards"
    );
    println!("  role-binding-real-traffic-record-v1 [trace-jsonl] <record-json>");
    println!(
        "            Validate and append one real traffic shadow trace row without changing the live LLM flow"
    );
    println!(
        "  role-binding-real-traffic-record-serve-v1 [trace-jsonl] [bind-addr] [request-limit]"
    );
    println!(
        "            Serve a bounded local HTTP recorder over /health /trace /metrics for real traffic shadow rows"
    );
    println!(
        "  role-binding-real-traffic-ingest-events-v1 <events-jsonl> [trace-jsonl] [ingest-report-json]"
    );
    println!(
        "            Convert agent/API event JSONL into the real traffic shadow trace contract"
    );
    println!(
        "  role-binding-real-traffic-codex-history-ingest-v1 [history-jsonl] [events-jsonl] [ingest-report-json] [max-events]"
    );
    println!(
        "            Convert local Codex history prompts into privacy-safe event fingerprints without raw text"
    );
    println!(
        "  role-binding-real-traffic-codex-history-route-candidates-v1 [history-jsonl] [registry-config-json] [events-jsonl] [route-report-json] [max-events]"
    );
    println!(
        "            Build route-only Nando shadow candidates from local Codex history; payload remains empty so scoring must fallback"
    );
    println!(
        "  role-binding-real-traffic-shadow-v1 [registry-config-json] [trace-jsonl] [shadow-report-json]"
    );
    println!(
        "            Analyze recorded real traffic in shadow mode against serving-only .nwrb profiles"
    );
    println!(
        "  role-binding-real-traffic-cpu-route-forecast-v1 [route-report-json] [shadow-report-json] [forecast-report-json]"
    );
    println!(
        "            Rank real CPU offload routes and forecast capacity before executable payload builders exist"
    );
    println!(
        "  role-binding-real-traffic-route-gap-catalog-v1 [history-jsonl] [registry-config-json] [route-gap-report-json] [max-events]"
    );
    println!(
        "            Classify no-candidate real Codex prompts into privacy-safe operator-family backlog counts"
    );
    println!(
        "  role-binding-real-traffic-route-gap-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]"
    );
    println!(
        "            Measure request-side payload-builder readiness for no-candidate route-gap families without enabling local accepts"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run planning-next-step payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-profile-v1 [base-registry-json] [planning-dry-run-trace-jsonl] [planning-package-nwrb] [overlay-registry-json] [profile-report-json]"
    );
    println!(
        "            Compile planning-next-step dry-run payload geometry into a .nwrb profile with local accepts disabled by threshold"
    );
    println!(
        "  role-binding-real-traffic-read-inspect-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run read-inspect payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-read-inspect-profile-v1 [base-registry-json] [read-inspect-dry-run-trace-jsonl] [read-inspect-package-nwrb] [overlay-registry-json] [profile-report-json]"
    );
    println!(
        "            Compile read-inspect dry-run payload geometry into a .nwrb profile with local accepts disabled by threshold"
    );
    println!(
        "  role-binding-real-traffic-read-inspect-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach Codex final-answer fingerprints plus conservative read-only path/evidence verifier results; local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-read-inspect-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Calibrate read-inspect score/readout thresholds against deterministic verifier labels; local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-git-control-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run git-control payloads from request text only; workspace mutations and local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-git-control-profile-v1 [base-registry-json] [git-control-dry-run-trace-jsonl] [git-control-package-nwrb] [overlay-registry-json] [profile-report-json]"
    );
    println!(
        "            Compile git-control dry-run payload geometry into a .nwrb profile with local accepts disabled by threshold"
    );
    println!(
        "  role-binding-real-traffic-git-control-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach Codex final-answer fingerprints plus conservative git command-outcome verifier results; workspace mutations and local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-git-control-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Calibrate git-control score/readout thresholds against final-answer verifier labels; real tool-output verifier still required before local accept"
    );
    println!(
        "  role-binding-real-traffic-metrics-report-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run metrics-report payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-metrics-report-profile-v1 [base-registry-json] [metrics-report-dry-run-trace-jsonl] [metrics-report-package-nwrb] [overlay-registry-json] [profile-report-json]"
    );
    println!(
        "            Compile metrics-report dry-run payload geometry into a .nwrb profile with local accepts disabled by threshold"
    );
    println!(
        "  role-binding-real-traffic-metrics-report-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach Codex final-answer fingerprints plus conservative numeric report-field verifier results; local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-metrics-report-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Calibrate metrics-report score/readout thresholds against deterministic verifier labels; local accepts stay disabled"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach Codex final-answer fingerprints plus conservative planning verifier results; true accepts require artifact-progress verification"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-artifact-progress-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [artifact-report-json]"
    );
    println!(
        "            Attach tool-call fingerprints for planning progress; true labels require successful project artifact progress"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-local-accept-calibration-v1 [registry-config-json] [artifact-progress-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Calibrate request-side planning accept policies against artifact-progress labels without enabling local accepts"
    );
    println!(
        "  role-binding-real-traffic-planning-next-step-admission-calibration-v1 [artifact-progress-trace-jsonl] [history-jsonl] [admission-report-json]"
    );
    println!(
        "            Calibrate prompt-side planning admission features against artifact-progress labels without writing raw prompt text"
    );
    println!(
        "  role-binding-real-traffic-agent-control-profile-v1 [base-registry-json] [agent-control-package-nwrb] [overlay-registry-json] [profile-report-json]"
    );
    println!(
        "            Build a serving-only .nwrb control-plane profile overlay; no real-traffic local accepts are enabled"
    );
    println!(
        "  role-binding-real-traffic-agent-control-payload-dry-run-v1 [history-jsonl] [agent-control-registry-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run agent-control payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-agent-control-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach Codex final-answer fingerprints plus deterministic control-plane verification to agent-control dry-run traces"
    );
    println!(
        "  role-binding-real-traffic-agent-control-admission-calibration-v1 [evidence-trace-jsonl] [history-jsonl] [admission-report-json]"
    );
    println!(
        "            Search request-side agent-control admission policies against evidence labels; no local accepts are enabled"
    );
    println!(
        "  role-binding-real-traffic-agent-control-safe-policy-promote-v1 [agent-control-registry-json] [evidence-trace-jsonl] [admission-report-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd] [history-jsonl]"
    );
    println!(
        "            Build a request-side-admitted hard-stop trace; broad agent-control rows remain fallback until shadow/audit pass"
    );
    println!(
        "  role-binding-real-traffic-edit-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]"
    );
    println!(
        "            Audit real edit-route candidates for request-side payload-builder readiness without writing raw text"
    );
    println!(
        "  role-binding-real-traffic-edit-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run edit payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-conditional-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]"
    );
    println!(
        "            Count real conditional route rows with enough request-side branch/evidence structure for payload dry-run"
    );
    println!(
        "  role-binding-real-traffic-conditional-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run conditional payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-mixed-payload-readiness-v1 [history-jsonl] [registry-config-json] [readiness-report-json] [max-events]"
    );
    println!(
        "            Count real mixed-map route rows with enough request-side map/update structure for payload dry-run"
    );
    println!(
        "  role-binding-real-traffic-mixed-payload-dry-run-v1 [history-jsonl] [registry-config-json] [trace-jsonl] [dry-run-report-json] [max-events]"
    );
    println!(
        "            Build scoreable dry-run mixed-map payloads from request text only; verified accepts remain disabled"
    );
    println!(
        "  role-binding-real-traffic-edit-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach response fingerprints and deterministic edit verification to scoreable real Codex edit rows"
    );
    println!(
        "  role-binding-real-traffic-conditional-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach response fingerprints and deterministic conditional verification to scoreable real Codex conditional rows"
    );
    println!(
        "  role-binding-real-traffic-mixed-output-evidence-v1 [input-trace-jsonl] [codex-sessions-root] [output-trace-jsonl] [evidence-report-json]"
    );
    println!(
        "            Attach response fingerprints and deterministic mixed-map verification to scoreable real Codex mixed rows"
    );
    println!(
        "  role-binding-real-traffic-edit-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Compare safe/unsafe readout policies on evidence-backed real Codex edit rows"
    );
    println!(
        "  role-binding-real-traffic-conditional-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Compare safe/unsafe readout policies on evidence-backed real Codex conditional rows"
    );
    println!(
        "  role-binding-real-traffic-conditional-safe-policy-promote-v1 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd] [history-jsonl]"
    );
    println!(
        "            Create a request-side admitted conditional registry/trace from safe evidence; still requires shadow/audit before claims"
    );
    println!(
        "  role-binding-real-traffic-mixed-local-accept-calibration-v1 [registry-config-json] [evidence-trace-jsonl] [calibration-report-json]"
    );
    println!(
        "            Compare safe/unsafe readout policies on evidence-backed real Codex mixed-map rows"
    );
    println!(
        "  role-binding-real-traffic-mixed-safe-policy-promote-v1 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd]"
    );
    println!(
        "            Create a promoted mixed-map registry/trace from a safe calibration policy; still requires shadow/audit before claims"
    );
    println!(
        "  role-binding-real-traffic-edit-safe-policy-promote-v1 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd]"
    );
    println!(
        "            Create a promoted edit-route registry/trace from a safe calibration policy; still requires shadow/audit before claims"
    );
    println!(
        "  role-binding-real-traffic-edit-admission-calibration-v1 [evidence-trace-jsonl] [history-jsonl] [admission-report-json]"
    );
    println!(
        "            Calibrate request-side edit admission gates without writing raw prompt/response text"
    );
    println!(
        "  role-binding-real-traffic-mixed-safe-policy-promote-v2 [base-registry-json] [evidence-trace-jsonl] [calibration-report-json] [promoted-registry-json] [promoted-trace-jsonl] [promote-report-json] [provider-cost-microusd] [history-jsonl]"
    );
    println!(
        "            Promote mixed-map safe policy with request-side goal/control admission before energy threshold"
    );
    println!(
        "  role-binding-real-traffic-verification-hook-audit-v1 [trace-jsonl] [shadow-report-json] [audit-report-json]"
    );
    println!(
        "            Audit whether shadow trace rows carry enough output evidence to count verified CPU accepts"
    );
    println!(
        "  role-binding-real-traffic-feedback-loop-v1 [forecast-report-json] [edit-dry-run-report-json] [verification-audit-report-json] [feedback-report-json] [planning-dry-run-report-json] [planning-local-accept-calibration-report-json] [planning-verification-audit-report-json] [agent-control-admission-calibration-report-json] [agent-control-safe-policy-audit-report-json] [mixed-safe-policy-audit-report-json] [read-inspect-dry-run-report-json] [read-inspect-verification-audit-report-json]"
    );
    println!(
        "            Summarize route -> payload -> verification -> verified CPU gap toward Routability 80"
    );
    println!(
        "            Auto-loads conditional / mixed / metrics-report / git-control route reports from default artifact paths when present; planning, agent-control, mixed, metrics, and git-control reports default to v1 unless supplied"
    );
    println!(
        "  role-binding-real-traffic-cpu-operator-catalog-v1 [feedback-report-json] [route-gap-report-json] [catalog-report-json] [route-gap-payload-readiness-report-json]"
    );
    println!(
        "            Rank existing profile routes and no-candidate route-gap families for the next CPU operator build"
    );
    println!(
        "  role-binding-real-traffic-shadow-smoke-v1 [binary-suite-report-json] [trace-jsonl] [max-unique-sequences-per-profile]"
    );
    println!(
        "            Build a synthetic smoke trace for the shadow analyzer; not a market savings claim"
    );
    println!("  phase-action-release-suite-v1 [release-suite-report-json]");
    println!(
        "            Build a release-suite report over the current generated/domain product-proof bundles"
    );
    println!("  phase-action-release-verify-v1 [release-suite-report-json]");
    println!("            Verify a release-suite report against its saved product-proof inputs");
    println!(
        "  phase-action-license-package-v1 [release-suite-report-json] [license-file] [license-package-report-json]"
    );
    println!("            Build a non-commercial license package over the release-suite");
    println!(
        "  phase-action-license-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json]"
    );
    println!("            Verify the non-commercial license package against repo metadata");
    println!(
        "  phase-action-offload-audit-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [offload-audit-report-json]"
    );
    println!(
        "            Audit local-operator vs LLM-fallback offload over packaged action runtimes"
    );
    println!(
        "  phase-action-offload-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json]"
    );
    println!("            Verify the offload audit report against package/license sources");
    println!(
        "  phase-action-cache-offload-bench-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [cache-offload-bench-report-json]"
    );
    println!(
        "            Compare no-cache, exact-cache, and exact-cache plus Nando local operator paths"
    );
    println!(
        "  phase-action-cache-offload-bench-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [simulated-calls] [cache-offload-bench-report-json]"
    );
    println!(
        "            Verify the cache-enabled offload benchmark against current release artifacts"
    );
    println!("  phase-action-daemon-smoke-v1 [daemon-smoke-report-json]");
    println!(
        "            Start a loopback HTTP service smoke over PhaseCenterOffloadRuntime package bytes"
    );
    println!(
        "  phase-action-daemon-package-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-package-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test HTTP scoring over an existing .nwpc action package");
    println!(
        "  phase-action-daemon-hardening-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-hardening-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test HTTP health, stats, route errors, and request limits");
    println!(
        "  phase-action-daemon-auth-smoke-v1 [package-path] [manifest-path] [corpus-jsonl] [daemon-auth-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test bearer auth for HTTP /score and /stats");
    println!(
        "  phase-action-daemon-registry-smoke-v1 [daemon-registry-smoke-report-json] [margin-threshold-micro]"
    );
    println!(
        "            Smoke-test multi-package HTTP registry routing over existing .nwpc packages"
    );
    println!(
        "  phase-action-daemon-registry-config-smoke-v1 [registry-config-json] [daemon-registry-config-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test loading multi-package HTTP registry routing from JSON config");
    println!(
        "  phase-action-daemon-config-validation-smoke-v1 [registry-config-json] [daemon-config-validation-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test registry config reject-before-serve validation");
    println!(
        "  phase-action-daemon-rate-limit-smoke-v1 [registry-config-json] [daemon-rate-limit-smoke-report-json] [margin-threshold-micro] [max-score-requests]"
    );
    println!("            Smoke-test HTTP /score rate-limit guard over JSON registry config");
    println!(
        "  phase-action-daemon-observability-smoke-v1 [registry-config-json] [daemon-observability-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test HTTP /stats counters and runtime provenance fields");
    println!(
        "  phase-action-daemon-audit-log-smoke-v1 [registry-config-json] [audit-log-jsonl] [daemon-audit-log-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test server-side structured JSONL audit events");
    println!(
        "  phase-action-daemon-error-taxonomy-smoke-v1 [registry-config-json] [daemon-error-taxonomy-smoke-report-json] [margin-threshold-micro]"
    );
    println!("            Smoke-test HTTP rejection taxonomy and no-scorer error counters");
    println!("  phase-action-daemon-proof-suite-v1 [daemon-proof-suite-report-json]");
    println!("            Verify saved HTTP daemon proof reports as one product bundle");
    println!("  phase-action-daemon-live-proof-suite-v1 [daemon-live-proof-suite-report-json]");
    println!("            Rerun HTTP daemon smoke gates and verify them as one product bundle");
    println!(
        "  phase-action-daemon-systemd-smoke-v1 [service-unit] [env-file] [registry-config-json] [daemon-systemd-smoke-report-json]"
    );
    println!(
        "            Generate and verify local systemd service/env artifacts for daemon packaging"
    );
    println!(
        "  phase-action-daemon-deployment-package-v1 [daemon-live-proof-suite-report-json] [daemon-systemd-smoke-report-json] [daemon-deployment-package-report-json]"
    );
    println!(
        "            Verify daemon live proof, systemd smoke, service, env, and registry artifacts as one deployment package"
    );
    println!(
        "  phase-action-daemon-deployment-verify-v1 [daemon-live-proof-suite-report-json] [daemon-systemd-smoke-report-json] [daemon-deployment-package-report-json]"
    );
    println!(
        "            Verify the saved daemon deployment package report against current proof sources"
    );
    println!(
        "  phase-action-daemon-serve-registry-v1 [registry-config-json] [bind-addr] [margin-threshold-micro] [auth-token] [max-score-requests] [audit-log-jsonl]"
    );
    println!("            Serve multiple existing .nwpc packages from a JSON registry config");
    println!(
        "  phase-action-daemon-serve-v1 [package-path] [bind-addr] [margin-threshold-micro] [auth-token] [max-score-requests] [audit-log-jsonl]"
    );
    println!("            Serve an existing .nwpc package through HTTP POST /score");
    println!(
        "  phase-action-workflow-bench-v1 [release-suite-report-json] [license-file] [license-package-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json]"
    );
    println!(
        "            Verify the workflow-shaped domain_action artifact over the frozen package chain"
    );
    println!(
        "  phase-action-workflow-bench-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json]"
    );
    println!("            Rebuild and verify the workflow-shaped benchmark proof");
    println!(
        "  phase-action-workflow-replay-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [workflow-sessions] [steps-per-session] [workflow-replay-report-json]"
    );
    println!(
        "            Replay deterministic multi-package workflow traces through frozen .nwpc packages"
    );
    println!(
        "  phase-action-workflow-replay-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [margin-threshold-micro] [workflow-sessions] [steps-per-session] [workflow-replay-report-json]"
    );
    println!("            Rebuild and verify the workflow replay proof");
    println!(
        "  phase-action-regression-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]"
    );
    println!(
        "            Freeze the current green release/license/offload/cache/workflow regression proof"
    );
    println!(
        "  phase-action-regression-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]"
    );
    println!("            Verify the frozen green regression proof against current sources");
    println!(
        "  phase-action-regression-freeze-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [regression-freeze-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]"
    );
    println!(
        "            Write a machine-checkable freeze checkpoint over a green regression proof"
    );
    println!(
        "  phase-action-regression-freeze-verify-v1 [release-suite-report-json] [license-file] [license-package-report-json] [offload-audit-report-json] [regression-report-json] [regression-freeze-report-json] [cache-offload-bench-report-json] [workflow-bench-report-json] [workflow-replay-report-json]"
    );
    println!("            Verify a regression freeze checkpoint against current proof sources");
    println!("  phase-action-package-verify-v1 [package-path] [manifest-path] [score-report-json]");
    println!("            Verify a clean action_tree package, manifest, and score report");
    println!("  strict-multiseed-rust-audit-v1 [diagnostics-root] [audit-report-json]");
    println!("            Parse Rust strict runtime logs for the v4 multi-seed robustness rung");
    println!("  strict-multiseed-rust-audit-verify-v1 [diagnostics-root] [audit-report-json]");
    println!("            Rebuild and verify the Rust strict multi-seed audit report");
    println!("  live-byte-train [seed] <text...>");
    println!("            Run primitive online byte prediction with local feedback");
    println!("  live-byte-learn [seed] <text...>");
    println!("            Train tiny online next-byte adapter from wave traces");
    println!("  live-byte-holdout [seed] <text...>");
    println!("            Train tiny byte adapter on first half and test second half");
    println!("  live-byte-holdout-suite [seed]");
    println!("            Run live byte holdout gates over built-in corpora");
    println!("  live-byte-holdout-seed-sweep");
    println!("            Sweep live byte holdout suite over fixed seeds");
    println!("  live-cell-promote [seed] <text...>");
    println!(
        "            Test candidate -> holdout gate -> promoted/rejected Cell32 learner state"
    );
    println!("  live-architecture-compare [seed]");
    println!("            Compare 3x/6x Cell32 local learners against mono96/mono192 controls");
    println!("  live-tissue-diagnose [seed]");
    println!("            Diagnose pair/triple LinkTissue gains, ablations, and typed topology");
    println!("  live-grok-trace [seed] [epochs] [interval]");
    println!("            Trace early grokking progress measures for LinkTissue");
    println!("  live-grok-sweep [epochs] [interval]");
    println!("            Compare LinkTissue update rules over fixed grokking seeds");
    println!("  eval-symbol-l3");
    println!("            Check SymbolL3 turbo/default/stress profile gates");
    println!("  eval-symbol-understanding");
    println!("            Check the first context-sensitive wave-center gate");
    println!("  eval-symbol-retrieval");
    println!("            Check noisy associative retrieval after pattern storage");
    println!("  eval-symbol-retrieval-sweep");
    println!("            Sweep turbo-256 retrieval capacity over 4/8/16/32 patterns");
    println!("  eval-symbol-retrieval-capacity");
    println!("            Sweep turbo-256 capacity over 32/64/128/256 patterns and two seeds");
    println!("  eval-symbol-retrieval-capacity-scale");
    println!("            Sweep 256-pattern capacity over 256/512/1024 cells and two seeds");
    println!("  eval-one-tick <input-byte> [seed]");
    println!("            Print a minimal Stage 2 one-tick eval report");
    println!("  eval-periodic [seed] [cases] [start] [step]");
    println!("            Run the first Stage 3 periodic baseline eval");
    println!("  eval-phase-composition [seed] [cases] [start] [input-step] [phase-step]");
    println!("            Run a synthetic phase-composition baseline eval");
    println!("  eval-phase-holdout [train-seed] [holdout-seed] [cases]");
    println!("            Check the phase-composition candidate on a holdout split");
    println!("  eval-carrier-control [train-seed] [holdout-seed] [cases]");
    println!("            Compare correct, missing, wrong, and corrupted CarrierWave");
    println!("  eval-bus-transfer [train-seed] [holdout-seed] [cases]");
    println!("            Test CarrierWave effect through WaveBus center phase only");
    println!("  eval-snapshot-memory [train-seed] [holdout-seed] [cases]");
    println!("            Test serialized snapshot replay against cold/wrong/corrupted state");
    println!("  eval-snapshot-transition [train-seed] [holdout-seed] [cases]");
    println!("            Test previous snapshot offset as a next-state transition predictor");
    println!("  eval-snapshot-dynamics [train-seed] [holdout-seed] [cases]");
    println!("            Test snapshot transition over a smooth CarrierWave sequence");
    println!("  eval-snapshot-multitick [train-seed] [holdout-seed] [cases]");
    println!("            Test one warm snapshot across several smooth CarrierWave ticks");
    println!("  eval-snapshot-adapt [train-seed] [holdout-seed] [cases]");
    println!("            Test online phase correction from feedback after warm snapshot");
    println!("  eval-snapshot-decoder [train-seed] [holdout-seed] [cases]");
    println!("            Test an online transition decoder with snapshot features");
    println!("  eval-snapshot-keyed [train-seed] [holdout-seed] [cases]");
    println!("            Test snapshot-private state against no-snapshot controls");
    println!("  eval-snapshot-keyed-transition [train-seed] [holdout-seed] [cases]");
    println!("            Test future wave-state combined with snapshot-private state");
    println!("  eval-snapshot-noisy-keyed-transition [train-seed] [holdout-seed] [cases]");
    println!("            Test noisy future transition with hidden snapshot modulation");
    println!("  eval-snapshot-noisy-keyed-transition-sweep [train-seed] [holdout-seed] [cases]");
    println!("            Sweep noisy snapshot transition over several horizons");
    println!("  eval-snapshot-noisy-keyed-transition-seed-sweep [cases]");
    println!("            Sweep noisy snapshot transition over fixed seed pairs");
    println!("  eval-byte-context [train-seed] [holdout-seed] [cases]");
    println!("            Train and test the first byte-stream context probe");
    println!("  eval-byte-context-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Train frozen byte-context prototypes and test holdout");
    println!("  eval-byte-context-offset-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test offset-only byte-context centroid prototypes");
    println!("  eval-byte-context-denoised-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test offset plus top-sin byte-context centroid prototypes");
    println!("  eval-byte-context-relative-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test seed-normalized relative byte-context centroid prototypes");
    println!("  eval-byte-context-lexical-carrier-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test lexical CarrierWave state formation for byte-context prototypes");
    println!("  eval-byte-context-cellular-carrier-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test task-cell CarrierWave lock for byte-context prototypes");
    println!("  eval-byte-context-trained-carrier-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test supervised harmonic CarrierWave lock cells");
    println!("  eval-byte-context-prompt-carrier-centroid [train-seed] [holdout-seed] [cases]");
    println!("            Test full-prompt harmonic CarrierWave lock cells");
    println!(
        "  eval-byte-context-prompt-carrier-diverse-centroid [train-seed] [holdout-seed] [cases]"
    );
    println!("            Test full-prompt lock cells across diverse prompt templates");
    println!("  eval-byte-context-centroid-seed-sweep [cases]");
    println!("            Sweep byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-offset-centroid-seed-sweep [cases]");
    println!("            Sweep offset-only byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-denoised-centroid-seed-sweep [cases]");
    println!("            Sweep denoised byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-relative-centroid-seed-sweep [cases]");
    println!("            Sweep relative byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-lexical-carrier-centroid-seed-sweep [cases]");
    println!("            Sweep lexical CarrierWave byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-cellular-carrier-centroid-seed-sweep [cases]");
    println!("            Sweep cellular CarrierWave byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-trained-carrier-centroid-seed-sweep [cases]");
    println!("            Sweep trained CarrierWave byte-context centroid over fixed seed pairs");
    println!("  eval-byte-context-prompt-carrier-centroid-seed-sweep [cases]");
    println!(
        "            Sweep prompt-cloud CarrierWave byte-context centroid over fixed seed pairs"
    );
    println!("  eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep [cases]");
    println!("            Sweep diverse prompt-cloud centroid over fixed seed pairs");
    println!("  eval-byte-context-centroid-ablation [train-seed] [holdout-seed] [cases]");
    println!("            Ablate byte-context centroid snapshot features");
    println!("  eval-byte-context-cellular-carrier-ablation [train-seed] [holdout-seed] [cases]");
    println!("            Ablate cellular CarrierWave lock cells");
    println!("  eval-byte-context-trained-carrier-ablation [train-seed] [holdout-seed] [cases]");
    println!("            Ablate trained CarrierWave lock cells");
    println!("  eval-byte-context-prompt-carrier-ablation [train-seed] [holdout-seed] [cases]");
    println!("            Ablate prompt-cloud CarrierWave lock cells");
    println!(
        "  eval-byte-context-prompt-carrier-diverse-ablation [train-seed] [holdout-seed] [cases]"
    );
    println!("            Ablate diverse prompt-cloud CarrierWave lock cells");
    println!("  eval-chat0 [train-seed] [holdout-seed] [cases]");
    println!("            Run the first short-response Chat-0 loop with feedback logging");
    println!("  eval-settle-word [train-seed] [holdout-seed] [cases]");
    println!("            Test short-word readout after multi-tick Organ192 settling");
    println!("  eval-settle-word-seed-sweep [cases]");
    println!("            Sweep gated short-word settling across fixed seed pairs");
    println!("  eval-chat0-route [train-seed] [holdout-seed] [cases]");
    println!("            Measure manual Chat-0 route quality on free prompt templates");
    println!("  eval-chat0-promote [feedback-log] [train-seed] [holdout-seed] [cases]");
    println!("            Test logged feedback as an eval-gated replay promotion candidate");
    println!("  eval-chat0-promoted-holdout [feedback-log] [train-seed] [holdout-seed] [cases]");
    println!("            Test whether promoted feedback transfers beyond exact prompt replay");
    println!(
        "  chat0-promote-save <feedback-log> <state-path> [train-seed] [holdout-seed] [cases]"
    );
    println!("            Save a promoted feedback state only after the promote eval passes");
    println!("  chat0-once <prompt> [expected] [trace-path]");
    println!("            Generate one Chat-0 answer and save trace/feedback files");
    println!("  chat0-once-promoted <state-path> <prompt> [expected] [trace-path]");
    println!("            Generate one Chat-0 answer with an eval-promoted state overlay");
    println!("  chat0-shell [trace-dir] [feedback-log]");
    println!("            Read prompts from stdin and write trace per answer");
}
