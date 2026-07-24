use std::env;
use std::path::Path;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!(
            "usage: nando-multi-source-audit <response-checkpoint> \
             <request-learning-checkpoint> <relation-frames-jsonl>"
        );
        std::process::exit(2);
    }
    match nando_transition_serving::multi_source_audit::run_multi_source_discovery_audit_v3(
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
    ) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("audit report serializes")
            );
        }
        Err(error) => {
            eprintln!("multi_source_audit_error:{error}");
            std::process::exit(1);
        }
    }
}
