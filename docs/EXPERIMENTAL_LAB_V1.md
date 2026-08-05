# Experimental Lab v1

`nando-experimental-lab` is the first bounded implementation of the
distinguishing-probe plan.

```text
bounded hypotheses
    -> predicted outcome partitions
    -> selected probe
    -> disposable filesystem/Git executor
    -> observed state + content-addressed receipt
    -> surviving hypotheses
    -> UniqueLawCandidate (no authority)
    -> independent natural holdout
    -> LawCertificate (still not an ACTIVE package)
```

The crate intentionally starts with two environments whose outcomes are fully
observable:

- filesystem copy and delete;
- Git rename, executed by the real `git` binary in a disposable directory.

The candidate hypotheses are predictors only. The candidate program is never
used as an oracle. Network access, production mounts, unbounded files, and
arbitrary commands are rejected. Cleanup is recorded in every receipt.

The first exam is executable in the crate tests:

- **E1:** the selector prefers the probe with the stronger outcome partition;
- **E2:** three laws are identified across filesystem and Git;
- **E3:** a lab candidate requires an independent natural holdout before a
  `LawCertificate` can be formed.

Run the compact exam report with:

```bash
cargo run -p nando-experimental-lab --bin nando-lab-exam
```

The expected verdict is `LAB_EXAM_PASS_NO_AUTHORITY`. The report includes the
selected probe, three opened candidate laws, two environments, and a holdout
certificate whose `authority_granted` and `active_package_allowed` fields are
both `false`.

This is laboratory evidence, not runtime authority and not a claim of general
intelligence. Production admission remains owned by the existing external
admission/verifier path.
