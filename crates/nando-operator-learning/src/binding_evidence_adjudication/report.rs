use serde::Serialize;

use super::canonical::sha256_json;
use super::wire::{
    BindingAdjudicationErrorV1, BindingCausalAdjudicationReportV1,
    BindingHypothesisAdjudicationStatusV1, BindingInterventionAdjudicationV1,
};

pub(super) fn adjudication_report_digest(
    report: &BindingCausalAdjudicationReportV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        stop_id: &'a str,
        trusted_label_manifest_sha256: &'a str,
        trusted_label_root_sha256: &'a str,
        physical_receipts_root_sha256: &'a str,
        support_rows: usize,
        future_rows: usize,
        support_positive_rows: usize,
        support_applicability_negative_rows: usize,
        future_positive_rows: usize,
        future_applicability_negative_rows: usize,
        b1a_ties_total: usize,
        b1a_ties_evaluated_against_relation: usize,
        causal_relation: &'a str,
        causal_relation_id_sha256: &'a str,
        h0_status: BindingHypothesisAdjudicationStatusV1,
        h1_status: BindingHypothesisAdjudicationStatusV1,
        wrong_bindings: usize,
        applicability_negative_accepts: usize,
        parity_failures: usize,
        interventions: &'a [BindingInterventionAdjudicationV1],
        selector_compiled: bool,
        protocol_mode_compiled: bool,
        f4_status: &'a str,
        execution_authority: bool,
    }
    sha256_json(&DigestFields {
        schema: &report.schema,
        stop_id: &report.stop_id,
        trusted_label_manifest_sha256: &report.trusted_label_manifest_sha256,
        trusted_label_root_sha256: &report.trusted_label_root_sha256,
        physical_receipts_root_sha256: &report.physical_receipts_root_sha256,
        support_rows: report.support_rows,
        future_rows: report.future_rows,
        support_positive_rows: report.support_positive_rows,
        support_applicability_negative_rows: report.support_applicability_negative_rows,
        future_positive_rows: report.future_positive_rows,
        future_applicability_negative_rows: report.future_applicability_negative_rows,
        b1a_ties_total: report.b1a_ties_total,
        b1a_ties_evaluated_against_relation: report.b1a_ties_evaluated_against_relation,
        causal_relation: &report.causal_relation,
        causal_relation_id_sha256: &report.causal_relation_id_sha256,
        h0_status: report.h0_status,
        h1_status: report.h1_status,
        wrong_bindings: report.wrong_bindings,
        applicability_negative_accepts: report.applicability_negative_accepts,
        parity_failures: report.parity_failures,
        interventions: &report.interventions,
        selector_compiled: report.selector_compiled,
        protocol_mode_compiled: report.protocol_mode_compiled,
        f4_status: &report.f4_status,
        execution_authority: report.execution_authority,
    })
}
