//! The forged and suffix catalog classes built over one tip history: branch values (a renamed
//! branch, a branch reset onto the tip's earlier head, payloads lagging the branch), rewrites (the
//! last payload rewritten into another replayable event, the first payload rewritten into the
//! last, the graph altered after binding, an ambiguous lineage), suffixes (a two-parent merge of
//! the tip's next event without and with each parent chosen and with a choice naming neither, a
//! selection with a forged recorded fence, an event admitted against the state before the tip's
//! head, new work after a stop, a replayed event, and envelopes forged in metadata, class,
//! schema, or projection identity), and forged requests (the tip under a projection bound one
//! below its lineage length, the campaign's root under foreign envelope identities, and the tip
//! on a graph that also holds another campaign's root under the same envelope). The
//! shared catalog builds them over the current head (H1); the held-lineage matrix over every held
//! prefix, so every class meets every holder kind and length.

mod declared;
mod suffixes;

/// One forged or suffix catalog class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::catalog::evidence) enum Class {
    Renamed,
    Reset,
    Lagging,
    RewrittenTail,
    RewrittenFirst,
    Altered,
    Ambiguous,
    TwoParentUnchosen,
    TwoParentControl,
    TwoParentForeign,
    TwoParentNeither,
    ForgedFence,
    Stale,
    WorkAfterStop,
    Replayed,
    ForeignMetadata,
    NonApplication,
    ForeignSchema,
    ForeignProjection,
    ProjectionLimit,
    ForeignEnvelopeRoot,
    SharedGraph,
}

/// Every class, in catalog order.
pub(in crate::catalog::evidence) const CLASSES: [Class; 22] = [
    Class::Renamed,
    Class::Reset,
    Class::Lagging,
    Class::RewrittenTail,
    Class::RewrittenFirst,
    Class::Altered,
    Class::Ambiguous,
    Class::TwoParentUnchosen,
    Class::TwoParentControl,
    Class::TwoParentForeign,
    Class::TwoParentNeither,
    Class::ForgedFence,
    Class::Stale,
    Class::WorkAfterStop,
    Class::Replayed,
    Class::ForeignMetadata,
    Class::NonApplication,
    Class::ForeignSchema,
    Class::ForeignProjection,
    Class::ProjectionLimit,
    Class::ForeignEnvelopeRoot,
    Class::SharedGraph,
];

impl Class {
    /// Whether the class needs a selection the tip's planner plans: a stopped or waiting tip
    /// admits no selection, so no selection with a forged fence can be built over it (post-stop
    /// work is its own class).
    pub(in crate::catalog::evidence) const fn needs_selection(self) -> bool {
        matches!(self, Self::ForgedFence)
    }
}

/// One tip the classes are built over: Campaign's projection of it as captured before any later
/// publication (a writer plans over it; a projection of a graph that already holds later control
/// drafts must republish them first), its history, the history before its head (a fork base),
/// and the latest state of the campaign before any stop (the tip's own state while the campaign
/// runs).
pub(in crate::catalog::evidence) struct Tip<'t> {
    pub(in crate::catalog::evidence) view: &'t campaign_choregraph::ValidatedControlProjection,
    pub(in crate::catalog::evidence) history: &'t crate::harness::oracle::HistoryView,
    pub(in crate::catalog::evidence) base: &'t super::super::lineages::forks::Base,
    pub(in crate::catalog::evidence) unstopped: &'t campaign_core::FrontierState,
}

/// Catalog entries of one class: its history (under its own binding unless given) and Campaign's
/// answer, alike under every holder.
type Built = alloc::vec::Vec<(Class, super::Entry)>;

/// Adds the entry of `class` for `history` under its own binding.
fn push(
    session: &crate::harness::Session<'_, '_>,
    built: &mut Built,
    class: Class,
    history: crate::harness::oracle::HistoryView,
    answer: Option<super::Answer>,
) -> crate::verdicts::ConformanceResult<()> {
    let answer =
        answer
            .or(declared::answer(class))
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "every catalog class declares Campaign's answer",
            })?;
    let entry = super::shapes::own(
        session,
        history,
        declared::effects(class)?,
        super::Answers::alike(&answer),
    )?;
    built.push((class, entry));
    Ok(())
}

/// Every class constructible over `tip`, in class order; the forged-fence class only when the
/// tip's planner selects.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction.
// r[impl campaign.conformance.bounds]
// r[impl campaign.conformance.negative]
pub(in crate::catalog::evidence) fn entries(
    session: &mut crate::harness::Session<'_, '_>,
    tip: &Tip<'_>,
) -> crate::verdicts::ConformanceResult<Built> {
    let mut built = Built::with_capacity(CLASSES.len());
    branch_values(session, tip, &mut built)?;
    rewrites(session, tip, &mut built)?;
    suffixes::appended(session, tip, &mut built)?;
    requests(session, tip, &mut built)?;
    debug_assert!(built.len() <= CLASSES.len());
    Ok(built)
}

