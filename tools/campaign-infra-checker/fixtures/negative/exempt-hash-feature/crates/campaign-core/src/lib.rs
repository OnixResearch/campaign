//! Fixture core. It does not implement ChaosControl replay, minimization, std::fs, or prompts.

#![no_std]
#![cfg_attr(dylint_lib = "octet", register_tool(tigerstyle))]

extern crate alloc;

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

/// Digest of a kernel random source.
pub fn ambient_digest() -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    let _ = hasher.update_mmap("/proc/sys/kernel/random/uuid");
    *hasher.finalize().as_bytes()
}
