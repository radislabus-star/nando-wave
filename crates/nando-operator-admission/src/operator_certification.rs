use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

pub const OPERATOR_CERTIFICATION_LEDGER_SCHEMA_V1: &str = "nando.operator-certification-ledger.v1";
pub const OPERATOR_CERTIFICATION_ENTRY_SCHEMA_V1: &str = "nando.operator-certification-entry.v1";
pub const EXECUTION_CERTIFICATE_SCHEMA_V1: &str = "nando.execution-certificate.v1";
pub const LAW_CERTIFICATE_SCHEMA_V1: &str = "nando.law-certificate.v1";
pub const MECHANISM_CERTIFICATE_SCHEMA_V1: &str = "nando.mechanism-certificate.v1";
pub const EXACT_MEMORY_CLEANUP_RECEIPT_SCHEMA_V1: &str = "nando.exact-memory-cleanup-receipt.v1";
pub const K1_VOCABULARY_GATE_SCHEMA_V1: &str = "nando.k1-vocabulary-gate.v1";

pub const K1_MIN_LAW_CERTIFICATES: u64 = 3;
pub const K1_MIN_SEMANTIC_LAWS: u64 = 3;
pub const K1_MIN_ROLE_TOPOLOGIES: u64 = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionCertificateStatusV1 {
    Pending,
    Pass,
    Revoked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawCertificateStatusV1 {
    Partial,
    Pass,
    Rejected,
    Legacy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MechanismCertificateStatusV1 {
    NotEvaluated,
    Collecting,
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorMechanismClassV1 {
    WaveCausal,
    Structural,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCertificateV1 {
    pub schema: String,
    pub certificate_root_sha256: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub status: ExecutionCertificateStatusV1,
    pub evidence_roots_sha256: Vec<String>,
    pub blocker: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawCertificateV1 {
    pub schema: String,
    pub certificate_root_sha256: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub status: LawCertificateStatusV1,
    pub evidence_roots_sha256: Vec<String>,
    pub cleanup_receipt_root_sha256: Option<String>,
    pub blocker: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismCertificateV1 {
    pub schema: String,
    pub certificate_root_sha256: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub status: MechanismCertificateStatusV1,
    pub classification: OperatorMechanismClassV1,
    pub evidence_roots_sha256: Vec<String>,
    pub blocker: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactMemoryCleanupReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub candidate_root_sha256: String,
    pub active_registry_root_sha256: String,
    pub standalone_restart_root_sha256: String,
    pub learner_state_absent: bool,
    pub raw_evidence_absent: bool,
    pub exact_example_authority_absent: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorCertificationEntryV1 {
    pub schema: String,
    pub entry_root_sha256: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub semantic_law_id_sha256: String,
    pub role_topology_id_sha256: String,
    pub execution: ExecutionCertificateV1,
    pub law: LawCertificateV1,
    pub mechanism: MechanismCertificateV1,
    pub false_bad_apply: u64,
    pub product_registry_member: bool,
    pub epistemic_registry_member: bool,
    pub k1_unit_eligible: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorCertificationLedgerV1 {
    pub schema: String,
    pub ledger_root_sha256: String,
    pub revision: u64,
    pub entries: Vec<OperatorCertificationEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1VocabularyGateV1 {
    pub schema: String,
    pub gate_root_sha256: String,
    pub law_certificates: u64,
    pub semantic_laws: u64,
    pub role_topologies: u64,
    pub cleanup_receipts: u64,
    pub false_bad_apply: u64,
    pub min_law_certificates: u64,
    pub min_semantic_laws: u64,
    pub min_role_topologies: u64,
    pub open: bool,
    pub blocker: String,
}

impl ExecutionCertificateV1 {
    pub fn seal(
        bundle_id_sha256: &str,
        package_id: &str,
        status: ExecutionCertificateStatusV1,
        evidence_roots_sha256: Vec<String>,
        blocker: &str,
    ) -> Result<Self, &'static str> {
        let evidence_roots_sha256 = canonical_roots(evidence_roots_sha256)?;
        let mut certificate = Self {
            schema: EXECUTION_CERTIFICATE_SCHEMA_V1.to_owned(),
            certificate_root_sha256: String::new(),
            bundle_id_sha256: bundle_id_sha256.to_owned(),
            package_id: package_id.to_owned(),
            status,
            evidence_roots_sha256,
            blocker: blocker.to_owned(),
        };
        certificate.certificate_root_sha256 = certificate.expected_root()?;
        certificate.validate()?;
        Ok(certificate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        validate_binding(&self.schema, EXECUTION_CERTIFICATE_SCHEMA_V1, self)?;
        if self.evidence_roots_sha256.is_empty()
            || (self.status == ExecutionCertificateStatusV1::Pass && !self.blocker.is_empty())
            || (self.status != ExecutionCertificateStatusV1::Pass && self.blocker.is_empty())
            || self.certificate_root_sha256 != self.expected_root()?
        {
            return Err("execution_certificate_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            EXECUTION_CERTIFICATE_SCHEMA_V1,
            self.bundle_id_sha256.as_str(),
            self.package_id.as_str(),
            self.status,
            &self.evidence_roots_sha256,
            self.blocker.as_str(),
        ))
    }
}

impl LawCertificateV1 {
    pub fn seal(
        bundle_id_sha256: &str,
        package_id: &str,
        status: LawCertificateStatusV1,
        evidence_roots_sha256: Vec<String>,
        cleanup_receipt_root_sha256: Option<String>,
        blocker: &str,
    ) -> Result<Self, &'static str> {
        let evidence_roots_sha256 = canonical_roots(evidence_roots_sha256)?;
        let mut certificate = Self {
            schema: LAW_CERTIFICATE_SCHEMA_V1.to_owned(),
            certificate_root_sha256: String::new(),
            bundle_id_sha256: bundle_id_sha256.to_owned(),
            package_id: package_id.to_owned(),
            status,
            evidence_roots_sha256,
            cleanup_receipt_root_sha256,
            blocker: blocker.to_owned(),
        };
        certificate.certificate_root_sha256 = certificate.expected_root()?;
        certificate.validate()?;
        Ok(certificate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        validate_binding(&self.schema, LAW_CERTIFICATE_SCHEMA_V1, self)?;
        if self.evidence_roots_sha256.is_empty()
            || self
                .cleanup_receipt_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || (self.status == LawCertificateStatusV1::Pass
                && (self.cleanup_receipt_root_sha256.is_none() || !self.blocker.is_empty()))
            || (self.status != LawCertificateStatusV1::Pass && self.blocker.is_empty())
            || self.certificate_root_sha256 != self.expected_root()?
        {
            return Err("law_certificate_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            LAW_CERTIFICATE_SCHEMA_V1,
            self.bundle_id_sha256.as_str(),
            self.package_id.as_str(),
            self.status,
            &self.evidence_roots_sha256,
            self.cleanup_receipt_root_sha256.as_deref(),
            self.blocker.as_str(),
        ))
    }
}

impl MechanismCertificateV1 {
    pub fn seal(
        bundle_id_sha256: &str,
        package_id: &str,
        status: MechanismCertificateStatusV1,
        classification: OperatorMechanismClassV1,
        evidence_roots_sha256: Vec<String>,
        blocker: &str,
    ) -> Result<Self, &'static str> {
        let evidence_roots_sha256 = canonical_roots(evidence_roots_sha256)?;
        let mut certificate = Self {
            schema: MECHANISM_CERTIFICATE_SCHEMA_V1.to_owned(),
            certificate_root_sha256: String::new(),
            bundle_id_sha256: bundle_id_sha256.to_owned(),
            package_id: package_id.to_owned(),
            status,
            classification,
            evidence_roots_sha256,
            blocker: blocker.to_owned(),
        };
        certificate.certificate_root_sha256 = certificate.expected_root()?;
        certificate.validate()?;
        Ok(certificate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        validate_binding(&self.schema, MECHANISM_CERTIFICATE_SCHEMA_V1, self)?;
        let terminal_pass = self.status == MechanismCertificateStatusV1::Pass;
        if self.evidence_roots_sha256.is_empty()
            || (terminal_pass && self.classification == OperatorMechanismClassV1::Unresolved)
            || (!terminal_pass && self.classification != OperatorMechanismClassV1::Unresolved)
            || (terminal_pass && !self.blocker.is_empty())
            || (!terminal_pass && self.blocker.is_empty())
            || self.certificate_root_sha256 != self.expected_root()?
        {
            return Err("mechanism_certificate_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MECHANISM_CERTIFICATE_SCHEMA_V1,
            self.bundle_id_sha256.as_str(),
            self.package_id.as_str(),
            self.status,
            self.classification,
            &self.evidence_roots_sha256,
            self.blocker.as_str(),
        ))
    }
}

impl ExactMemoryCleanupReceiptV1 {
    pub fn seal(
        bundle_id_sha256: &str,
        package_id: &str,
        candidate_root_sha256: &str,
        active_registry_root_sha256: &str,
        standalone_restart_root_sha256: &str,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: EXACT_MEMORY_CLEANUP_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            bundle_id_sha256: bundle_id_sha256.to_owned(),
            package_id: package_id.to_owned(),
            candidate_root_sha256: candidate_root_sha256.to_owned(),
            active_registry_root_sha256: active_registry_root_sha256.to_owned(),
            standalone_restart_root_sha256: standalone_restart_root_sha256.to_owned(),
            learner_state_absent: true,
            raw_evidence_absent: true,
            exact_example_authority_absent: true,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let roots = [
            self.receipt_root_sha256.as_str(),
            self.bundle_id_sha256.as_str(),
            self.candidate_root_sha256.as_str(),
            self.active_registry_root_sha256.as_str(),
            self.standalone_restart_root_sha256.as_str(),
        ];
        if self.schema != EXACT_MEMORY_CLEANUP_RECEIPT_SCHEMA_V1
            || self.package_id.is_empty()
            || !roots.into_iter().all(valid_nonzero_sha256)
            || !self.learner_state_absent
            || !self.raw_evidence_absent
            || !self.exact_example_authority_absent
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err("exact_memory_cleanup_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            EXACT_MEMORY_CLEANUP_RECEIPT_SCHEMA_V1,
            self.bundle_id_sha256.as_str(),
            self.package_id.as_str(),
            self.candidate_root_sha256.as_str(),
            self.active_registry_root_sha256.as_str(),
            self.standalone_restart_root_sha256.as_str(),
            true,
            true,
            true,
        ))
    }
}

impl OperatorCertificationEntryV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        bundle_id_sha256: &str,
        package_id: &str,
        semantic_law_id_sha256: &str,
        role_topology_id_sha256: &str,
        execution: ExecutionCertificateV1,
        law: LawCertificateV1,
        mechanism: MechanismCertificateV1,
        false_bad_apply: u64,
    ) -> Result<Self, &'static str> {
        let product_registry_member = execution.status == ExecutionCertificateStatusV1::Pass;
        let epistemic_registry_member = law.status == LawCertificateStatusV1::Pass;
        let k1_unit_eligible =
            product_registry_member && epistemic_registry_member && false_bad_apply == 0;
        let mut entry = Self {
            schema: OPERATOR_CERTIFICATION_ENTRY_SCHEMA_V1.to_owned(),
            entry_root_sha256: String::new(),
            bundle_id_sha256: bundle_id_sha256.to_owned(),
            package_id: package_id.to_owned(),
            semantic_law_id_sha256: semantic_law_id_sha256.to_owned(),
            role_topology_id_sha256: role_topology_id_sha256.to_owned(),
            execution,
            law,
            mechanism,
            false_bad_apply,
            product_registry_member,
            epistemic_registry_member,
            k1_unit_eligible,
        };
        entry.entry_root_sha256 = entry.expected_root()?;
        entry.validate()?;
        Ok(entry)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.execution.validate()?;
        self.law.validate()?;
        self.mechanism.validate()?;
        let roots = [
            self.entry_root_sha256.as_str(),
            self.bundle_id_sha256.as_str(),
            self.semantic_law_id_sha256.as_str(),
            self.role_topology_id_sha256.as_str(),
        ];
        let binding_matches = [
            &self.execution.bundle_id_sha256,
            &self.law.bundle_id_sha256,
            &self.mechanism.bundle_id_sha256,
        ]
        .into_iter()
        .all(|bundle| bundle == &self.bundle_id_sha256)
            && [
                &self.execution.package_id,
                &self.law.package_id,
                &self.mechanism.package_id,
            ]
            .into_iter()
            .all(|package| package == &self.package_id);
        let expected_product = self.execution.status == ExecutionCertificateStatusV1::Pass;
        let expected_epistemic = self.law.status == LawCertificateStatusV1::Pass;
        if self.schema != OPERATOR_CERTIFICATION_ENTRY_SCHEMA_V1
            || self.package_id.is_empty()
            || !roots.into_iter().all(valid_nonzero_sha256)
            || !binding_matches
            || self.product_registry_member != expected_product
            || self.epistemic_registry_member != expected_epistemic
            || self.k1_unit_eligible
                != (expected_product && expected_epistemic && self.false_bad_apply == 0)
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err("operator_certification_entry_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            OPERATOR_CERTIFICATION_ENTRY_SCHEMA_V1,
            self.bundle_id_sha256.as_str(),
            self.package_id.as_str(),
            self.semantic_law_id_sha256.as_str(),
            self.role_topology_id_sha256.as_str(),
            self.execution.certificate_root_sha256.as_str(),
            self.law.certificate_root_sha256.as_str(),
            self.mechanism.certificate_root_sha256.as_str(),
            self.false_bad_apply,
            self.product_registry_member,
            self.epistemic_registry_member,
            self.k1_unit_eligible,
        ))
    }
}

impl OperatorCertificationLedgerV1 {
    pub fn empty() -> Result<Self, &'static str> {
        let mut ledger = Self {
            schema: OPERATOR_CERTIFICATION_LEDGER_SCHEMA_V1.to_owned(),
            ledger_root_sha256: String::new(),
            revision: 0,
            entries: Vec::new(),
        };
        ledger.reseal()?;
        Ok(ledger)
    }

    pub fn append(&mut self, entry: OperatorCertificationEntryV1) -> Result<bool, &'static str> {
        self.validate()?;
        entry.validate()?;
        let previous = self
            .entries
            .iter()
            .rev()
            .find(|existing| existing.package_id == entry.package_id);
        if let Some(previous) = previous {
            if previous.entry_root_sha256 == entry.entry_root_sha256 {
                return Ok(false);
            }
            validate_entry_transition(previous, &entry)?;
        }
        self.entries.push(entry);
        self.revision = u64::try_from(self.entries.len()).map_err(|_| "certification_revision")?;
        self.reseal()?;
        self.validate()?;
        Ok(true)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let expected_revision =
            u64::try_from(self.entries.len()).map_err(|_| "certification_revision")?;
        let mut roots = BTreeSet::new();
        if self.schema != OPERATOR_CERTIFICATION_LEDGER_SCHEMA_V1
            || self.revision != expected_revision
            || !valid_nonzero_sha256(&self.ledger_root_sha256)
            || self.entries.iter().any(|entry| entry.validate().is_err())
            || self
                .entries
                .iter()
                .any(|entry| !roots.insert(entry.entry_root_sha256.as_str()))
            || self.ledger_root_sha256 != self.expected_root()?
        {
            return Err("operator_certification_ledger_invalid");
        }
        let mut latest: BTreeMap<&str, &OperatorCertificationEntryV1> = BTreeMap::new();
        for entry in &self.entries {
            if let Some(previous) = latest.insert(entry.package_id.as_str(), entry) {
                validate_entry_transition(previous, entry)?;
            }
        }
        Ok(())
    }

    pub fn latest_entries(&self) -> Vec<&OperatorCertificationEntryV1> {
        let mut latest = BTreeMap::new();
        for entry in &self.entries {
            latest.insert(entry.package_id.as_str(), entry);
        }
        latest.into_values().collect()
    }

    pub fn k1_vocabulary_gate(&self) -> Result<K1VocabularyGateV1, &'static str> {
        self.validate()?;
        K1VocabularyGateV1::seal(self.latest_entries())
    }

    fn reseal(&mut self) -> Result<(), &'static str> {
        self.ledger_root_sha256 = self.expected_root()?;
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            OPERATOR_CERTIFICATION_LEDGER_SCHEMA_V1,
            self.revision,
            self.entries
                .iter()
                .map(|entry| entry.entry_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
    }
}

impl K1VocabularyGateV1 {
    fn seal(entries: Vec<&OperatorCertificationEntryV1>) -> Result<Self, &'static str> {
        let eligible = entries
            .into_iter()
            .filter(|entry| entry.k1_unit_eligible)
            .collect::<Vec<_>>();
        let semantic_laws = eligible
            .iter()
            .map(|entry| entry.semantic_law_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let role_topologies = eligible
            .iter()
            .map(|entry| entry.role_topology_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let law_certificates = u64::try_from(eligible.len()).map_err(|_| "k1_count")?;
        let cleanup_receipts = u64::try_from(
            eligible
                .iter()
                .filter(|entry| entry.law.cleanup_receipt_root_sha256.is_some())
                .count(),
        )
        .map_err(|_| "k1_count")?;
        let false_bad_apply = eligible
            .iter()
            .map(|entry| entry.false_bad_apply)
            .sum::<u64>();
        let semantic_laws = u64::try_from(semantic_laws.len()).map_err(|_| "k1_count")?;
        let role_topologies = u64::try_from(role_topologies.len()).map_err(|_| "k1_count")?;
        let open = law_certificates >= K1_MIN_LAW_CERTIFICATES
            && semantic_laws >= K1_MIN_SEMANTIC_LAWS
            && role_topologies >= K1_MIN_ROLE_TOPOLOGIES
            && cleanup_receipts == law_certificates
            && false_bad_apply == 0;
        let blocker = if open {
            ""
        } else if law_certificates < K1_MIN_LAW_CERTIFICATES {
            "law_certificate_count_below_k1_minimum"
        } else if semantic_laws < K1_MIN_SEMANTIC_LAWS {
            "semantic_law_diversity_below_k1_minimum"
        } else if role_topologies < K1_MIN_ROLE_TOPOLOGIES {
            "role_topology_diversity_below_k1_minimum"
        } else if cleanup_receipts != law_certificates {
            "exact_memory_cleanup_incomplete"
        } else {
            "false_bad_apply_nonzero"
        }
        .to_owned();
        let mut gate = Self {
            schema: K1_VOCABULARY_GATE_SCHEMA_V1.to_owned(),
            gate_root_sha256: String::new(),
            law_certificates,
            semantic_laws,
            role_topologies,
            cleanup_receipts,
            false_bad_apply,
            min_law_certificates: K1_MIN_LAW_CERTIFICATES,
            min_semantic_laws: K1_MIN_SEMANTIC_LAWS,
            min_role_topologies: K1_MIN_ROLE_TOPOLOGIES,
            open,
            blocker,
        };
        gate.gate_root_sha256 = gate.expected_root()?;
        gate.validate()?;
        Ok(gate)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let expected_open = self.law_certificates >= self.min_law_certificates
            && self.semantic_laws >= self.min_semantic_laws
            && self.role_topologies >= self.min_role_topologies
            && self.cleanup_receipts == self.law_certificates
            && self.false_bad_apply == 0;
        if self.schema != K1_VOCABULARY_GATE_SCHEMA_V1
            || !valid_nonzero_sha256(&self.gate_root_sha256)
            || self.min_law_certificates != K1_MIN_LAW_CERTIFICATES
            || self.min_semantic_laws != K1_MIN_SEMANTIC_LAWS
            || self.min_role_topologies != K1_MIN_ROLE_TOPOLOGIES
            || self.open != expected_open
            || self.open != self.blocker.is_empty()
            || self.gate_root_sha256 != self.expected_root()?
        {
            return Err("k1_vocabulary_gate_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_VOCABULARY_GATE_SCHEMA_V1,
            self.law_certificates,
            self.semantic_laws,
            self.role_topologies,
            self.cleanup_receipts,
            self.false_bad_apply,
            self.min_law_certificates,
            self.min_semantic_laws,
            self.min_role_topologies,
            self.open,
            self.blocker.as_str(),
        ))
    }
}

fn validate_binding<T>(schema: &str, expected_schema: &str, value: &T) -> Result<(), &'static str>
where
    T: CertificateBinding,
{
    if schema != expected_schema
        || value.package_id().is_empty()
        || !valid_nonzero_sha256(value.bundle_id())
        || !valid_nonzero_sha256(value.certificate_root())
        || !canonical_roots_are_valid(value.evidence_roots())
    {
        return Err("operator_certificate_binding_invalid");
    }
    Ok(())
}

trait CertificateBinding {
    fn certificate_root(&self) -> &str;
    fn bundle_id(&self) -> &str;
    fn package_id(&self) -> &str;
    fn evidence_roots(&self) -> &[String];
}

macro_rules! certificate_binding {
    ($type:ty) => {
        impl CertificateBinding for $type {
            fn certificate_root(&self) -> &str {
                &self.certificate_root_sha256
            }
            fn bundle_id(&self) -> &str {
                &self.bundle_id_sha256
            }
            fn package_id(&self) -> &str {
                &self.package_id
            }
            fn evidence_roots(&self) -> &[String] {
                &self.evidence_roots_sha256
            }
        }
    };
}

certificate_binding!(ExecutionCertificateV1);
certificate_binding!(LawCertificateV1);
certificate_binding!(MechanismCertificateV1);

fn canonical_roots(mut roots: Vec<String>) -> Result<Vec<String>, &'static str> {
    roots.sort();
    roots.dedup();
    canonical_roots_are_valid(&roots)
        .then_some(roots)
        .ok_or("operator_certificate_evidence_roots_invalid")
}

fn canonical_roots_are_valid(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

fn validate_entry_transition(
    previous: &OperatorCertificationEntryV1,
    next: &OperatorCertificationEntryV1,
) -> Result<(), &'static str> {
    if previous.package_id != next.package_id
        || previous.bundle_id_sha256 != next.bundle_id_sha256
        || previous.semantic_law_id_sha256 != next.semantic_law_id_sha256
        || previous.role_topology_id_sha256 != next.role_topology_id_sha256
        || matches!(
            (previous.execution.status, next.execution.status),
            (
                ExecutionCertificateStatusV1::Pass,
                ExecutionCertificateStatusV1::Pending
            ) | (
                ExecutionCertificateStatusV1::Revoked,
                ExecutionCertificateStatusV1::Pending | ExecutionCertificateStatusV1::Pass
            )
        )
        || matches!(
            (previous.law.status, next.law.status),
            (
                LawCertificateStatusV1::Pass,
                LawCertificateStatusV1::Partial | LawCertificateStatusV1::Legacy
            ) | (
                LawCertificateStatusV1::Rejected,
                LawCertificateStatusV1::Partial
                    | LawCertificateStatusV1::Pass
                    | LawCertificateStatusV1::Legacy
            ) | (
                LawCertificateStatusV1::Legacy,
                LawCertificateStatusV1::Partial | LawCertificateStatusV1::Pass
            ) | (
                LawCertificateStatusV1::Partial,
                LawCertificateStatusV1::Legacy
            )
        )
        || matches!(
            (previous.mechanism.status, next.mechanism.status),
            (
                MechanismCertificateStatusV1::Collecting,
                MechanismCertificateStatusV1::NotEvaluated
            )
        )
        || matches!(
            previous.mechanism.status,
            MechanismCertificateStatusV1::Pass | MechanismCertificateStatusV1::Fail
        ) && previous.mechanism.status != next.mechanism.status
    {
        return Err("operator_certification_transition_invalid");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn entry(
        package: &str,
        law_id: char,
        topology_id: char,
        law_status: LawCertificateStatusV1,
        mechanism_status: MechanismCertificateStatusV1,
        mechanism_class: OperatorMechanismClassV1,
    ) -> OperatorCertificationEntryV1 {
        let bundle = root('a');
        let cleanup = (law_status == LawCertificateStatusV1::Pass).then(|| root('f'));
        let execution = ExecutionCertificateV1::seal(
            &bundle,
            package,
            ExecutionCertificateStatusV1::Pass,
            vec![root('b'), root('c')],
            "",
        )
        .expect("execution");
        let law = LawCertificateV1::seal(
            &bundle,
            package,
            law_status,
            vec![root('c'), root('d')],
            cleanup,
            if law_status == LawCertificateStatusV1::Pass {
                ""
            } else {
                "cleanup_missing"
            },
        )
        .expect("law");
        let mechanism = MechanismCertificateV1::seal(
            &bundle,
            package,
            mechanism_status,
            mechanism_class,
            vec![root('e')],
            if mechanism_status == MechanismCertificateStatusV1::Pass {
                ""
            } else {
                "collecting"
            },
        )
        .expect("mechanism");
        OperatorCertificationEntryV1::seal(
            &bundle,
            package,
            &root(law_id),
            &root(topology_id),
            execution,
            law,
            mechanism,
            0,
        )
        .expect("entry")
    }

    #[test]
    fn wave_failure_does_not_revoke_execution_or_law() {
        let certified = entry(
            "package-one",
            '1',
            '4',
            LawCertificateStatusV1::Pass,
            MechanismCertificateStatusV1::Fail,
            OperatorMechanismClassV1::Unresolved,
        );
        assert!(certified.product_registry_member);
        assert!(certified.epistemic_registry_member);
        assert!(certified.k1_unit_eligible);
    }

    #[test]
    fn partial_law_never_enters_epistemic_registry() {
        let partial = entry(
            "package-one",
            '1',
            '4',
            LawCertificateStatusV1::Partial,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
        );
        assert!(partial.product_registry_member);
        assert!(!partial.epistemic_registry_member);
        assert!(!partial.k1_unit_eligible);
    }

    #[test]
    fn append_only_ledger_rejects_certificate_downgrades() {
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("ledger");
        let passed = entry(
            "operator-a",
            '1',
            'a',
            LawCertificateStatusV1::Pass,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
        );
        ledger.append(passed).expect("pass entry");

        let mut downgraded = entry(
            "operator-a",
            '1',
            'a',
            LawCertificateStatusV1::Partial,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
        );
        downgraded.execution = ExecutionCertificateV1::seal(
            &downgraded.bundle_id_sha256,
            &downgraded.package_id,
            ExecutionCertificateStatusV1::Pending,
            vec![root('8')],
            "ordinary_cpu_completion_pending",
        )
        .expect("pending execution");
        downgraded.product_registry_member = false;
        downgraded.epistemic_registry_member = false;
        downgraded.k1_unit_eligible = false;
        downgraded.entry_root_sha256 = downgraded.expected_root().expect("entry root");

        assert_eq!(
            ledger.append(downgraded),
            Err("operator_certification_transition_invalid")
        );
        assert_eq!(ledger.revision, 1);
    }

    #[test]
    fn unchanged_package_append_is_idempotent_across_interleaved_packages() {
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("ledger");
        let first = entry(
            "operator-a",
            '1',
            'a',
            LawCertificateStatusV1::Partial,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
        );
        ledger.append(first.clone()).expect("first package");
        ledger
            .append(entry(
                "operator-b",
                '2',
                'b',
                LawCertificateStatusV1::Partial,
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
            ))
            .expect("second package");

        assert!(!ledger.append(first).expect("idempotent first package"));
        assert_eq!(ledger.revision, 2);
        assert_eq!(ledger.latest_entries().len(), 2);
    }

    #[test]
    fn k1_requires_law_and_topology_diversity() {
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("ledger");
        ledger
            .append(entry(
                "package-one",
                '1',
                '4',
                LawCertificateStatusV1::Pass,
                MechanismCertificateStatusV1::Fail,
                OperatorMechanismClassV1::Unresolved,
            ))
            .expect("append one");
        ledger
            .append(entry(
                "package-two",
                '2',
                '4',
                LawCertificateStatusV1::Pass,
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
            ))
            .expect("append two");
        assert!(!ledger.k1_vocabulary_gate().expect("gate").open);
        ledger
            .append(entry(
                "package-three",
                '3',
                '5',
                LawCertificateStatusV1::Pass,
                MechanismCertificateStatusV1::Pass,
                OperatorMechanismClassV1::WaveCausal,
            ))
            .expect("append three");
        let gate = ledger.k1_vocabulary_gate().expect("gate");
        assert!(gate.open);
        assert_eq!(gate.semantic_laws, 3);
        assert_eq!(gate.role_topologies, 2);
    }
}