/// The renamed, reset, and lagging branch values of the tip.
fn branch_values(
    session: &crate::harness::Session<'_, '_>,
    tip: &Tip<'_>,
    built: &mut Built,
) -> crate::verdicts::ConformanceResult<()> {
    let kept = tip.history.payloads.len().saturating_sub(1);
    let lagging = crate::harness::oracle::HistoryView {
        payloads: tip.history.payloads.get(..kept).map(<[_]>::to_vec).ok_or(
            crate::verdicts::ConformanceError::Diverged {
                expected: "the tip published a history prefix",
            },
        )?,
        ..tip.history.clone()
    };
    let reset = super::super::lineages::reset(&tip.base.history, tip.history)?;
    debug_assert_eq!(reset.branch.head, tip.base.history.branch.head);
    push(
        session,
        built,
        Class::Renamed,
        super::super::lineages::renamed(tip.history),
        None,
    )?;
    push(session, built, Class::Reset, reset, None)?;
    push(session, built, Class::Lagging, lagging, None)
}

/// The rewritten tail and first payload, the altered graph, and the ambiguous lineage of the tip.
fn rewrites(
    session: &mut crate::harness::Session<'_, '_>,
    tip: &Tip<'_>,
    built: &mut Built,
) -> crate::verdicts::ConformanceResult<()> {
    let replacement = suffixes::replacement(session, tip)?;
    let tail = super::super::forged::rewritten_tail(tip.history, replacement)?;
    let first = super::super::forged::rewritten(tip.history)?;
    debug_assert_eq!(tail.payloads.len(), tip.history.payloads.len());
    debug_assert_ne!(first.payloads, tip.history.payloads);
    push(session, built, Class::RewrittenTail, tail, None)?;
    push(session, built, Class::RewrittenFirst, first, None)?;
    let binding = campaign_choregraph::derive_projection_binding(
        &tip.history.graph,
        &tip.history.branch,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    let altered = super::super::forged::altered(tip.history)?;
    let moved =
        declared::answer(Class::Altered).ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the altered class declares its answer",
        })?;
    built.push((
        Class::Altered,
        super::shapes::bound(
            altered,
            binding,
            declared::effects(Class::Altered)?,
            super::Answers::alike(&moved),
        ),
    ));
    let ambiguous = super::super::forged::ambiguous(tip.history)?;
    push(session, built, Class::Ambiguous, ambiguous, None)
}

/// The tip under a projection bound one event below its lineage length, the campaign's root under
/// foreign envelope identities, and the tip on a graph that also holds another campaign's root
/// under the same envelope.
///
/// # Errors
///
/// Returns the error of a construction and `ArithmeticOverflow` for a lineage whose length is
/// not representable.
fn requests(
    session: &crate::harness::Session<'_, '_>,
    tip: &Tip<'_>,
    built: &mut Built,
) -> crate::verdicts::ConformanceResult<()> {
    let lineage_count = u32::try_from(tip.history.payloads.len())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let bound_count = lineage_count
        .checked_sub(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let exceeded = super::Answer::Refused(campaign_choregraph::AdapterError::History(
        campaign_choregraph::HistoryError::ProjectionLimitExceeded {
            observed: lineage_count,
            maximum: bound_count,
        },
    ));
    push(
        session,
        built,
        Class::ProjectionLimit,
        tip.history.clone(),
        Some(exceeded),
    )?;
    if let Some((_, limited)) = built.last_mut() {
        limited.maximum_events = bound_count;
    }
    let (foreign, foreign_binding) =
        super::super::lineages::envelopes::foreign_root((tip.history, tip.view.state()))?;
    let crossed = declared::answer(Class::ForeignEnvelopeRoot).ok_or(
        crate::verdicts::ConformanceError::Diverged {
            expected: "the foreign-root class declares its answer",
        },
    )?;
    built.push((
        Class::ForeignEnvelopeRoot,
        super::shapes::bound(
            foreign,
            foreign_binding,
            declared::effects(Class::ForeignEnvelopeRoot)?,
            super::Answers::alike(&crossed),
        ),
    ));
    let (shared, second) = super::super::lineages::envelopes::second_root(session, tip.history)?;
    let sharing = super::Answer::Refused(campaign_choregraph::AdapterError::SharedControlGraph {
        event_id: second.history_event_id,
    });
    let shared = crate::harness::oracle::HistoryView {
        graph: shared,
        ..tip.history.clone()
    };
    push(session, built, Class::SharedGraph, shared, Some(sharing))?;
    debug_assert!(bound_count < lineage_count);
    Ok(())
}
