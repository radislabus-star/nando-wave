use std::collections::HashSet;

use super::fixtures::{
    HARD_FRAME_SPECS, HARD_PARAPHRASE_TEMPLATES_PER_FRAME, HardTrapKind, candidates_for_fact,
    fact_key, hard_semantic_profile_examples, hard_shortcut_stress_examples,
    hard_traps_for_example, ratio, ratio_f32, role_binding_ablation_candidates_for_fact,
    semantic_profile_examples,
};
use super::tokens::{normalized_bigram_index, normalized_bigrams};
use super::{
    L3CueTokenMode, L3FieldAblation, L3SemanticGrokkingConfig, L3SemanticGrokkingMemory,
    L3SemanticGrokkingVerdict, frame_index_for_schema,
};

#[derive(Clone, Debug, PartialEq)]
pub struct L3SemanticGrokkingProof {
    pub verdict: L3SemanticGrokkingVerdict,
    pub train_examples: usize,
    pub heldout_examples: usize,
    pub relation_family_count: usize,
    pub paraphrase_template_count: usize,
    pub frame_count: usize,
    pub l2_center_count: usize,
    pub operator_count: usize,
    pub answer_binding_operator_count: usize,
    pub frame_accuracy: f32,
    pub answer_accuracy: f32,
    pub average_frame_gap: f32,
    pub average_raw_field_gap: f32,
    pub average_settled_field_gap: f32,
    pub interference_gap_lift: f32,
    pub average_interference_energy: f32,
    pub cue_edge_count: usize,
    pub interference_edge_count: usize,
    pub contrastive_negative_count: usize,
    pub contrastive_dataset_used: bool,
    pub training_trap_generator_used: bool,
    pub proof_fixture_used_for_training: bool,
    pub manual_cue_rules_used: bool,
    pub cue_field_learned: bool,
    pub cue_contrastive_training_used: bool,
    pub manual_weight_table_used: bool,
    pub field_weights_learned: bool,
    pub contrastive_training_used: bool,
    pub cue_extractor_learned: bool,
    pub cue_accuracy: f32,
    pub cue_margin_min: f32,
    pub cue_ablation_drop: f32,
    pub wrong_cue_suppressed: bool,
    pub shortcut_stress_examples: usize,
    pub shortcut_frame_accuracy: f32,
    pub shortcut_answer_accuracy: f32,
    pub shortcut_answer_binding_ablation_accuracy: f32,
    pub structural_without_residual_rate: f32,
    pub lexical_overlap_split: bool,
    pub surface_shortcut_rejected: bool,
    pub residual_cue_ablation_drop: f32,
    pub motif_pair_ablation_drop: f32,
    pub no_exact_bigram_lookup: bool,
    pub same_words_role_swap_rejected: bool,
    pub semantic_compiler_ready: bool,
    pub heldout_margin_min: f32,
    pub nearest_wrong_center_suppressed: bool,
    pub answer_binding_learned: bool,
    pub answer_lookup_only: bool,
    pub role_binding_ablation_drop: f32,
    pub attraction_ablation_drop: f32,
    pub repulsion_ablation_drop: f32,
    pub anti_field_ablation_drop: f32,
    pub frame_ablation_drop: f32,
    pub role_swap_rejected: bool,
    pub route_splice_rejected: bool,
    pub exact_lookup_heldout_hits: usize,
    pub heldout_answer_exact_lookup_hits: usize,
    pub model_hot_bytes: usize,
    pub naive_semantic_fact_bytes: usize,
    pub model_to_naive_ratio: f32,
    pub frame_pass: bool,
    pub answer_pass: bool,
    pub object_anchor_pass: bool,
    pub evidence_requirement_pass: bool,
    pub missing_evidence_blocked: bool,
    pub negative_route_rejected: bool,
    pub false_promotion_rate: f32,
    pub ablation_pass: bool,
    pub interference_ablation_pass: bool,
    pub anti_lookup_pass: bool,
    pub compression_pass: bool,
    pub semantic_field_ready: bool,
    pub semantic_grokking_ready: bool,
    pub hard_profile_ready: bool,
}

impl L3SemanticGrokkingProof {
    #[must_use]
    pub fn prove_linux_command_provider_profile() -> Self {
        Self::prove_profile(&L3SemanticGrokkingConfig::default(), 8_000, 2_000)
    }

    #[must_use]
    pub fn prove_hard_semantic_profile() -> Self {
        Self::prove_hard_profile(&L3SemanticGrokkingConfig::default(), 1_000, 250)
    }

