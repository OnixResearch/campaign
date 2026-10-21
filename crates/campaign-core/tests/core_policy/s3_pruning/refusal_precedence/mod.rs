//! `campaign.guidance.declared_order` for retention: pruning planning, prune admission and replay
//! refuse a retention rank order beyond the fixed rank shape with `InvalidConfig`, but only after
//! their own earlier typed checks, which answer instead.

/// Admission, one candidate batch and one guidance record.
const GUIDED_CONTROL_EVENTS: u64 = 3;
/// Room for the three events and the stop reserve, but not for a prune.
const WITHOUT_PRUNE_ROOM: u64 = 4;
/// Room for the three events, a prune, and the stop reserve.
const WITH_PRUNE_ROOM: u64 = 5;

/// A retention-profile prune request for `candidates` under `rank_order`.
fn ordered(
    state: &campaign_core::FrontierState,
    candidates: Vec<campaign_core::Id>,
    rank_order: Vec<u32>,
) -> campaign_core::PruneRequest {
    campaign_core::PruneRequest {
        retention_profile_id: Some(crate::suite::s3_pruning::profile(b"retention")),
        rank_order,
        ..crate::suite::s3_pruning::explicit(state, candidates)
    }
}

/// Asserts that prune admission, its replay, and pruning planning under its rank order all answer
/// `expected` without state mutation.
fn assert_answers(
    events: &crate::suite::s3_fixture::History,
    state: &campaign_core::FrontierState,
    request: campaign_core::PruneRequest,
    expected: &campaign_core::CampaignError,
) -> Result<(), String> {
    const ANSWERS: usize = 3;
    let planned = campaign_core::plan_pruning(
        state,
        crate::suite::s3_pruning::plan_request(state, request.rank_order.clone()),
    );
    let forged = crate::suite::s3_pruning::forged_prune(state, request.clone())?;
    let answers = [
        crate::suite::s3_fixture::denial(campaign_core::prune_frontier(state, request))?,
        crate::suite::s3_fixture::replay_denial(events, forged)?,
        crate::suite::s3_fixture::denial(planned)?,
    ];
    assert_eq!(answers.each_ref(), [expected; ANSWERS]);
    crate::suite::s3_fixture::assert_unchanged(events, state)
}

/// Over the prune-batch bound a prune request answers `BoundExceeded`, and on a stopped campaign
/// pruning planning, prune admission and replay answer `CampaignStopped`, for an out-of-shape
/// order; the same order within the batch on the open campaign is `InvalidConfig`.
#[test]
// r[verify campaign.guidance.declared_order]
fn stop_and_prune_batch_checks_answer_before_the_rank_order() -> Result<(), String> {
    let frontier = crate::suite::s3_pruning::frontier(crate::suite::s3_pruning::NARROW_BATCH)?;
    let (events, state) = (&frontier.events, &frontier.state);
    let within = vec![frontier.middle, frontier.lowest];
    let beyond = vec![crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION];
    let config = campaign_core::CampaignError::InvalidConfig;
    let request = ordered(state, within.clone(), beyond.clone());
    assert_answers(events, state, request, &config)?;
    let over_batch = ordered(
        state,
        vec![frontier.kept, frontier.middle, frontier.lowest],
        beyond.clone(),
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::prune_frontier(state, over_batch.clone()))?,
        campaign_core::CampaignError::BoundExceeded
    );
    let forged = crate::suite::s3_pruning::forged_prune(state, over_batch)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(events, forged)?,
        campaign_core::CampaignError::BoundExceeded
    );
    let stop = campaign_core::admit_stop(
        state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"prune-stop"),
        },
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, stopped) = crate::suite::s3_fixture::append(events.clone(), stop)?;
    let request = ordered(&stopped, within, beyond);
    let stopped_answer = campaign_core::CampaignError::CampaignStopped;
    assert_answers(&events, &stopped, request, &stopped_answer)?;
    Ok(())
}

/// Without control-event room for new work, pruning planning, prune admission and replay answer
/// `ControlEventLimitReached` for an out-of-shape order; one more control event of room turns the
/// same answers into `InvalidConfig`.
#[test]
// r[verify campaign.guidance.declared_order]
fn control_event_room_is_checked_before_the_rank_order() -> Result<(), String> {
    for (max_control_events, expected) in [
        (
            WITHOUT_PRUNE_ROOM,
            campaign_core::CampaignError::ControlEventLimitReached,
        ),
        (WITH_PRUNE_ROOM, campaign_core::CampaignError::InvalidConfig),
    ] {
        let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
            bounds.max_control_events = max_control_events;
        })?;
        let (events, state, guided) = crate::suite::s3_fixture::guided(
            events,
            &state,
            b"room",
            crate::suite::constants::RANK_HIGH,
        )?;
        assert_eq!(state.control_event_count, GUIDED_CONTROL_EVENTS);
        assert!(state.stopped.is_none());
        assert!(state.outstanding.is_empty());
        let request = ordered(
            &state,
            vec![guided],
            vec![crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION],
        );
        assert_answers(&events, &state, request, &expected)?;
    }
    Ok(())
}

/// Before any guidance record fixes the rank shape, a retention order is checked against the
/// admitted rank-dimension bound only: an index below it plans, the bound itself is `InvalidConfig`.
#[test]
// r[verify campaign.guidance.declared_order]
fn an_unfixed_rank_shape_checks_rank_orders_against_the_dimension_bound() -> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|_| {})?;
    let (_events, state, _unguided) = crate::suite::candidate::admit(
        events,
        &state,
        b"unshaped",
        crate::suite::constants::COST_ONE,
    )?;
    let bound = u32::try_from(state.admission.bounds.max_rank_dimensions)
        .map_err(crate::suite::s3_fixture::err)?;
    let last = bound.checked_sub(1).ok_or("empty rank-dimension bound")?;
    for order in [
        vec![crate::suite::s3_pruning::FIRST_DIMENSION],
        vec![crate::suite::s3_pruning::BEYOND_RANK_SHAPE_DIMENSION],
        vec![last],
    ] {
        let planned = campaign_core::plan_pruning(
            &state,
            crate::suite::s3_pruning::plan_request(&state, order),
        );
        assert!(planned.is_ok(), "{planned:?}");
    }
    let planned = campaign_core::plan_pruning(
        &state,
        crate::suite::s3_pruning::plan_request(&state, vec![bound]),
    );
    assert_eq!(planned, Err(campaign_core::CampaignError::InvalidConfig));
    Ok(())
}
