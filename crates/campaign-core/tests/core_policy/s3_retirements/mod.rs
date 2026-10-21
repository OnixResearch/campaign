//! Explicit candidate retirement: valid and outstanding work.

mod refusals;

pub(crate) fn retire(
    events: crate::suite::s3_fixture::History,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    origin: campaign_core::RetirementOrigin,
) -> Result<
    (
        crate::suite::s3_fixture::History,
        campaign_core::FrontierState,
        campaign_core::RetirementEvent,
    ),
    String,
> {
    let event = campaign_core::retire_candidate(
        state,
        crate::suite::s3_fixture::retirement(state, candidate_id, origin),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidateRetired(retirement) = event.clone() else {
        return Err(format!("expected a retirement event, got {event:?}"));
    };
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    Ok((events, state, retirement))
}

#[test]
// r[verify campaign.retirement]
// r[verify campaign.retirement.valid]
fn a_retired_candidate_is_never_planned_and_keeps_its_identity_and_history() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, best) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"best",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, other) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"other",
        crate::suite::constants::RANK_LOW,
    )?;
    let before = state
        .candidate(best)
        .map_err(crate::suite::s3_fixture::err)?
        .clone();
    let (events, state, retirement) = retire(
        events,
        &state,
        best,
        campaign_core::RetirementOrigin::Consumer,
    )?;
    let after = state
        .candidate(best)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        after.status,
        campaign_core::CandidateStatus::Retired {
            reason_id: retirement.reason_id
        }
    );
    assert_eq!(after.candidate, before.candidate);
    assert_eq!(after.latest_guidance, before.latest_guidance);
    assert_eq!(after.selection_count, before.selection_count);
    let selection = crate::suite::selection::plan(&state)?;
    assert_eq!(selection.candidate_id, other);
    let (_events, state, _retirement) = retire(
        events,
        &state,
        other,
        campaign_core::RetirementOrigin::Policy,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::planned_stop(&state)?.reason,
        campaign_core::StopReason::FrontierExhausted
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.dynamics.invalidate]
// r[verify campaign.retirement.valid]
fn retirement_is_an_explicit_invalidation_and_never_an_addition() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"invalidate",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (_events, retired, retirement) = retire(
        events,
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Policy,
    )?;
    assert_eq!(
        retirement.invalidation_id.kind,
        campaign_core::IdKind::Invalidation
    );
    assert_eq!(
        retirement.dynamics(),
        campaign_core::FrontierDynamics::Invalidation {
            invalidation_id: retirement.invalidation_id,
            cleared_dependency_id: candidate_id,
        }
    );
    campaign_core::validate_dynamics(&retirement.dynamics())
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(retired.candidates.len(), state.candidates.len());
    assert_eq!(retired.discoveries, state.discoveries);
    assert_eq!(retired.accepted_results, state.accepted_results);
    assert_eq!(retired.guidance_record_count, state.guidance_record_count);
    Ok(())
}

#[test]
// r[verify campaign.retirement.valid]
fn a_replayed_selection_of_a_retired_candidate_is_denied() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"replayed",
        crate::suite::constants::RANK_HIGH,
    )?;
    let earlier = crate::suite::selection::plan(&state)?;
    assert_eq!(earlier.candidate_id, candidate_id);
    let (events, retired, _retirement) = retire(
        events,
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Consumer,
    )?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::replanned_on(&retired, &earlier)?
        )?,
        campaign_core::CampaignError::CandidateRetired
    );
    Ok(())
}

#[test]
// r[verify campaign.retirement.outstanding]
fn a_retired_candidate_with_outstanding_work_still_settles_and_stays_retired() -> Result<(), String>
{
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"busy",
        crate::suite::constants::RANK_HIGH,
    )?;
    let (events, state, first) = crate::suite::s3_fixture::published(events, &state)?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    assert_eq!(first.candidate_id, candidate_id);
    assert_eq!(second.candidate_id, candidate_id);
    let (events, state, retirement) = retire(
        events,
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Consumer,
    )?;
    let result = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(
            &state,
            &first,
            vec![crate::suite::s3_fixture::child(
                &state,
                &first,
                b"busy-child",
                b"busy-payload",
            )?],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, result)?;
    let closure = campaign_core::close_unobserved(
        &state,
        crate::suite::s3_fixture::closure(&state, &second, b"busy-closure", true)?,
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, state) = crate::suite::s3_fixture::append(events, closure)?;
    assert!(state.outstanding.is_empty());
    assert_eq!(state.settled.len(), 2);
    assert_eq!(
        state
            .candidate(candidate_id)
            .map_err(crate::suite::s3_fixture::err)?
            .status,
        campaign_core::CandidateStatus::Retired {
            reason_id: retirement.reason_id
        }
    );
    Ok(())
}
