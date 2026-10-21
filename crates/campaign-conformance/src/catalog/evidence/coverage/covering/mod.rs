//! The covering rule, checked on what a fixture presented: every 4-way combination of feature
//! values a declared class carries is carried by a presented state, (for the claims) every
//! (receipt kind, 3-way combination) pair a declared class is compatible with was presented, and
//! (for loads and caches) every (fork depth value or head sub-kind, 3-way combination) pair some
//! selected state carrying the combination admits was presented to both the load path and cache
//! validation through such a state.

/// Requires the covering rule over `presented` (each state's class with the receipt kinds
/// presented at it) and `forks` (the fork values each state admits and presented).
///
/// # Errors
///
/// Returns `Diverged` when a combination or pair is missing.
// r[impl campaign.conformance.state_coverage]
// r[impl campaign.conformance.bounds]
// r[impl campaign.history_projection.stale]
// r[impl campaign.cache.invalid]
pub(super) fn require(
    presented: &[(super::features::StateClass, u32)],
    forks: &super::forked::Pairs,
) -> crate::verdicts::ConformanceResult<()> {
    let required: alloc::vec::Vec<(super::features::StateClass, u32)> = super::classes::all()
        .into_iter()
        .map(|class| (class, super::kinds::compatible(class)))
        .collect();
    let missing =
        super::combos::Carried::of(presented).missing(&super::combos::Carried::of(&required));
    let forked = forks.missing();
    super::super::super::common::require(
        missing == (0, 0) && forked == (0, 0),
        "the presented states carry every declared 4-way combination, every compatible \
         (receipt kind, 3-way combination) pair, and every admitted (fork value, 3-way \
         combination) pair",
    )
}
