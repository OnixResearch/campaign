#![no_std]

//! Fixture conformance. It does not model ChaosControl, VM execution, replay, or minimization.

pub fn ambiguous_control_lineage() -> &'static str {
    "negative.ambiguous_control_lineage"
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bin;
