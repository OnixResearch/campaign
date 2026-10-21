//! Event receipts of a claims session in every state it reaches: after the admission and after
//! every later publication, running or stopped, the adapter loads the state reached and is asked
//! for the claims of the event receipt of every control event published so far (the admission,
//! batches, guidance, selections, results, closures, retirements, prunes, and stops), projected
//! against that state for a ranked and an exploratory request.

/// Makes `session` a witnessed claims session: presents its current state now and every state
/// each later publication reaches.
///
/// # Errors
///
/// Returns the error of the presentation, and `ForbiddenAccepted` naming the first claimed
/// receipt.
pub(super) fn witnessed(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    session.witness = Some(present);
    present(session)
}

/// Loads the state the session reached, so the adapter holds it, and presents the event receipt
/// of every published control event projected against it under every signal combination: for a
/// ranked and for an exploratory request, and a selection event for the request of the profile
/// it carries.
///
/// # Errors
///
/// Returns the adapter error of the load, the Campaign error of a receipt, and
/// `ForbiddenAccepted` naming the first claimed receipt.
pub(in crate::catalog::evidence) fn present(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    session.load()?;
    let profiles = [
        session.selection_profile(false)?,
        session.selection_profile(true)?,
    ];
    let binding = session.binding()?;
    let request_id = crate::harness::suite_id(campaign_core::IdKind::Request, "event-receipt")?;
    let events = session.control_events();
    let mut receipts = alloc::vec::Vec::with_capacity(events.len().saturating_mul(profiles.len()));
    for event in &events {
        let requested =
            if let campaign_core::CampaignControlEvent::SelectionPublished(selection) = event {
                core::slice::from_ref(selection.profile.as_ref())
            } else {
                profiles.as_slice()
            };
        for profile in requested {
            let request = campaign_core::EventReceiptRequest {
                request_id,
                event: event.clone(),
                profile: profile.clone(),
            };
            let receipt = campaign_core::project_event_receipt(&session.state, &request, &binding)?;
            receipts.push((super::kinds::event_kind(event), receipt));
        }
    }
    debug_assert!(receipts.len() >= events.len());
    debug_assert!(
        receipts
            .iter()
            .all(|(_, receipt)| receipt.state_id == session.state.state_id)
    );
    super::require_unclaimed(session, &receipts)
}
