// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

/// Kernel randomness at build time, read through a macro argument.
pub(crate) const SEED: &[u8] = call!(include_bytes, "/proc/sys/kernel/random/uuid");
