//! The shared history catalog the load and cache oracles present.
//!
//! One campaign publishes a selection over a guided frontier. The catalog then holds the honest
//! history and every forged class built around it: the earlier branch value, a reset branch,
//! forks over H0 that drop the current head and forks over the history before H0's head that
//! drop H0, each at the dropped head's own depth, one to `MAX_FORK_DEPTH` events deeper, and at
//! the deepest depth the bounds admit, with every head kind (stop, guidance, candidate batch,
//! selection, result, closure), a renamed branch, lagging payloads, a rewritten payload, an
//! altered graph, an ambiguous lineage, a parent choice naming a non-parent, a two-parent merge
//! without and with each parent chosen, a forged recorded fence, a stale-state suffix, new work
//! after a stop, a replayed control event, another campaign's control event, suffix envelopes
//! with another adapter's metadata, a non-application class, a foreign schema, and a foreign
//! projection identity, and the honest history under a projection bound below its lineage
//! length. Each entry declares Campaign's answer cold, through the held projection one event
//! older than the head (H0), and through the held projection of the current head (H1); the
//! oracles present every entry all three ways (a fork whose head is not a stop only through the
//! held projections that refuse it) and require the adapter's answer to equal Campaign's. The
//! held-lineage matrix (`held`) then presents rewinds, forks, and fast-forwards through every
//! prefix of the campaign continued to its stop.

pub(super) mod cached;
pub(super) mod classes;
pub(super) mod held;
pub(super) mod presented;
pub(super) mod shapes;

/// Campaign's declared answer for one presented history.
#[derive(Clone, Debug)]
pub(super) enum Answer {
    /// The history loads.
    Loads,
    /// The history is refused with exactly this adapter error.
    Refused(campaign_choregraph::AdapterError),
    /// The history is refused as an ambiguous lineage.
    Ambiguous,
}

impl Answer {
    /// Whether a load answer is this declared answer.
    pub(super) fn is_answer<T>(&self, answer: &campaign_choregraph::AdapterResult<T>) -> bool {
        match (self, answer) {
            (Self::Loads, Ok(_)) => true,
            (Self::Refused(expected), Err(error)) => error == expected,
            (Self::Ambiguous, Err(error)) => matches!(
                error,
                campaign_choregraph::AdapterError::History(
                    campaign_choregraph::HistoryError::AmbiguousLineage { .. }
                )
            ),
            _ => false,
        }
    }
}

/// The projection a host holds when the catalog presents a history.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Holder {
    /// Cold start: no validated projection.
    Cold,
    /// The held projection one event older than the current head (H0).
    Earlier,
    /// The held projection of the current head (H1).
    Current,
}

/// Every holder, cold first.
pub(super) const HOLDERS: [Holder; 3] = [Holder::Cold, Holder::Earlier, Holder::Current];

/// The held projection of the current head (H1), the only holder that refuses a fork over H0.
pub(super) const CURRENT: [Holder; 1] = [Holder::Current];

/// Both held projections, which refuse a fork over the history before H0's head.
pub(super) const HELD: [Holder; 2] = [Holder::Earlier, Holder::Current];

/// Campaign's declared answers for one history under each holder.
#[derive(Clone, Debug)]
pub(super) struct Answers {
    /// Campaign's answer at cold start.
    pub(super) cold: Answer,
    /// Campaign's answer through the held projection one event older than the head (H0).
    pub(super) earlier: Answer,
    /// Campaign's answer through the held projection of the current head (H1).
    pub(super) current: Answer,
}

impl Answers {
    /// The same answer under every holder.
    pub(super) fn alike(answer: &Answer) -> Self {
        Self {
            cold: answer.clone(),
            earlier: answer.clone(),
            current: answer.clone(),
        }
    }
}

