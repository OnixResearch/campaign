//! The fill chain a writer publishes over one fork base, keeping the points forks start their
//! heads from.

/// One kept point: `history` with Campaign's projection of it advanced from `previous`, an
/// earlier point of the same fill (which validates the fill after it), which must replay to
/// `state`.
///
/// # Errors
///
/// Returns Campaign's refusal of the point and `Diverged` when it replays to another state.
fn kept(
    session: &crate::harness::Session<'_, '_>,
    previous: &campaign_choregraph::ValidatedControlProjection,
    history: &crate::harness::oracle::HistoryView,
    state: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<Point> {
    let view = campaign_choregraph::validate_control_advance(
        previous,
        &super::super::projection_request(session, history)?,
    )?;
    super::super::super::super::common::require(
        view.state() == state,
        "a fork's fill replays to the state Campaign applied",
    )?;
    Ok((history.clone(), view))
}

/// One point of a fill chain: the published history and Campaign's projection of it.
pub(super) type Point = (
    crate::harness::oracle::HistoryView,
    campaign_choregraph::ValidatedControlProjection,
);

/// The fill chain over one base: the point after each fill event a presented fork starts its
/// head from (`None` elsewhere), and the fill drafts in publication order.
pub(super) struct Chain {
    points: alloc::vec::Vec<Option<Point>>,
    pub(super) drafts: alloc::vec::Vec<campaign_choregraph::EventDraft>,
}

impl Chain {
    /// Appends `deepest` fill events over `base` as a writer's control children, applying each to
    /// Campaign's state, and keeps the points at and one before each of `depths` with Campaign's
    /// projection of the point advanced from the previous kept point (which validates the fill
    /// between them).
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of a fill event or of a kept point.
    pub(super) fn over(
        session: &mut crate::harness::Session<'_, '_>,
        base: &super::Base,
        depths: &[usize],
    ) -> crate::verdicts::ConformanceResult<Self> {
        let deepest = depths.iter().copied().max().unwrap_or(0);
        let is_kept = |step: usize| {
            depths
                .iter()
                .any(|depth| *depth == step || depth.checked_sub(1) == Some(step))
        };
        let mut points = alloc::vec::Vec::with_capacity(deepest.saturating_add(1));
        let mut drafts = alloc::vec::Vec::with_capacity(deepest);
        let mut history = base.history.clone();
        let mut state = base.state.clone();
        let mut validated = base.view.clone();
        for step in 0..deepest {
            let point = if step == 0 {
                Some((history.clone(), base.view.clone()))
            } else if is_kept(step) {
                let point = kept(session, &validated, &history, &state)?;
                validated = point.1.clone();
                Some(point)
            } else {
                None
            };
            points.push(point);
            let filled = super::fill::event(session, &base.state, (&history, &state), step)?;
            let draft = campaign_choregraph::control_event_draft(
                &filled,
                session.identities.projection_schema_id,
                session.identities.adapter_id,
                session.identities.control_domain_id,
                Some(history.branch.head),
            )?;
            let admitted = history.graph.admit(draft.clone())?;
            state = campaign_core::apply_control_event(state, &filled)?;
            history
                .payloads
                .push(campaign_choregraph::ControlPayloadEnvelope {
                    history_event_id: admitted.event,
                    control_event: filled,
                });
            history.graph = admitted.graph;
            history.branch = campaign_choregraph::BranchRef {
                head: admitted.event,
                generation: history
                    .branch
                    .generation
                    .checked_add(1)
                    .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
                ..history.branch.clone()
            };
            drafts.push(draft);
        }
        let last = if deepest == 0 {
            (history, base.view.clone())
        } else {
            kept(session, &validated, &history, &state)?
        };
        points.push(Some(last));
        debug_assert_eq!(points.len(), drafts.len().saturating_add(1));
        Ok(Self { points, drafts })
    }

    /// The kept point after `step` fill events.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for a point the chain did not keep.
    pub(super) fn point(&self, step: usize) -> crate::verdicts::ConformanceResult<&Point> {
        self.points.get(step).and_then(Option::as_ref).ok_or(
            crate::verdicts::ConformanceError::Diverged {
                expected: "the fill chain keeps the point a fork starts its head from",
            },
        )
    }
}
