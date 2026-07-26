use std::collections::BTreeSet;
use std::io::Write;

use super::*;

fn terminal_line(request: &str, completed: u64) -> String {
    format!(
        "{{\"schema\":\"nando.nginx-terminal.v1\",\"request_id\":\"{request}\",\"status\":200,\"completed_at_unix_seconds\":\"{completed}.000\",\"request_time_seconds\":\"0.100\"}}\n"
    )
}

#[test]
fn archive_retains_old_receipt_beyond_previous_tail_limit_and_restart() {
    let root = std::env::temp_dir().join(format!(
        "nando-terminal-archive-retain-{}",
        std::process::id()
    ));
    let source_path = root.join("terminal.jsonl");
    std::fs::create_dir_all(&root).expect("root");
    let mut source = File::create(&source_path).expect("source");
    source
        .write_all(terminal_line("wanted", 1_000).as_bytes())
        .expect("wanted");
    for index in 0..16_500 {
        source
            .write_all(terminal_line(&format!("noise-{index}"), 2_000 + index).as_bytes())
            .expect("noise");
    }
    source.sync_all().expect("source sync");

    let archive_root = root.join("archive");
    let mut archive = TerminalReceiptArchive::open(&archive_root).expect("archive");
    archive.sync_source(&source_path).expect("sync");
    assert_eq!(archive.len(), 16_501);
    drop(archive);

    let restored = TerminalReceiptArchive::open(&archive_root).expect("restore");
    let wanted = BTreeSet::from([nando_operator_kernel::sha256_bytes(b"wanted")]);
    assert_eq!(restored.receipts_for_requests(&wanted).len(), 1);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_keeps_committed_receipts_across_source_rotation() {
    let root = std::env::temp_dir().join(format!(
        "nando-terminal-archive-rotate-{}",
        std::process::id()
    ));
    let source_path = root.join("terminal.jsonl");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(&source_path, terminal_line("before", 1_000)).expect("before");
    let mut archive = TerminalReceiptArchive::open(&root.join("archive")).expect("archive");
    archive.sync_source(&source_path).expect("first sync");

    std::fs::remove_file(&source_path).expect("remove old source");
    std::fs::write(&source_path, terminal_line("after", 2_000)).expect("after");
    archive.sync_source(&source_path).expect("rotation sync");
    let ids = BTreeSet::from([
        nando_operator_kernel::sha256_bytes(b"before"),
        nando_operator_kernel::sha256_bytes(b"after"),
    ]);
    assert_eq!(archive.receipts_for_requests(&ids).len(), 2);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_does_not_advance_past_partial_source_line() {
    let root = std::env::temp_dir().join(format!(
        "nando-terminal-archive-partial-{}",
        std::process::id()
    ));
    let source_path = root.join("terminal.jsonl");
    std::fs::create_dir_all(&root).expect("root");
    let full = terminal_line("partial", 1_000);
    let split = full.len() / 2;
    std::fs::write(&source_path, &full.as_bytes()[..split]).expect("partial");
    let mut archive = TerminalReceiptArchive::open(&root.join("archive")).expect("archive");
    archive.sync_source(&source_path).expect("partial sync");
    assert_eq!(archive.len(), 0);
    OpenOptions::new()
        .append(true)
        .open(&source_path)
        .expect("append")
        .write_all(&full.as_bytes()[split..])
        .expect("finish line");
    archive.sync_source(&source_path).expect("complete sync");
    assert_eq!(archive.len(), 1);
    std::fs::remove_dir_all(root).expect("cleanup");
}
