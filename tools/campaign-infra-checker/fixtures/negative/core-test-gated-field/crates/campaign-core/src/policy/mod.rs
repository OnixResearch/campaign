// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

/// Probe.
pub(crate) struct Probe {
    #[cfg(test)]
    pub(crate) merge_probe: u8,
    pub(crate) universal_weave: u8,
}
