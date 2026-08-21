use nando_operator_learning::run_self_formed_r8b_authorizer_process_v2;

fn main() {
    if let Err(error) = run_self_formed_r8b_authorizer_process_v2() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
