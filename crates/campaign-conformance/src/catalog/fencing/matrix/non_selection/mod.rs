//! Gate questions that present publications of other control events (a guidance improvement, a
//! candidate batch, and a stop), each with its own honest committed evidence, for selections
//! they do not publish: the durably published selection, a compare-and-set loser, and a
//! selection that was planned and never published. Campaign refuses every one of them.

/// Levels of the strict guidance improvement the session publishes as other control work.
const IMPROVED_LEVELS: [u32; 2] = [40, 4];

/// Operation the session admits in the candidate batch it publishes as other control work.
const EXTRA_OPERATION: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(4);

/// Publications of other control events with the forbidden effect of executing a selection under
/// each, and a selection planned between them that is never published.
pub(super) struct OtherWork {
    /// Each non-selection publication with the forbidden effect of a grant under it.
    publications: alloc::vec::Vec<(&'static str, campaign_choregraph::ControlPublication)>,
    /// A selection planned after the guidance publication and never published.
    pub(super) unpublished: campaign_core::SelectionDecision,
}

/// Publishes a guidance improvement, plans a selection it never publishes, and publishes a
/// candidate batch.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the publications and the plan.
pub(super) fn publish(
    session: &mut crate::harness::Session<'_, '_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<OtherWork> {
    let guidance = session.guide_published(&super::super::super::common::preference(
        super::super::super::common::OP_C,
        super::super::super::common::SECOND_GENERATION,
        IMPROVED_LEVELS,
    ))?;
    let unpublished = session.select(draw)?;
    let batch = session.admit_published(&[super::super::super::common::operation(
        EXTRA_OPERATION,
        crate::harness::history::ROOT,
        super::super::super::common::UNIT_COST,
    )])?;
    debug_assert_ne!(guidance.history_event_id(), batch.history_event_id());
    Ok(OtherWork {
        publications: alloc::vec![
            (
                "a selection executed under a guidance publication's committed evidence",
                guidance,
            ),
            (
                "a selection executed under a candidate batch publication's committed evidence",
                batch,
            ),
        ],
        unpublished,
    })
}

impl OtherWork {
    /// Adds the committed stop publication.
    pub(super) fn with_stop(mut self, stop: campaign_choregraph::ControlPublication) -> Self {
        self.publications.push((
            "a selection executed under a stop publication's committed evidence",
            stop,
        ));
        self
    }

    /// Presents every non-selection publication with its own committed evidence for each of
    /// `selections` and the unpublished selection; the adapter must refuse every one.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of a gate answer and `Diverged` when a presented publication
    /// publishes a selection.
    pub(super) fn ask(
        &self,
        session: &mut crate::harness::Session<'_, '_>,
        selections: &[&campaign_core::SelectionDecision],
    ) -> crate::verdicts::ConformanceResult<usize> {
        let mut asked: usize = 0;
        for (forbidden, publication) in &self.publications {
            super::super::super::common::require(
                !matches!(
                    publication.control_event(),
                    campaign_core::CampaignControlEvent::SelectionPublished(_)
                ),
                "every other presented publication publishes a non-selection control event",
            )?;
            let own = crate::harness::decisions::durable_evidence(
                publication,
                crate::harness::decisions::Durability::COMMITTED,
            );
            for selection in selections.iter().copied().chain([&self.unpublished]) {
                super::refuse(session, selection, Some(publication), Some(&own), forbidden)?;
                asked = asked.saturating_add(1);
            }
        }
        debug_assert!(asked >= self.publications.len());
        Ok(asked)
    }
}
