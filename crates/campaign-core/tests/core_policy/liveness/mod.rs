//! `campaign.wait.guidance` / `campaign.stop.frontier`: a candidate that can no longer receive
//! guidance under `max_guidance_records` is not awaited; the planner stops instead of waiting.

/// Admits two candidates, guides the first, and retires it, under `max_guidance_records`.
fn one_unguided(
    max_guidance_records: u64,
) -> Result<(campaign_core::FrontierState, campaign_core::Id), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_guidance_records = max_guidance_records;
    })?;
    let (events, state, first) = crate::suite::candidate::admit(
        events,
        &state,
        b"budget-first",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, second) = crate::suite::candidate::admit(
        events,
        &state,
        b"budget-second",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance) = crate::suite::candidate::guide(
        events,
        &state,
        first,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::GENERATION_NEXT,
    )?;
    let retire = campaign_core::retire_candidate(
        &state,
        crate::suite::s3_fixture::retirement(
            &state,
            first,
            campaign_core::RetirementOrigin::Consumer,
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, state) = crate::suite::s3_fixture::append(events, retire)?;
    assert_eq!(state.guidance_record_count, 1);
    assert!(
        state
            .candidate(second)
            .is_ok_and(|candidate| candidate.latest_guidance.is_none())
    );
    Ok((state, second))
}

/// With the guidance-record bound used up, the unguided candidate can never be guided: the
/// planner produces the frontier stop and `admit_stop` accepts it.
#[test]
// r[verify campaign.wait.guidance_exhausted]
// r[verify campaign.stop.frontier]
fn an_exhausted_guidance_budget_stops_instead_of_waiting() -> Result<(), String> {
    let (state, unguided) = one_unguided(1)?;
    assert_eq!(
        state.guidance_record_count,
        state.admission.bounds.max_guidance_records
    );
    let guidance = campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id: unguided,
        generation: crate::suite::constants::GENERATION_NEXT,
        ranks: vec![crate::suite::constants::RANK_HIGH],
        source_facts_id: crate::suite::admission::xid(
            campaign_core::IdKind::SourceFacts,
            b"budget-facts",
        ),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    };
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_guidance(&state, guidance))?,
        campaign_core::CampaignError::BoundExceeded
    );
    let campaign_core::NextDecision::Stop(stop) = crate::suite::s3_fixture::plan_decision(&state)?
    else {
        return Err(String::from("an exhausted guidance budget must stop"));
    };
    assert_eq!(stop.reason, campaign_core::StopReason::FrontierExhausted);
    let event = campaign_core::admit_stop(&state, campaign_core::StopReason::FrontierExhausted)
        .map_err(crate::suite::s3_fixture::err)?;
    let stopped =
        campaign_core::apply_control_event(state, &event).map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        stopped.stopped.map(|stop| stop.reason),
        Some(campaign_core::StopReason::FrontierExhausted)
    );
    Ok(())
}

/// While one more guidance record fits, the unguided candidate is awaited and the frontier stop
/// is not admissible.
#[test]
// r[verify campaign.wait.guidance]
fn a_remaining_guidance_budget_still_waits_for_guidance() -> Result<(), String> {
    let (state, _unguided) = one_unguided(2)?;
    assert_eq!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::FrontierExhausted
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    Ok(())
}
