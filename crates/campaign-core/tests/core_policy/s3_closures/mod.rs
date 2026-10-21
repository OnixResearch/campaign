//! Explicit unobserved closure: valid closure infers nothing; missing acknowledgement fails.

#[test]
// r[verify campaign.unobserved]
// r[verify campaign.unobserved.valid]
fn an_acknowledged_closure_settles_without_inferring_a_child_or_outcome() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"closure")?;
    let closure = crate::suite::s3_fixture::closure(&state, &selection, b"closure-reason", true)?;
    let event = campaign_core::close_unobserved(&state, closure.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let (_events, closed) = crate::suite::s3_fixture::append(events, event)?;
    assert!(closed.outstanding.is_empty());
    assert_eq!(
        closed.settled,
        vec![campaign_core::Settlement {
            selection_id: selection.selection_id,
            settlement_id: closure.closure_id,
        }]
    );
    assert_eq!(closed.known_moments, state.known_moments);
    assert_eq!(closed.discoveries, state.discoveries);
    assert_eq!(closed.reservations, state.reservations);
    assert_eq!(closed.accepted_results, state.accepted_results);
    assert_eq!(closed.findings, state.findings);
    assert_eq!(closed.reserved_resources, state.reserved_resources);
    Ok(())
}

#[test]
// r[verify campaign.unobserved.invalid]
fn a_closure_without_acknowledgement_is_denied_at_entry_and_replay() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"unacknowledged")?;
    let unacknowledged =
        crate::suite::s3_fixture::closure(&state, &selection, b"unacknowledged-reason", false)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::close_unobserved(
            &state,
            unacknowledged.clone()
        ))?,
        campaign_core::CampaignError::MissingUncertaintyAcknowledgement
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_closure(&state, unacknowledged)?
        )?,
        campaign_core::CampaignError::MissingUncertaintyAcknowledgement
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.retry_boundary.invalid]
// r[verify campaign.unobserved.valid]
// r[verify campaign.observations.duplicate]
fn a_closure_needs_an_outstanding_selection_and_settles_it_once() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"once")?;
    let mut unknown = selection.clone();
    unknown.selection_id =
        crate::suite::admission::xid(campaign_core::IdKind::Selection, b"unknown-selection");
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::close_unobserved(
            &state,
            crate::suite::s3_fixture::closure(&state, &unknown, b"once-reason", true)?,
        ))?,
        campaign_core::CampaignError::MissingSelection
    );
    let closure = crate::suite::s3_fixture::closure(&state, &selection, b"once-reason", true)?;
    let event = campaign_core::close_unobserved(&state, closure.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let (events, closed) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::close_unobserved(&closed, closure))?,
        campaign_core::CampaignError::DuplicateResult
    );
    let other_reason =
        crate::suite::s3_fixture::closure(&closed, &selection, b"other-reason", true)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::close_unobserved(
            &closed,
            other_reason.clone()
        ))?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_closure(&closed, other_reason)?
        )?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(
            &closed,
            crate::suite::s3_fixture::observation(&closed, &selection, Vec::new()),
        ))?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &closed)
}
