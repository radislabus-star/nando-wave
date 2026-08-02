use crate::live_economics::PackageCpuCompletionReceiptV1;

pub(super) fn first_post_terminal_completion(
    receipts: &[PackageCpuCompletionReceiptV1],
    terminal_at_unix: u64,
) -> Option<&PackageCpuCompletionReceiptV1> {
    receipts
        .iter()
        .find(|receipt| receipt.accepted_at_unix >= terminal_at_unix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(sequence: u64, accepted_at_unix: u64) -> PackageCpuCompletionReceiptV1 {
        PackageCpuCompletionReceiptV1 {
            schema: "nando.package-cpu-completion-receipt.v1".to_owned(),
            completion_root_sha256: format!("{:064x}", 100 + sequence),
            package_id: "package-one".to_owned(),
            intent_sha256: format!("{:064x}", 200 + sequence),
            exact_input_tokens: 1_000,
            accepted_at_unix,
            verification_receipt_root_sha256: format!("{:064x}", 300 + sequence),
        }
    }

    #[test]
    fn historical_accept_cannot_satisfy_post_terminal_cpu_requirement() {
        let receipts = vec![receipt(1, 999), receipt(2, 1_001)];
        assert_eq!(
            first_post_terminal_completion(&receipts, 1_000)
                .expect("post-terminal receipt")
                .accepted_at_unix,
            1_001
        );
        assert!(first_post_terminal_completion(&receipts[..1], 1_000).is_none());
    }

    #[test]
    fn terminal_timestamp_itself_is_post_terminal_evidence() {
        let receipts = vec![receipt(1, 1_000)];
        assert!(first_post_terminal_completion(&receipts, 1_000).is_some());
    }
}
