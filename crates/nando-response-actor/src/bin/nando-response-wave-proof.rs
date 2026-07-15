use nando_response_actor::{
    ResponseExecutionStatus, ResponseProgram, ResponseRegistry, ResponseRelationObservation,
    ResponseShadowObservation, compile_response_registry, execute_response,
};
use serde_json::json;

fn main() {
    let relations = (0..64)
        .map(|index| ResponseRelationObservation {
            schema: "nando.response-relation-observation.v1".to_owned(),
            relation_id: format!("proof-relation-{index}"),
            observed_at: format!("2026-07-{:02}T{:02}:00:00Z", 1 + index / 24, index % 24),
            relation: "yielded_cell_to_wait_function_call".to_owned(),
            program_hint: nando_response_actor::ResponseProgramHint {
                op: "wait_on_yielded_cell".to_owned(),
                prefix: String::new(),
            },
            source_session_id_sha256: format!("proof-session-{}", index % 4),
            source_turn_id_sha256: format!("proof-turn-{index}"),
            surface_id_sha256: format!("proof-surface-{}", index % 2),
            verifier_ok: true,
            guard_schema: "wait_long_running_batch_guard.v5".to_owned(),
        })
        .collect::<Vec<_>>();
    let mut registry =
        compile_response_registry(1, &relations, &[] as &[ResponseShadowObservation], true);
    let wait = registry.packages.pop().expect("wait package");
    let mut packages = vec![wait.clone()];
    for index in 0..15 {
        let mut distractor = wait.clone();
        distractor.package_id = format!("distractor-{index}");
        distractor.program = ResponseProgram::copy_after_prefix([format!("distractor-{index}:")]);
        distractor.phase_centers = vec![0x1000 + index];
        packages.push(distractor);
    }
    let executor = nando_response_actor::ResponseExecutor::from_registry(ResponseRegistry {
        schema: "nando.response-registry.v5".to_owned(),
        revision: 1,
        packages: packages.clone(),
    })
    .expect("executor");
    let mut correct = 0_u64;
    let mut wrong = 0_u64;
    let mut full_checks = 0_u64;
    for index in 0..1_440 {
        let cell_id = format!("heldout-{index}");
        let payload = json!({"input":[
            {"type":"function_call","name":"exec","call_id":"exec-1","arguments":"{\"cmd\":\"cargo test\"}"},
            {"type":"function_call_output","call_id":"exec-1","output":format!("Script running with cell ID {cell_id}\nWall time 10.0 seconds\nOutput:\n")}
        ]});
        let result = executor.execute_shadow("", &payload);
        full_checks += u64::try_from(result.exact_actor_checks).unwrap_or(u64::MAX);
        if result.status == ResponseExecutionStatus::Executed
            && result.package_id.as_deref() == Some("raw-phase-wait-on-yielded-cell-v1")
        {
            correct += 1;
        } else {
            wrong += 1;
        }
        let exhaustive_correct = packages
            .iter()
            .filter(|package| {
                execute_response(&package.program, "", &payload).status
                    == ResponseExecutionStatus::Executed
            })
            .count();
        assert_eq!(exhaustive_correct, 1);
    }
    let no_phase_checks = 1_440_u64 * u64::try_from(packages.len()).unwrap_or(u64::MAX);
    let proof = json!({
        "schema":"nando.response-wave-causal-proof.v1",
        "verdict": if correct == 1_440 && wrong == 0 && full_checks < no_phase_checks {"PASS"} else {"FAIL"},
        "heldout_correct":correct,
        "heldout_total":1_440,
        "wrong_accepts":wrong,
        "full_phase_exact_checks":full_checks,
        "no_phase_exact_checks":no_phase_checks,
        "shuffled_phase_exact_checks":no_phase_checks,
        "execution_authority": false,
        "registry_mode": "v5_shadow_diagnostic_only",
        "claim_boundary":"frozen causal ablation of response L2 routing; live promotion still requires chronological raw-phase support and future evidence",
    });
    println!("{}", serde_json::to_string_pretty(&proof).expect("proof"));
}
