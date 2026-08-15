use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, composition_decode_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CASE_JOURNAL_EVENT_SCHEMA_V2, K2_UNCERTAINTY_CASE_JOURNAL_SCHEMA_V2,
    K2UncertaintyPlanDispatchV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

const CASE_JOURNAL_FILE_V2: &str = "case-journal-v2.cbor";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyCaseJournalEventKindV2 {
    PlanDispatched,
    ProbeExecutionStarted,
    ProbeObservationFrozen,
    ObservationVectorFrozen,
    CaseTerminal,
    ModelsUpdated,
    CleanupFrozen,
    IndeterminateExecutionFrozen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyCaseJournalFaultV2 {
    None,
    BeforeRename,
    AfterRename,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCaseJournalEventV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub sequence: u64,
    pub kind: K2UncertaintyCaseJournalEventKindV2,
    pub probe_ordinal: Option<u64>,
    pub workspace_identity_root_sha256: Option<String>,
    pub previous_event_root_sha256: Option<String>,
    pub owner_executable_sha256: String,
    pub request_root_sha256: String,
    pub payload_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub event_root_sha256: String,
}

impl K2UncertaintyCaseJournalEventV2 {
    fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.owner_executable_sha256,
            &self.request_root_sha256,
            &self.payload_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if let Some(root) = &self.workspace_identity_root_sha256 {
            require_composition_root_v1(root)?;
        }
        if let Some(root) = &self.previous_event_root_sha256 {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CASE_JOURNAL_EVENT_SCHEMA_V2
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_event_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CASE_JOURNAL_EVENT_SCHEMA_V2,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            self.sequence,
            self.kind,
            self.probe_ordinal,
            &self.workspace_identity_root_sha256,
            &self.previous_event_root_sha256,
            &self.owner_executable_sha256,
            &self.request_root_sha256,
            &self.payload_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCaseJournalStateV2 {
    pub schema: String,
    pub dispatch: K2UncertaintyPlanDispatchV2,
    pub events: Vec<K2UncertaintyCaseJournalEventV2>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub journal_root_sha256: String,
}

impl K2UncertaintyCaseJournalStateV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.dispatch.validate()?;
        let mut machine = CaseMachineV2::AwaitingPlanDispatch;
        let mut previous: Option<&str> = None;
        for (sequence, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.case_id_sha256 != self.dispatch.closure_plan.case_id_sha256
                || event.closure_plan_root_sha256 != self.dispatch.closure_plan.plan_root_sha256
                || event.sequence != sequence as u64
                || event.previous_event_root_sha256.as_deref() != previous
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_case_journal_chain_v2_invalid",
                ));
            }
            machine = apply_event_v2(&self.dispatch, machine, event)?;
            previous = Some(&event.event_root_sha256);
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CASE_JOURNAL_SCHEMA_V2
            || self.journal_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let event_roots = self
            .events
            .iter()
            .map(|value| value.event_root_sha256.as_str())
            .collect::<Vec<_>>();
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CASE_JOURNAL_SCHEMA_V2,
            &self.dispatch.dispatch_root_sha256,
            event_roots,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum K2UncertaintyCaseJournalPhaseV2 {
    AwaitingPlanDispatch,
    ReadyForProbe { probe_ordinal: u64 },
    IndeterminateExecution { probe_ordinal: u64 },
    ReadyForObservationVector,
    ObservationVectorFrozen,
    CaseTerminal,
    ModelsUpdated,
    CleanupFrozen,
    IndeterminateTerminal { probe_ordinal: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyCaseJournalProjectionV2 {
    pub phase: K2UncertaintyCaseJournalPhaseV2,
    pub last_event_root_sha256: Option<String>,
    pub journal_root_sha256: String,
}

#[derive(Debug)]
pub struct K2UncertaintyExecutionPermitV2 {
    case_id_sha256: String,
    closure_plan_root_sha256: String,
    dispatch_root_sha256: String,
    probe_ordinal: u64,
    workspace_identity_root_sha256: String,
    execution_started_event_root_sha256: String,
}

pub struct K2UncertaintyCaseJournalV2 {
    root: PathBuf,
    state: K2UncertaintyCaseJournalStateV2,
}

impl K2UncertaintyCaseJournalV2 {
    pub fn create(
        root: &Path,
        dispatch: K2UncertaintyPlanDispatchV2,
    ) -> K2CompositionResultV1<Self> {
        dispatch.validate()?;
        fs::create_dir_all(root)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_case_journal_v2"))?;
        let path = root.join(CASE_JOURNAL_FILE_V2);
        if path.exists() {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_v2_already_exists",
            ));
        }
        let mut state = K2UncertaintyCaseJournalStateV2 {
            schema: K2_UNCERTAINTY_CASE_JOURNAL_SCHEMA_V2.to_owned(),
            dispatch,
            events: Vec::new(),
            authority: denied_authority_v1(),
            journal_root_sha256: String::new(),
        };
        state.journal_root_sha256 = state.expected_root()?;
        state.validate()?;
        atomic_write_case_journal_v2(
            root,
            &path,
            &composition_bytes_v1(&state)?,
            0,
            K2UncertaintyCaseJournalFaultV2::None,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            state,
        })
    }

    pub fn reopen(root: &Path) -> K2CompositionResultV1<Self> {
        let bytes = fs::read(root.join(CASE_JOURNAL_FILE_V2))
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_case_journal_v2"))?;
        let state: K2UncertaintyCaseJournalStateV2 = composition_decode_v1(&bytes)?;
        state.validate()?;
        Ok(Self {
            root: root.to_path_buf(),
            state,
        })
    }

    pub fn state(&self) -> &K2UncertaintyCaseJournalStateV2 {
        &self.state
    }

    pub fn projection(&self) -> K2CompositionResultV1<K2UncertaintyCaseJournalProjectionV2> {
        self.state.validate()?;
        let machine = project_machine_v2(&self.state)?;
        let phase = match machine {
            CaseMachineV2::AwaitingPlanDispatch => {
                K2UncertaintyCaseJournalPhaseV2::AwaitingPlanDispatch
            }
            CaseMachineV2::ReadyForProbe(probe_ordinal) => {
                K2UncertaintyCaseJournalPhaseV2::ReadyForProbe { probe_ordinal }
            }
            CaseMachineV2::Executing(probe_ordinal) => {
                K2UncertaintyCaseJournalPhaseV2::IndeterminateExecution { probe_ordinal }
            }
            CaseMachineV2::ReadyForObservationVector => {
                K2UncertaintyCaseJournalPhaseV2::ReadyForObservationVector
            }
            CaseMachineV2::ObservationVectorFrozen => {
                K2UncertaintyCaseJournalPhaseV2::ObservationVectorFrozen
            }
            CaseMachineV2::CaseTerminal => K2UncertaintyCaseJournalPhaseV2::CaseTerminal,
            CaseMachineV2::ModelsUpdated => K2UncertaintyCaseJournalPhaseV2::ModelsUpdated,
            CaseMachineV2::CleanupFrozen => K2UncertaintyCaseJournalPhaseV2::CleanupFrozen,
            CaseMachineV2::IndeterminateTerminal(probe_ordinal) => {
                K2UncertaintyCaseJournalPhaseV2::IndeterminateTerminal { probe_ordinal }
            }
        };
        Ok(K2UncertaintyCaseJournalProjectionV2 {
            phase,
            last_event_root_sha256: self
                .state
                .events
                .last()
                .map(|value| value.event_root_sha256.clone()),
            journal_root_sha256: self.state.journal_root_sha256.clone(),
        })
    }

    pub fn record_plan_dispatch(
        &mut self,
        owner_executable_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        if self.projection()?.phase != K2UncertaintyCaseJournalPhaseV2::AwaitingPlanDispatch {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_plan_redispatch_v2",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::PlanDispatched,
            None,
            None,
            owner_executable_sha256,
            self.state.dispatch.batch_precommit_root_sha256.clone(),
            self.state.dispatch.dispatch_root_sha256.clone(),
            fault,
        )?;
        Ok(())
    }

    pub fn begin_probe_execution(
        &mut self,
        probe_ordinal: u64,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<K2UncertaintyExecutionPermitV2> {
        if self.projection()?.phase
            != (K2UncertaintyCaseJournalPhaseV2::ReadyForProbe { probe_ordinal })
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_probe_redispatch_v2",
            ));
        }
        let item = self.dispatch_item_v2(probe_ordinal)?.clone();
        let event = self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::ProbeExecutionStarted,
            Some(probe_ordinal),
            Some(item.workspace_identity.identity_root_sha256.clone()),
            item.worker_request.worker_executable_sha256,
            item.worker_request.request_root_sha256,
            item.item_root_sha256,
            fault,
        )?;
        Ok(K2UncertaintyExecutionPermitV2 {
            case_id_sha256: self.state.dispatch.closure_plan.case_id_sha256.clone(),
            closure_plan_root_sha256: self.state.dispatch.closure_plan.plan_root_sha256.clone(),
            dispatch_root_sha256: self.state.dispatch.dispatch_root_sha256.clone(),
            probe_ordinal,
            workspace_identity_root_sha256: item.workspace_identity.identity_root_sha256,
            execution_started_event_root_sha256: event.event_root_sha256,
        })
    }

    pub fn record_probe_observation(
        &mut self,
        permit: K2UncertaintyExecutionPermitV2,
        observation_receipt_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&observation_receipt_root_sha256)?;
        let item = self.dispatch_item_v2(permit.probe_ordinal)?.clone();
        let last = self
            .state
            .events
            .last()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_observation_without_execution_v2",
            ))?;
        if permit.case_id_sha256 != self.state.dispatch.closure_plan.case_id_sha256
            || permit.closure_plan_root_sha256 != self.state.dispatch.closure_plan.plan_root_sha256
            || permit.dispatch_root_sha256 != self.state.dispatch.dispatch_root_sha256
            || permit.workspace_identity_root_sha256 != item.workspace_identity.identity_root_sha256
            || permit.execution_started_event_root_sha256 != last.event_root_sha256
            || last.kind != K2UncertaintyCaseJournalEventKindV2::ProbeExecutionStarted
            || last.probe_ordinal != Some(permit.probe_ordinal)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_observation_permit_v2_invalid",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::ProbeObservationFrozen,
            Some(permit.probe_ordinal),
            Some(item.workspace_identity.identity_root_sha256),
            item.observer_request.observer_executable_sha256,
            item.observer_request.request_root_sha256,
            observation_receipt_root_sha256,
            fault,
        )?;
        Ok(())
    }

    pub fn freeze_observation_vector(
        &mut self,
        owner_executable_sha256: String,
        request_root_sha256: String,
        observation_vector_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        if self.projection()?.phase != K2UncertaintyCaseJournalPhaseV2::ReadyForObservationVector {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_observation_vector_order_v2_invalid",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::ObservationVectorFrozen,
            None,
            None,
            owner_executable_sha256,
            request_root_sha256,
            observation_vector_root_sha256,
            fault,
        )?;
        Ok(())
    }

    pub fn record_models_updated(
        &mut self,
        owner_executable_sha256: String,
        verifier_request_root_sha256: String,
        verifier_receipt_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        if self.projection()?.phase != K2UncertaintyCaseJournalPhaseV2::CaseTerminal {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_models_update_order_v2_invalid",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::ModelsUpdated,
            None,
            None,
            owner_executable_sha256,
            verifier_request_root_sha256,
            verifier_receipt_root_sha256,
            fault,
        )?;
        Ok(())
    }

    pub fn record_case_terminal(
        &mut self,
        owner_executable_sha256: String,
        verifier_request_root_sha256: String,
        verifier_receipt_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        if self.projection()?.phase != K2UncertaintyCaseJournalPhaseV2::ObservationVectorFrozen {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_terminal_order_v2_invalid",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::CaseTerminal,
            None,
            None,
            owner_executable_sha256,
            verifier_request_root_sha256,
            verifier_receipt_root_sha256,
            fault,
        )?;
        Ok(())
    }

    pub fn freeze_cleanup(
        &mut self,
        owner_executable_sha256: String,
        cleanup_request_root_sha256: String,
        cleanup_receipt_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        if self.projection()?.phase != K2UncertaintyCaseJournalPhaseV2::ModelsUpdated {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_cleanup_order_v2_invalid",
            ));
        }
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::CleanupFrozen,
            None,
            None,
            owner_executable_sha256,
            cleanup_request_root_sha256,
            cleanup_receipt_root_sha256,
            fault,
        )?;
        Ok(())
    }

    pub fn freeze_indeterminate_execution(
        &mut self,
        owner_executable_sha256: String,
        terminal_receipt_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<()> {
        let probe_ordinal = match self.projection()?.phase {
            K2UncertaintyCaseJournalPhaseV2::IndeterminateExecution { probe_ordinal } => {
                probe_ordinal
            }
            _ => {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_case_journal_indeterminate_order_v2_invalid",
                ));
            }
        };
        let item = self.dispatch_item_v2(probe_ordinal)?.clone();
        self.append_event_v2(
            K2UncertaintyCaseJournalEventKindV2::IndeterminateExecutionFrozen,
            Some(probe_ordinal),
            Some(item.workspace_identity.identity_root_sha256),
            owner_executable_sha256,
            item.worker_request.request_root_sha256,
            terminal_receipt_root_sha256,
            fault,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn append_event_v2(
        &mut self,
        kind: K2UncertaintyCaseJournalEventKindV2,
        probe_ordinal: Option<u64>,
        workspace_identity_root_sha256: Option<String>,
        owner_executable_sha256: String,
        request_root_sha256: String,
        payload_root_sha256: String,
        fault: K2UncertaintyCaseJournalFaultV2,
    ) -> K2CompositionResultV1<K2UncertaintyCaseJournalEventV2> {
        let mut event = K2UncertaintyCaseJournalEventV2 {
            schema: K2_UNCERTAINTY_CASE_JOURNAL_EVENT_SCHEMA_V2.to_owned(),
            case_id_sha256: self.state.dispatch.closure_plan.case_id_sha256.clone(),
            closure_plan_root_sha256: self.state.dispatch.closure_plan.plan_root_sha256.clone(),
            sequence: self.state.events.len() as u64,
            kind,
            probe_ordinal,
            workspace_identity_root_sha256,
            previous_event_root_sha256: self
                .state
                .events
                .last()
                .map(|value| value.event_root_sha256.clone()),
            owner_executable_sha256,
            request_root_sha256,
            payload_root_sha256,
            authority: denied_authority_v1(),
            event_root_sha256: String::new(),
        };
        event.event_root_sha256 = event.expected_root()?;
        event.validate()?;
        let mut next = self.state.clone();
        next.events.push(event.clone());
        next.journal_root_sha256 = next.expected_root()?;
        next.validate()?;
        let path = self.root.join(CASE_JOURNAL_FILE_V2);
        atomic_write_case_journal_v2(
            &self.root,
            &path,
            &composition_bytes_v1(&next)?,
            next.events.len() as u64,
            fault,
        )?;
        self.state = next;
        Ok(event)
    }

    fn dispatch_item_v2(
        &self,
        probe_ordinal: u64,
    ) -> K2CompositionResultV1<&super::K2UncertaintyProbeDispatchItemV2> {
        self.state
            .dispatch
            .items
            .get(probe_ordinal as usize)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_probe_ordinal_v2_invalid",
            ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaseMachineV2 {
    AwaitingPlanDispatch,
    ReadyForProbe(u64),
    Executing(u64),
    ReadyForObservationVector,
    ObservationVectorFrozen,
    CaseTerminal,
    ModelsUpdated,
    CleanupFrozen,
    IndeterminateTerminal(u64),
}

fn project_machine_v2(
    state: &K2UncertaintyCaseJournalStateV2,
) -> K2CompositionResultV1<CaseMachineV2> {
    let mut machine = CaseMachineV2::AwaitingPlanDispatch;
    for event in &state.events {
        machine = apply_event_v2(&state.dispatch, machine, event)?;
    }
    Ok(machine)
}

fn apply_event_v2(
    dispatch: &K2UncertaintyPlanDispatchV2,
    machine: CaseMachineV2,
    event: &K2UncertaintyCaseJournalEventV2,
) -> K2CompositionResultV1<CaseMachineV2> {
    use K2UncertaintyCaseJournalEventKindV2 as Event;
    let next = match (machine, event.kind) {
        (CaseMachineV2::AwaitingPlanDispatch, Event::PlanDispatched)
            if event.probe_ordinal.is_none()
                && event.workspace_identity_root_sha256.is_none()
                && event.payload_root_sha256 == dispatch.dispatch_root_sha256 =>
        {
            CaseMachineV2::ReadyForProbe(0)
        }
        (CaseMachineV2::ReadyForProbe(expected), Event::ProbeExecutionStarted)
            if event.probe_ordinal == Some(expected)
                && dispatch_item_matches_event_v2(dispatch, expected, event, true) =>
        {
            CaseMachineV2::Executing(expected)
        }
        (CaseMachineV2::Executing(expected), Event::ProbeObservationFrozen)
            if event.probe_ordinal == Some(expected)
                && dispatch_item_matches_event_v2(dispatch, expected, event, false) =>
        {
            if expected + 1 < dispatch.closure_plan.plan_length {
                CaseMachineV2::ReadyForProbe(expected + 1)
            } else {
                CaseMachineV2::ReadyForObservationVector
            }
        }
        (CaseMachineV2::ReadyForObservationVector, Event::ObservationVectorFrozen)
            if event.probe_ordinal.is_none() && event.workspace_identity_root_sha256.is_none() =>
        {
            CaseMachineV2::ObservationVectorFrozen
        }
        (CaseMachineV2::ObservationVectorFrozen, Event::CaseTerminal)
            if event.probe_ordinal.is_none() && event.workspace_identity_root_sha256.is_none() =>
        {
            CaseMachineV2::CaseTerminal
        }
        (CaseMachineV2::CaseTerminal, Event::ModelsUpdated)
            if event.probe_ordinal.is_none() && event.workspace_identity_root_sha256.is_none() =>
        {
            CaseMachineV2::ModelsUpdated
        }
        (CaseMachineV2::ModelsUpdated, Event::CleanupFrozen)
            if event.probe_ordinal.is_none() && event.workspace_identity_root_sha256.is_none() =>
        {
            CaseMachineV2::CleanupFrozen
        }
        (CaseMachineV2::Executing(expected), Event::IndeterminateExecutionFrozen)
            if event.probe_ordinal == Some(expected)
                && dispatch_item_matches_workspace_v2(dispatch, expected, event) =>
        {
            CaseMachineV2::IndeterminateTerminal(expected)
        }
        _ => {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_journal_event_order_v2_invalid",
            ));
        }
    };
    Ok(next)
}

