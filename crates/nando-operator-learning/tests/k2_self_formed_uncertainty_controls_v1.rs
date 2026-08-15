#[path = "k2_self_formed_uncertainty_controls_v1/boundary.rs"]
mod boundary;
#[path = "k2_self_formed_uncertainty_controls_v1/closure.rs"]
mod closure;
#[path = "k2_self_formed_uncertainty_controls_v1/fixture.rs"]
mod fixture;
#[path = "k2_self_formed_uncertainty_controls_v1/identity.rs"]
mod identity;
#[path = "k2_self_formed_uncertainty_controls_v1/induction.rs"]
mod induction;
#[path = "k2_self_formed_uncertainty_controls_v1/ledger.rs"]
mod ledger;
#[path = "k2_self_formed_uncertainty_controls_v1/probe.rs"]
mod probe;
#[path = "k2_self_formed_uncertainty_controls_v1/temporal.rs"]
mod temporal;
#[path = "k2_self_formed_uncertainty_controls_v1/v4.rs"]
mod v4;

use fixture::R7Fixture;
use ledger::ControlLedger;

#[test]
fn r7_exact_negative_controls_and_v3_shortcut_controls_pass() {
    let fixture = R7Fixture::new();
    let mut ledger = ControlLedger::new();
    identity::run(&fixture, &mut ledger);
    induction::run(&fixture, &mut ledger);
    probe::run(&fixture, &mut ledger);
    temporal::run(&fixture, &mut ledger);
    boundary::run(&fixture, &mut ledger);
    ledger.finish();
}

#[test]
fn r7a_closure_planner_freezes_complete_bounded_census() {
    closure::run();
}

#[test]
fn r7d_v4_exact_j1_j16_controls_pass() {
    let fixture = R7Fixture::new();
    let mut ledger = ledger::V4ControlLedger::new();
    v4::run(&fixture, &mut ledger);
    ledger.finish();
}
