use super::*;

const K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1: &str =
    "nando.k1-durable-independent-future-prediction.v1";

pub(in crate::k1_natural_scheduler_runtime) fn durable_future_prediction_contract(
    identification: &K1IdentificationFreezeV1,
) -> bool {
    identification.prediction_schema == K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_partition_schema_is_not_a_durable_future_precommit() {
        assert_ne!(
            K1_PREDICTION_SCHEMA_V1,
            K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
        );
    }
}
