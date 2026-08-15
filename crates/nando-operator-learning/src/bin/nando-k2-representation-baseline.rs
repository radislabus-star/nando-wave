use nando_operator_learning::run_representation_baseline_process_v1;

fn main() {
    if let Err(error) = run_representation_baseline_process_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
