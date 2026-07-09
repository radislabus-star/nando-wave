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
    println!(
        "  phase-stream-test-output-parse-v1 [trace-jsonl] [shadow-report-json] [cells] [candidate-package-path]"
    );
    println!(
        "            Build a shadow-only phase-center status/raw-output report and candidate .nwpc"
    );
    println!(
        "  phase-stream-test-output-raw-log-trace-v1 [trace-jsonl] [trace-report-json] [raw-log ...]"
    );
    println!(
        "            Convert existing raw stdout/stderr log artifacts into test-output phase trace JSONL"
    );
    println!(
        "  phase-stream-discovery-v1 [report-json] [candidate-dir] [cells] [model-price-config-json] [trace-jsonl ...]"
    );
    println!(
        "            Discover verifier-bound phase-center buckets across trace JSONL inputs and write quarantine .nwpc candidates"
    );
    println!(
        "  phase-stream-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]"
    );
    println!(
        "            Stream trace rows in order, compile verifier-bound .nwpc buckets after threshold, and shadow-score only future events"
    );
    println!(
        "  phase-stream-online-miner-daemon-v1 [report-json] [checkpoint-dir] [decision-log-jsonl] [cells] [min-bucket-events] [base-margin-floor-micro] [compile-every-rows] [max-active-buckets] [reservoir-per-label] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Scan append-only phase-atom events, update phase-center buckets online, compile quarantine .nwpc checkpoints, and keep local accept disabled"
    );
    println!(
        "  phase-stream-online-miner-value-pass-v1 [report-json] [top-k] [--price-config json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Fast no-compile selector pass over phase-atom events; ranks candidate buckets for later .nwpc proof without local accept or money claims"
    );
    println!(
        "  phase-stream-online-miner-targeted-shadow-v1 [report-json] [checkpoint-dir] [cells] [top-k] [train-permille] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Compile only value-pass-selected buckets into quarantine .nwpc packages and shadow-score future rows; no epochs, serving, local accept, or money claims"
    );
    println!(
        "  phase-stream-online-miner-targeted-rejection-drilldown-v1 [report-json] [value-pass-report-json] [targeted-shadow-report-json] [promotion-registry-gate-report-json]"
    );
    println!(
        "            Explain value-pass -> targeted-shadow product-hot losses by bucket; audit only, no compile, threshold tuning, serving, or local accept"
    );
    println!(
        "  phase-stream-online-miner-targeted-split-refinement-v1 [report-json] [candidate-jsonl] [rejection-drilldown-report-json] [targeted-shadow-report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Rank source-neutral split atoms for false-accept buckets; audit only, no compile, threshold tuning, serving, or local accept"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-gate-v1 [report-json] [accepted-events-jsonl] [targeted-shadow-report-json] [promotion-registry-gate-report-json] [split-shadow-replay-report-json]"
    );
    println!(
        "            Dedupe product-hot and targeted split shadow accepts into one calls/tokens denominator; no serving, local accept, or money claim"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-billing-request-v1 [report-json] [billing-request-jsonl] [targeted-aggregate-report-json]"
    );
    println!(
        "            Export aggregate accepted events as billing-request rows for external provider evidence; no money claim by itself"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-admission-gate-v1 [report-json] [targeted-aggregate-report-json] [aggregate-billing-request-report-json] [billing-evidence-gate-report-json]"
    );
    println!(
        "            Join aggregate calls/tokens proof with billing evidence status; leaves money/local_accept blocked without provider evidence"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-provider-export-acquisition-pack-v1 [report-json] [output-dir] [targeted-aggregate-report-json]"
    );
    println!(
        "            Export the 677-row aggregate billing worklist and required provider-export schema without creating money evidence"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [targeted-aggregate-report-json]"
    );
    println!(
        "            Normalize an external provider billing export for aggregate accepts, run evidence/admission gates, and require attestation before money"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-provider-export-attestation-contract-v1 [report-json] <provider-export-jsonl> [attestation-template-json]"
    );
    println!(
        "            Write the aggregate provider-export attestation contract/template required before external billing evidence can unlock money"
    );
    println!(
        "  phase-stream-online-miner-targeted-aggregate-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [targeted-aggregate-report-json]"
    );
    println!(
        "            Scan local provider-export candidates for aggregate billing keys and run provider-export admission without local accept"
    );
    println!(
        "  phase-stream-online-miner-promotion-registry-gate-v1 [report-json] [shadow-registry-dir] [product-hot-promotion-registry-json]"
    );
    println!(
        "            Validate product-hot quarantine registry packages and copy accepted .nwpc files into a shadow registry without serving/local_accept"
    );
    println!("  phase-stream-opportunity-board-v1 [report-json] [phase-atom-trace-jsonl ...]");
    println!(
        "            Rank real stream classes by traffic, verifier coverage, exact-cache overlap, token weight, and next adapter/miner action"
    );
    println!(
        "  phase-stream-constrained-split-miner-v1 [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Automatically split broad stream classes under a zero-false-accept constraint and report safe future accepts/tokens over exact cache"
    );
    println!(
        "  phase-stream-automatic-continuation-split-v1 [report-json] [selected-split-report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Refine selected broad result/evidence splits into automatic continuation sub-splits before .nwpc compilation"
    );
    println!(
        "  phase-stream-verifier-evidence-join-v1 [report-json] [output-jsonl] [base-phase-atom-trace-jsonl] [verifier-evidence-jsonl ...]"
    );
    println!(
        "            Join verifier labels and safe evidence atoms into matching trace rows by request_fingerprint/exact_cache_key without scoring or promotion"
    );
    println!(
        "  phase-stream-phase-atom-trace-sample-v1 [report-json] [output-jsonl] [sample-modulus] [sample-remainder] [--keep-verified-safe] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Copy a deterministic modulo slice of phase-atom JSONL for mini .nwpc survival diagnostics; optional verifier-positive oversampling, no scoring, promotion, or local_accept"
    );
    println!(
        "  phase-stream-selected-split-nwpc-quarantine-v1 [report-json] [package-dir] [cells] [selected-split-report-json] [--hash-train-future] [--auto-multi-split] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Compile automatic selected split children into verifier-bound .nwpc quarantine packages and shadow-score future rows; optional hash split removes file-order split bias, optional auto-multi-split expands selected classes into observable atom subcenters"
    );
    println!(
        "  phase-stream-selected-split-nwpc-promotion-gate-v1 [report-json] [shadow-registry-dir] [quarantine-report-json]"
    );
    println!(
        "            Copy only zero-false-accept, parity-clean .nwpc quarantine packages into a shadow registry without serving/local_accept"
    );
    println!(
        "  phase-stream-selected-split-nwpc-shadow-replay-v1 [report-json] [promotion-gate-report-json] [--hash-train-future] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Reload promoted shadow-registry .nwpc packages and recompute future decisions over trace rows; use --hash-train-future only for packages compiled with the same split mode"
    );
    println!(
        "  phase-stream-selected-split-nwpc-portfolio-select-v1 [report-json] [portfolio-promotion-report-json] [shadow-replay-report-json ...]"
    );
    println!(
        "            Build a cold global-overlap-aware .nwpc shadow portfolio from replay evidence; writes a filtered promotion report without serving/local_accept"
    );
    println!(
        "  phase-stream-selected-split-nwpc-billing-request-v1 [report-json] [billing-request-jsonl] [shadow-replay-report-json]"
    );
    println!(
        "            Export runtime-replayed accepted events that require external provider billing evidence"
    );
    println!(
        "  phase-stream-selected-split-nwpc-admission-gate-v1 [report-json] [shadow-replay-report-json] [billing-request-report-json] [billing-evidence-gate-report-json]"
    );
    println!(
        "            Combine selected-split .nwpc runtime replay and billing evidence gates into a shadow-ready or money-blocked admission verdict"
    );
    println!(
        "  phase-stream-selected-split-nwpc-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [shadow-replay-report-json] [billing-request-report-json] [billing-request-jsonl]"
    );
    println!(
        "            Normalize external provider billing export and rerun selected-split .nwpc billing/admission gates without local accept"
    );
    println!(
        "  phase-stream-selected-split-nwpc-provider-export-attestation-contract-v1 [report-json] <provider-export-jsonl> [attestation-template-json]"
    );
    println!(
        "            Write an attestation contract/template for a real external provider export; template is not valid evidence"
    );
    println!(
        "  phase-stream-selected-split-nwpc-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [shadow-replay-report-json] [billing-request-report-json] [billing-request-jsonl]"
    );
    println!(
        "            Scan local provider-export candidates for selected billing keys and run selected-split provider admission on matches"
    );
    println!(
        "  phase-stream-selected-split-nwpc-evidence-chain-audit-v1 [report-json] [quarantine-report-json] [promotion-report-json] [shadow-replay-report-json] [billing-request-report-json] [admission-report-json] [provider-export-admission-report-json]"
    );
    println!(
        "            Audit selected-split .nwpc report lineage from quarantine through provider-export admission"
    );
    println!(
        "  phase-stream-selected-split-nwpc-loss-audit-v1 [report-json] [selected-split-report-json] [quarantine-report-json] [shadow-replay-report-json]"
    );
    println!(
        "            Compare automatic split value against compiled .nwpc quarantine/replay value and report where accepts disappear"
    );
    println!(
        "  phase-stream-selected-split-nwpc-stage-filter-v1 [report-json] [filtered-selected-split-report-json] [selected-split-report-json] [quarantine-report-json ...]"
    );
    println!(
        "            Keep the union of package-level .nwpc survivors from one or more evidence reports before the next full quarantine pass; no local_accept"
    );
    println!(
        "  phase-stream-live-store-adapter-smoke-v1 [report-json] [cells] [min-bucket-events] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Feed cold phase-atom JSONL into the numeric PhaseCenterLiveOperatorStore boundary; no promotion, no local accept, no market claim"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-shadow-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Load frozen clean .nwpc manifest into PhaseCenterHotRuntime and shadow-score trace with parity, false-accept, token and cost counters"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-admission-gate-v1 [report-json] [manifest-json] [shadow-report-json] [prepared-hot-pack-report-json]"
    );
    println!(
        "            Validate clean .nwpc manifest, shadow replay, and prepared hot pack as a shadow-registry candidate without local accept or market money claim"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-live-policy-stage-v1 [report-json] [policy-json] [clean-manifest-admission-report-json]"
    );
    println!(
        "            Convert a SHADOW_READY clean .nwpc admission report into a shadow-only live policy artifact without registry mutation, local accept, or money claim"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-live-policy-shadow-review-v1 [report-json] [stage-report-json] [policy-json] [live-source-worker-report-json]"
    );
    println!(
        "            Review shadow-only live policy plus live-source worker evidence; safety can pass while latency remains WATCH, with no local accept or registry mutation"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-prepared-policy-shadow-review-v1 [report-json] [stage-report-json] [policy-json] [prepared-hot-pack-report-json] [memory-worker-report-json]"
    );
    println!(
        "            Review shadow-only live policy against prepared/numeric hot-path evidence; no source JSON in timed hot loops, local accept, or registry mutation"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-shadow-registry-handoff-v1 [report-json] [shadow-registry-dir] [prepared-policy-shadow-review-report-json]"
    );
    println!(
        "            Copy prepared-review-approved verifier-bound .nwpc packages into a shadow registry without serving/local accept or money claim"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-shadow-registry-replay-v1 [report-json] [shadow-registry-handoff-report-json] [prepared-hot-pack-json]"
    );
    println!(
        "            Reload copied shadow-registry .nwpc packages and replay prepared numeric rows without serving/local accept or money claim"
    );
    println!(
        "  phase-stream-live-store-clean-manifest-shadow-registry-billing-request-v1 [report-json] [billing-request-jsonl] [shadow-registry-replay-report-json] [prepared-hot-pack-json] [correlation-sidecar-jsonl]"
    );
    println!(
        "            Export verifier-bound .nwpc shadow accepts as billing-request rows; blocks money until external provider correlation evidence exists"
    );
    println!(
        "  phase-stream-live-store-prepared-hot-pack-v1 [manifest-json] [pack-json] [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Build a cold numeric route_index/atom_ids pack from trace, then replay it through PhaseCenterHotRuntime without JSON/String/BTreeMap/file IO in the timed hot loops"
    );
    println!(
        "  phase-stream-live-store-prepared-hot-pack-correlation-sidecar-v1 [report-json] [sidecar-jsonl] [prepared-hot-pack-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Build a cold request/trace/provider correlation sidecar for a numeric prepared hot pack without changing the hot pack"
    );
    println!(
        "  phase-stream-live-worker-memory-smoke-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Build prepared phase vectors in memory, then score through PhaseCenterHotWorker without JSON/String/BTreeMap/file IO/package compile in the timed worker loop"
    );
    println!(
        "  phase-stream-live-source-adapter-worker-v1 [manifest-json] [report-json] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Read phase-atom events as a line stream, encode each safe event, and score immediately through PhaseCenterHotWorker while keeping local accept disabled"
    );
    println!(
        "  phase-stream-live-worker-queue-smoke-v1 [manifest-json] [report-json] [queue-batch-capacity] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Feed source-adapter events into a bounded memory queue, then drain prepared vectors through an isolated PhaseCenterHotWorker loop"
    );
    println!(
        "  phase-stream-live-worker-thread-smoke-v1 [manifest-json] [report-json] [channel-capacity] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Feed source-adapter events through a bounded sync channel into a dedicated PhaseCenterHotWorker thread with separate source/queue/score latency metrics"
    );
    println!(
        "  phase-stream-live-worker-batch-thread-smoke-v1 [manifest-json] [report-json] [channel-capacity] [source-batch-capacity] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Feed source-adapter batches through a bounded sync channel into a dedicated PhaseCenterHotWorker thread to separate wakeup cost from hot batch score"
    );
    println!(
        "  phase-stream-live-store-direct-batch-thread-smoke-v1 [report-json] [channel-capacity] [source-batch-capacity] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Build a mutable phase-center live store from trace, export hot runtime/table directly without manifest/package roundtrip, then score prepared batches in a hot worker thread"
    );
    println!(
        "  phase-stream-hot-path-benchmark-v1 [report-json] [timed-score-iterations] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Benchmark only the product hot score lane: route_index + fixed phase vector + PhaseCenterHotRuntime + scratch, with JSON/String/BTreeMap/file IO outside timing"
    );
    println!(
        "  phase-stream-hot-path-daemon-admission-policy-smoke-v1 [daemon-policy-json] [policy-smoke-report-json]"
    );
    println!(
        "            Consume a hot-path daemon admission policy artifact and emit a shadow-only daemon staging decision without registry/runtime mutation"
    );
    println!(
        "  phase-stream-hot-path-daemon-shadow-gate-v1 [policy-smoke-json] [shadow-report-json] [decision-log-jsonl] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Consume a hot-path policy smoke, rebuild a bounded hot snapshot from trace, and write shadow decisions without registry/runtime mutation"
    );
    println!(
        "  phase-stream-hot-path-daemon-append-shadow-gate-v1 [policy-smoke-json] [append-shadow-report-json] [decision-log-jsonl] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]"
    );
    println!(
        "            Build a bounded hot snapshot from a watermark trace, then score only append-window events through PhaseCenterHotRuntime without registry/runtime mutation"
    );
    println!(
        "  phase-stream-hot-path-daemon-live-loop-budget-smoke-v1 [report-json] [cells] [min-bucket-events] [phase-atom-trace-jsonl|- ...]"
    );
    println!(
        "            Feed phase-atom events into the mutable PhaseCenterLiveOperatorStore and report live HOT/WARM budgets without package compile, registry mutation, or local_accept"
    );
    println!(
        "  phase-stream-hot-path-daemon-append-live-loop-smoke-v1 [report-json] [decision-log-jsonl] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]"
    );
    println!(
        "            Initialize the mutable live store from a watermark trace, score append events before update, refresh the hot view, and evaluate a shadow-only admission queue"
    );
    println!(
        "  phase-stream-hot-path-daemon-append-live-tail-v1 [report-json] [decision-log-jsonl] [cells] [min-bucket-events] [idle-sleep-ms] [max-idle-ms] [max-append-events] [watermark-trace-jsonl] [append-tail-jsonl] [product-hot-registry-json]"
    );
    println!(
        "            Bootstrap from a watermark trace, seek to the end of an append-only JSONL file, wait for new phase-atom rows, score before update, and keep local_accept disabled"
    );
    println!(
        "  phase-stream-hot-path-daemon-live-loop-numeric-benchmark-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]"
    );
    println!(
        "            Benchmark the daemon live-loop numeric lane only: prepared route_id/bucket_id/atom_ids into PhaseCenterLiveOperatorStore, with JSON/String/file IO outside timing"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-package-shadow-audit-v1 [numeric-report-json] [audit-report-json] [candidate-index]"
    );
    println!(
        "            Load a quarantine .nwpc candidate into PhaseCenterHotRuntime by numeric route/profile ids and shadow-score matching append rows without promotion or local_accept"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-future-package-audit-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]"
    );
    println!(
        "            Freeze a verifier-bound .nwpc candidate before later append rows, then shadow-score only future matching rows through prepared-vector PhaseCenterHotRuntime"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1 [report-json] [cells] [min-bucket-events] [watermark-trace-jsonl|-] [append-trace-jsonl|- ...]"
    );
    println!(
        "            Build child fresh-future .nwpc audits, policy smokes, a portfolio gate, and a runtime replay without promotion or local_accept"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-admission-portfolio-gate-v1 [portfolio-report-json] [future-audit-report-json ...]"
    );
    println!(
        "            Aggregate fresh-future .nwpc audits and policy smokes into a shadow-only portfolio, rejecting WATCH/costless/false-accept evidence"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-admission-portfolio-runtime-replay-v1 [portfolio-gate-report-json] [runtime-replay-report-json]"
    );
    println!(
        "            Reload accepted portfolio .nwpc packages and re-score their recorded fresh-future windows through PhaseCenterHotRuntime without mutation"
    );
    println!(
        "  phase-stream-hot-path-daemon-numeric-false-accept-split-audit-v1 [future-audit-report-json] [split-report-json] [top-k]"
    );
    println!(
        "            Diagnose a red fresh-future .nwpc bucket by ranking observable atom ids that separate clean score candidates from false accepts; no threshold tuning or promotion"
    );
    println!(
        "  phase-stream-real-traffic-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]"
    );
    println!(
        "            Discover non-legacy verifier-bound phase centers from real agent-loop nando_shadow_request traces"
    );
    println!(
        "  phase-stream-real-traffic-refined-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]"
    );
    println!(
        "            Discover phase centers in request-shape sub-buckets from real agent-loop traces"
    );
    println!(
        "  phase-stream-real-traffic-action-family-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]"
    );
    println!("            Discover phase centers in broader request-side action-family buckets");
    println!(
        "  phase-stream-real-traffic-state-action-online-discovery-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [model-price-config-json] [trace-jsonl ...]"
    );
    println!(
        "            Discover phase centers in coarse state/action buckets from real agent-loop traces"
    );
    println!(
        "  phase-stream-real-traffic-frontier-union-v1 [union-report-json] [online-discovery-report-json ...]"
    );
    println!(
        "            Union safe verifier-bound phase-center discovery reports without serving promotion"
    );
    println!(
        "  phase-stream-real-traffic-cpu10-gap-audit-v1 [gap-report-json] [frontier-union-report-json] [trace-jsonl ...]"
    );
    println!("            Audit current verifier-ready trace ceiling and remaining CPU10 gap");
    println!(
        "  phase-stream-real-traffic-shadow-request-gap-audit-v1 [gap-report-json] [trace-jsonl ...]"
    );
    println!(
        "            Audit missing nando_shadow_request rows and rank adapter gaps before phase-center scoring"
    );
    println!(
        "  phase-stream-real-traffic-mining-input-readiness-v1 [readiness-report-json] [trace-jsonl ...]"
    );
    println!(
        "            Audit whether traces contain request-side atoms needed for new route-family mining"
    );
    println!(
        "  phase-stream-real-traffic-phase-atom-trace-v1 [report-json] [output-jsonl] [trace-jsonl ...]"
    );
    println!(
        "            Build real_traffic_phase_atom_trace_v1 JSONL without raw response or target/proof labels"
    );
    println!(
        "  phase-stream-codex-history-phase-atom-trace-v1 [report-json] [output-jsonl] [history-jsonl] [max-rows]"
    );
    println!(
        "            Convert local Codex request history into phase atom trace rows without storing raw text"
    );
    println!(
        "  phase-stream-phase-atom-verifier-needed-ranking-v1 [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Rank phase atom action families by verifier/result capture priority before .nwpc compile"
    );
    println!(
        "  phase-stream-agent-continue-active-turn-state-v1 [report-json] [output-jsonl] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Build agent_continue active-turn atoms and subroute labels without raw prompt/answer text"
    );
    println!(
        "  phase-stream-agent-continue-command-result-followup-pack-v1 [report-json] [output-jsonl] [tool-status-phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Repack observed tool_status result atoms into command_result_followup audit rows"
    );
    println!(
        "  phase-stream-agent-continue-subroute-scoreboard-v1 [report-json] [agent-continue-active-turn-jsonl]"
    );
    println!(
        "            Score agent_continue subroutes before verifier-bound phase-center mining"
    );
    println!(
        "  phase-stream-auto-subcenter-discovery-v1 [report-json] [candidate-trace-jsonl] [rejections-jsonl] [max-selected-candidates] [max-positive-rows-per-candidate] [background-rows-per-positive] [agent-continue-phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Automatically rank split atoms and attach background negatives for quarantine phase-center mining"
    );
    println!(
        "  phase-stream-codex-session-run-check-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]"
    );
    println!(
        "            Extract run_check verifier labels from Codex session exec_command_end tool outputs"
    );
    println!(
        "  phase-stream-codex-session-planning-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]"
    );
    println!(
        "            Extract planning verifier labels from Codex update_plan tool outputs without raw plan text"
    );
    println!(
        "  phase-stream-codex-session-tool-status-verifier-trace-v1 [report-json] [output-jsonl] [sessions-dir] [max-events]"
    );
    println!(
        "            Extract generic tool_status verifier labels from Codex session exec_command_end status/output-shape metadata"
    );
    println!(
        "  phase-stream-codex-session-live-append-v1 [report-json] [append-jsonl] [session-jsonl] [poll-ms] [max-idle-ms] [max-rows]"
    );
    println!(
        "            Tail one Codex session from EOF and append verifier-bound tool_status phase-atom rows to the live append source without raw tool/request/response text"
    );
    println!(
        "  phase-stream-codex-sessions-live-append-v1 [report-json] [append-jsonl] [sessions-dir] [poll-ms] [max-idle-ms] [max-rows] [max-recent-files]"
    );
    println!(
        "            Tail recent Codex session files from EOF and append verifier-bound tool_status phase-atom rows to the live append source without raw tool/request/response text"
    );
    println!(
        "  phase-stream-phase-atom-run-check-discovery-v1 [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Compile verifier-bound run_check phase atoms into a quarantine .nwpc candidate"
    );
    println!(
        "  phase-stream-phase-atom-run-check-time-split-discovery-v1 [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [train-permille] [phase-atom-trace-jsonl ...]"
    );
    println!("            Compile older run_check phase atoms and shadow-score newer events only");
    println!(
        "  phase-stream-phase-atom-action-family-time-split-discovery-v1 [action-family] [report-json] [candidate-package-path] [cells] [margin-threshold-micro] [train-permille] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Compile older verifier-bound action-family phase atoms and shadow-score newer events only"
    );
    println!(
        "  phase-stream-phase-atom-action-family-separability-audit-v1 [action-family-or-bucket] [report-json] [phase-atom-trace-jsonl ...]"
    );
    println!("            Diagnose positive/negative phase atom separability before .nwpc compile");
    println!(
        "  phase-stream-phase-atom-run-check-time-split-promotion-audit-v1 [discovery-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]"
    );
    println!(
        "            Audit time-split .nwpc promotion-candidate eligibility without serving accept"
    );
    println!(
        "  phase-stream-phase-atom-action-family-time-split-promotion-audit-v1 [discovery-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]"
    );
    println!(
        "            Audit action-family time-split .nwpc promotion-candidate eligibility without serving accept"
    );
    println!(
        "  phase-stream-phase-atom-action-family-serving-admission-audit-v1 [promotion-audit-json] [admission-report-json] [candidate-package-path] [margin-threshold-micro] [model-price-config-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Replay a quarantine .nwpc through PhaseCenterOffloadRuntime and audit serving-admission eligibility without local accept"
    );
    println!(
        "  phase-stream-phase-atom-serving-shadow-replay-v1 [shadow-report-json] [phase-atom-trace-jsonl] [serving-admission-report-json ...]"
    );
    println!(
        "            Load admitted .nwpc profiles into a shadow runtime registry and score routed trace events without local accept"
    );
    println!(
        "  phase-stream-phase-atom-serving-future-shadow-replay-v1 [shadow-report-json] [phase-atom-trace-jsonl] [serving-admission-report-json ...]"
    );
    println!(
        "            Load admitted .nwpc profiles and score only future heldout trace events after the admission train window"
    );
    println!(
        "  phase-stream-phase-atom-serving-append-shadow-replay-v1 [shadow-report-json] [watermark-trace-jsonl] [append-trace-jsonl] [serving-admission-report-json ...]"
    );
    println!(
        "            Load admitted .nwpc profiles and score only trace events newer than the watermark trace"
    );
    println!(
        "  phase-stream-phase-atom-live-admission-manifest-v1 [serving-admission-report-json] [shadow-replay-report-json] [manifest-report-json]"
    );
    println!(
        "            Combine serving-admission and fresh shadow replay into a live-eligible manifest without enabling local accept"
    );
    println!(
        "  phase-stream-phase-atom-live-admission-policy-smoke-v1 [manifest-report-json] [policy-smoke-report-json]"
    );
    println!(
        "            Consume live-admission manifest and emit a daemon-admission shadow policy smoke without enabling local accept"
    );
    println!(
        "  phase-stream-phase-atom-live-daemon-shadow-gate-v1 [policy-smoke-report-json] [live-trace-jsonl] [decision-log-jsonl] [gate-report-json] [exact-cache-watermark-trace-jsonl]"
    );
    println!(
        "            Load the admitted .nwpc fingerprint as a daemon shadow profile, write decision log rows, and keep local accept disabled"
    );
    println!(
        "  phase-stream-phase-atom-live-self-mining-loop-v1 [report-json] [candidate-dir] [cells] [min-class-events] [margin-threshold-micro] [train-permille] [top-n] [model-price-config-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Rank live action-family traffic, compile quarantine .nwpc candidates, and shadow-score them without local accept"
    );
    println!(
        "  phase-stream-global-denominator-compressibility-audit-v1 [report-json] [current5k-feedback-report-json] [phase-center-self-mining-report-json] [global-phase-atom-trace-jsonl] [mining-phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Preserve the global current5k denominator while checking phase-center mining fingerprint/bucket join evidence without promoting"
    );
    println!(
        "  phase-stream-phase-atom-compatible-denominator-shadow-v1 [report-json] [decision-log-jsonl] [self-mining-report-json] [compatible-phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Replay accepted quarantine .nwpc packages against a compatible agent-loop denominator in shadow-only mode"
    );
    println!(
        "  phase-stream-phase-atom-market-money-claim-gate-v1 [report-json] [compatible-shadow-report-json] [cost-audit-report-json] [model-price-config-json] [provider-billing-evidence-json]"
    );
    println!(
        "            Gate external money-savings claims on safe compatible shadow plus provider/user-approved price evidence without enabling local accept"
    );
    println!(
        "  phase-stream-provider-billing-evidence-join-v1 [report-json] <provider-billing-jsonl> [output-dir] [trace-jsonl ...]"
    );
    println!(
        "            Join external provider billing token/cost counters into trace copies without compile, promotion, local accept, or market claim"
    );
    println!(
        "  phase-stream-online-miner-portfolio-selector-v1 [report-json] [online-miner-report-json] [decision-log-jsonl] [max-selected-buckets]"
    );
    println!(
        "            Legacy baseline/debug selector: materialize constrained/fixed bucket baselines for comparison or NP-rescue input; not an admission path"
    );
    println!(
        "  phase-stream-online-miner-portfolio-np-rescue-v1 [report-json] [selector-report-json] [decision-log-jsonl] [max-selected-subcenters] [trace-jsonl ...]"
    );
    println!(
        "            Split unsafe fixed-greedy buckets into source-neutral Neyman-Pearson subcenters without promotion or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1 [report-json] [np-rescue-report-json]"
    );
    println!(
        "            Replay selected NP-rescue subcenters through prepared-hot .nwpc runtime without promotion or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-runtime-replay-v1 [report-json] [portfolio-selector-report-json]"
    );
    println!(
        "            Replay legacy selector baselines through prepared-hot .nwpc runtime for review only; active admission uses NP-rescue replay"
    );
    println!(
        "  phase-stream-online-miner-portfolio-future-tail-replay-v1 [report-json] [portfolio-selector-report-json] [future-trace-jsonl] [min-future-row-index]"
    );
    println!(
        "            Score already selected online .nwpc portfolio packages on later trace rows without recompiling or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-live-tail-score-only-v1 [report-json] [decision-log-jsonl] [product-hot-registry-json] [append-tail-jsonl] [idle-sleep-ms] [max-idle-ms] [max-append-events]"
    );
    println!(
        "            Tail new verifier-labeled phase-atom rows from EOF and score only clean product-hot .nwpc registry packages; no mining, compile, promotion, or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-live-tail-billing-request-v1 [report-json] [billing-request-jsonl] [live-score-report-json] [decision-log-jsonl]"
    );
    println!(
        "            Export clean live score-only .nwpc accepts into provider billing match-key requests; no money claim without external provider evidence"
    );
    println!(
        "  phase-stream-online-miner-portfolio-clean-subset-manifest-v1 [report-json] [clean-selector-report-json] <future-tail-report-json>"
    );
    println!(
        "            Materialize only future-tail zero-false auto-subcenters into a selector-compatible clean manifest; no promotion or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-future-tail-billing-request-v1 [report-json] [billing-request-jsonl] <clean-future-tail-report-json>"
    );
    println!(
        "            Export clean future-tail accepted rows as provider-billing requests without money claim, promotion, or local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-admission-gate-v1 [report-json] [runtime-replay-report-json] [provider-billing-evidence-join-report-json]"
    );
    println!(
        "            Gate NP-rescue portfolio promotion/economics on runtime replay and external provider billing evidence without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-billing-request-v1 [report-json] [billing-request-jsonl] [runtime-replay-report-json]"
    );
    println!(
        "            Export provider-billing match keys for selected online-miner portfolio accepts without money claim"
    );
    println!(
        "  phase-stream-online-miner-promotion-billing-request-v1 [report-json] [billing-request-jsonl] [promotion-registry-gate-report-json] [decision-log-jsonl]"
    );
    println!(
        "            Export provider-billing request rows for compact promoted online-miner .nwpc shadow accepts; no local accept or money claim"
    );
    println!(
        "  phase-stream-online-miner-targeted-billing-request-v1 [report-json] [billing-request-jsonl] [targeted-shadow-report-json] [targeted-decision-log-jsonl]"
    );
    println!(
        "            Export provider-billing request rows for clean product-hot targeted .nwpc shadow accepts; no local accept or money claim"
    );
    println!(
        "  phase-stream-online-miner-targeted-admission-gate-v1 [report-json] [targeted-shadow-report-json] [promotion-registry-gate-report-json] [billing-evidence-gate-report-json] [provider-coverage-report-json]"
    );
    println!(
        "            Join targeted shadow, shadow registry, provider capture coverage, and billing evidence gates without serving/local_accept"
    );
    println!(
        "  phase-stream-online-miner-promotion-provider-capture-request-v1 [report-json] [capture-request-jsonl] [billing-request-jsonl]"
    );
    println!(
        "            Convert compact promotion billing rows into provider-boundary capture worklist; no provider evidence or money claim"
    );
    println!(
        "  phase-stream-online-miner-portfolio-billing-evidence-gate-v1 [report-json] [billing-request-jsonl] <provider-billing-evidence-jsonl> [missing-request-jsonl]"
    );
    println!(
        "            Validate external provider billing evidence against selected online-miner portfolio requests without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-billing-evidence-contract-v1 [report-json] [billing-request-report-json] [template-jsonl]"
    );
    println!(
        "            Emit the exact external provider billing evidence contract for the selected online-miner portfolio"
    );
    println!(
        "  phase-stream-online-miner-portfolio-selector-billing-request-v1 [report-json] [billing-request-jsonl] <selector-report-json>"
    );
    println!(
        "            Export selected selector-shadow CPU accepts as provider billing requests; no runtime promotion or money claim without external billing evidence"
    );
    println!(
        "  phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1 [report-json] [output-billing-request-jsonl] <billing-request-jsonl> <provider-boundary-jsonl ...>"
    );
    println!(
        "            Enrich billing-request rows with provider-boundary correlation metadata; no billing evidence, promotion, local accept, or money claim"
    );
    println!(
        "  phase-stream-online-miner-portfolio-evidence-chain-audit-v1 [report-json] [runtime-replay-report-json] [billing-request-report-json] [billing-contract-report-json] [provider-normalize-report-json] [billing-evidence-gate-report-json] [admission-report-json] [promotion-report-json] [provider-correlation-audit-report-json]"
    );
    println!(
        "            Summarize selected online-miner portfolio runtime, billing, admission, and promotion blockers without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-provider-export-admission-v1 [report-json] <provider-export-jsonl> [work-dir] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]"
    );
    println!(
        "            Run provider export normalization, evidence validation, admission, promotion manifest, and chain audit without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-provider-export-autoscan-v1 [report-json] [scan-dir] [work-dir] [max-evaluated-candidates] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]"
    );
    println!(
        "            Scan local provider-export candidates for online-miner billing keys and run provider-export admission without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-provider-export-watch-v1 [report-json] [scan-dir] [work-dir] [cycles] [sleep-ms] [max-evaluated-candidates] [runtime-replay-report-json] [billing-request-report-json] [billing-request-jsonl] [billing-contract-report-json] [provider-correlation-audit-report-json]"
    );
    println!(
        "            Bounded provider-export inbox watch: repeat autoscan/admission cycles and write history without local accept or money claim"
    );
    println!(
        "  phase-stream-online-miner-portfolio-provider-correlation-audit-v1 [report-json] <jsonl ...>"
    );
    println!(
        "            Audit whether live/decision/billing rows preserve provider correlation keys for external billing join without local accept"
    );
    println!(
        "  phase-stream-automatic-discovery-chain-gate-v1 [report-json] [capture-readiness-report-json] [selector-report-json] [runtime-replay-report-json]"
    );
    println!(
        "            Join capture readiness, automatic selector, and runtime replay evidence before any dynamic discovery claim"
    );
    println!(
        "  phase-stream-phase-atom-live-capture-readiness-v1 [report-json] <phase-atom-trace-jsonl ...>"
    );
    println!(
        "            Audit whether phase-atom trace rows are ready for automatic discovery and future provider-billing join without local accept"
    );
    println!(
        "  phase-stream-provider-boundary-phase-atom-trace-v1 [report-json] [output-jsonl] <provider-boundary-event-jsonl ...>"
    );
    println!(
        "            Capture provider-boundary events as phase-atom rows with provider correlation kept as metadata, no mining, no local accept"
    );
    println!(
        "  phase-stream-provider-boundary-correlation-join-v1 [report-json] [output-jsonl] <phase-atom-trace-jsonl> <provider-boundary-jsonl ...>"
    );
    println!(
        "            Enrich existing phase-atom trace rows with provider correlation metadata from provider-boundary events without changing atoms"
    );
    println!(
        "  phase-stream-provider-boundary-match-readiness-v1 [report-json] <phase-atom-trace-jsonl ...> --provider <provider-boundary-jsonl ...>"
    );
    println!(
        "            Audit whether score-ready phase traces have provider-boundary key coverage before NP evidence chain; no mining or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-capture-request-v1 [report-json] [output-jsonl] <phase-atom-trace-jsonl ...> [--provider <provider-boundary-jsonl ...>]"
    );
    println!(
        "            Export the exact score-ready rows that still need provider-boundary capture; no mining, scoring, or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-billing-capture-contract-v1 [report-json] [template-jsonl] [template-csv] <capture-request-jsonl>"
    );
    println!(
        "            Emit the fillable live provider-boundary billing capture contract; no fabricated evidence, promotion, or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-billing-capture-evidence-gate-v1 [report-json] <capture-request-jsonl> <filled-provider-evidence-jsonl> [missing-jsonl]"
    );
    println!(
        "            Validate filled provider-boundary billing evidence against capture requests; rejects template/synthetic rows and never local-accepts"
    );
    println!(
        "  phase-stream-provider-boundary-billing-capture-chain-v1 [report-json] [artifact-prefix] <capture-request-jsonl> <phase-atom-trace-jsonl ...> --provider-evidence <filled-provider-evidence-jsonl>"
    );
    println!(
        "            Run evidence-gate-first provider-boundary billing chain; blocks template/synthetic evidence before append/join and never local-accepts"
    );
    println!(
        "  phase-stream-provider-boundary-codex-token-backfill-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <phase-atom-trace-jsonl ...>"
    );
    println!(
        "            Backfill provider-boundary token metadata from local Codex token_count events; no provider request id, cost claim, serving, promotion, or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-realtrace-token-cost-backfill-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <phase-atom-trace-jsonl ...>"
    );
    println!(
        "            Backfill provider-boundary token metadata from embedded realtrace token_cost rows; no provider request id, cost claim, serving, promotion, or local accept"
    );
    println!(
        "  phase-stream-provider-export-acquisition-pack-v1 [report-json] [output-dir] [billing-request-jsonl]"
    );
    println!(
        "            Export a provider billing worklist/schema from verifier-bound .nwpc billing requests; requires real provider export later and never claims money"
    );
    println!(
        "  phase-stream-provider-export-evidence-chain-v1 [report-json] [work-dir] [billing-request-jsonl] [provider-boundary-capture-request-jsonl] [provider-export-jsonl]"
    );
    println!(
        "            Join a real provider export to verifier-bound .nwpc billing requests through ingest/backfill/normalize/evidence gates; no local accept"
    );
    println!(
        "  phase-stream-provider-boundary-capture-coverage-gate-v1 [report-json] <capture-request-jsonl> --provider <provider-boundary-jsonl ...>"
    );
    println!(
        "            Gate whether provider-boundary rows cover the capture-request worklist before match-readiness/NP evidence chain"
    );
    println!(
        "  phase-stream-provider-boundary-export-ingest-v1 [report-json] [output-provider-boundary-jsonl] <capture-request-jsonl> <provider-export-jsonl ...>"
    );
    println!(
        "            Normalize external provider export rows into provider-boundary metadata rows for capture coverage; no mining or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-append-sink-v1 [report-json] [append-provider-boundary-jsonl] <provider-event-jsonl|- ...>"
    );
    println!(
        "            Append live/provider boundary metadata rows from files or stdin; no mining, scoring, serving, or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-live-chain-v1 [report-json] [artifact-prefix] <capture-request-jsonl> <phase-atom-trace-jsonl ...> --provider-events <provider-event-jsonl|- ...>"
    );
    println!(
        "            Append provider events, run capture coverage and match-readiness gates in one evidence chain; no serving or local accept"
    );
    println!(
        "  phase-stream-provider-boundary-live-np-chain-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <capture-request-jsonl> <score-ready-phase-atom-trace-jsonl> --provider-events <provider-event-jsonl|- ...>"
    );
    println!(
        "            Append provider events, run capture coverage and cold NP evidence chain in one path; no serving, promotion, local accept, or money claim"
    );
    println!(
        "  phase-stream-provider-boundary-np-chain-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <provider-boundary-event-jsonl ...>"
    );
    println!(
        "            Run provider-boundary capture/join/miner/NP/billing evidence chain as cold proof only; no serving, promotion, local accept, or money claim"
    );
    println!(
        "  phase-stream-provider-boundary-np-chain-from-phase-trace-v1 [report-json] [artifact-prefix] [provider-export-jsonl-or--] <score-ready-phase-atom-trace-jsonl> <provider-boundary-event-jsonl ...>"
    );
    println!(
        "            Join existing verifier-bound agent phase trace with provider-boundary keys, then run the same cold NP evidence chain without local accept"
    );
    println!(
        "  phase-stream-online-miner-portfolio-provider-export-normalize-v1 [report-json] [billing-request-jsonl] <provider-export-jsonl> [normalized-evidence-jsonl]"
    );
    println!(
        "            Normalize external provider export rows into provider billing evidence JSONL for the selected online-miner portfolio"
    );
    println!(
        "  phase-stream-online-miner-portfolio-promotion-manifest-v1 [report-json] [admission-gate-report-json] [billing-contract-report-json]"
    );
    println!(
        "            Freeze selected online-miner portfolio promotion evidence and blockers without serving mutation"
    );
    println!(
        "  phase-stream-phase-atom-frontier-billing-request-v1 [report-json] [billing-request-jsonl] [frontier-shadow-replay-report-json]"
    );
    println!(
        "            Export exact provider-billing request keys for frontier CPU accepts missing provider cost evidence"
    );
    println!(
        "  phase-stream-phase-atom-frontier-shadow-replay-v1 [report-json] [decision-log-jsonl] [frontier-union-report-json] [phase-atom-trace-jsonl ...]"
    );
    println!(
        "            Replay safe phase-atom frontier .nwpc packages after their train windows without local accept"
    );
    println!(
        "  phase-stream-phase-atom-frontier-claim-audit-v1 [claim-audit-report-json] [frontier-shadow-replay-report-json]"
    );
    println!(
        "            Split CPU10, safety, diversity, and money gates for frontier .nwpc shadow replay reports"
    );
    println!(
        "  phase-stream-phase-atom-diversity-backlog-v1 [backlog-report-json] [claim-audit-report-json] [verifier-needed-ranking-json]"
    );
    println!(
        "            Calculate the non-top verified-accept gap and rank next verifier-bound phase-center mining targets"
    );
    println!(
        "  phase-stream-real-traffic-separator-audit-v1 [report-json] [min-true-over-exact] [top-n] [trace-jsonl ...]"
    );
    println!(
        "            Rank request-side separator atoms for future phase-center bucket experiments"
    );
    println!(
        "  phase-stream-real-traffic-guarded-separator-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [max-guards] [separator-report-json] [trace-jsonl ...]"
    );
    println!("            Run candidate-selected separator-guarded phase-center shadow review");
    println!(
        "  phase-stream-real-traffic-guarded-separator-split-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [margin-threshold-micro] [max-guards] [selector-permille] [train-permille] [trace-jsonl ...]"
    );
    println!("            Run split-window selector/train/shadow guarded phase-center review");
    println!(
        "  phase-stream-real-traffic-guarded-separator-calibrated-split-shadow-v1 [report-json] [candidate-dir] [cells] [min-bucket-events] [calibration-floor-micro] [calibration-guard-micro] [max-guards] [selector-permille] [compile-permille] [calibration-permille] [trace-jsonl ...]"
    );
    println!(
        "            Run route-local selector/compile/calibration/shadow guarded phase-center review"
    );
    println!("  phase-stream-real-traffic-cost-evidence-audit-v1 [report-json] [trace-jsonl ...]");
    println!(
        "            Rank non-legacy real-traffic phase-center buckets by verifier plus token/cost evidence"
    );
    println!(
        "  phase-stream-real-traffic-token-cost-enrich-v1 [report-json] [readiness-report-json] [output-dir] [trace-jsonl ...]"
    );
    println!(
        "            Copy readiness-report token/cost estimates into real-traffic traces by request fingerprint"
    );
    println!(
        "  phase-stream-test-output-parse-promotion-audit-v1 [trace-jsonl] [shadow-report-json] [candidate-package-path] [audit-report-json] [margin-threshold-micro] [model-price-config-json]"
    );
    println!(
        "            Audit quarantine .nwpc metadata-status eligibility and estimated savings without serving accept"
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
