//! Forks, stable discovery identity, and child reservation ordering.

#[test]
// r[verify campaign.fork]
// r[verify campaign.fork.valid]
fn one_selection_yields_two_distinct_children_with_one_parent_and_selection() -> Result<(), String>
{
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"fork")?;
    let left =
        crate::suite::s3_fixture::child(&state, &selection, b"fork-left", b"fork-left-event")?;
    let right =
        crate::suite::s3_fixture::child(&state, &selection, b"fork-right", b"fork-right-event")?;
    let event = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(
            &state,
            &selection,
            vec![left.clone(), right.clone()],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::ResultObserved(result) = event.clone() else {
        return Err(String::from("expected a result event"));
    };
    let (_events, forked) = crate::suite::s3_fixture::append(events, event)?;
    assert_ne!(left.reservation_id, right.reservation_id);
    assert_ne!(left.discovery_id, right.discovery_id);
    assert_ne!(left.event_payload_id, right.event_payload_id);
    let parent_depth = state
        .candidate(selection.candidate_id)
        .map_err(crate::suite::s3_fixture::err)?
        .candidate
        .depth;
    for child in [&left, &right] {
        let rederived = child
            .clone()
            .derive(state.admission.campaign_id, selection.selection_id)
            .map_err(crate::suite::s3_fixture::err)?;
        assert_eq!(&rederived, child);
        let discovery = forked
            .discovery(child.child_moment_id)
            .ok_or_else(|| String::from("missing fork discovery"))?;
        assert_eq!(discovery.first_result_id, result.observation.result_id);
        assert_eq!(
            forked.known_moment_depth(child.child_moment_id),
            Some(parent_depth)
        );
    }
    assert_eq!(
        result.observation.parent_moment_id,
        selection.parent_moment_id
    );
    assert_eq!(forked.discoveries.len(), 2);
    Ok(())
}

#[test]
// r[verify campaign.fork.duplicate]
fn a_repeated_child_or_a_shared_sibling_event_is_not_a_fork() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"conflict")?;
    let repeated = crate::suite::s3_fixture::child(
        &state,
        &first,
        b"conflict-repeat",
        b"conflict-repeat-event",
    )?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(
            &state,
            crate::suite::s3_fixture::observation(&state, &first, vec![repeated.clone(), repeated]),
        ))?,
        campaign_core::CampaignError::DuplicateChildMoment
    );
    let siblings_share_event = crate::suite::s3_fixture::observation(
        &state,
        &first,
        vec![
            crate::suite::s3_fixture::child(
                &state,
                &first,
                b"conflict-a",
                b"conflict-shared-event",
            )?,
            crate::suite::s3_fixture::child(
                &state,
                &first,
                b"conflict-b",
                b"conflict-shared-event",
            )?,
        ],
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(
            &state,
            siblings_share_event
        ))?,
        campaign_core::CampaignError::ConflictingResult
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.fd.identity]
// r[verify campaign.fork.duplicate]
fn a_child_event_owned_by_another_moment_is_a_conflict() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"conflict")?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let accepted = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(
            &state,
            &first,
            vec![crate::suite::s3_fixture::child(
                &state,
                &first,
                b"conflict-x",
                b"conflict-event",
            )?],
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, accepted)?;
    let collision = crate::suite::s3_fixture::observation(
        &state,
        &second,
        vec![crate::suite::s3_fixture::child(
            &state,
            &second,
            b"conflict-y",
            b"conflict-event",
        )?],
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, collision.clone()))?,
        campaign_core::CampaignError::ConflictingResult
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, collision)?
        )?,
        campaign_core::CampaignError::ConflictingResult
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.fd.identity.shared]
fn two_paths_to_one_child_observe_one_stable_discovery() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"shared")?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let first_path =
        crate::suite::s3_fixture::child(&state, &first, b"shared-child", b"shared-first-event")?;
    let event = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &first, vec![first_path.clone()]),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::ResultObserved(first_result) = event.clone() else {
        return Err(String::from("expected a result event"));
    };
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    let stable = state
        .discovery(first_path.child_moment_id)
        .cloned()
        .ok_or_else(|| String::from("missing first discovery"))?;
    let second_path =
        crate::suite::s3_fixture::child(&state, &second, b"shared-child", b"shared-second-event")?;
    let event = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &second, vec![second_path.clone()]),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(state.discovery(second_path.child_moment_id), Some(&stable));
    assert_eq!(stable.first_result_id, first_result.observation.result_id);
    assert_eq!(stable.event_payload_id, first_path.event_payload_id);
    assert_eq!(state.discoveries.len(), 1);
    assert_eq!(state.accepted_results.len(), 2);
    assert!(state.reservations.contains(&first_path.reservation_id));
    assert!(state.reservations.contains(&second_path.reservation_id));
    Ok(())
}

