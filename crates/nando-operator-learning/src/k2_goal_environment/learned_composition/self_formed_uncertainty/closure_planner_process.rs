use std::io::{Read, Write};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_bytes_v1, composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, decode_self_formed_closure_planner_request_v1,
    plan_self_formed_uncertainty_closure_v1,
};

pub fn run_self_formed_closure_planner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_closure_planner_stdin"))?;
    let request = decode_self_formed_closure_planner_request_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_closure_planner"))?;
    if composition_sha256_file_v1(&executable)? != request.planner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_planner_executable_mismatch",
        ));
    }
    let census = plan_self_formed_uncertainty_closure_v1(&request)?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&census)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_closure_planner_stdout"))
}
