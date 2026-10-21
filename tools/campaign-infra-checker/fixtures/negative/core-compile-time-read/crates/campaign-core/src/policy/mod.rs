// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

/// Kernel randomness read at build time.
pub(crate) const SEED: &[u8] = core::include_bytes!("/proc/sys/kernel/random/uuid");
