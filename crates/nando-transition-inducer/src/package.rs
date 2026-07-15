use nando_transition_actor::{ExecutionStatus, SurfaceAdapter, TransitionProgram, execute_surface};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{GuardProgram, PortableRoutingSignature, VerifierProgram};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InducedTransition {
    pub action_surface: String,
    pub program: TransitionProgram,
    pub adapter: SurfaceAdapter,
    pub guard: GuardProgram,
    pub verifier: VerifierProgram,
    pub routing_atoms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routing_atom_ids: Vec<u64>,
    pub wave_margin_micro: i64,
    pub support_traces: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InducedTransitionPackage {
    pub schema: String,
    pub package_id: String,
    pub transitions: Vec<InducedTransition>,
    pub wave_memory_bytes: usize,
    pub routing_signature: PortableRoutingSignature,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InducedExecutionStatus {
    Executed,
    Abstain,
    VerifyFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InducedExecution {
    pub status: InducedExecutionStatus,
    pub reason: String,
    pub after: Option<Value>,
    pub transition_index: Option<usize>,
}

impl InducedTransitionPackage {
    #[must_use]
    pub fn route_margin(&self, transition_index: usize) -> Option<i64> {
        self.transitions.get(transition_index).map(|transition| {
            if transition.routing_atom_ids.is_empty() {
                self.routing_signature
                    .score_atoms(&transition.routing_atoms)
            } else {
                self.routing_signature
                    .score_atom_ids(&transition.routing_atom_ids)
            }
        })
    }

    #[must_use]
    pub fn execute(&self, before: &Value, action: &Value) -> InducedExecution {
        for (index, transition) in self.transitions.iter().enumerate() {
            if transition.adapter.adapt_action(action).is_err() {
                continue;
            }
            return execute_transition(index, transition, before, action);
        }
        InducedExecution {
            status: InducedExecutionStatus::Abstain,
            reason: "action_surface_unrecognized".to_owned(),
            after: None,
            transition_index: None,
        }
    }

    #[must_use]
    pub fn execute_routed(&self, before: &Value, action: &Value) -> InducedExecution {
        self.execute_routed_indices(before, action, 0..self.transitions.len())
    }

    #[must_use]
    pub fn execute_routed_indices(
        &self,
        before: &Value,
        action: &Value,
        allowed_indices: impl IntoIterator<Item = usize>,
    ) -> InducedExecution {
        let transitions = &self.transitions;
        let selected = allowed_indices
            .into_iter()
            .filter_map(|index| transitions.get(index).map(|transition| (index, transition)))
            .filter(|(_, transition)| transition.adapter.adapt_action(action).is_ok())
            .filter_map(|(index, transition)| {
                self.route_margin(index)
                    .filter(|margin| *margin > 0)
                    .map(|margin| (margin, index, transition))
            })
            .max_by_key(|(margin, index, _)| (*margin, std::cmp::Reverse(*index)));
        let Some((_, index, transition)) = selected else {
            return InducedExecution {
                status: InducedExecutionStatus::Abstain,
                reason: "phase_route_not_found".to_owned(),
                after: None,
                transition_index: None,
            };
        };
        execute_transition(index, transition, before, action)
    }

    pub fn artifact_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

fn execute_transition(
    index: usize,
    transition: &InducedTransition,
    before: &Value,
    action: &Value,
) -> InducedExecution {
    if let Err(error) = transition.guard.check(&transition.adapter, before, action) {
        return InducedExecution {
            status: InducedExecutionStatus::Abstain,
            reason: format!("guard:{}", error.0),
            after: None,
            transition_index: Some(index),
        };
    }
    let result = execute_surface(&transition.program, &transition.adapter, before, action);
    if result.status != ExecutionStatus::Executed {
        return InducedExecution {
            status: InducedExecutionStatus::Abstain,
            reason: format!("actor:{}", result.reason),
            after: None,
            transition_index: Some(index),
        };
    }
    let Some(after) = result.concrete_after else {
        return InducedExecution {
            status: InducedExecutionStatus::VerifyFailed,
            reason: "actor_output_missing".to_owned(),
            after: None,
            transition_index: Some(index),
        };
    };
    if let Err(error) = transition
        .verifier
        .verify(&transition.adapter, before, action, &after)
    {
        return InducedExecution {
            status: InducedExecutionStatus::VerifyFailed,
            reason: format!("verifier:{}", error.0),
            after: None,
            transition_index: Some(index),
        };
    }
    InducedExecution {
        status: InducedExecutionStatus::Executed,
        reason: "executed".to_owned(),
        after: Some(after),
        transition_index: Some(index),
    }
}
