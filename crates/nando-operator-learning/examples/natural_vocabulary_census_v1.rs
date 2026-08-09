use std::error::Error;
use std::io::{self, Write};
use std::path::PathBuf;

use nando_operator_learning::multi_source::{
    PreActionTopologyAuditRowV1, build_natural_vocabulary_census_v1,
};
use nando_operator_learning::{RelationFrame, read_framed_cbor};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(io::Error::other(
            "usage: natural_vocabulary_census_v1 TOPOLOGY_ARCHIVE FRAME_ARCHIVE",
        )
        .into());
    }
    let topology_directory = PathBuf::from(&arguments[0]);
    let frame_directory = PathBuf::from(&arguments[1]);
    let topologies = read_framed_cbor::<PreActionTopologyAuditRowV1>(
        &topology_directory,
        "multi-source-topology",
    )
    .map_err(io::Error::other)?;
    let frames = read_framed_cbor::<RelationFrame>(&frame_directory, "multi-source-frame")
        .map_err(io::Error::other)?;
    let report =
        build_natural_vocabulary_census_v1(&topologies, &frames).map_err(io::Error::other)?;
    let mut output = io::stdout().lock();
    output.write_all(&report.canonical_bytes()?)?;
    output.write_all(b"\n")?;
    Ok(())
}
