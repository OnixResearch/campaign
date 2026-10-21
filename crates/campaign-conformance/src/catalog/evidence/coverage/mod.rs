//! Bounded 4-way state coverage.
//!
//! The declared features of a frontier state (`features`) are its status and ten booleans. The
//! declared classes (`classes`) are every feature vector the Campaign rules admit; a bounded
//! reachability test over campaign-core shows that every state reached from an admission within
//! its bound lies in a declared class, and replaying every class's canonical recipe (`recipes`)
//! reaches it. The pinned table (`table`) selects classes so that every 4-way combination of
//! feature values a declared class carries, and every compatible (receipt kind, 3-way
//! combination) pair, is carried by a selected class. Each selected class is built once through
//! the adapter from its recipe; `held` presents every history shape through that state to the
//! load and cache paths, including forks of the history before its head at every fork depth value
//! and head sub-kind some 3-way combination of the state still lacks (`forked`), `claims`
//! presents every receipt kind at it to the claims oracle, and the covering rule is checked on
//! what was presented (`covering`).

mod claims;
mod classes;
mod combos;
mod covering;
mod features;
mod forked;
mod held;
mod kinds;
#[cfg(test)]
mod reach;
mod recipes;
mod table;

/// Builds every selected class through the adapter from its recipe and, at the state reached,
/// presents every history shape to the load and cache paths (`held`) and every receipt kind to
/// the claims oracle (`claims`); then requires the covering rule over the presented classes,
/// receipt kinds, and fork values.
///
/// # Errors
///
/// Returns the error of a replay or construction, the oracle mismatch of a presentation,
/// `ForbiddenAccepted` naming the first claimed receipt, and `Diverged` when a replay misses its
/// class or the presentations miss a declared 4-way combination, a compatible (receipt kind,
/// 3-way combination) pair, or an admitted (fork value, 3-way combination) pair.
// r[impl campaign.conformance.state_coverage]
// r[impl campaign.conformance.negative]
// r[impl campaign.conformance.claims]
pub(super) fn four_way_states(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let classes = table::selected()?;
    let mut presented = alloc::vec::Vec::with_capacity(classes.len());
    let mut forks = forked::Pairs::empty();
    for class in classes {
        let (mut session, replayed) = recipes::replay::build(harness, class)?;
        held::present(&mut session, (&replayed, class), &mut forks)?;
        let kinds = claims::present(&mut session, &replayed)?;
        presented.push((class, kinds));
    }
    covering::require(&presented, &forks)
}
