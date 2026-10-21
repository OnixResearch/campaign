// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

splice! {
    #[path = "clock.inc"]
    mod clock;
}
