#![no_std]

//! Fixture conformance. It does not model ChaosControl, VM execution, replay, or minimization.

pub fn ambiguous_control_lineage() -> &'static str {
    "negative.ambiguous_control_lineage"
}

pub fn minimize_finding() -> bool {
    true
}
