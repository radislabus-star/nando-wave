use std::{env, fs, io, io::Write};

use nando_operator_learning::LawLabContractV1;

fn main() -> io::Result<()> {
    let contract = LawLabContractV1::preregistered_v1().map_err(io::Error::other)?;
    let bytes = contract.canonical_bytes().map_err(io::Error::other)?;
    if let Some(output_path) = env::args_os().nth(1) {
        fs::write(output_path, bytes)
    } else {
        io::stdout().lock().write_all(&bytes)
    }
}
