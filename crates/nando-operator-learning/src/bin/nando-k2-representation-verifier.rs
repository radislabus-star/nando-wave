use nando_operator_learning::run_hidden_representation_verifier_process_v1;

fn main() {
    if let Err(error) = run_hidden_representation_verifier_process_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