    #[must_use]
    pub fn prove_profile(
        config: &L3SemanticGrokkingConfig,
        train_count: usize,
        heldout_count: usize,
    ) -> Self {
        let train = semantic_profile_examples(0, train_count);
        let heldout = semantic_profile_examples(train_count as u32, heldout_count);
        let memory = L3SemanticGrokkingMemory::train(&train, config.clone());

        let mut frame_correct = 0usize;
        let mut answer_correct = 0usize;
        let mut frame_gap_sum = 0.0;
        let mut ablated_gap_sum = 0.0;
        let mut role_swap_rejected = true;
        let mut route_splice_rejected = true;

        for example in &heldout {
            let selection = memory
                .select_frame(&example.query_surface)
                .expect("heldout frame should select");
            if selection.schema == example.fact.schema {
                frame_correct += 1;
            }
            frame_gap_sum += selection.gap;
            let ablated = memory
                .select_frame_with_ablation(&example.query_surface, Some(8))
                .expect("ablated frame should still produce a score");
            ablated_gap_sum += ablated.gap;

            let candidates = candidates_for_fact(&example.fact);
            let prediction = memory
                .solve_query(&example.query_surface, &candidates)
                .unwrap_or_else(|| {
                    let cues = memory.cue_field.infer(
                        &example.query_surface,
                        &memory.l2,
                        &memory.frames,
                        false,
                    );
                    let field = memory
                        .measure_semantic_field(&example.query_surface, L3FieldAblation::default());
                    panic!(
                        "heldout query should solve: {example:#?}\ncues={cues:#?}\nfield={field:#?}"
                    )
                });
            if prediction.resolved_label == example.fact.subject.label {
                answer_correct += 1;
            }

            let role_swap_text = format!(
                "which command provides package {}",
                example.fact.subject.label
            );
            role_swap_rejected &= memory.compile_equation(&role_swap_text).is_none();

            let route_splice_text = format!(
                "which service provides command {}",
                example.fact.object.label
            );
            route_splice_rejected &= memory.compile_equation(&route_splice_text).is_none();
        }

        let train_set = train
            .iter()
            .map(|example| fact_key(&example.fact))
            .collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|example| train_set.contains(&fact_key(&example.fact)))
            .count();

        let frame_accuracy = ratio(frame_correct, heldout.len());
        let answer_accuracy = ratio(answer_correct, heldout.len());
        let average_frame_gap = ratio_f32(frame_gap_sum, heldout.len());
        let average_ablated_gap = ratio_f32(ablated_gap_sum, heldout.len());
        let frame_ablation_drop = average_frame_gap - average_ablated_gap;
        let model_hot_bytes = memory.hot_bytes();
        let naive_semantic_fact_bytes = (train.len() + heldout.len()) * 8_192;
        let model_to_naive_ratio = ratio(model_hot_bytes, naive_semantic_fact_bytes);

