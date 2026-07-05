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
