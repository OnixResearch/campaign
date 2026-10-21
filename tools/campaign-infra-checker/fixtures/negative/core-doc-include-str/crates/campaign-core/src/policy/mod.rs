// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

#[doc = include_str!("/proc/sys/kernel/random/uuid")]
pub(crate) struct Seeded;
