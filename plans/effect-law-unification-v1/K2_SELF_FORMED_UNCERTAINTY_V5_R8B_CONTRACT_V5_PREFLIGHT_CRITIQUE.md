# K2 Self-Formed Uncertainty V5 R8B Contract V5 Preflight Critique

Status: `P1 DEFECTS REPAIRED BEFORE IMPLEMENTATION PREFLIGHT / GATES MUST RERUN`

Date: `2026-08-20`

## Verdict

V5 was not yet precise enough to freeze byte identity. Structural route PASS did
not detect two cryptographic binding omissions. No implementation authority was
active, so the candidate paper is repaired in place and every affected
structural receipt must be regenerated.

## Findings

| Priority | Finding | Consequence | Repair |
|---|---|---|---|
| P1 | Every new struct is required to contain denied authority, but the stored-artifact root formula omitted that field. | The artifact's own semantic identifier did not bind every authority-relevant field. Validation could reject a modified field, but root equality alone could not prove its value. | Append denied authority to the exact stored-artifact root tuple. |
| P1 | The private reconstruction root had no schema or domain literal. Current `uncertainty_root_v1` delegates directly to generic composition hashing and adds no implicit type domain. | Another tuple with the same serialized shape and values could share the identifier without representing the same protocol object. | Prefix the exact reconstruction tuple with `nando.k2-self-formed-development-rehearsal-private-reconstruction.v1`. |

## Claim Boundary

These repairs define identifiers only. They do not prove source feasibility,
correct implementation, crash recovery, scientific success or deployment
safety. V5 remains blocked before Rust until fresh structural and implementation
preflight receipts pass against the repaired bytes.
