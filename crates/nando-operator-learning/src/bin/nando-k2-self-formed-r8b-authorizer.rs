use std::io::{Read, Write};

use nando_operator_learning::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2CompositionErrorV1, K2CompositionResultV1,
    K2UncertaintyR8BAuthorizationRequestV3, authorize_self_formed_r8b_v3,
    composition_sha256_file_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_authorizer_stdin"))?;
    let request: K2UncertaintyR8BAuthorizationRequestV3 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_authorizer"))?;
    if composition_sha256_file_v1(&executable)? != request.authorizer_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r8b_authorizer_executable_mismatch",
        ));
    }
    let packet_root = std::env::current_dir()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_r8b_packet_root"))?;
    let receipt = authorize_self_formed_r8b_v3(&request, &packet_root)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_r8b_authorizer_stdout"))
}
