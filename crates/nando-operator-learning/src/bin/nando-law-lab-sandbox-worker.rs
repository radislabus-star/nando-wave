use nando_operator_learning::run_law_lab_sandbox_worker_v1;

fn main() {
    if let Err(error) = run_law_lab_sandbox_worker_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
