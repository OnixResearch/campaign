//! History-load oracle matrix.
//!
//! The adapter's load path faces the honest history through a held projection one event older
//! than its head (a fast-forward it must load) and every history of the shared catalog cold,
//! through that older held projection (H0), and through the held projection of the current head
//! (H1). The campaign then continues to its stop, and the held-lineage matrix presents rewinds,
//! forks, and fast-forwards through every held prefix. Every answer must equal Campaign's own
//! validation.

/// Loads the honest history through the older held projection, then every catalog history.
pub(super) fn load_matrix(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let (mut session, earlier) = super::histories::prepared(harness)?;
    let honest = session.history();
    let loaded = session.present_history(
        &honest,
        crate::harness::oracle::Presentation::ThroughHeld,
        "an honest history was loaded as another projection",
    )??;
    super::super::common::require(
        *loaded.state() == session.state,
        "an honest history loads through a held projection older than its head",
    )?;
    debug_assert_ne!(*loaded.state(), earlier.head.state);
    let catalog = super::histories::assembled(&mut session, &earlier)?;
    super::histories::presented::loads(&mut session, &catalog, &earlier)?;
    super::histories::held::loads(&mut session, &earlier)
}
