#![no_std]

//! Fixture conformance. It does not model ChaosControl, VM execution, replay, or minimization.

pub fn ambiguous_control_lineage() -> &'static str {
    "negative.ambiguous_control_lineage"
}

pub fn entropy(buffer: &mut [u8]) {
    let _ = getrandom::fill(buffer);
}