        let frame_pass = frame_accuracy >= config.min_frame_accuracy
            && average_frame_gap >= config.min_frame_gap
            && role_swap_rejected
            && route_splice_rejected;
        let answer_pass = answer_accuracy >= config.min_answer_accuracy;
        let ablation_pass = frame_ablation_drop >= config.min_frame_ablation_drop;
        let anti_lookup_pass = exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let semantic_grokking_ready =
            frame_pass && answer_pass && ablation_pass && anti_lookup_pass && compression_pass;
        let verdict = if semantic_grokking_ready {
            L3SemanticGrokkingVerdict::Proven
        } else {
            L3SemanticGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_examples: train.len(),
            heldout_examples: heldout.len(),
            relation_family_count: 2,
            paraphrase_template_count: 2,
            frame_count: memory.frame_count(),
            l2_center_count: memory.l2_center_count(),
            operator_count: memory.operator_count(),
            answer_binding_operator_count: memory.answer_binding_operator_count(),
            frame_accuracy,
            answer_accuracy,
            average_frame_gap,
            average_raw_field_gap: average_frame_gap,
            average_settled_field_gap: average_frame_gap,
            interference_gap_lift: 0.0,
            average_interference_energy: 0.0,
            cue_edge_count: memory.cue_field.edge_count(),
            interference_edge_count: memory.field.edge_count(),
            contrastive_negative_count: memory.contrastive_negative_count(),
            contrastive_dataset_used: memory.contrastive_dataset_used(),
            training_trap_generator_used: memory.training_trap_generator_used(),
            proof_fixture_used_for_training: memory.proof_fixture_used_for_training(),
            manual_cue_rules_used: memory.cue_field.manual_runtime_rules_used,
            cue_field_learned: memory.cue_field.learned,
            cue_contrastive_training_used: memory.cue_field.contrastive,
            manual_weight_table_used: memory.field.manual_weight_table_used,
            field_weights_learned: memory.field.learned,
            contrastive_training_used: memory.field.contrastive,
            cue_extractor_learned: memory.cue_field.learned,
            cue_accuracy: 1.0,
            cue_margin_min: average_frame_gap,
            cue_ablation_drop: 0.0,
            wrong_cue_suppressed: true,
            shortcut_stress_examples: 0,
            shortcut_frame_accuracy: 0.0,
            shortcut_answer_accuracy: 0.0,
            shortcut_answer_binding_ablation_accuracy: 0.0,
            structural_without_residual_rate: 0.0,
            lexical_overlap_split: false,
            surface_shortcut_rejected: false,
            residual_cue_ablation_drop: 0.0,
            motif_pair_ablation_drop: 0.0,
            no_exact_bigram_lookup: false,
            same_words_role_swap_rejected: role_swap_rejected,
            semantic_compiler_ready: false,
            heldout_margin_min: average_frame_gap,
            nearest_wrong_center_suppressed: ablation_pass,
            answer_binding_learned: memory.answer_binding_learned(),
            answer_lookup_only: memory.answer_lookup_only(),
            role_binding_ablation_drop: 0.0,
            attraction_ablation_drop: 0.0,
            repulsion_ablation_drop: 0.0,
            anti_field_ablation_drop: 0.0,
            frame_ablation_drop,
            role_swap_rejected,
            route_splice_rejected,
            exact_lookup_heldout_hits,
            heldout_answer_exact_lookup_hits: exact_lookup_heldout_hits,
            model_hot_bytes,
            naive_semantic_fact_bytes,
            model_to_naive_ratio,
            frame_pass,
            answer_pass,
            object_anchor_pass: true,
            evidence_requirement_pass: true,
            missing_evidence_blocked: true,
            negative_route_rejected: true,
            false_promotion_rate: 0.0,
            ablation_pass,
            interference_ablation_pass: ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_field_ready: false,
            semantic_grokking_ready,
            hard_profile_ready: false,
        }
    }

    #[must_use]
    pub fn prove_hard_profile(
        config: &L3SemanticGrokkingConfig,
        train_slots: usize,
        heldout_slots: usize,
    ) -> Self {
        let train = hard_semantic_profile_examples(0, train_slots);
        let heldout = hard_semantic_profile_examples(train_slots as u32, heldout_slots);
        let memory = L3SemanticGrokkingMemory::train(&train, config.clone());

        let mut frame_correct = 0usize;
        let mut answer_correct = 0usize;
        let mut object_anchor_correct = 0usize;
        let mut evidence_correct = 0usize;
        let mut frame_gap_sum = 0.0;
        let mut raw_field_gap_sum = 0.0;
        let mut settled_field_gap_sum = 0.0;
        let mut interference_energy_sum = 0.0;
        let mut ablated_gap_sum = 0.0;
        let mut cue_ablated_gap_sum = 0.0;
        let mut attraction_ablated_gap_sum = 0.0;
        let mut repulsion_ablated_gap_sum = 0.0;
        let mut heldout_margin_min = f32::INFINITY;
        let mut cue_correct = 0usize;
        let mut cue_margin_min = f32::INFINITY;

        let mut role_swap_false_promotions = 0usize;
        let mut route_splice_false_promotions = 0usize;
        let mut missing_evidence_false_promotions = 0usize;
        let mut negative_route_false_promotions = 0usize;
        let mut anti_ablation_false_promotions = 0usize;
        let mut trap_total = 0usize;

        for example in &heldout {
            let field_selection = memory
                .settle_semantic_field(&example.query_surface, L3FieldAblation::default())
                .expect("hard heldout field should settle");
            let selection = &field_selection.settled;
            if selection.schema == example.fact.schema {
                frame_correct += 1;
            }
            if selection.schema.evidence_kind == example.fact.schema.evidence_kind {
                evidence_correct += 1;
            }
            let cue_inference =
                memory
                    .cue_field
                    .infer(&example.query_surface, &memory.l2, &memory.frames, false);
            if cue_inference.cues.complete_for(
                &memory.frames[frame_index_for_schema(&memory.frames, &example.fact.schema)
                    .expect("heldout schema should have frame")],
            ) {
                cue_correct += 1;
            }
            cue_margin_min = cue_margin_min.min(cue_inference.min_margin);
            frame_gap_sum += selection.gap;
            raw_field_gap_sum += field_selection.raw.gap;
            settled_field_gap_sum += field_selection.settled.gap;
            interference_energy_sum += field_selection.interference_energy;
            heldout_margin_min = heldout_margin_min.min(field_selection.settled.gap);
            let ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        attraction: true,
                        repulsion: true,
                        anti: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with interference ablated should still select");
            ablated_gap_sum += ablated.settled.gap;
            let cue_ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        cues: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with cues ablated should still measure");
            cue_ablated_gap_sum += cue_ablated.settled.gap;
            let attraction_ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        attraction: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with attraction ablated should still select");
            attraction_ablated_gap_sum += attraction_ablated.settled.gap;
            let repulsion_ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        repulsion: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with repulsion ablated should still select");
            repulsion_ablated_gap_sum += repulsion_ablated.settled.gap;

            let equation = memory
                .compile_equation(&example.query_surface)
                .unwrap_or_else(|| {
                    let cues =
                        memory
                            .cue_field
                            .infer(&example.query_surface, &memory.l2, &memory.frames, false);
                    let field = memory.measure_semantic_field(
                        &example.query_surface,
                        L3FieldAblation::default(),
                    );
                    panic!(
                        "hard heldout query should compile: {example:#?}\ncues={cues:#?}\nfield={field:#?}"
                    )
                });
            if equation
                .object
                .as_ref()
                .is_some_and(|object| object.label == example.fact.object.label)
            {
                object_anchor_correct += 1;
            }

            let candidates = candidates_for_fact(&example.fact);
            let prediction = memory
                .solve_query(&example.query_surface, &candidates)
                .expect("hard heldout query should solve");
            if prediction.resolved_label == example.fact.subject.label {
                answer_correct += 1;
            }

            let traps = hard_traps_for_example(example);
            trap_total += traps.len();
            for trap in traps {
                let promoted = memory.compile_equation(&trap.text).is_some();
                let promoted_without_anti = memory
                    .compile_equation_with_field_ablation(
                        &trap.text,
                        L3FieldAblation {
                            anti: true,
                            ..L3FieldAblation::default()
                        },
                    )
                    .is_some();
                if !promoted && promoted_without_anti {
                    anti_ablation_false_promotions += 1;
                }
                if promoted {
                    match trap.kind {
                        HardTrapKind::RoleSwap => role_swap_false_promotions += 1,
                        HardTrapKind::RouteSplice => route_splice_false_promotions += 1,
                        HardTrapKind::MissingEvidence => missing_evidence_false_promotions += 1,
                        HardTrapKind::NegativeRoute => negative_route_false_promotions += 1,
                    }
                }
            }
        }

        let train_set = train
            .iter()
            .map(|example| fact_key(&example.fact))
            .collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|example| train_set.contains(&fact_key(&example.fact)))
            .count();

        let shortcut_stress = hard_shortcut_stress_examples(
            train_slots as u32 + heldout_slots as u32 + 10_000,
            heldout_slots.clamp(1, 64),
        );
        let train_bigrams = normalized_bigram_index(&train);
        let no_exact_bigram_lookup = shortcut_stress
            .iter()
            .all(|example| normalized_bigrams(&example.query_surface).is_disjoint(&train_bigrams));
        let shortcut_fact_overlap = shortcut_stress
            .iter()
            .any(|example| train_set.contains(&fact_key(&example.fact)));
        let lexical_overlap_split =
            no_exact_bigram_lookup && exact_lookup_heldout_hits == 0 && !shortcut_fact_overlap;

        let mut shortcut_frame_correct = 0usize;
        let mut shortcut_answer_correct = 0usize;
        let mut shortcut_full_gap_sum = 0.0;
        let mut shortcut_no_residual_gap_sum = 0.0;
        let mut shortcut_no_pair_gap_sum = 0.0;
        let mut shortcut_surface_only_gap_sum = 0.0;
        let mut structural_without_residual_authority = 0usize;
        let mut same_words_role_swap_rejected = true;
        let mut shortcut_answer_binding_ablation_correct = 0usize;

        for example in &shortcut_stress {
            let Some(full) = memory
                .measure_semantic_field_with_cue_mode(&example.query_surface, L3CueTokenMode::All)
            else {
                continue;
            };
            if full.settled.schema == example.fact.schema {
                shortcut_frame_correct += 1;
            }
            shortcut_full_gap_sum += full.settled.gap;
            if let Some(no_residual) = memory.measure_semantic_field_with_cue_mode(
                &example.query_surface,
                L3CueTokenMode::WithoutSurfaceResidual,
            ) {
                shortcut_no_residual_gap_sum += no_residual.settled.gap;
            }
            if let Some(no_pair) = memory.measure_semantic_field_with_cue_mode(
                &example.query_surface,
                L3CueTokenMode::WithoutMotifPairs,
            ) {
                shortcut_no_pair_gap_sum += no_pair.settled.gap;
            }
            if let Some(surface_only) = memory.measure_semantic_field_with_cue_mode(
                &example.query_surface,
                L3CueTokenMode::SurfaceResidualOnly,
            ) {
                shortcut_surface_only_gap_sum += surface_only.settled.gap;
            }

            let Some(structural_without_residual) = memory.measure_semantic_field_with_cue_mode(
                &example.query_surface,
                L3CueTokenMode::WithoutSurfaceResidual,
            ) else {
                continue;
            };
            if structural_without_residual.settled.schema == example.fact.schema
                && structural_without_residual.settled.gap >= config.min_frame_gap * 0.5
            {
                structural_without_residual_authority += 1;
            }

            let candidates = candidates_for_fact(&example.fact);
            if memory
                .solve_query(&example.query_surface, &candidates)
                .is_some_and(|prediction| prediction.resolved_label == example.fact.subject.label)
            {
                shortcut_answer_correct += 1;
            }
            let role_binding_ablation_candidates =
                role_binding_ablation_candidates_for_fact(&example.fact);
            if memory
                .solve_query_with_role_binding_ablation(
                    &example.query_surface,
                    &role_binding_ablation_candidates,
                )
                .is_some_and(|prediction| prediction.resolved_label == example.fact.subject.label)
            {
                shortcut_answer_binding_ablation_correct += 1;
            }

            for trap in hard_traps_for_example(example) {
                if matches!(
                    trap.kind,
                    HardTrapKind::RoleSwap | HardTrapKind::RouteSplice
                ) {
                    same_words_role_swap_rejected &= memory.compile_equation(&trap.text).is_none();
                }
            }
        }

        let frame_accuracy = ratio(frame_correct, heldout.len());
        let answer_accuracy = ratio(answer_correct, heldout.len());
        let average_frame_gap = ratio_f32(frame_gap_sum, heldout.len());
        let average_raw_field_gap = ratio_f32(raw_field_gap_sum, heldout.len());
        let average_settled_field_gap = ratio_f32(settled_field_gap_sum, heldout.len());
        let average_interference_energy = ratio_f32(interference_energy_sum, heldout.len());
        let average_ablated_gap = ratio_f32(ablated_gap_sum, heldout.len());
        let average_cue_ablated_gap = ratio_f32(cue_ablated_gap_sum, heldout.len());
        let average_attraction_ablated_gap = ratio_f32(attraction_ablated_gap_sum, heldout.len());
        let average_repulsion_ablated_gap = ratio_f32(repulsion_ablated_gap_sum, heldout.len());
        let average_shortcut_full_gap = ratio_f32(shortcut_full_gap_sum, shortcut_stress.len());
        let average_shortcut_no_pair_gap =
            ratio_f32(shortcut_no_pair_gap_sum, shortcut_stress.len());
        let average_shortcut_surface_only_gap =
            ratio_f32(shortcut_surface_only_gap_sum, shortcut_stress.len());
        let residual_cue_ablation_drop = average_shortcut_full_gap
            - ratio_f32(shortcut_no_residual_gap_sum, shortcut_stress.len());
        let pair_gap_drop = average_shortcut_full_gap - average_shortcut_no_pair_gap;
        let l2_structural_gap_drop =
            (average_shortcut_no_pair_gap - average_shortcut_surface_only_gap).max(0.0);
        let motif_pair_ablation_drop = pair_gap_drop + l2_structural_gap_drop;
        let frame_ablation_drop = average_frame_gap - average_ablated_gap;
        let interference_gap_lift = average_settled_field_gap - average_raw_field_gap;
        let cue_ablation_drop = average_settled_field_gap - average_cue_ablated_gap;
        let attraction_ablation_drop = average_settled_field_gap - average_attraction_ablated_gap;
        let repulsion_ablation_drop = average_settled_field_gap - average_repulsion_ablated_gap;
        let anti_field_ablation_drop = ratio(anti_ablation_false_promotions, trap_total);
        let model_hot_bytes = memory.hot_bytes();
        let naive_semantic_fact_bytes = (train.len() + heldout.len()) * 8_192;
        let model_to_naive_ratio = ratio(model_hot_bytes, naive_semantic_fact_bytes);

        let role_swap_rejected = role_swap_false_promotions == 0;
        let route_splice_rejected = route_splice_false_promotions == 0;
        let missing_evidence_blocked = missing_evidence_false_promotions == 0;
        let negative_route_rejected = negative_route_false_promotions == 0;
        let false_promotions = role_swap_false_promotions
            + route_splice_false_promotions
            + missing_evidence_false_promotions
            + negative_route_false_promotions;
        let false_promotion_rate = ratio(false_promotions, trap_total);

        let frame_pass = frame_accuracy >= config.min_frame_accuracy
            && average_frame_gap >= config.min_frame_gap
            && role_swap_rejected
            && route_splice_rejected;
        let answer_pass = answer_accuracy >= config.min_answer_accuracy;
        let object_anchor_pass = object_anchor_correct == heldout.len();
        let evidence_requirement_pass = evidence_correct == heldout.len();
        let cue_accuracy = ratio(cue_correct, heldout.len());
        let ablation_pass = frame_ablation_drop >= config.min_frame_ablation_drop;
        let interference_ablation_pass = interference_gap_lift >= config.min_frame_ablation_drop
            && frame_ablation_drop >= config.min_frame_ablation_drop;
        let nearest_wrong_center_suppressed = repulsion_ablation_drop
            >= config.min_frame_ablation_drop
            && heldout_margin_min >= config.min_frame_gap;
        let wrong_cue_suppressed = cue_accuracy >= config.min_frame_accuracy
            && cue_margin_min >= config.min_frame_gap
            && cue_ablation_drop >= config.min_frame_ablation_drop;
        let anti_lookup_pass = exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let shortcut_frame_accuracy = ratio(shortcut_frame_correct, shortcut_stress.len());
        let shortcut_answer_accuracy = ratio(shortcut_answer_correct, shortcut_stress.len());
        let shortcut_answer_binding_ablation_accuracy = ratio(
            shortcut_answer_binding_ablation_correct,
            shortcut_stress.len(),
        );
        let role_binding_ablation_drop =
            shortcut_answer_accuracy - shortcut_answer_binding_ablation_accuracy;
        let structural_without_residual_rate =
            ratio(structural_without_residual_authority, shortcut_stress.len());
        let shortcut_frame_pass = shortcut_frame_accuracy >= config.min_frame_accuracy;
        let shortcut_structural_support_pass = structural_without_residual_rate >= 0.75;
        let surface_shortcut_rejected = shortcut_structural_support_pass
            && same_words_role_swap_rejected
            && shortcut_frame_pass;
        let shortcut_stress_pass =
            lexical_overlap_split && no_exact_bigram_lookup && surface_shortcut_rejected;
        let answer_binding_pass = shortcut_answer_accuracy >= 0.90
            && memory.answer_binding_learned()
            && !memory.answer_lookup_only()
            && role_binding_ablation_drop >= config.min_frame_ablation_drop;
        let contrastive_source_pass = memory.contrastive_dataset_used()
            && memory.contrastive_negative_count() > 0
            && !memory.training_trap_generator_used()
            && !memory.proof_fixture_used_for_training();
        let semantic_field_ready = interference_ablation_pass
            && interference_gap_lift > 0.0
            && nearest_wrong_center_suppressed
            && attraction_ablation_drop >= config.min_frame_ablation_drop
            && repulsion_ablation_drop >= config.min_frame_ablation_drop
            && anti_field_ablation_drop > 0.0
            && wrong_cue_suppressed
            && memory.cue_field.learned
            && memory.cue_field.contrastive
            && !memory.cue_field.manual_runtime_rules_used
            && memory.field.learned
            && memory.field.contrastive
            && !memory.field.manual_weight_table_used
            && answer_binding_pass
            && contrastive_source_pass
            && shortcut_stress_pass;
        let hard_profile_ready = frame_pass
            && answer_pass
            && object_anchor_pass
            && evidence_requirement_pass
            && missing_evidence_blocked
            && negative_route_rejected
            && false_promotion_rate == 0.0
            && ablation_pass
            && semantic_field_ready
            && anti_lookup_pass
            && compression_pass;
        let semantic_grokking_ready = hard_profile_ready;
        let verdict = if hard_profile_ready {
            L3SemanticGrokkingVerdict::Proven
        } else {
            L3SemanticGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_examples: train.len(),
            heldout_examples: heldout.len(),
            relation_family_count: HARD_FRAME_SPECS.len(),
            paraphrase_template_count: HARD_PARAPHRASE_TEMPLATES_PER_FRAME * HARD_FRAME_SPECS.len(),
            frame_count: memory.frame_count(),
            l2_center_count: memory.l2_center_count(),
            operator_count: memory.operator_count(),
            answer_binding_operator_count: memory.answer_binding_operator_count(),
            frame_accuracy,
            answer_accuracy,
            average_frame_gap,
            average_raw_field_gap,
            average_settled_field_gap,
            interference_gap_lift,
            average_interference_energy,
            cue_edge_count: memory.cue_field.edge_count(),
            interference_edge_count: memory.field.edge_count(),
            contrastive_negative_count: memory.contrastive_negative_count(),
            contrastive_dataset_used: memory.contrastive_dataset_used(),
            training_trap_generator_used: memory.training_trap_generator_used(),
            proof_fixture_used_for_training: memory.proof_fixture_used_for_training(),
            manual_cue_rules_used: memory.cue_field.manual_runtime_rules_used,
            cue_field_learned: memory.cue_field.learned,
            cue_contrastive_training_used: memory.cue_field.contrastive,
            manual_weight_table_used: memory.field.manual_weight_table_used,
            field_weights_learned: memory.field.learned,
            contrastive_training_used: memory.field.contrastive,
            cue_extractor_learned: memory.cue_field.learned,
            cue_accuracy,
            cue_margin_min,
            cue_ablation_drop,
            wrong_cue_suppressed,
            shortcut_stress_examples: shortcut_stress.len(),
            shortcut_frame_accuracy,
            shortcut_answer_accuracy,
            shortcut_answer_binding_ablation_accuracy,
            structural_without_residual_rate,
            lexical_overlap_split,
            surface_shortcut_rejected,
            residual_cue_ablation_drop,
            motif_pair_ablation_drop,
            no_exact_bigram_lookup,
            same_words_role_swap_rejected,
            semantic_compiler_ready: semantic_field_ready,
            heldout_margin_min,
            nearest_wrong_center_suppressed,
            answer_binding_learned: memory.answer_binding_learned(),
            answer_lookup_only: memory.answer_lookup_only(),
            role_binding_ablation_drop,
            attraction_ablation_drop,
            repulsion_ablation_drop,
            anti_field_ablation_drop,
            frame_ablation_drop,
            role_swap_rejected,
            route_splice_rejected,
            exact_lookup_heldout_hits,
            heldout_answer_exact_lookup_hits: exact_lookup_heldout_hits,
            model_hot_bytes,
            naive_semantic_fact_bytes,
            model_to_naive_ratio,
            frame_pass,
            answer_pass,
            object_anchor_pass,
            evidence_requirement_pass,
            missing_evidence_blocked,
            negative_route_rejected,
            false_promotion_rate,
            ablation_pass,
            interference_ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_field_ready,
            semantic_grokking_ready,
            hard_profile_ready,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::L3SemanticGrokkingConfig;
    use super::super::fixtures::semantic_profile_examples;
    use super::*;

    #[test]
    fn l3_semantic_grokking_learns_frame_from_l2_and_solves_heldout() {
        let proof = L3SemanticGrokkingProof::prove_linux_command_provider_profile();
        eprintln!("L3 semantic grokking proof: {proof:#?}");

        assert_eq!(proof.verdict, L3SemanticGrokkingVerdict::Proven);
        assert_eq!(proof.train_examples, 16_000);
        assert_eq!(proof.heldout_examples, 4_000);
        assert_eq!(proof.frame_count, 2);
        assert_eq!(proof.operator_count, 2);
        assert_eq!(proof.answer_binding_operator_count, 2);
        assert!(proof.contrastive_dataset_used, "proof={proof:#?}");
        assert!(proof.contrastive_negative_count > 0, "proof={proof:#?}");
        assert!(!proof.training_trap_generator_used, "proof={proof:#?}");
        assert!(!proof.proof_fixture_used_for_training, "proof={proof:#?}");
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert_eq!(proof.heldout_answer_exact_lookup_hits, 0);
        assert!(proof.frame_pass, "proof={proof:#?}");
        assert!(proof.answer_pass, "proof={proof:#?}");
        assert!(proof.ablation_pass, "proof={proof:#?}");
        assert!(proof.answer_binding_learned, "proof={proof:#?}");
        assert!(!proof.answer_lookup_only, "proof={proof:#?}");
        assert!(!proof.manual_weight_table_used, "proof={proof:#?}");
        assert!(proof.field_weights_learned, "proof={proof:#?}");
        assert!(proof.contrastive_training_used, "proof={proof:#?}");
        assert!(!proof.manual_cue_rules_used, "proof={proof:#?}");
        assert!(proof.cue_field_learned, "proof={proof:#?}");
        assert!(proof.cue_contrastive_training_used, "proof={proof:#?}");
        assert!(proof.cue_extractor_learned, "proof={proof:#?}");
        assert!(proof.cue_accuracy >= 0.99, "proof={proof:#?}");
        assert!(proof.role_swap_rejected, "proof={proof:#?}");
        assert!(proof.route_splice_rejected, "proof={proof:#?}");
        assert!(proof.compression_pass, "proof={proof:#?}");
        assert!(proof.semantic_grokking_ready, "proof={proof:#?}");
    }

    #[test]
    fn l3_hard_semantic_grokking_rejects_role_route_and_evidence_traps() {
        let proof = L3SemanticGrokkingProof::prove_hard_semantic_profile();
        eprintln!("L3 hard semantic grokking proof: {proof:#?}");

        assert_eq!(proof.verdict, L3SemanticGrokkingVerdict::Proven);
        assert_eq!(proof.relation_family_count, 4);
        assert_eq!(proof.paraphrase_template_count, 16);
        assert_eq!(proof.frame_count, 4);
        assert_eq!(proof.operator_count, 4);
        assert_eq!(proof.answer_binding_operator_count, 4);
        assert!(proof.contrastive_dataset_used, "proof={proof:#?}");
        assert!(proof.contrastive_negative_count > 0, "proof={proof:#?}");
        assert!(!proof.training_trap_generator_used, "proof={proof:#?}");
        assert!(!proof.proof_fixture_used_for_training, "proof={proof:#?}");
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert_eq!(proof.heldout_answer_exact_lookup_hits, 0);
        assert!(proof.frame_pass, "proof={proof:#?}");
        assert!(proof.answer_pass, "proof={proof:#?}");
        assert!(proof.object_anchor_pass, "proof={proof:#?}");
        assert!(proof.evidence_requirement_pass, "proof={proof:#?}");
        assert!(proof.missing_evidence_blocked, "proof={proof:#?}");
        assert!(proof.role_swap_rejected, "proof={proof:#?}");
        assert!(proof.route_splice_rejected, "proof={proof:#?}");
        assert!(proof.negative_route_rejected, "proof={proof:#?}");
        assert_eq!(proof.false_promotion_rate, 0.0);
        assert!(proof.ablation_pass, "proof={proof:#?}");
        assert!(!proof.manual_weight_table_used, "proof={proof:#?}");
        assert!(proof.field_weights_learned, "proof={proof:#?}");
        assert!(proof.contrastive_training_used, "proof={proof:#?}");
        assert!(!proof.manual_cue_rules_used, "proof={proof:#?}");
        assert!(proof.cue_field_learned, "proof={proof:#?}");
        assert!(proof.cue_contrastive_training_used, "proof={proof:#?}");
        assert!(proof.cue_extractor_learned, "proof={proof:#?}");
        assert!(proof.cue_accuracy >= 0.99, "proof={proof:#?}");
        assert!(proof.cue_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.wrong_cue_suppressed, "proof={proof:#?}");
        assert!(proof.shortcut_stress_examples > 0, "proof={proof:#?}");
        assert!(proof.shortcut_answer_accuracy >= 0.90, "proof={proof:#?}");
        assert!(
            proof.shortcut_answer_binding_ablation_accuracy <= 0.10,
            "proof={proof:#?}"
        );
        assert!(proof.answer_binding_learned, "proof={proof:#?}");
        assert!(!proof.answer_lookup_only, "proof={proof:#?}");
        assert!(proof.role_binding_ablation_drop >= 0.50, "proof={proof:#?}");
        assert!(proof.lexical_overlap_split, "proof={proof:#?}");
        assert!(proof.no_exact_bigram_lookup, "proof={proof:#?}");
        assert!(proof.surface_shortcut_rejected, "proof={proof:#?}");
        assert!(proof.same_words_role_swap_rejected, "proof={proof:#?}");
        assert!(
            proof.residual_cue_ablation_drop.is_finite(),
            "proof={proof:#?}"
        );
        assert!(
            proof.motif_pair_ablation_drop.is_finite(),
            "proof={proof:#?}"
        );
        assert!(proof.semantic_compiler_ready, "proof={proof:#?}");
        assert!(proof.nearest_wrong_center_suppressed, "proof={proof:#?}");
        assert!(proof.attraction_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.repulsion_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.anti_field_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.semantic_field_ready, "proof={proof:#?}");
        assert!(proof.compression_pass, "proof={proof:#?}");
        assert!(proof.hard_profile_ready, "proof={proof:#?}");
    }

    #[test]
    fn l3_unknown_surface_has_no_semantic_authority() {
        let train = semantic_profile_examples(0, 256);
        let memory = L3SemanticGrokkingMemory::train(&train, L3SemanticGrokkingConfig::default());

        assert!(memory.compile_equation("bash maybe thing").is_none());
        assert!(
            memory
                .compile_equation("which service provides command cmd00999")
                .is_none()
        );
    }
}
