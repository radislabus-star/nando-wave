use nando_response_actor::{DeterministicEvidenceLedger, EvidencePolicyV1};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: nando-evidence-checkpoint <ledger.jsonl>");
        std::process::exit(2);
    };
    match DeterministicEvidenceLedger::open(path, EvidencePolicyV1::streaming_bounded()) {
        Ok(ledger) => {
            let accounting = ledger.accounting();
            println!(
                "ingress={} normalized={} rejected={} duplicates={} conflicts={} identity={}",
                accounting.ingress_total,
                accounting.normalized_total,
                accounting.rejected_total,
                accounting.duplicate_idempotent_total,
                accounting.duplicate_conflict_total,
                accounting.identity_holds()
            );
        }
        Err(error) => {
            eprintln!("nando-evidence-checkpoint: {error}");
            std::process::exit(1);
        }
    }
}
