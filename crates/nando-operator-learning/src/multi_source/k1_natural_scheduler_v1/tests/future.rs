use nando_operator_kernel::ResponseProgram;

use super::*;

fn durable_generation() -> (
    K1NaturalCandidateFreezeV1,
    K1IdentificationFreezeV1,
    K1FuturePredictionContractV1,
) {
    let candidate = candidate_freeze(1);
    let semantic_class = root(1_100);
    let identification = K1IdentificationFreezeV1::seal(
        &candidate,
        root(1_101),
        GENERATOR_SCHEMA.to_owned(),
        vec![semantic_class.clone()],
        root(1_102),
        root(1_103),
        K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1.to_owned(),
    )
    .expect("identification");
    let contract = K1FuturePredictionContractV1::seal(
        candidate.freeze_root_sha256.clone(),
        identification.freeze_root_sha256.clone(),
        semantic_class,
        root(1_104),
        ResponseProgram::advance_plan("update_plan"),
        2_000_000,
    )
    .expect("contract");
    (candidate, identification, contract)
}

fn prediction(
    candidate: &K1NaturalCandidateFreezeV1,
    identification: &K1IdentificationFreezeV1,
    contract: &K1FuturePredictionContractV1,
) -> K1FuturePredictionReceiptV1 {
    K1FuturePredictionReceiptV1::seal(
        contract.contract_root_sha256.clone(),
        candidate.freeze_root_sha256.clone(),
        identification.freeze_root_sha256.clone(),
        contract.semantic_class_root_sha256.clone(),
        root(1_105),
        root(1_106),
        root(1_107),
        root(1_108),
        &contract.canonical_program_root_sha256,
        candidate.future_min_sequence,
        1,
        3_000_000,
    )
    .expect("prediction")
}

fn ledger_before_prediction(
    candidate: &K1NaturalCandidateFreezeV1,
    identification: &K1IdentificationFreezeV1,
    contract: &K1FuturePredictionContractV1,
) -> K1SchedulerLedgerV1 {
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate.clone(),
        ))
        .expect("candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::IdentificationFreeze(
            identification.clone(),
        ))
        .expect("identification");
    ledger
        .append(K1SchedulerEventPayloadV1::FuturePredictionContract(
            contract.clone(),
        ))
        .expect("contract");
    ledger
}

#[test]
fn future_outcome_requires_an_earlier_durable_prediction() {
    let (candidate, identification, contract) = durable_generation();
    let prediction = prediction(&candidate, &identification, &contract);
    let outcome = K1FutureOutcomeReceiptV1::seal(
        prediction.prediction_root_sha256.clone(),
        root(1_109),
        root(1_110),
        root(1_111),
        root(1_112),
        4_000_000,
        true,
        true,
    )
    .expect("outcome");
    let mut ledger = ledger_before_prediction(&candidate, &identification, &contract);

    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::FutureOutcome(outcome.clone())),
        Err("k1_scheduler_future_prediction_missing")
    );
    ledger
        .append(K1SchedulerEventPayloadV1::FuturePrediction(prediction))
        .expect("prediction");
    ledger
        .append(K1SchedulerEventPayloadV1::FutureOutcome(outcome))
        .expect("outcome after prediction");
    ledger.validate().expect("durable sequence");
}

#[test]
fn backdated_outcome_and_duplicate_prediction_are_rejected() {
    let (candidate, identification, contract) = durable_generation();
    let prediction = prediction(&candidate, &identification, &contract);
    let mut ledger = ledger_before_prediction(&candidate, &identification, &contract);
    ledger
        .append(K1SchedulerEventPayloadV1::FuturePrediction(
            prediction.clone(),
        ))
        .expect("prediction");
    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::FuturePrediction(
            prediction.clone()
        )),
        Err("k1_scheduler_future_prediction_invalid")
    );
    let backdated = K1FutureOutcomeReceiptV1::seal(
        prediction.prediction_root_sha256,
        root(1_113),
        root(1_114),
        root(1_115),
        root(1_116),
        prediction.predicted_at_unix_nanos,
        true,
        true,
    )
    .expect("structurally valid backdated outcome");
    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::FutureOutcome(backdated)),
        Err("k1_scheduler_future_outcome_invalid")
    );
}

#[test]
fn independently_verified_counterevidence_is_preserved() {
    let (candidate, identification, contract) = durable_generation();
    let prediction = prediction(&candidate, &identification, &contract);
    let counterevidence = K1FutureOutcomeReceiptV1::seal(
        prediction.prediction_root_sha256.clone(),
        root(1_117),
        root(1_118),
        root(1_119),
        root(1_120),
        prediction.predicted_at_unix_nanos + 1,
        false,
        true,
    )
    .expect("verified counterevidence");
    let mut ledger = ledger_before_prediction(&candidate, &identification, &contract);
    ledger
        .append(K1SchedulerEventPayloadV1::FuturePrediction(prediction))
        .expect("prediction");
    ledger
        .append(K1SchedulerEventPayloadV1::FutureOutcome(counterevidence))
        .expect("counterevidence");
    ledger.validate().expect("counterevidence replay");
}
