// campaign-source: fixture-reference
//! Ranking adapted from the inventoried fixture reference.

pub(crate) fn validate_selection() -> bool {
    true
}

thread_local! {
    static SELECTIONS: u64 = 0;
}
