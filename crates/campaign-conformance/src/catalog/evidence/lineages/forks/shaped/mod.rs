//! Forks of one dropped head built one (depth, head) shape at a time over a shared fill chain:
//! the head is any event kind a fork can end in (`Head`), so the catalog's six head kinds and
//! the state coverage's event sub-kinds share one construction.

/// The event that ends a fork.
pub(in crate::catalog::evidence) trait Head: Copy {
    /// Whether the head settles an outstanding selection: over a fork point with nothing
    /// outstanding, the fork publishes a ranked selection as its last fill event.
    fn settles(self) -> bool;

    /// Control events the head leaves room for beyond a stop head at its depth (none over a
    /// stopped base).
    fn reserve(self, is_stopped: bool) -> usize;

    /// The head event admitted against the fork point `(history, view)`, `step` fill events
    /// beyond the dropped head of a fork over `base`; `None` when the point admits no such head.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the head.
    fn event(
        self,
        session: &mut crate::harness::Session<'_, '_>,
        base: &campaign_core::FrontierState,
        point: (
            &crate::harness::oracle::HistoryView,
            &campaign_choregraph::ValidatedControlProjection,
        ),
        step: usize,
    ) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>>;
}

/// Forks over one base that drop one head: the fill chain over the base, kept at the presented
/// depths, and the deepest depth the bounds admit.
pub(in crate::catalog::evidence) struct Forking<'b> {
    base: &'b super::Base,
    dropped: (
        &'b campaign_choregraph::BranchRef,
        &'b campaign_core::CampaignControlEvent,
    ),
    chain: super::points::Chain,
    maximal: usize,
}

impl<'b> Forking<'b> {
    /// Fills the chain over `base` keeping the points at, and one before, each of `depths`;
    /// `dropped` is the head one event after `base` (its branch value and event).
    ///
    /// # Errors
    ///
    /// Returns the error of a fill event or of the bounds (`maximal_depth`).
    pub(in crate::catalog::evidence) fn over(
        session: &mut crate::harness::Session<'_, '_>,
        base: &'b super::Base,
        dropped: (
            &'b campaign_choregraph::BranchRef,
            &'b campaign_core::CampaignControlEvent,
        ),
        depths: &[usize],
    ) -> crate::verdicts::ConformanceResult<Self> {
        let maximal = super::extent::maximal_depth(session, base)?;
        debug_assert!(depths.iter().all(|depth| *depth <= maximal));
        let chain = super::points::Chain::over(session, base, depths)?;
        Ok(Self {
            base,
            dropped,
            chain,
            maximal,
        })
    }

    /// The deepest fork the bounds admit over the base (`maximal_depth`).
    pub(in crate::catalog::evidence) const fn maximal(&self) -> usize {
        self.maximal
    }

    /// The deepest depth of a fork ending in `head`: the maximal depth less the room the head
    /// leaves.
    pub(in crate::catalog::evidence) fn ceiling<H: Head>(&self, head: H) -> usize {
        self.maximal
            .saturating_sub(head.reserve(self.base.state.stopped.is_some()))
    }

    /// The fork `depth` events beyond the dropped head ending in `head`, admitted next to the
    /// current head, or `None` when the fork state admits no such head or the fork would
    /// reproduce the dropped head.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the construction, and `Diverged`
    /// for a depth the chain did not keep or a fork of another depth or generation than
    /// requested.
    // r[impl campaign.conformance.bounds]
    pub(in crate::catalog::evidence) fn fork<H: Head>(
        &self,
        session: &mut crate::harness::Session<'_, '_>,
        (depth, head): (usize, H),
    ) -> crate::verdicts::ConformanceResult<Option<super::Fork<H>>> {
        let (dropped, dropped_event) = self.dropped;
        let needs_selection =
            head.settles() && self.chain.point(depth)?.1.state().outstanding.is_empty();
        let filled = if needs_selection {
            let Some(filled) = depth.checked_sub(1) else {
                return Ok(None);
            };
            filled
        } else {
            depth
        };
        let (mut history, mut view) = self.chain.point(filled)?.clone();
        let mut drafts = self
            .chain
            .drafts
            .get(..filled)
            .map(<[_]>::to_vec)
            .unwrap_or_default();
        let steps: &[bool] = if needs_selection {
            &[false, true]
        } else {
            &[true]
        };
        for is_head in steps.iter().copied() {
            let event = if is_head {
                head.event(session, &self.base.state, (&history, &view), filled)?
            } else {
                super::heads::selection(session, &view)?
            };
            let Some(event) = event else {
                return Ok(None);
            };
            if history.payloads.len() == self.base.history.payloads.len() && event == *dropped_event
            {
                return Ok(None);
            }
            let published = super::publish_over(session, &history, &view, event)?;
            drafts.push(published.draft);
            history = published.history;
            view = published.view;
        }
        let forked = super::over_current(session, history, drafts)?;
        let dropped_events = self.base.history.payloads.len().saturating_add(1);
        let generations = u64::try_from(depth).ok();
        super::super::super::super::common::require(
            forked.payloads.len() == dropped_events.saturating_add(depth)
                && generations.and_then(|depth| dropped.generation.checked_add(depth))
                    == Some(forked.branch.generation)
                && !forked
                    .payloads
                    .iter()
                    .any(|payload| payload.history_event_id == dropped.head),
            "the fork is the requested depth deeper than the dropped head and drops it",
        )?;
        debug_assert!(forked.graph.contains(dropped.head));
        Ok(Some(super::Fork {
            depth,
            is_deepest: depth == self.ceiling(head),
            head,
            history: forked,
            state: view.state().clone(),
        }))
    }
}
