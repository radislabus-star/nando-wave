use nando_transition_serving::ServingConfig;
use nando_transition_serving::grounded_decision_census::{
    GroundedDecisionCensusConfigV1, run_grounded_decision_census_v1,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("nando-grounded-decision-census: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let serving = ServingConfig::from_env()?;
    let mut config = GroundedDecisionCensusConfigV1::from_serving_config(&serving);
    if let Some(output) = std::env::args_os().nth(1) {
        config.output_directory = output.into();
    }
    let run = run_grounded_decision_census_v1(&config)?;
    println!(
        "{}",
        serde_json::json!({
            "schema": run.census.schema,
            "report_root_sha256": run.census.report_root_sha256,
            "transition_rows_scanned": run.census.transition_rows_scanned,
            "transition_rows_projected": run.census.transition_rows_projected,
            "transition_rows_censored": run.census.transition_rows_censored,
            "transition_censor_counts": run.census.transition_censor_counts,
            "goal_bound": run.census.goal_bound,
            "alternative_bearing": run.census.alternative_bearing,
            "horizon_bound": run.census.horizon_bound,
            "satisfaction_verifiable": run.census.satisfaction_verifiable,
            "dynamics_only": run.census.dynamics_only,
            "decision_episodes": run.census.decision_episodes,
            "distinct_transition_lineages": run.census.distinct_transition_lineages,
            "distinct_decision_lineages": run.census.distinct_decision_lineages,
            "verdict": run.census.verdict,
            "blocker": run.census.blocker,
            "model_training_allowed": run.census.model_training_allowed,
            "authority_ready": run.census.authority_ready,
            "phase_mutation_allowed": run.census.phase_mutation_allowed,
        })
    );
    Ok(())
}
