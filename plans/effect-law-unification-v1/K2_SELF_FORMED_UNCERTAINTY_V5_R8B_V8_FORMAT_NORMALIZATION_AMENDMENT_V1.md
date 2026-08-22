# K2 Self-Formed Uncertainty V5 R8B V8 Format Normalization Amendment V1

Status: `PAPER ONLY / FORMAT-ONLY / NO EXECUTION AUTHORITY`

Date: `2026-08-22`

## 1. Parent Bindings

```text
V8 contract SHA-256
  1e6f44a88cbc2c38173e7b6dc0a1f272e0bfea074cbbe11caf5d1ff26eda1844

V8 spectral amendment SHA-256
  50494e74d272651628bfee710a1a09a18206bfb5cef9469483bf40ba2f7ae12a

implementation HEAD
  0015d265506aa1da440fb33708124cf4789a62c8

formatter
  rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14), edition 2024
```

This amendment reconciles the mandatory global formatter check with the
existing raw-source line budgets after the completed V8 spectral split. It does
not redefine the source as skip-free normalized code and changes no process
route, schema, serialized byte formula, validator, authority owner, scientific
denominator, execution boundary or claim.

## 2. Measured Blocker

The current global command

```text
cargo fmt --all --check
```

fails on exactly eight files. The pinned formatter produces these measured
line counts from the same source:

```text
path                                      current  normalized
confirm_owner.rs                             706         756
confirm_public_coordinator.rs                567         641
immutable_publication.rs                     536         640
r8b_authorizer.rs                            443         527
r8b_process_model.rs                         819         857
r8b_publisher.rs                             237         233
r8b_mode_matrix_v1.rs                       343         346
r8b_restart_v1.rs                           445         436
```

The measured checkpoint cannot simultaneously retain those compact bytes and
pass the mandatory global formatter gate. This amendment resolves only that
concrete conflict; it does not promote physical line count into semantic proof.

## 3. Binding Formatter Rule

The implementation checkpoint is formatted once with the pinned formatter and
then measured. It must satisfy both:

```text
cargo fmt --all --check                         PASS
every frozen post-formatter line budget         PASS
```

The repair may not add a `rustfmt::skip`, ignore list, alternate formatter
configuration, generated-source exclusion or wrapper that hides a failing
workspace path. Eight `rustfmt::skip` directives already present in four of the
eight measured donor files remain frozen and are not evidence of skip-free
normalization. The eight measured files are otherwise formatted mechanically
as one format-only operation. No manual behavior edit may be mixed into it.

## 4. Corrected Post-Formatter Budgets

Only two existing budgets are below the post-formatter size of the same already
accepted source:

```text
r8b_authorizer.rs       <= 550 lines
r8b_process_model.rs    <= 900 lines
```

All other V8 and spectral-amendment budgets remain unchanged. The corrected
limits provide respectively 23 and 43 lines of headroom over the measured
formatter output. The budget remains an engineering source-size guard under
the frozen directives, not a skip-free complexity metric or scientific proof.

## 5. Format-Only Vetoes

The amendment is invalid if formatting changes any:

```text
public or private symbol set
module ownership or visibility
canonical serialized bytes or roots
process argv, environment, timing or failure classification
ledger event order or packet membership
P00-P09 transition order
test denominator or ignored-test status
authority, execution or claim boundary
```

The post-format checkpoint must retain exact before and after hashes for all
eight files, prove that no skip directive was added, and pass the existing
source-route, source-scope, line-budget, compile, test, Clippy and
canonical-byte parity gates.

## 6. Authority Boundary

This amendment authorizes only formatter normalization after adversarial and
structural acceptance. It does not authorize an R8B suite, M24, M25, M26, P09,
a transient unit, deployment, dashboard mutation, push, scientific attempt or
scientific claim. Separate explicit R8B execution authorization remains absent.
