use std::io::{Read, Write};

use nando_operator_learning::k2_goal_environment::learned_capability::{
    K2_LEARNER_MAX_REQUEST_BYTES_V1, K2EffectLearnerProtocolRequestV1,
};

fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_LEARNER_MAX_REQUEST_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| ())?;
    if input.is_empty() || input.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
        return Err(());
    }
    let request =
        K2EffectLearnerProtocolRequestV1::from_canonical_bytes_v1(&input).map_err(|_| ())?;
    let outcome = request.evaluate_v1().map_err(|_| ())?;
    let bytes = outcome.canonical_bytes_v1().map_err(|_| ())?;
    std::io::stdout().write_all(&bytes).map_err(|_| ())
}
