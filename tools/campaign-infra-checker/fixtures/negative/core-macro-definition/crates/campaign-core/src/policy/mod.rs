// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

macro_rules! call {
    ($m:ident, $a:literal) => {
        $m!($a)
    };
}
/// Ambient build environment.
pub(crate) const HOST: Option<&str> = call!(option_env, "CAMPAIGN_AMBIENT");
