// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

/// Topology kinds.
pub(crate) enum Topology {
    #[cfg(test)]
    Probe,
    UniversalWeave,
}
