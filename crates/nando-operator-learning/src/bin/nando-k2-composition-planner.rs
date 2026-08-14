use nando_operator_learning::run_composition_planner_process_v1;

fn main() {
    if let Err(error) = run_composition_planner_process_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
