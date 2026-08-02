use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::multi_source::K1GenerationTerminalVerdictV1;
use serde::{Deserialize, Serialize};

const REPORT_SCHEMA_V1: &str = "nando.k1-transfer-lifecycle-report.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum K1TransferLifecycleStageV1 {
    PackageCandidatePending,
    ExternalAdmissionPending,
    OrdinaryCpuPending,
    CleanupVerifierPending,
    Revoked,
    Settled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1TransferLifecycleReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub generated_at_unix: u64,
    pub stage: K1TransferLifecycleStageV1,
    pub blocker: String,
    pub terminal_verdict_root_sha256: String,
    pub identification_report_root_sha256: String,
    pub package_id: String,
    pub package_candidate_root_sha256: Option<String>,
    pub bundle_id_sha256: Option<String>,
    pub external_admission_pass: bool,
    pub ordinary_cpu_receipt_root_sha256: Option<String>,
    pub ordinary_cpu_completion_root_sha256: Option<String>,
    pub cleanup_receipt_root_sha256: Option<String>,
    pub certification_entry_root_sha256: Option<String>,
    pub certification_ledger_root_sha256: Option<String>,
    pub law_certificate_root_sha256: Option<String>,
    pub settlement_root_sha256: Option<String>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct ReportDigestV1<'a> {
    schema: &'static str,
    generated_at_unix: u64,
    stage: K1TransferLifecycleStageV1,
    blocker: &'a str,
    terminal_verdict_root_sha256: &'a str,
    identification_report_root_sha256: &'a str,
    package_id: &'a str,
    package_candidate_root_sha256: Option<&'a str>,
    bundle_id_sha256: Option<&'a str>,
    external_admission_pass: bool,
    ordinary_cpu_receipt_root_sha256: Option<&'a str>,
    ordinary_cpu_completion_root_sha256: Option<&'a str>,
    cleanup_receipt_root_sha256: Option<&'a str>,
    certification_entry_root_sha256: Option<&'a str>,
    certification_ledger_root_sha256: Option<&'a str>,
    law_certificate_root_sha256: Option<&'a str>,
    settlement_root_sha256: Option<&'a str>,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl K1TransferLifecycleReportV1 {
    pub(crate) fn pending(
        terminal: &K1GenerationTerminalVerdictV1,
        generated_at_unix: u64,
        blocker: impl Into<String>,
    ) -> Result<Self, String> {
        terminal.validate().map_err(str::to_owned)?;
        let identification = terminal
            .transfer_identification
            .as_ref()
            .ok_or_else(|| "k1_transfer_identification_missing".to_owned())?;
        let mut report = Self {
            schema: REPORT_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            generated_at_unix,
            stage: K1TransferLifecycleStageV1::PackageCandidatePending,
            blocker: blocker.into(),
            terminal_verdict_root_sha256: terminal.verdict_root_sha256.clone(),
            identification_report_root_sha256: identification.report_root_sha256.clone(),
            package_id: String::new(),
            package_candidate_root_sha256: None,
            bundle_id_sha256: None,
            external_admission_pass: false,
            ordinary_cpu_receipt_root_sha256: None,
            ordinary_cpu_completion_root_sha256: None,
            cleanup_receipt_root_sha256: None,
            certification_entry_root_sha256: None,
            certification_ledger_root_sha256: None,
            law_certificate_root_sha256: None,
            settlement_root_sha256: None,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        report.reseal()?;
        Ok(report)
    }

    pub(super) fn reseal(&mut self) -> Result<(), String> {
        self.report_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub(crate) fn settled(&self) -> bool {
        self.stage == K1TransferLifecycleStageV1::Settled
    }

    fn validate(&self) -> Result<(), String> {
        let optional_roots = [
            self.package_candidate_root_sha256.as_deref(),
            self.bundle_id_sha256.as_deref(),
            self.ordinary_cpu_receipt_root_sha256.as_deref(),
            self.ordinary_cpu_completion_root_sha256.as_deref(),
            self.cleanup_receipt_root_sha256.as_deref(),
            self.certification_entry_root_sha256.as_deref(),
            self.certification_ledger_root_sha256.as_deref(),
            self.law_certificate_root_sha256.as_deref(),
            self.settlement_root_sha256.as_deref(),
        ];
        let settled = self.stage == K1TransferLifecycleStageV1::Settled;
        if self.schema != REPORT_SCHEMA_V1
            || self.generated_at_unix == 0
            || !valid_nonzero_sha256(&self.terminal_verdict_root_sha256)
            || !valid_nonzero_sha256(&self.identification_report_root_sha256)
            || optional_roots
                .into_iter()
                .flatten()
                .any(|root| !valid_nonzero_sha256(root))
            || (self.stage != K1TransferLifecycleStageV1::PackageCandidatePending
                && self.package_id.is_empty())
            || (settled && !self.blocker.is_empty())
            || (!settled && self.blocker.is_empty())
            || (settled
                && [
                    self.package_candidate_root_sha256.as_ref(),
                    self.bundle_id_sha256.as_ref(),
                    self.ordinary_cpu_receipt_root_sha256.as_ref(),
                    self.ordinary_cpu_completion_root_sha256.as_ref(),
                    self.cleanup_receipt_root_sha256.as_ref(),
                    self.certification_entry_root_sha256.as_ref(),
                    self.certification_ledger_root_sha256.as_ref(),
                    self.law_certificate_root_sha256.as_ref(),
                    self.settlement_root_sha256.as_ref(),
                ]
                .into_iter()
                .any(|root| root.is_none()))
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.report_root_sha256 != self.expected_root()?
        {
            return Err("k1_transfer_lifecycle_report_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&ReportDigestV1 {
            schema: REPORT_SCHEMA_V1,
            generated_at_unix: self.generated_at_unix,
            stage: self.stage,
            blocker: &self.blocker,
            terminal_verdict_root_sha256: &self.terminal_verdict_root_sha256,
            identification_report_root_sha256: &self.identification_report_root_sha256,
            package_id: &self.package_id,
            package_candidate_root_sha256: self.package_candidate_root_sha256.as_deref(),
            bundle_id_sha256: self.bundle_id_sha256.as_deref(),
            external_admission_pass: self.external_admission_pass,
            ordinary_cpu_receipt_root_sha256: self.ordinary_cpu_receipt_root_sha256.as_deref(),
            ordinary_cpu_completion_root_sha256: self
                .ordinary_cpu_completion_root_sha256
                .as_deref(),
            cleanup_receipt_root_sha256: self.cleanup_receipt_root_sha256.as_deref(),
            certification_entry_root_sha256: self.certification_entry_root_sha256.as_deref(),
            certification_ledger_root_sha256: self.certification_ledger_root_sha256.as_deref(),
            law_certificate_root_sha256: self.law_certificate_root_sha256.as_deref(),
            settlement_root_sha256: self.settlement_root_sha256.as_deref(),
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(str::to_owned)
    }
}
