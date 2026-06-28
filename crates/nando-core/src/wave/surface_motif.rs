//! Surface motif bank for L1 reuse.
//!
//! A motif is a repeated surface span. It is still not meaning. It is the first
//! reusable layer above raw n-gram wire compilation.

use std::collections::{HashMap, HashSet};

use super::{SURFACE_WAVE_BYTES, SurfaceWave4096};

pub const SURFACE_MOTIF_REF_BYTES: usize = 12;
pub const SURFACE_MOTIF_RECORD_BYTES: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceMotifSpec {
    pub window_len: usize,
    pub stride: usize,
    pub min_len: usize,
    pub min_support: usize,
    pub max_motifs: usize,
}

impl Default for SurfaceMotifSpec {
    fn default() -> Self {
        Self {
            window_len: 16,
            stride: 1,
            min_len: 16,
            min_support: 8,
            max_motifs: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceMotif {
    pub id: u32,
    pub byte_hash: u64,
    pub byte_len: u32,
    pub support_docs: u32,
    pub wave: SurfaceWave4096,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceMotifRef {
    pub motif_id: u32,
    pub start: u32,
    pub byte_len: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceResidualRecord {
    pub source_hash: u64,
    pub source_len: u32,
    pub residual_hash: u64,
    pub uncovered_bytes: u32,
    pub motif_refs: Vec<SurfaceMotifRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceMotifBank {
    pub spec: SurfaceMotifSpec,
    pub motifs: Vec<SurfaceMotif>,
    pub cold_motif_bytes: Vec<Vec<u8>>,
    pub records: Vec<SurfaceResidualRecord>,
    pub naive_wave_bytes: usize,
    pub motif_wave_bytes: usize,
    pub motif_cold_bytes: usize,
    pub residual_ref_bytes: usize,
    pub residual_record_bytes: usize,
    pub residual_raw_bytes: usize,
    pub encoded_bytes: usize,
}

impl SurfaceMotifBank {
    #[must_use]
    pub fn build<'a, I>(texts: I, spec: SurfaceMotifSpec) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let texts: Vec<Vec<u8>> = texts
            .into_iter()
            .map(|text| text.as_bytes().to_vec())
            .collect();
        let candidates = candidate_motifs(&texts, spec);
        let selected = select_motifs(candidates, spec);
        let mut motifs: Vec<_> = selected
            .iter()
            .map(|selected| selected.motif.clone())
            .collect();
        let mut cold_motif_bytes: Vec<_> = selected
            .iter()
            .map(|selected| selected.bytes.clone())
            .collect();
        let mut records = encode_records(&texts, &cold_motif_bytes);
        prune_unused_motifs(&mut motifs, &mut cold_motif_bytes, &mut records);

        let naive_wave_bytes = texts.len() * SURFACE_WAVE_BYTES;
        let motif_wave_bytes = motifs.len() * SURFACE_WAVE_BYTES;
        let motif_cold_bytes = cold_motif_bytes.iter().map(Vec::len).sum();
        let residual_ref_bytes = records
            .iter()
            .map(|record| record.motif_refs.len() * SURFACE_MOTIF_REF_BYTES)
            .sum();
        let residual_record_bytes = records.len() * SURFACE_MOTIF_RECORD_BYTES;
        let residual_raw_bytes = records
            .iter()
            .map(|record| record.uncovered_bytes as usize)
            .sum();
        let encoded_bytes = motif_wave_bytes
            + motif_cold_bytes
            + residual_ref_bytes
            + residual_record_bytes
            + residual_raw_bytes;

        Self {
            spec,
            motifs,
            cold_motif_bytes,
            records,
            naive_wave_bytes,
            motif_wave_bytes,
            motif_cold_bytes,
            residual_ref_bytes,
            residual_record_bytes,
            residual_raw_bytes,
            encoded_bytes,
        }
    }

    #[must_use]
    pub fn compression_ratio(&self) -> f32 {
        if self.naive_wave_bytes == 0 {
            return 1.0;
        }
        self.encoded_bytes as f32 / self.naive_wave_bytes as f32
    }

    #[must_use]
    pub fn bytes_saved(&self) -> isize {
        self.naive_wave_bytes as isize - self.encoded_bytes as isize
    }

    #[must_use]
    pub fn record_motif_ids(&self, record_index: usize) -> Vec<u32> {
        self.records
            .get(record_index)
            .map(|record| {
                record
                    .motif_refs
                    .iter()
                    .map(|motif_ref| motif_ref.motif_id)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
struct CandidateMotif {
    bytes: Vec<u8>,
    support_docs: usize,
}

#[derive(Clone, Debug)]
struct SelectedMotif {
    bytes: Vec<u8>,
    motif: SurfaceMotif,
}

#[derive(Clone, Copy, Debug)]
struct Occurrence {
    doc: usize,
    start: usize,
}

fn candidate_motifs(texts: &[Vec<u8>], spec: SurfaceMotifSpec) -> Vec<CandidateMotif> {
    if spec.window_len == 0 || spec.stride == 0 {
        return Vec::new();
    }

    let mut anchors: HashMap<Vec<u8>, Vec<Occurrence>> = HashMap::new();
    for (doc, bytes) in texts.iter().enumerate() {
        if bytes.len() < spec.window_len {
            continue;
        }
        for start in (0..=bytes.len() - spec.window_len).step_by(spec.stride) {
            anchors
                .entry(bytes[start..start + spec.window_len].to_vec())
                .or_default()
                .push(Occurrence { doc, start });
        }
    }

    let mut by_bytes: HashMap<Vec<u8>, usize> = HashMap::new();
    for occurrences in anchors.values() {
        let support_docs = unique_doc_count(occurrences);
        if support_docs < spec.min_support {
            continue;
        }

        let bytes = extend_right(texts, occurrences, spec.window_len);
        if bytes.len() < spec.min_len {
            continue;
        }
        by_bytes
            .entry(bytes)
            .and_modify(|support| *support = (*support).max(support_docs))
            .or_insert(support_docs);
    }

    let mut candidates: Vec<_> = by_bytes
        .into_iter()
        .map(|(bytes, support_docs)| CandidateMotif {
            bytes,
            support_docs,
        })
        .collect();
    candidates.sort_by(|left, right| {
        let left_score = left.bytes.len() * left.support_docs;
        let right_score = right.bytes.len() * right.support_docs;
        right_score
            .cmp(&left_score)
            .then_with(|| right.bytes.len().cmp(&left.bytes.len()))
            .then_with(|| right.support_docs.cmp(&left.support_docs))
    });
    candidates
}

fn select_motifs(candidates: Vec<CandidateMotif>, spec: SurfaceMotifSpec) -> Vec<SelectedMotif> {
    let mut selected: Vec<CandidateMotif> = Vec::new();

    for candidate in candidates {
        if selected.len() >= spec.max_motifs {
            break;
        }
        let is_subspan = selected.iter().any(|motif| {
            motif.support_docs >= candidate.support_docs
                && contains_subslice(&motif.bytes, &candidate.bytes)
        });
        if is_subspan {
            continue;
        }
        selected.push(candidate);
    }

    selected
        .into_iter()
        .enumerate()
        .map(|(id, candidate)| {
            let motif = SurfaceMotif {
                id: id as u32,
                byte_hash: hash_bytes(&candidate.bytes),
                byte_len: candidate.bytes.len() as u32,
                support_docs: candidate.support_docs as u32,
                wave: SurfaceWave4096::compile_bytes(&candidate.bytes),
            };
            SelectedMotif {
                bytes: candidate.bytes,
                motif,
            }
        })
        .collect()
}

fn encode_records(texts: &[Vec<u8>], motif_bytes: &[Vec<u8>]) -> Vec<SurfaceResidualRecord> {
    let mut records = Vec::with_capacity(texts.len());
    let mut motif_order: Vec<(u32, usize)> = motif_bytes
        .iter()
        .enumerate()
        .map(|(id, bytes)| (id as u32, bytes.len()))
        .collect();
    motif_order.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    for bytes in texts {
        let mut covered = vec![false; bytes.len()];
        let mut motif_refs = Vec::new();

        for (motif_id, motif_len) in &motif_order {
            let motif = &motif_bytes[*motif_id as usize];
            for start in find_all(bytes, motif) {
                if span_is_free(&covered, start, *motif_len) {
                    covered[start..start + motif_len].fill(true);
                    motif_refs.push(SurfaceMotifRef {
                        motif_id: *motif_id,
                        start: start as u32,
                        byte_len: *motif_len as u16,
                    });
                }
            }
        }

        motif_refs.sort_by_key(|motif_ref| motif_ref.start);
        let uncovered_bytes = covered.iter().filter(|covered| !**covered).count() as u32;
        let residual_hash = hash_uncovered(bytes, &covered);
        records.push(SurfaceResidualRecord {
            source_hash: hash_bytes(bytes),
            source_len: bytes.len() as u32,
            residual_hash,
            uncovered_bytes,
            motif_refs,
        });
    }

    records
}

fn prune_unused_motifs(
    motifs: &mut Vec<SurfaceMotif>,
    motif_bytes: &mut Vec<Vec<u8>>,
    records: &mut [SurfaceResidualRecord],
) {
    let mut used = HashSet::new();
    for record in records.iter() {
        for motif_ref in &record.motif_refs {
            used.insert(motif_ref.motif_id);
        }
    }

    if used.len() == motifs.len() {
        return;
    }

    let mut remap = HashMap::new();
    let mut kept_motifs = Vec::with_capacity(used.len());
    let mut kept_bytes = Vec::with_capacity(used.len());

    for (old_id, (motif, bytes)) in motifs.iter().zip(motif_bytes.iter()).enumerate() {
        let old_id = old_id as u32;
        if !used.contains(&old_id) {
            continue;
        }
        let new_id = kept_motifs.len() as u32;
        remap.insert(old_id, new_id);
        let mut motif = motif.clone();
        motif.id = new_id;
        kept_motifs.push(motif);
        kept_bytes.push(bytes.clone());
    }

    for record in records.iter_mut() {
        for motif_ref in &mut record.motif_refs {
            if let Some(new_id) = remap.get(&motif_ref.motif_id) {
                motif_ref.motif_id = *new_id;
            }
        }
        record.motif_refs.sort_by_key(|motif_ref| motif_ref.start);
    }

    *motifs = kept_motifs;
    *motif_bytes = kept_bytes;
}

fn unique_doc_count(occurrences: &[Occurrence]) -> usize {
    occurrences
        .iter()
        .map(|occurrence| occurrence.doc)
        .collect::<HashSet<_>>()
        .len()
}

fn extend_right(texts: &[Vec<u8>], occurrences: &[Occurrence], window_len: usize) -> Vec<u8> {
    let first = occurrences[0];
    let mut bytes = texts[first.doc][first.start..first.start + window_len].to_vec();

    loop {
        let offset = bytes.len();
        let Some(next) = texts[first.doc].get(first.start + offset).copied() else {
            break;
        };
        let all_match = occurrences.iter().all(|occurrence| {
            texts[occurrence.doc]
                .get(occurrence.start + offset)
                .is_some_and(|byte| *byte == next)
        });
        if !all_match {
            break;
        }
        bytes.push(next);
    }

    bytes
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return Vec::new();
    }

    let mut starts = Vec::new();
    for start in 0..=haystack.len() - needle.len() {
        if &haystack[start..start + needle.len()] == needle {
            starts.push(start);
        }
    }
    starts
}

fn span_is_free(covered: &[bool], start: usize, len: usize) -> bool {
    covered
        .get(start..start + len)
        .is_some_and(|span| span.iter().all(|covered| !*covered))
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn hash_uncovered(bytes: &[u8], covered: &[bool]) -> u64 {
    let mut state = 0x5245_5349_4455_414Cu64;
    for (index, byte) in bytes.iter().enumerate() {
        if covered.get(index).copied().unwrap_or(false) {
            continue;
        }
        state ^= ((index as u64) << 32) ^ u64::from(*byte);
        state = splitmix64(state);
    }
    state
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut state = 0x5355_5246_4D4F_5446u64 ^ bytes.len() as u64;
    for byte in bytes {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    state
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ten_thousand_url_like_pages() -> Vec<String> {
        (0..10_000)
            .map(|index| match index % 4 {
                0 => format!("https://mirror.dxdy.ru/topic{index:04}.html"),
                1 => format!("https://mirror.dxdy.ru/post{index:04}.html#p{index:04}"),
                2 => format!("https://docs.rs/nando-wave/{index:04}/surface_wave/index.html"),
                _ => format!("https://github.com/nando-wave/core/issues/{index:04}"),
            })
            .collect()
    }

    #[test]
    fn ten_thousand_surfaces_reuse_motifs_instead_of_full_waves() {
        let pages = ten_thousand_url_like_pages();
        let spec = SurfaceMotifSpec {
            min_support: 64,
            max_motifs: 64,
            ..SurfaceMotifSpec::default()
        };
        let bank = SurfaceMotifBank::build(pages.iter().map(String::as_str), spec);

        assert_eq!(bank.records.len(), 10_000);
        assert_eq!(bank.naive_wave_bytes, 10_000 * SURFACE_WAVE_BYTES);
        assert!(!bank.motifs.is_empty());
        assert!(
            bank.encoded_bytes < bank.naive_wave_bytes / 10,
            "encoded={} naive={}",
            bank.encoded_bytes,
            bank.naive_wave_bytes
        );
        assert!(bank.bytes_saved() > 0);
    }

    #[test]
    fn motif_refs_keep_family_near_and_unrelated_apart() {
        let pages = ten_thousand_url_like_pages();
        let bank = SurfaceMotifBank::build(
            pages.iter().map(String::as_str),
            SurfaceMotifSpec {
                min_support: 64,
                max_motifs: 64,
                ..SurfaceMotifSpec::default()
            },
        );

        let dxdy_topic = bank.record_motif_ids(0);
        let dxdy_topic_next = bank.record_motif_ids(4);
        let github_issue = bank.record_motif_ids(3);

        assert!(!dxdy_topic.is_empty());
        assert_eq!(dxdy_topic, dxdy_topic_next);
        assert_ne!(dxdy_topic, github_issue);
    }

    #[test]
    fn residual_hash_preserves_distinguishing_suffix() {
        let pages = [
            "https://mirror.dxdy.ru/topic3420.html",
            "https://mirror.dxdy.ru/topic3421.html",
            "https://mirror.dxdy.ru/topic3422.html",
            "https://mirror.dxdy.ru/topic3423.html",
            "https://mirror.dxdy.ru/topic3424.html",
            "https://mirror.dxdy.ru/topic3425.html",
            "https://mirror.dxdy.ru/topic3426.html",
            "https://mirror.dxdy.ru/topic3427.html",
        ];
        let bank = SurfaceMotifBank::build(
            pages,
            SurfaceMotifSpec {
                min_support: 4,
                max_motifs: 8,
                ..SurfaceMotifSpec::default()
            },
        );

        assert_eq!(bank.records[0].motif_refs, bank.records[1].motif_refs);
        assert_ne!(bank.records[0].residual_hash, bank.records[1].residual_hash);
    }

    #[test]
    fn singletons_do_not_create_fake_motifs() {
        let pages: Vec<String> = (0..128)
            .map(|index| {
                let mut state = splitmix64(index ^ 0x0C01_1EC7);
                let mut text = String::with_capacity(48);
                for _ in 0..48 {
                    state = splitmix64(state);
                    let byte = b'a' + (state % 26) as u8;
                    text.push(char::from(byte));
                }
                text
            })
            .collect();
        let bank = SurfaceMotifBank::build(
            pages.iter().map(String::as_str),
            SurfaceMotifSpec {
                min_support: 64,
                max_motifs: 64,
                ..SurfaceMotifSpec::default()
            },
        );

        assert!(bank.motifs.is_empty());
        assert_eq!(bank.residual_ref_bytes, 0);
    }
}
