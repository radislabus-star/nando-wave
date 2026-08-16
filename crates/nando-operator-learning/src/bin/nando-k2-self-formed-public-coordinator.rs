use nando_operator_learning::run_self_formed_public_coordinator_process_v1;

fn main() {
    if let Err(error) = run_self_formed_public_coordinator_process_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
