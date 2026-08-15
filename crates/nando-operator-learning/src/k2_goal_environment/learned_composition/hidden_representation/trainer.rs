use std::io::{Read, Write};

use super::super::{K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1};
use super::model::{
    K2_REPRESENTATION_FEATURE_SCALE_V1, K2_REPRESENTATION_FEATURES_V1,
    K2_REPRESENTATION_HIDDEN_UNITS_V1, K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1,
    K2_REPRESENTATION_MODEL_SCHEMA_V1, K2MeaningPolicySnapshotV1, K2RepresentationTrainerRequestV1,
    representation_bytes_v1, representation_decode_v1, representation_executable_matches_v1,
};

const WEIGHT_CLIP_V1: i64 = 100_000;
const PAIR_MARGIN_V1: i64 = 100;

pub fn train_hidden_representation_v1(
    request: &K2RepresentationTrainerRequestV1,
) -> K2CompositionResultV1<K2MeaningPolicySnapshotV1> {
    request.validate()?;
    let mut weights = initial_linear_weights_v1();
    let mut update_count = 0_u64;
    for epoch in 0..request.epochs {
        let learning_rate = match epoch {
            0..=63 => 8,
            64..=127 => 4,
            128..=191 => 2,
            _ => 1,
        };
        for group in &request.corpus.groups {
            let positives = group
                .rows
                .iter()
                .filter(|row| row.positive_continuation)
                .collect::<Vec<_>>();
            let negatives = group
                .rows
                .iter()
                .filter(|row| !row.positive_continuation)
                .collect::<Vec<_>>();
            for positive in &positives {
                for negative in &negatives {
                    let positive_score = linear_score_v1(&weights, &positive.features.values);
                    let negative_score = linear_score_v1(&weights, &negative.features.values);
                    if positive_score <= negative_score.saturating_add(PAIR_MARGIN_V1) {
                        for (index, weight) in weights.iter_mut().enumerate() {
                            let delta = positive.features.values[index]
                                .saturating_sub(negative.features.values[index])
                                .saturating_div(K2_REPRESENTATION_FEATURE_SCALE_V1)
                                .saturating_mul(learning_rate);
                            *weight = weight
                                .saturating_add(delta)
                                .clamp(-WEIGHT_CLIP_V1, WEIGHT_CLIP_V1);
                        }
                        update_count += 1;
                    }
                }
            }
        }
    }
    build_snapshot_v1(request, weights, update_count, "trained")
}

pub fn initial_hidden_representation_control_v1(
    request: &K2RepresentationTrainerRequestV1,
) -> K2CompositionResultV1<K2MeaningPolicySnapshotV1> {
    request.validate()?;
    build_snapshot_v1(
        request,
        initial_linear_weights_v1(),
        0,
        "frozen_initialization",
    )
}

pub fn retrain_with_permuted_labels_control_v1(
    request: &K2RepresentationTrainerRequestV1,
) -> K2CompositionResultV1<K2MeaningPolicySnapshotV1> {
    let mut permuted = request.clone();
    for group in &mut permuted.corpus.groups {
        let mut labels = group
            .rows
            .iter()
            .map(|row| row.positive_continuation)
            .collect::<Vec<_>>();
        labels.rotate_left(1);
        for (row, label) in group.rows.iter_mut().zip(labels) {
            row.positive_continuation = label;
            row.row_root_sha256 = super::super::composition_root_v1(&(
                "nando.k2-representation-training-row.v1",
                &row.features,
                label,
            ))?;
        }
    }
    permuted.corpus.corpus_root_sha256 = super::super::composition_root_v1(&(
        "nando.k2-representation-permuted-corpus-control.v1",
        &permuted.corpus.groups,
    ))?;
    permuted.request_root_sha256 = super::super::composition_root_v1(&(
        "nando.k2-representation-permuted-request-control.v1",
        &permuted.corpus,
    ))?;
    let mut weights = initial_linear_weights_v1();
    let mut update_count = 0_u64;
    for _ in 0..permuted.epochs {
        for group in &permuted.corpus.groups {
            let positives = group
                .rows
                .iter()
                .filter(|row| row.positive_continuation)
                .collect::<Vec<_>>();
            let negatives = group
                .rows
                .iter()
                .filter(|row| !row.positive_continuation)
                .collect::<Vec<_>>();
            for positive in &positives {
                for negative in &negatives {
                    if linear_score_v1(&weights, &positive.features.values)
                        <= linear_score_v1(&weights, &negative.features.values)
                            .saturating_add(PAIR_MARGIN_V1)
                    {
                        for (index, weight) in weights.iter_mut().enumerate() {
                            *weight = weight
                                .saturating_add(
                                    positive.features.values[index]
                                        .saturating_sub(negative.features.values[index])
                                        .saturating_div(K2_REPRESENTATION_FEATURE_SCALE_V1),
                                )
                                .clamp(-WEIGHT_CLIP_V1, WEIGHT_CLIP_V1);
                        }
                        update_count += 1;
                    }
                }
            }
        }
    }
    let mut snapshot = build_snapshot_v1(&permuted, weights, update_count, "permuted_labels")?;
    snapshot.trainer_request_root_sha256 = request.request_root_sha256.clone();
    snapshot.corpus_root_sha256 = request.corpus.corpus_root_sha256.clone();
    snapshot.reseal()?;
    Ok(snapshot)
}

