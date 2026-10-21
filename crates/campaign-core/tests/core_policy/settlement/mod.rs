#[test]
// r[verify campaign.selection.reuse]
fn dynamic_candidate_selection_result_and_reuse_replay() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"candidate",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let first_selection = crate::suite::selection::plan(&state)?;
    let (events, state) = crate::suite::selection::publish(events, first_selection.clone())?;
    let result_event = (campaign_core::admit_result(
        &state,
        crate::suite::observation::result_for(
            &state,
            &first_selection,
            b"child",
            b"child-payload",
        )?,
    ))
    .map_err(|error| format!("{error:?}"))?;
    let mut events = events;
    events.push(result_event);
    let state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert!(state.has_known_moment(crate::suite::admission::xid(
        campaign_core::IdKind::ChildMoment,
        b"child"
    )));
    let second_selection = crate::suite::selection::plan(&state)?;
    assert_ne!(first_selection.selection_id, second_selection.selection_id);
    assert_ne!(first_selection.operation_id, second_selection.operation_id);
    Ok(())
}
#[test]
// r[verify campaign.fd.reservation.unreserved]
fn result_conflicts_duplicates_and_child_reservation_fail() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"result",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (_events, state) = crate::suite::selection::publish(events, selection.clone())?;
    let mut duplicate_child =
        crate::suite::observation::result_for(&state, &selection, b"child", b"payload")?;
    duplicate_child
        .children
        .push(duplicate_child.children[0].clone());
    assert_eq!(
        (campaign_core::admit_result(&state, duplicate_child))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::DuplicateChildMoment
    );
    let mut unreserved =
        crate::suite::observation::result_for(&state, &selection, b"child", b"payload")?;
    unreserved.children[0].reservation_id = crate::suite::admission::xid(
        campaign_core::IdKind::ChildReservation,
        b"wrong-reservation",
    );
    assert_eq!(
        (campaign_core::admit_result(&state, unreserved))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::InvalidChildReservation
    );
    Ok(())
}
#[test]
// r[verify campaign.stop.recorded]
// r[verify campaign.stop.terminal]
fn stop_is_monotonic_but_results_remain_admissible() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, &state, b"stop", crate::suite::constants::COST_ONE)?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (mut events, state) = crate::suite::selection::publish(events, selection.clone())?;
    events.push(
        (campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::ConsumerRequested {
                reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"stop-now"),
            },
        ))
        .map_err(|error| format!("{error:?}"))?,
    );
    let stopped = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        (campaign_core::admit_candidates(
            &stopped,
            vec![crate::suite::candidate::record(
                &stopped,
                b"late",
                crate::suite::constants::COST_ONE
            )],
            campaign_core::AdmissionRule {
                duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
            },
            crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce")
        ))
        .err()
        .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::CampaignStopped
    );
    let result = campaign_core::admit_result(
        &stopped,
        crate::suite::observation::result_for(
            &stopped,
            &selection,
            b"settle-child",
            b"settle-payload",
        )?,
    );
    assert!(result.is_ok());
    Ok(())
}
