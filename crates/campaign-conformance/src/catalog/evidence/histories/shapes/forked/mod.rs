//! Catalog forks of the held heads with every presented depth and head kind.

/// Answers of a history Campaign loads cold but refuses through H0 and through H1 as not a
/// fast-forward of either held head.
const fn behind_both() -> super::super::Answers {
    super::super::Answers {
        cold: super::super::Answer::Loads,
        earlier: super::NOT_FORWARD,
        current: super::NOT_FORWARD,
    }
}

/// Forks of the held heads: forks over H0 that drop the current head (H1) and forks over the
/// history before H0's head that drop H0, each at the dropped head's own depth, one to
/// `MAX_FORK_DEPTH` events beyond it, and at the deepest depth the projection bound, the
/// control-event room, and the graph room admit, with every head kind the fork state admits.
/// Each is valid history on its own; forks over H0 are fast-forwards of H0, forks over the
/// earlier history of neither held head.
///
/// # Errors
///
/// Returns the error of a construction.
// r[impl campaign.conformance.bounds]
// r[impl campaign.conformance.negative]
pub(in crate::catalog::evidence::histories) fn forks(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::super::super::lineages::forks::Earlier,
    honest: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::super::Entry>> {
    let over_head = super::super::super::lineages::forks::extent::depths(session, &earlier.head)?;
    let over_before =
        super::super::super::lineages::forks::extent::depths(session, &earlier.before)?;
    let current = session.branch.clone();
    let current_event = head_event(honest)?;
    let held = earlier.head.history.branch.clone();
    let held_event = head_event(&earlier.head.history)?;
    let dropping_current = super::super::super::lineages::forks::dropping(
        session,
        &earlier.head,
        (&current, &current_event),
        (
            &over_head,
            &super::super::super::lineages::forks::heads::HEAD_KINDS,
        ),
    )?;
    let dropping_held = super::super::super::lineages::forks::dropping(
        session,
        &earlier.before,
        (&held, &held_event),
        (
            &over_before,
            &super::super::super::lineages::forks::heads::HEAD_KINDS,
        ),
    )?;
    let covers_every_head = |forks: &[super::super::super::lineages::forks::Fork]| {
        super::super::super::lineages::forks::heads::HEAD_KINDS
            .iter()
            .all(|head| forks.iter().any(|fork| fork.head == *head))
    };
    super::super::super::super::common::require(
        covers_every_head(&dropping_current) && covers_every_head(&dropping_held),
        "the catalog presents forks of every head kind dropping either held head",
    )?;
    let mut forks =
        alloc::vec::Vec::with_capacity(dropping_current.len().saturating_add(dropping_held.len()));
    for fork in dropping_current {
        debug_assert!(fork.history.payloads.len() >= honest.payloads.len());
        forks.push(entry_of(
            session,
            fork,
            (DROPS_CURRENT, super::behind_current()),
            &super::super::CURRENT,
        )?);
    }
    for fork in dropping_held {
        debug_assert!(fork.history.payloads.len() >= earlier.head.history.payloads.len());
        forks.push(entry_of(
            session,
            fork,
            (DROPS_EARLIER, behind_both()),
            &super::super::HELD,
        )?);
    }
    Ok(forks)
}

/// Forbidden effects of loading, and of using a cache of, a fork that drops the held head (H1).
const DROPS_CURRENT: super::Names = (
    "a branch moved onto a fork that drops the held head was loaded as a fast-forward",
    "a cache of a fork that drops the held head was used as current state",
);

/// Forbidden effects of loading, and of using a cache of, a fork that drops the earlier held
/// head (H0).
const DROPS_EARLIER: super::Names = (
    "a branch moved onto a fork that drops the earlier held head was loaded as a fast-forward",
    "a cache of a fork that drops the earlier held head was used as current state",
);

/// The catalog entry of one fork: its history under its own binding with the fork's own state,
/// presented by its head kind.
///
/// # Errors
///
/// Returns the adapter error of the binding.
fn entry_of(
    session: &crate::harness::Session<'_, '_>,
    fork: super::super::super::lineages::forks::Fork,
    (names, answers): (super::Names, super::super::Answers),
    refusing: &'static [super::super::Holder],
) -> crate::verdicts::ConformanceResult<super::super::Entry> {
    let entry = super::super::Entry {
        replayed: Some(fork.state),
        ..super::own(session, fork.history, names, answers)?
    };
    Ok(presented_by_head(entry, fork.head, refusing))
}

/// A fork entry presented under every holder when its head is a stop, otherwise only under the
/// `refusing` holders (the stop-headed fork of the same depth carries the cold and fast-forward
/// answers).
fn presented_by_head(
    entry: super::super::Entry,
    head: super::super::super::lineages::forks::heads::HeadKind,
    refusing: &'static [super::super::Holder],
) -> super::super::Entry {
    if matches!(
        head,
        super::super::super::lineages::forks::heads::HeadKind::Stop
    ) {
        return entry;
    }
    super::super::Entry {
        holders: refusing,
        ..entry
    }
}

/// The control event at the head of `history`.
///
/// # Errors
///
/// Returns `Diverged` for a history without control events.
pub(in crate::catalog::evidence) fn head_event(
    history: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
    history
        .payloads
        .last()
        .map(|payload| payload.control_event.clone())
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the history publishes a control head",
        })
}