#[test]
// r[verify campaign.fd.reservation.ordering]
// r[verify campaign.fd.reservation.unreserved]
fn a_parent_commits_only_with_every_child_reserved() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"reserve")?;
    let mut unreserved =
        crate::suite::s3_fixture::child(&state, &selection, b"reserve-child", b"reserve-event")?;
    unreserved.reservation_id = campaign_core::Id::absent();
    let observation = crate::suite::s3_fixture::observation(&state, &selection, vec![unreserved]);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, observation.clone()))?,
        campaign_core::CampaignError::InvalidChildReservation
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, observation)?
        )?,
        campaign_core::CampaignError::InvalidChildReservation
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    let reserved =
        crate::suite::s3_fixture::child(&state, &selection, b"reserve-child", b"reserve-event")?;
    let event = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &selection, vec![reserved.clone()]),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, committed) = crate::suite::s3_fixture::append(events, event)?;
    assert!(committed.reservations.contains(&reserved.reservation_id));
    assert!(committed.has_known_moment(reserved.child_moment_id));
    assert!(
        committed
            .candidates
            .iter()
            .all(|candidate| candidate.candidate.parent_moment_id != reserved.child_moment_id)
    );
    Ok(())
}

#[test]
fn a_result_repeating_one_finding_is_denied_at_entry_and_replay() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"repeated-finding")?;
    let mut repeated = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    repeated.findings = vec![
        crate::suite::s3_fixture::finding(b"finding-a"),
        crate::suite::s3_fixture::finding(b"finding-a"),
    ];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, repeated.clone()))?,
        campaign_core::CampaignError::InvalidFinding
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, repeated)?
        )?,
        campaign_core::CampaignError::InvalidFinding
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.findings]
// r[verify campaign.findings.non_terminal]
// r[verify campaign.findings.valid]
fn findings_are_retained_bounded_per_campaign_and_do_not_stop_exploration() -> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_findings = crate::suite::constants::COST_ONE;
    })?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"finding")?;
    let (events, state, second) = crate::suite::s3_fixture::published(events, &state)?;
    let mut found = crate::suite::s3_fixture::observation(&state, &first, Vec::new());
    found.findings = vec![crate::suite::s3_fixture::finding(b"finding-a")];
    let event =
        campaign_core::admit_result(&state, found).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(
        state.findings,
        vec![crate::suite::s3_fixture::finding(b"finding-a")]
    );
    assert!(matches!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Select(_)
    ));
    let mut over_total = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    over_total.findings = vec![crate::suite::s3_fixture::finding(b"finding-b")];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, over_total.clone()))?,
        campaign_core::CampaignError::BoundExceeded
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, over_total)?
        )?,
        campaign_core::CampaignError::BoundExceeded
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    let mut known_again = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    known_again.findings = vec![crate::suite::s3_fixture::finding(b"finding-a")];
    let event =
        campaign_core::admit_result(&state, known_again).map_err(crate::suite::s3_fixture::err)?;
    let (_events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(state.findings.len(), 1);
    Ok(())
}
