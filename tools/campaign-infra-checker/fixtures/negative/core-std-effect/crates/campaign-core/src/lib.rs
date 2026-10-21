//! Fixture core. It does not implement ChaosControl replay, minimization, std::fs, or prompts.

#![no_std]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

extern crate alloc;
extern crate std;

mod frontier;
mod policy;

/// Structural non-claims that mention replay and release without granting authority.
#[derive(Debug)]
pub enum NonClaim {
    ReplaySuccess,
    ReleaseReadiness,
    FindingCorrectness,
}

pub fn admitted(buffer: &'static mut [u8]) -> bool {
    debug_assert!(buffer.len() < 8, "assertion text is a literal");
    frontier::replay::reconstruct()
}

#[cfg(test)]
mod tests;
