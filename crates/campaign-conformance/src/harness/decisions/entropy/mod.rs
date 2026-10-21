//! Adapter-issued entropy tickets, decoded against the scenario draw and the entropy context.

/// Expectation of every adapter-issued ticket: it decodes to exactly the scenario draw.
pub(crate) const EXACT_DRAWS: &str =
    "the ticket carries the scenario's exact explore and index draws";

/// Expectation of every adapter-issued ticket: its counter is the context's next selection
/// ordinal.
pub(crate) const NEXT_ORDINAL: &str = "the ticket's counter is the next selection ordinal";

/// Requires an adapter-issued ticket to decode to exactly the scenario draw (its explore draw
/// and its index draw are the scenario's, at full width) and to carry the context's counter, the
/// next selection ordinal.
///
/// # Errors
///
/// Returns `Diverged` when either decoded draw differs from the scenario draw or the counter
/// differs from the context's.
// r[impl campaign.conformance.bounds]
pub(crate) fn decoded(
    ticket: campaign_core::EntropyTicket,
    draw: &crate::vocabulary::ScenarioDraw,
    context: &crate::port::EntropyContext,
) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
    if ticket.explore_draw() != draw.explore_draw || ticket.index_draw() != draw.index_draw {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: EXACT_DRAWS,
        });
    }
    if ticket.counter != context.counter {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: NEXT_ORDINAL,
        });
    }
    debug_assert_eq!(ticket.index_draw(), draw.index_draw);
    Ok(ticket)
}

impl crate::harness::Session<'_, '_> {
    /// Issues one ticket through the adapter for the current state; the ticket must carry
    /// exactly the scenario draw.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the ticket and `Diverged` for a ticket whose decoded draws
    /// are not the scenario draw.
    pub(crate) fn ticket(
        &mut self,
        draw: &crate::vocabulary::ScenarioDraw,
        profile: &campaign_core::SelectionProfile,
    ) -> crate::verdicts::ConformanceResult<campaign_core::EntropyTicket> {
        let context = crate::port::EntropyContext {
            campaign_id: self.state.admission.campaign_id,
            state_id: self.state.state_id,
            selection_profile_id: profile.profile_id,
            counter: self
                .state
                .selection_count
                .checked_add(1)
                .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        };
        decoded(
            self.harness.adapter.entropy_ticket(draw, &context)?,
            draw,
            &context,
        )
    }
}
