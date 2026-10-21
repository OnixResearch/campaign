//! An accepted stop is terminal for new work at every entry point and on replay.

struct Stopped {
    events: crate::suite::s3_fixture::History,
    state: campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    first: campaign_core::SelectionDecision,
    second: campaign_core::SelectionDecision,
    later: campaign_core::SelectionDecision,
    stop: campaign_core::StopEvent,
}

/// A consumer stop accepted while two selections of one candidate remain outstanding.
fn stopped() -> Result<Stopped, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"terminal",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, first) = crate::suite::s3_fixture::published(events, &state)?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let later = crate::suite::selection::plan(&state)?;
    let stop = crate::suite::s3_stops::accepted(&state, crate::suite::s3_stops::consumer_reason())?;
    let (events, state) = crate::suite::s3_fixture::append(
        events,
        campaign_core::CampaignControlEvent::StopAccepted(stop.clone()),
    )?;
    assert_eq!(state.outstanding.len(), 2);
    assert_eq!(state.stopped.as_ref(), Some(&stop.stop));
    Ok(Stopped {
        events,
        state,
        candidate_id,
        first,
        second,
        later,
        stop,
    })
}

fn forged_retirement(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut retirement = campaign_core::RetirementEvent {
        event_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidate_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"late-retire"),
        policy_id: state.admission.policy_id,
        adapter_id: state.admission.adapter_id,
        origin: campaign_core::RetirementOrigin::Consumer,
        invalidation_id: campaign_core::Id::absent(),
    };
    retirement.invalidation_id = campaign_core::derive_retirement_invalidation_id(&retirement)
        .map_err(crate::suite::s3_fixture::err)?;
    retirement.event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::CandidateRetired,
        retirement.campaign_id,
        retirement.state_before_id,
        &[
            retirement.candidate_id,
            retirement.reason_id,
            retirement.invalidation_id,
        ],
    )
    .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        retirement.invalidation_id.kind,
        campaign_core::IdKind::Invalidation
    );
    assert_eq!(retirement.event_id.kind, campaign_core::IdKind::Event);
    Ok(campaign_core::CampaignControlEvent::CandidateRetired(
        retirement,
    ))
}

#[test]
// r[verify campaign.stop.recorded]
// r[verify campaign.stop.terminal]
fn an_accepted_stop_denies_new_work_at_every_entry_point() -> Result<(), String> {
    let Stopped {
        events,
        state,
        candidate_id,
        stop,
        ..
    } = stopped()?;
    let refusal = crate::suite::s3_fixture::err(campaign_core::CampaignError::CampaignStopped);
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&state)?.stop_id,
        stop.stop.stop_id
    );
    assert_eq!(
        crate::suite::candidate::admit(
            events.clone(),
            &state,
            b"late",
            crate::suite::constants::COST_ONE
        )
        .err(),
        Some(refusal.clone())
    );
    assert_eq!(
        crate::suite::candidate::guide(
            events,
            &state,
            candidate_id,
            crate::suite::constants::RANK_HIGH,
            crate::suite::constants::GENERATION_NEXT
        )
        .err(),
        Some(refusal)
    );
    let retirement = crate::suite::s3_fixture::retirement(
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Consumer,
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::retire_candidate(&state, retirement))?,
        campaign_core::CampaignError::CampaignStopped
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            crate::suite::s3_stops::consumer_reason()
        ))?,
        campaign_core::CampaignError::CampaignStopped
    );
    Ok(())
}

#[test]
// r[verify campaign.stop.terminal]
// r[verify campaign.stop.recorded]
fn prior_outstanding_selections_still_settle_after_an_accepted_stop() -> Result<(), String> {
    let Stopped {
        events,
        state,
        first,
        second,
        stop,
        ..
    } = stopped()?;
    let result = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &first, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, result)?;
    let closure = campaign_core::close_unobserved(
        &state,
        crate::suite::s3_fixture::closure(&state, &second, b"terminal-closure", true)?,
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, settled) = crate::suite::s3_fixture::append(events, closure)?;
    assert!(settled.outstanding.is_empty());
    assert_eq!(settled.stopped, Some(stop.stop));
    Ok(())
}

#[test]
// r[verify campaign.selection.stopped]
// r[verify campaign.stop.terminal]
fn replay_denies_new_work_after_an_accepted_stop() -> Result<(), String> {
    let Stopped {
        events,
        state,
        candidate_id,
        later,
        ..
    } = stopped()?;
    assert_eq!(later.candidate_id, candidate_id);
    assert!(state.stopped.is_some());
    let forged = [
        crate::suite::s3_fixture::replanned_on(&state, &later)?,
        crate::suite::s3_stops::forged_reason(&state, crate::suite::s3_stops::consumer_reason())?,
        forged_retirement(&state, candidate_id)?,
        crate::suite::s3_pruning::forged_prune(
            &state,
            crate::suite::s3_pruning::explicit(&state, vec![candidate_id]),
        )?,
    ];
    for event in forged {
        assert_eq!(
            crate::suite::s3_fixture::replay_denial(&events, event)?,
            campaign_core::CampaignError::CampaignStopped
        );
    }
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.stop.terminal]
fn pruning_after_a_stop_is_denied_at_entry() -> Result<(), String> {
    let Stopped {
        events,
        state,
        candidate_id,
        ..
    } = stopped()?;
    let request = crate::suite::s3_pruning::explicit(&state, vec![candidate_id]);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::prune_frontier(&state, request))?,
        campaign_core::CampaignError::CampaignStopped
    );
    let plan = campaign_core::RetentionPlanRequest {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        retention_profile_id: crate::suite::admission::xid(
            campaign_core::IdKind::RetentionProfile,
            b"late-retention",
        ),
        rank_order: vec![crate::suite::constants::FIRST_RANK_DIMENSION],
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"late-prune"),
    };
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::plan_pruning(&state, plan))?,
        campaign_core::CampaignError::CampaignStopped
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.stop.non_claim]
fn replay_requires_the_incomplete_search_non_claim_and_the_campaign() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let unacknowledged = crate::suite::s3_stops::forged_stop(
        &state,
        state.admission.campaign_id,
        crate::suite::s3_stops::consumer_reason(),
        false,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&events, unacknowledged)?,
        campaign_core::CampaignError::InvalidStop
    );
    let crossed = crate::suite::s3_stops::forged_stop(
        &state,
        crate::suite::admission::xid(campaign_core::IdKind::Campaign, b"other-campaign"),
        crate::suite::s3_stops::consumer_reason(),
        true,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(&events, crossed)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let (_events, stopped) = crate::suite::s3_fixture::append(
        events,
        crate::suite::s3_stops::forged_reason(&state, crate::suite::s3_stops::consumer_reason())?,
    )?;
    assert!(
        stopped
            .stopped
            .is_some_and(|stop| stop.incomplete_search_acknowledged)
    );
    Ok(())
}
