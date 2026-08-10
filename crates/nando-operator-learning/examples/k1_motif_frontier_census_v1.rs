use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{self, Write};
use std::path::PathBuf;

use nando_operator_kernel::{MultiSourceContainerClassV1, MultiSourceTypeClassV1, RelationFrame};
use nando_operator_learning::multi_source::{
    BlindThenRevealJoinedTransitionV1, CompletedEffectFormV1, K1ConsequenceTypeV1,
    MultiSourceJoinLedgerV1, PreActionTopologyAuditRowV1, factor_multi_source_row_v1,
    source_neutral_topology_motifs_v1,
};
use nando_operator_learning::read_framed_cbor;
use serde::Serialize;

#[derive(Default)]
struct MotifAccumulator {
    settled_rows: u64,
    verified_rows: u64,
    input_tokens: u64,
    lineages: BTreeSet<String>,
    role_counts: BTreeSet<u8>,
    relation_counts: BTreeSet<u8>,
}

#[derive(Serialize)]
struct MotifSummary<'a> {
    motif_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    settled_rows: u64,
    verified_rows: u64,
    independent_lineages: usize,
    input_tokens: u64,
    role_counts: &'a BTreeSet<u8>,
    relation_counts: &'a BTreeSet<u8>,
    readiness_pass: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(io::Error::other(
            "usage: k1_motif_frontier_census_v1 TOPOLOGY_ARCHIVE FRAME_ARCHIVE",
        )
        .into());
    }
    let topologies = read_framed_cbor::<PreActionTopologyAuditRowV1>(
        &PathBuf::from(&arguments[0]),
        "multi-source-topology",
    )
    .map_err(io::Error::other)?;
    let frames =
        read_framed_cbor::<RelationFrame>(&PathBuf::from(&arguments[1]), "multi-source-frame")
            .map_err(io::Error::other)?;
    let joins = MultiSourceJoinLedgerV1::build(&topologies, &frames).into_rows();
    let mut motifs = BTreeMap::<(K1ConsequenceTypeV1, String), MotifAccumulator>::new();
    let mut motif_budget_rows = 0u64;
    let mut motif_empty_rows = 0u64;
    for joined in &joins {
        let consequence = consequence_type(joined);
        let row_motifs = match source_neutral_topology_motifs_v1(&joined.topology) {
            Ok(motifs) => motifs,
            Err("source_neutral_topology_motif_budget") => {
                motif_budget_rows = motif_budget_rows.saturating_add(1);
                continue;
            }
            Err(error) => return Err(io::Error::other(error).into()),
        };
        if row_motifs.is_empty() {
            motif_empty_rows = motif_empty_rows.saturating_add(1);
        }
        for motif in row_motifs {
            let accumulator = motifs
                .entry((consequence, motif.motif_root_sha256))
                .or_default();
            accumulator.settled_rows = accumulator.settled_rows.saturating_add(1);
            accumulator.verified_rows = accumulator
                .verified_rows
                .saturating_add(u64::from(joined.accepted));
            accumulator.input_tokens = accumulator.input_tokens.saturating_add(joined.input_tokens);
            accumulator
                .lineages
                .insert(joined.session_lineage_sha256.clone());
            accumulator.role_counts.insert(motif.role_count);
            accumulator.relation_counts.insert(motif.relation_count);
        }
    }

    let mut summaries = motifs
        .iter()
        .map(|((consequence_type, root), accumulator)| MotifSummary {
            motif_root_sha256: root,
            consequence_type: *consequence_type,
            settled_rows: accumulator.settled_rows,
            verified_rows: accumulator.verified_rows,
            independent_lineages: accumulator.lineages.len(),
            input_tokens: accumulator.input_tokens,
            role_counts: &accumulator.role_counts,
            relation_counts: &accumulator.relation_counts,
            readiness_pass: accumulator.settled_rows >= 8
                && accumulator.verified_rows >= 2
                && accumulator.lineages.len() >= 2,
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .readiness_pass
            .cmp(&left.readiness_pass)
            .then_with(|| right.settled_rows.cmp(&left.settled_rows))
            .then_with(|| right.input_tokens.cmp(&left.input_tokens))
            .then_with(|| left.motif_root_sha256.cmp(right.motif_root_sha256))
    });
    let readiness_by_type = summaries.iter().filter(|row| row.readiness_pass).fold(
        BTreeMap::<K1ConsequenceTypeV1, u64>::new(),
        |mut counts, row| {
            *counts.entry(row.consequence_type).or_default() += 1;
            counts
        },
    );
    let output = serde_json::json!({
        "schema": "nando.k1-motif-frontier-census.v1",
        "topology_rows": topologies.len(),
        "frame_rows": frames.len(),
        "joined_rows": joins.len(),
        "motif_candidates": summaries.len(),
        "motif_budget_rows": motif_budget_rows,
        "motif_empty_rows": motif_empty_rows,
        "readiness_pass": summaries.iter().filter(|row| row.readiness_pass).count(),
        "readiness_pass_by_type": readiness_by_type,
        "top": summaries.into_iter().take(200).collect::<Vec<_>>(),
    });
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &output)?;
    stdout.write_all(b"\n")?;
    Ok(())
}

fn consequence_type(joined: &BlindThenRevealJoinedTransitionV1) -> K1ConsequenceTypeV1 {
    match factor_multi_source_row_v1(joined).completed_effect {
        CompletedEffectFormV1::StatusValueBranch => K1ConsequenceTypeV1::Boolean,
        CompletedEffectFormV1::CollectionTransform => K1ConsequenceTypeV1::Collection,
        CompletedEffectFormV1::MultiRoleRendering => K1ConsequenceTypeV1::RenderedSequence,
        CompletedEffectFormV1::CrossOutputComposition => K1ConsequenceTypeV1::Record,
        CompletedEffectFormV1::SingleRoleProjection => {
            if joined
                .topology
                .roles
                .iter()
                .any(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
            {
                K1ConsequenceTypeV1::Collection
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Boolean)
            {
                K1ConsequenceTypeV1::Boolean
            } else if joined
                .topology
                .roles
                .iter()
                .any(|role| role.type_class == MultiSourceTypeClassV1::Object)
            {
                K1ConsequenceTypeV1::Record
            } else {
                K1ConsequenceTypeV1::Scalar
            }
        }
        CompletedEffectFormV1::Unexplained => K1ConsequenceTypeV1::Record,
    }
}
