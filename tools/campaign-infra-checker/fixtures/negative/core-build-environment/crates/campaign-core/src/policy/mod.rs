// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

/// Ambient build environment.
pub(crate) const AMBIENT: Option<&str> = option_env!("CAMPAIGN_AMBIENT");