pub fn run_representation_trainer_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_representation_trainer_stdin"))?;
    let request: K2RepresentationTrainerRequestV1 = representation_decode_v1(&input)?;
    representation_executable_matches_v1(&request.trainer_executable_sha256)?;
    let model = train_hidden_representation_v1(&request)?;
    std::io::stdout()
        .write_all(&representation_bytes_v1(&model)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_trainer_stdout"))
}

fn build_snapshot_v1(
    request: &K2RepresentationTrainerRequestV1,
    weights: Vec<i64>,
    update_count: u64,
    control_variant: &str,
) -> K2CompositionResultV1<K2MeaningPolicySnapshotV1> {
    let correctly_ranked_pairs = correctly_ranked_pairs_v1(request, &weights);
    let mut encoder_weights =
        vec![vec![0_i64; K2_REPRESENTATION_FEATURES_V1]; K2_REPRESENTATION_HIDDEN_UNITS_V1];
    encoder_weights[0] = weights.clone();
    encoder_weights[1] = weights.iter().map(|weight| -*weight).collect();
    let mut output_weights = vec![0_i64; K2_REPRESENTATION_HIDDEN_UNITS_V1];
    output_weights[0] = 1;
    output_weights[1] = -1;
    let mut snapshot = K2MeaningPolicySnapshotV1 {
        schema: K2_REPRESENTATION_MODEL_SCHEMA_V1.to_owned(),
        trainer_executable_sha256: request.trainer_executable_sha256.clone(),
        trainer_request_root_sha256: request.request_root_sha256.clone(),
        corpus_root_sha256: request.corpus.corpus_root_sha256.clone(),
        feature_language_root_sha256: request.corpus.feature_language_root_sha256.clone(),
        encoder_weights,
        output_weights,
        epochs: request.epochs,
        update_count,
        training_pairs: request.corpus.pair_count,
        correctly_ranked_pairs,
        nonzero_parameters: 0,
        parameter_l1: 0,
        control_variant: control_variant.to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        model_root_sha256: String::new(),
    };
    snapshot.reseal()?;
    snapshot.validate()?;
    Ok(snapshot)
}

fn correctly_ranked_pairs_v1(request: &K2RepresentationTrainerRequestV1, weights: &[i64]) -> u64 {
    request
        .corpus
        .groups
        .iter()
        .map(|group| {
            let positives = group
                .rows
                .iter()
                .filter(|row| row.positive_continuation)
                .collect::<Vec<_>>();
            let negatives = group
                .rows
                .iter()
                .filter(|row| !row.positive_continuation)
                .collect::<Vec<_>>();
            positives
                .iter()
                .flat_map(|positive| negatives.iter().map(move |negative| (*positive, *negative)))
                .filter(|(positive, negative)| {
                    linear_score_v1(weights, &positive.features.values)
                        > linear_score_v1(weights, &negative.features.values)
                })
                .count() as u64
        })
        .sum()
}

fn linear_score_v1(weights: &[i64], features: &[i64]) -> i64 {
    weights
        .iter()
        .zip(features)
        .map(|(weight, feature)| weight.saturating_mul(*feature))
        .sum::<i64>()
        .saturating_div(K2_REPRESENTATION_FEATURE_SCALE_V1)
}

fn initial_linear_weights_v1() -> Vec<i64> {
    (0..K2_REPRESENTATION_FEATURES_V1)
        .map(|index| match index % 4 {
            0 => 2,
            1 => -1,
            2 => 1,
            _ => -2,
        })
        .collect()
}