/// One catalog history with Campaign's declared answers.
pub(super) struct Entry {
    /// The presented history.
    pub(super) history: crate::harness::oracle::HistoryView,
    /// Binding the load expects and caches bind: the history's own, or the binding the history
    /// was altered after.
    pub(super) binding: campaign_core::ProjectionBinding,
    /// Largest control lineage the host's request admits.
    pub(super) maximum_events: u32,
    /// Forbidden effect of loading the history where Campaign refuses it.
    pub(super) loaded: &'static str,
    /// Forbidden effect of using a cache of the history where Campaign refuses it.
    pub(super) cached: &'static str,
    /// Campaign's answers under each holder.
    pub(super) answers: Answers,
    /// The holders the entry is presented under: every holder, or only the held projections
    /// that refuse a fork whose head is not a stop (a stop-headed fork at the same depth is
    /// presented under every holder).
    pub(super) holders: &'static [Holder],
    /// The state the history's payloads replay to, when its constructor knows it (a fork's own
    /// state); otherwise the cache oracle replays the payloads.
    pub(super) replayed: Option<campaign_core::FrontierState>,
}

impl Entry {
    /// The declared answer under one holder.
    pub(super) const fn answer(&self, holder: Holder) -> &Answer {
        if matches!(holder, Holder::Cold) {
            &self.answers.cold
        } else if matches!(holder, Holder::Earlier) {
            &self.answers.earlier
        } else {
            &self.answers.current
        }
    }
}

/// Opens a guided campaign, loads it, and publishes one selection without loading again: the
/// session holds the projection one event older than its head (H0). Returns the session, H0's
/// history, projection, and state, the history before H0's head with Campaign's cold projection
/// of it and its state, and the histories before the earlier opening events.
///
/// # Errors
///
/// Returns the error of the scenario.
pub(super) fn prepared<'s, 'a>(
    harness: &'s mut crate::harness::Driver<'a>,
) -> crate::verdicts::ConformanceResult<(
    crate::harness::Session<'s, 'a>,
    super::lineages::forks::Earlier,
)> {
    let mut session = super::super::common::ample(harness)?;
    let [before_batch, before_first, before_second, before_last] =
        super::super::common::guided_frontier_after(&mut session, super::super::common::UNIT_COST)?;
    let before = before_head(&session, before_last)?;
    let history = session.history();
    let state = session.state.clone();
    let view = session.load()?;
    session.execute(None)?;
    super::super::common::require(
        session.payloads.len() == history.payloads.len().saturating_add(1)
            && session.state.outstanding.len() == 1,
        "one selection was published after the earlier history",
    )?;
    debug_assert_eq!(*view.state(), state);
    Ok((
        session,
        super::lineages::forks::Earlier {
            head: super::lineages::forks::Base {
                history,
                view,
                state,
            },
            before,
            opening: alloc::vec![before_batch, before_first, before_second],
        },
    ))
}

/// The history before the session's head, as captured before the head's publication (its graph
/// holds no control draft after it), with Campaign's cold projection of it and its state.
///
/// # Errors
///
/// Returns Campaign's refusal of the history.
// r[impl campaign.conformance.bounds]
fn before_head(
    session: &crate::harness::Session<'_, '_>,
    history: crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<super::lineages::forks::Base> {
    let view = campaign_choregraph::validate_control_projection(
        &super::lineages::projection_request(session, &history)?,
    )?;
    let state = view.state().clone();
    debug_assert_eq!(
        view.control_events().len().saturating_add(1),
        session.payloads.len()
    );
    Ok(super::lineages::forks::Base {
        history,
        view,
        state,
    })
}

/// Loads the current head through the adapter and assembles the catalog around it; the session
/// then holds the projection of the current head.
///
/// # Errors
///
/// Returns the error of the load or of a construction.
pub(super) fn assembled(
    session: &mut crate::harness::Session<'_, '_>,
    earlier: &super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Entry>> {
    let view = session.load()?;
    let honest = session.history();
    let unstopped = session.state.clone();
    let mut assembled = shapes::branch_values(session, earlier, &honest)?;
    assembled.extend(shapes::forked::forks(session, earlier, &honest)?);
    let tip = classes::Tip {
        view: &view,
        history: &honest,
        base: &earlier.head,
        unstopped: &unstopped,
    };
    let built = classes::entries(session, &tip)?;
    super::super::common::require(
        built.len() == classes::CLASSES.len(),
        "the catalog builds every class over the current head",
    )?;
    assembled.extend(built.into_iter().map(|(_, entry)| entry));
    super::super::common::require(
        assembled
            .iter()
            .any(|entry| entry.history.payloads == honest.payloads),
        "the catalog holds the honest history",
    )?;
    debug_assert!(assembled.len() > 1);
    Ok(assembled)
}
