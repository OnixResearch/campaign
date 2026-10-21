//! Selection event receipts name the event's own profile and bind its ticket, guidance record and
//! cost reservation; reference-evidence profiles are refused for event and refusal receipts.

/// A published exploratory selection event and the state after it.
struct Published {
    state: campaign_core::FrontierState,
    event: campaign_core::CampaignControlEvent,
    selection: campaign_core::SelectionDecision,
    profile: campaign_core::SelectionProfile,
    ticket: campaign_core::EntropyTicket,
}

fn published() -> Result<Published, String> {
    let frontier = crate::suite::schedules::frontier()?;
    let profile = crate::suite::schedules::always_explore(&frontier.state)?;
    let draw = crate::suite::schedules::EXPLORE_SECOND;
    let ticket = crate::suite::schedules::ticket(&frontier.state, &profile, draw)?;
    let (plan, _request) =
        crate::suite::schedules::planned(&frontier.state, profile.clone(), Some(ticket.clone()))?;
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Err(String::from("expected a selection"));
    };
    let selection = *selection;
    let event = crate::suite::selection::event_with(
        selection.clone(),
        profile.clone(),
        Some(ticket.clone()),
    )?;
    let state = campaign_core::apply_control_event(frontier.state, &event)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(selection.entropy_id, Some(ticket.entropy_id));
    assert_eq!(state.selection_count, selection.ordinal);
    Ok(Published {
        state,
        event,
        selection,
        profile,
        ticket,
    })
}

/// A selection event receipt names the profile the event carries and binds its entropy ticket,
/// guidance record and cost reservation; the same event under another production profile is
/// crossed, and under a reference-evidence profile refused.
#[test]
// r[verify campaign.receipts]
// r[verify campaign.receipts.crossed]
fn a_selection_event_receipt_binds_the_events_profile_and_ticket() -> Result<(), String> {
    let published = published()?;
    let state = &published.state;
    let binding = crate::suite::admission::current_projection(state)?;
    let receipt = |profile: &campaign_core::SelectionProfile| {
        crate::suite::certified::event_receipt(state, &published.event, profile, &binding)
    };
    let own = receipt(&published.profile).map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(own.decision_id, published.event.event_id());
    assert_eq!(own.selection_profile_id, published.profile.profile_id);
    assert_eq!(own.entropy_ticket_id, Some(published.ticket.entropy_id));
    assert_eq!(
        own.guidance_record_id,
        Some(published.selection.guidance_id)
    );
    let reservation = campaign_core::derive_cost_reservation_id(&published.selection)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(own.cost_reservation_id, Some(reservation));
    let no_claims = campaign_core::ReceiptClaims::no_claims();
    crate::suite::world::observed(campaign_core::validate_receipt(&own, &no_claims))?;
    let ranked = crate::suite::selection::ranked_profile()?;
    assert_eq!(
        crate::suite::s3_fixture::denial(receipt(&ranked))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let reference = crate::suite::certified::reference_profile()?;
    assert_eq!(
        crate::suite::s3_fixture::denial(receipt(&reference))?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    Ok(())
}

/// Receipts of non-selection events and of admission refusals accept a production profile and
/// refuse a reference-evidence profile with `ReferencePolicySelected`.
#[test]
// r[verify campaign.fd.oracle.isolated]
fn event_and_refusal_receipts_refuse_a_reference_profile() -> Result<(), String> {
    let ranked = crate::suite::selection::ranked_profile()?;
    let reference = crate::suite::certified::reference_profile()?;
    let (events, admitted) = crate::suite::admission::initial_state()?;
    let binding = crate::suite::admission::current_projection(&admitted)?;
    let admission = &events[0];
    let receipt = |profile: &campaign_core::SelectionProfile| {
        crate::suite::certified::event_receipt(&admitted, admission, profile, &binding)
    };
    receipt(&ranked).map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(receipt(&reference))?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    let cost = crate::suite::constants::COST_ONE;
    let (_events, state, existing_id) =
        crate::suite::candidate::admit(events.clone(), &admitted, b"bound-refused", cost)?;
    let existing = state
        .candidate(existing_id)
        .map_err(crate::suite::s3_fixture::err)?
        .candidate
        .clone();
    let refusal = campaign_core::explain_admission_refusal(
        &state,
        vec![existing],
        crate::suite::ingress::support::REJECT,
        crate::suite::ingress::support::coalescing_policy(),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or("the reject rule refuses a duplicate")?;
    let refusal_binding = crate::suite::admission::current_projection(&state)?;
    let refused = |profile: campaign_core::SelectionProfile| {
        let request = campaign_core::RefusalReceiptRequest {
            request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"refusal"),
            refusal: refusal.clone(),
            profile,
        };
        campaign_core::project_refusal_receipt(&state, &request, &refusal_binding)
    };
    refused(ranked).map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(refused(reference))?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    Ok(())
}
