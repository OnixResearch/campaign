//! The held-lineage matrix the load and cache oracles present after the shared catalog.
//!
//! The session's campaign continues past H1: its outstanding selection settles by a result with a
//! finding, a second selection executes and closes unobserved, two more execute, the consumer
//! stops while both are outstanding, and a result settles one of them after the stop. The honest
//! lineage therefore passes through held projections of every kind (idle, outstanding, settled
//! by result, settled by closure, stopped). The matrix holds every prefix of that lineage (every
//! held length) and presents through it the honest head (a fast-forward it loads), its own lineage
//! on the older graph captured before the later drafts it recorded, every rewind
//! to a shorter prefix, forks of the history before its head at depths zero to three and at the
//! deepest depth the history admits with every head kind the fork state admits (a writer forks
//! that history as it stood before the head was published, when the head was not yet its pending
//! control child), and every forged and suffix catalog class built over the prefix; through a
//! stopped prefix it also presents the post-stop settlements a writer publishes over it (a
//! fast-forward it loads). The root, which has no earlier history, meets another campaign's
//! root. Campaign refuses every rewind, fork, class, and foreign root through the holder. The
//! matrix requires the covering rule of `covering`.

mod covering;
mod lineage;
mod shapes;

/// Kind of a held projection: what its head leaves the campaign in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HolderKind {
    /// Nothing outstanding and no settlement at the head.
    Idle,
    /// A published selection outstanding.
    Outstanding,
    /// The head settles a selection by its result.
    SettledByResult,
    /// The head closes a selection unobserved.
    SettledByClosure,
    /// The head is an accepted stop.
    Stopped,
}

/// Every holder kind.
const HOLDER_KINDS: [HolderKind; 5] = [
    HolderKind::Idle,
    HolderKind::Outstanding,
    HolderKind::SettledByResult,
    HolderKind::SettledByClosure,
    HolderKind::Stopped,
];

/// Kind of one shape presented through a holder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShapeKind {
    /// The honest head, a fast-forward of every prefix.
    FastForward,
    /// A rewind this many events behind the held head.
    Rewind(usize),
    /// A fork of the history before the held head.
    Fork {
        depth: usize,
        is_deepest: bool,
        head: super::super::lineages::forks::heads::HeadKind,
    },
    /// A forged or suffix catalog class built over the held head.
    Class(super::classes::Class),
    /// A settlement published over a stopped held head after its stop (a fast-forward).
    Advance,
    /// The held lineage on a graph older than the held projection's.
    StaleGraph,
    /// Another campaign's root, presented through the held root.
    ForeignRoot,
}

/// One held prefix of the honest lineage: its history under the final graph, Campaign's cold
/// projection of it, its kind, and the history before its head (none for the root).
struct Holder {
    history: crate::harness::oracle::HistoryView,
    view: campaign_choregraph::ValidatedControlProjection,
    kind: HolderKind,
    base: Option<super::super::lineages::forks::Base>,
}

/// One presentation through a holder: the holder's position, the shape, the catalog entry, and
/// Campaign's declared answer through the holder.
struct Presented {
    holder: usize,
    shape: ShapeKind,
    entry: super::Entry,
    answer: super::Answer,
}

/// Presents the held-lineage matrix to the adapter's load path; each answer must equal
/// Campaign's declared answer.
///
/// # Errors
///
/// Returns the error of a construction, the oracle mismatch of a presentation, and `Diverged`
/// when the matrix misses a covered dimension or Campaign's answer is not the declared one.
pub(in crate::catalog::evidence) fn loads(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<()> {
    let (holders, presented) = matrix(session, earlier)?;
    for presentation in &presented {
        let holder = holder_at(&holders, presentation.holder)?;
        let host = crate::harness::oracle::hosted::Host {
            maximum_events: presentation.entry.maximum_events,
            previous: Some(&holder.view),
        };
        super::presented::present_load(session, &presentation.entry, host, &presentation.answer)?;
    }
    Ok(())
}

/// Presents caches of the replayed, final, and held states of every matrix presentation to the
/// adapter's cache validation through its holder; each answer must equal Campaign's.
///
/// # Errors
///
/// Returns the error of a construction, the oracle mismatch of a presentation, and `Diverged`
/// when the matrix misses a covered dimension or Campaign's answer contradicts its load answer.
pub(in crate::catalog::evidence) fn caches(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<()> {
    let (holders, presented) = matrix(session, earlier)?;
    let anchors: alloc::vec::Vec<&campaign_choregraph::ValidatedControlProjection> = holders
        .iter()
        .flat_map(|holder| {
            core::iter::once(&holder.view).chain(holder.base.iter().map(|base| &base.view))
        })
        .collect();
    debug_assert!(anchors.len() >= holders.len());
    debug_assert!(presented.len() >= holders.len());
    for presentation in &presented {
        let holder = holder_at(&holders, presentation.holder)?;
        let states =
            super::cached::states(session, &presentation.entry, holder.view.state(), &anchors);
        let host = crate::harness::oracle::hosted::Host {
            maximum_events: presentation.entry.maximum_events,
            previous: Some(&holder.view),
        };
        super::presented::present_caches(
            session,
            &presentation.entry,
            host,
            &presentation.answer,
            &states,
        )?;
    }
    Ok(())
}

/// The holder at `position`.
///
/// # Errors
///
/// Returns `Diverged` for a position the matrix does not hold.
fn holder_at(holders: &[Holder], position: usize) -> crate::verdicts::ConformanceResult<&Holder> {
    holders
        .get(position)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "every matrix presentation names a held prefix",
        })
}

/// Continues the session's campaign past its stop, holds every prefix, and builds the
/// presentations; requires the covering rule of `covering`.
///
/// # Errors
///
/// Returns the error of a construction and `Diverged` for a matrix that misses a covered
/// dimension.
// r[impl campaign.conformance.negative]
// r[impl campaign.history_projection.stale]
// r[impl campaign.cache.invalid]
fn matrix(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<(alloc::vec::Vec<Holder>, alloc::vec::Vec<Presented>)> {
    let bases = lineage::bases(session, earlier)?;
    let holders = lineage::holders(session, bases)?;
    let presented = shapes::presented(session, &holders)?;
    covering::require(&holders, &presented)?;
    debug_assert_eq!(holders.len(), session.payloads.len());
    debug_assert!(presented.len() >= holders.len());
    Ok((holders, presented))
}
