use nando_operator_learning::run_self_formed_r7k_control_case_process_v1;

fn main() {
    if let Err(error) = run_self_formed_r7k_control_case_process_v1() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
