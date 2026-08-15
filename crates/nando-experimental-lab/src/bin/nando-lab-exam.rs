use nando_experimental_lab::{
    LabError, LawCertificate, ProbeSelection, execute_probe, filesystem_copy_probe,
    filesystem_delete_probe, git_rename_probe, select_probe,
};
use serde::Serialize;

#[derive(Serialize)]
struct ExamReport {
    schema: &'static str,
    verdict: &'static str,
    e1_probe_selection: ProbeSelection,
    e2_laws_opened: usize,
    e2_environments: usize,
    e3_status: &'static str,
    e3_certificate: Option<LawCertificate>,
    e3_candidate_law_id: String,
    authority_granted: bool,
    active_package_allowed: bool,
}

fn run() -> Result<ExamReport, LabError> {
    let probes = vec![
        git_rename_probe()?,
        filesystem_copy_probe()?,
        filesystem_delete_probe()?,
    ];
    let e1_probe_selection = select_probe(&probes)?;
    let receipts = probes
        .iter()
        .map(execute_probe)
        .collect::<Result<Vec<_>, _>>()?;
    let candidates = receipts
        .iter()
        .filter_map(|receipt| receipt.unique_law_candidate.as_ref())
        .collect::<Vec<_>>();
    if candidates.len() != 3 {
        return Err(LabError::NoUniqueLaw);
    }
    let e3_candidate = candidates[0];
    Ok(ExamReport {
        schema: "nando.experimental-lab-exam.v1",
        verdict: "LAB_EXAM_PASS_NO_AUTHORITY",
        e1_probe_selection,
        e2_laws_opened: candidates.len(),
        e2_environments: 2,
        e3_status: "WAITING_FOR_INDEPENDENT_NATURAL_HOLDOUT",
        e3_certificate: None,
        e3_candidate_law_id: e3_candidate.law_id.clone(),
        authority_granted: false,
        active_package_allowed: false,
    })
}

fn main() {
    match run().and_then(|report| {
        serde_json::to_string_pretty(&report).map_err(|_| LabError::CanonicalEncoding)
    }) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("nando-lab-exam: {error}");
            std::process::exit(1);
        }
    }
}
