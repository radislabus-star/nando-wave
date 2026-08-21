use nando_operator_learning::run_self_formed_r8b_evidence_publisher_process_v3;

fn main() {
    if let Err(error) = run_self_formed_r8b_evidence_publisher_process_v3() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
