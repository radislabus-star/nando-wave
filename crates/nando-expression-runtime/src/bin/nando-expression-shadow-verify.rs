use std::{env, fs, process};

use nando_expression_runtime::ExpressionRuntime;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct MinerReceipt {
    verdict: String,
    source_prefix_bytes: usize,
    source_prefix_sha256: String,
    package_sha256: String,
    wrong: usize,
    missing_receipts: usize,
    wave_causal_pass: bool,
}

struct Frame {
    before: Value,
    action: Value,
    after: Value,
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!("usage: nando-expression-shadow-verify PACKAGE RECEIPT SOURCE_JSONL");
        process::exit(2);
    }
    let package = fs::read(&args[1]).expect("read package");
    let receipt: MinerReceipt =
        serde_json::from_slice(&fs::read(&args[2]).expect("read receipt")).expect("parse receipt");
    let source = fs::read(&args[3]).expect("read source");
    let prefix = source
        .get(..receipt.source_prefix_bytes)
        .expect("source shorter than receipt prefix");
    let package_sha256 = format!("{:x}", Sha256::digest(&package));
    let prefix_sha256 = format!("{:x}", Sha256::digest(prefix));
    let runtime = ExpressionRuntime::load(&package).expect("load expression package");
    let rows = prefix
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(parse_frame)
        .collect::<Vec<_>>();
    let cut = rows.len() * 7 / 10;
    let future = &rows[cut..];
    let mut correct = 0usize;
    let mut wrong = 0usize;
    let mut abstain = 0usize;
    for frame in future {
        let result = runtime.execute(&frame.before, &frame.action);
        match result.after {
            Some(after) if after == frame.after => correct += 1,
            Some(_) => wrong += 1,
            None => abstain += 1,
        }
    }
    let mut route_splice_accepts = 0usize;
    let mut route_splice_cases = 0usize;
    for frame in future {
        let original_shape = action_shape(&frame.action);
        if let Some(spliced) = future
            .iter()
            .find(|candidate| action_shape(&candidate.action) != original_shape)
            .map(|candidate| &candidate.action)
        {
            route_splice_cases += 1;
            if runtime.execute(&frame.before, spliced).after.is_some() {
                route_splice_accepts += 1;
            }
        }
    }
    let mutations = package_mutations(&package);
    let mutation_rejections = mutations
        .iter()
        .filter(|bytes| ExpressionRuntime::load(bytes).is_err())
        .count();
    let pass = receipt.verdict == "PASS"
        && receipt.wrong == 0
        && receipt.missing_receipts == 0
        && receipt.wave_causal_pass
        && package_sha256 == receipt.package_sha256
        && prefix_sha256 == receipt.source_prefix_sha256
        && !future.is_empty()
        && correct == future.len()
        && wrong == 0
        && abstain == 0
        && route_splice_cases > 0
        && route_splice_accepts == 0
        && mutation_rejections == mutations.len();
    println!(
        "{}",
        json!({
            "schema":"nando.expression-shadow-verifier-receipt.v1",
            "verdict":if pass {"PASS"} else {"VETO"},
            "execution_authority":false,
            "package_sha256":package_sha256,
            "source_prefix_sha256":prefix_sha256,
            "programs":runtime.program_count(), "nodes":runtime.node_count(), "support_total":runtime.support_total(),
            "rows":rows.len(), "train_rows":cut, "future_rows":future.len(),
            "correct":correct, "wrong":wrong, "abstain":abstain,
        "route_splice_cases":route_splice_cases, "route_splice_accepts":route_splice_accepts,
            "package_mutations":mutations.len(), "package_mutation_rejections":mutation_rejections,
            "miner_receipt_bound":package_sha256 == receipt.package_sha256 && prefix_sha256 == receipt.source_prefix_sha256,
        })
    );
    if !pass {
        process::exit(1);
    }
}

fn action_shape(value: &Value) -> String {
    fn shape(value: &Value, output: &mut String) {
        match value {
            Value::Object(object) => {
                output.push('{');
                for (key, child) in object {
                    output.push_str(key);
                    output.push(':');
                    shape(child, output);
                    output.push(',');
                }
                output.push('}');
            }
            Value::Array(_) => output.push_str("array"),
            Value::Null => output.push_str("null"),
            Value::Bool(_) => output.push_str("bool"),
            Value::Number(_) => output.push_str("number"),
            Value::String(_) => output.push_str("string"),
        }
    }
    let mut output = String::new();
    shape(value, &mut output);
    output
}

fn parse_frame(line: &[u8]) -> Option<Frame> {
    let value: Value = serde_json::from_slice(line).ok()?;
    Some(Frame {
        before: value
            .get("before")?
            .as_object()
            .map(|_| value.get("before").cloned())??,
        action: value
            .get("action")?
            .as_object()
            .map(|_| value.get("action").cloned())??,
        after: value
            .get("after")?
            .as_object()
            .map(|_| value.get("after").cloned())??,
    })
}

fn package_mutations(package: &[u8]) -> Vec<Vec<u8>> {
    let mut header = package.to_vec();
    if let Some(byte) = header.first_mut() {
        *byte ^= 0xff;
    }
    let truncated = package
        .get(..package.len().saturating_sub(1))
        .unwrap_or_default()
        .to_vec();
    let mut trailing = package.to_vec();
    trailing.push(0);
    let mut constructor = package.to_vec();
    if constructor.len() > 16 {
        constructor[16] = 0xff;
    }
    vec![header, truncated, trailing, constructor]
}
