use std::{
    env, fs, process,
    time::{SystemTime, UNIX_EPOCH},
};

use nando_expression_runtime::ExpressionRuntime;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Artifact {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Candidate {
    schema: String,
    state: String,
    execution_authority: bool,
    source_prefix_bytes: usize,
    source_prefix_sha256: String,
    package: Artifact,
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: nando-expression-future-verify CANDIDATE SOURCE_JSONL");
        process::exit(2);
    }
    let candidate_bytes = fs::read(&args[1]).expect("read candidate");
    let candidate: Candidate = serde_json::from_slice(&candidate_bytes).expect("parse candidate");
    let source = fs::read(&args[2]).expect("read source");
    let package = fs::read(&candidate.package.path).expect("read package");
    let package_sha256 = format!("{:x}", Sha256::digest(&package));
    let prefix = source
        .get(..candidate.source_prefix_bytes)
        .expect("source shorter than candidate prefix");
    let prefix_sha256 = format!("{:x}", Sha256::digest(prefix));
    let runtime = ExpressionRuntime::load(&package).expect("load package");
    let tail = source
        .get(candidate.source_prefix_bytes..)
        .expect("tail boundary");
    let complete_tail = tail
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(&[][..], |end| &tail[..=end]);
    let mut observed = 0usize;
    let mut correct = 0usize;
    let mut wrong = 0usize;
    let mut abstain = 0usize;
    let mut malformed = 0usize;
    for line in complete_tail
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let Some((before, action, after)) = parse_frame(line) else {
            malformed += 1;
            continue;
        };
        observed += 1;
        match runtime.execute(&before, &action).after {
            Some(candidate_after) if candidate_after == after => correct += 1,
            Some(_) => wrong += 1,
            None => abstain += 1,
        }
    }
    let contract_bound = candidate.schema == "nando.expression-quarantine-candidate.v1"
        && candidate.state == "quarantine"
        && !candidate.execution_authority
        && package_sha256 == candidate.package.sha256
        && prefix_sha256 == candidate.source_prefix_sha256;
    let verdict = if !contract_bound || wrong > 0 || malformed > 0 {
        "VETO"
    } else if observed >= 32 && abstain == 0 {
        "PASS"
    } else {
        "WATCH"
    };
    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs();
    println!(
        "{}",
        json!({
            "schema":"nando.expression-post-candidate-future.v1",
            "verdict":verdict, "execution_authority":false, "generated_at_unix":generated_at_unix,
            "candidate_sha256":format!("{:x}", Sha256::digest(&candidate_bytes)),
            "package_sha256":package_sha256, "source_prefix_bytes":candidate.source_prefix_bytes,
            "source_prefix_sha256":prefix_sha256, "tail_complete_bytes":complete_tail.len(),
            "future_rows":observed, "correct":correct, "wrong":wrong, "abstain":abstain,
            "malformed_rows":malformed, "minimum_future_rows":32, "contract_bound":contract_bound,
        })
    );
    if verdict == "VETO" {
        process::exit(1);
    }
}

fn parse_frame(line: &[u8]) -> Option<(Value, Value, Value)> {
    let value: Value = serde_json::from_slice(line).ok()?;
    let before = value.get("before")?.clone();
    let action = value.get("action")?.clone();
    let after = value.get("after")?.clone();
    (before.is_object() && action.is_object() && after.is_object())
        .then_some((before, action, after))
}