fn dispatch_item_matches_event_v2(
    dispatch: &K2UncertaintyPlanDispatchV2,
    probe_ordinal: u64,
    event: &K2UncertaintyCaseJournalEventV2,
    worker: bool,
) -> bool {
    let Some(item) = dispatch.items.get(probe_ordinal as usize) else {
        return false;
    };
    dispatch_item_matches_workspace_v2(dispatch, probe_ordinal, event)
        && if worker {
            event.owner_executable_sha256 == item.worker_request.worker_executable_sha256
                && event.request_root_sha256 == item.worker_request.request_root_sha256
                && event.payload_root_sha256 == item.item_root_sha256
        } else {
            event.owner_executable_sha256 == item.observer_request.observer_executable_sha256
                && event.request_root_sha256 == item.observer_request.request_root_sha256
        }
}

fn dispatch_item_matches_workspace_v2(
    dispatch: &K2UncertaintyPlanDispatchV2,
    probe_ordinal: u64,
    event: &K2UncertaintyCaseJournalEventV2,
) -> bool {
    dispatch
        .items
        .get(probe_ordinal as usize)
        .is_some_and(|item| {
            event.workspace_identity_root_sha256.as_deref()
                == Some(item.workspace_identity.identity_root_sha256.as_str())
        })
}

fn atomic_write_case_journal_v2(
    root: &Path,
    path: &Path,
    bytes: &[u8],
    sequence: u64,
    fault: K2UncertaintyCaseJournalFaultV2,
) -> K2CompositionResultV1<()> {
    let temporary = root.join(format!(".{CASE_JOURNAL_FILE_V2}.{sequence}.tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_case_journal_v2_temp"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_case_journal_v2_temp"))?;
    if fault == K2UncertaintyCaseJournalFaultV2::BeforeRename {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "self_formed_case_journal_v2_fault_before_rename",
        ));
    }
    fs::rename(&temporary, path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_case_journal_v2"))?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_case_journal_v2_directory"))?;
    if fault == K2UncertaintyCaseJournalFaultV2::AfterRename {
        return Err(K2CompositionErrorV1::Io(
            "self_formed_case_journal_v2_fault_after_rename",
        ));
    }
    Ok(())
}
