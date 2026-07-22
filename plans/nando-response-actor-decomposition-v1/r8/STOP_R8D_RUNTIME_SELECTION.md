# STOP-R8D Runtime Selection Owner

The structural observation and value-selection route moved from the runtime
executor into `runtime/selection.rs`.

```text
runtime.rs                         VM/action/projection orchestration
runtime/selection.rs               structural roles and value selection
```

The selection module does not execute VM opcodes, render final responses, run
the verifier, or grant authority. Public crate-root APIs remain unchanged; six
private bridges are visible only to the parent runtime module.

```text
runtime.rs before                    3,708 lines
runtime.rs after                     2,144 lines
selection.rs                         1,588 lines
runtime tests                          6/6 PASS
runtime Clippy                              PASS
response historical fingerprint             PASS
new failures                                    0
deployment                                      no
restart                                         no
authority                                    false
```

Both production files are below the 2,500-line hard budget.
