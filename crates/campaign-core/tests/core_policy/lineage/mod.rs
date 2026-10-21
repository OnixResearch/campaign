const GRANDCHILD_DEPTH: u64 = 2;
const SKIPPED_DEPTH: u64 = 3;
const ROOT_DEPTH: u64 = 0;
const SHALLOW_MAX_DEPTH: u64 = 1;

struct Settled {
    events: Vec<campaign_core::CampaignControlEvent>,
    state: campaign_core::FrontierState,
    child_moment_id: campaign_core::Id,
    selection_id: campaign_core::Id,
    result: campaign_core::ResultObservation,
}

fn settle_with_child(
    events: Vec<campaign_core::CampaignControlEvent>,
    state: &campaign_core::FrontierState,
) -> Result<Settled, String> {
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        state,
        b"parent",
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
    let (mut events, state) = crate::suite::selection::publish(events, selection.clone())?;
    let event = campaign_core::admit_result(
        &state,
        crate::suite::observation::result_for(&state, &selection, b"lineage-child", b"payload")?,
    )
    .map_err(|error| format!("{error:?}"))?;
    let campaign_core::CampaignControlEvent::ResultObserved(result_event) = &event else {
        return Err(format!("expected a result event, got {event:?}"));
    };
    let result = result_event.observation.clone();
    events.push(event);
    let state = campaign_core::reconstruct(&events).map_err(|error| format!("{error:?}"))?;
    assert!(state.outstanding.is_empty());
    assert!(state.accepted_results.contains(&result.result_id));
    Ok(Settled {
        events,
        state,
        child_moment_id: crate::suite::admission::xid(
            campaign_core::IdKind::ChildMoment,
            b"lineage-child",
        ),
        selection_id: selection.selection_id,
        result,
    })
}

fn under(
    state: &campaign_core::FrontierState,
    parent_moment_id: campaign_core::Id,
    depth: u64,
) -> campaign_core::CandidateRecord {
    campaign_core::CandidateRecord {
        parent_moment_id,
        depth,
        ..crate::suite::candidate::record(state, b"lineage", crate::suite::constants::COST_ONE)
    }
}

fn admit(
    state: &campaign_core::FrontierState,
    candidate: campaign_core::CandidateRecord,
) -> campaign_core::CampaignResult<campaign_core::CampaignControlEvent> {
    campaign_core::admit_candidates(
        state,
        vec![candidate],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    )
}

/// Builds a well-formed batch event that bypasses intake so replay validation is exercised.
fn forged_batch(
    state: &campaign_core::FrontierState,
    candidate: campaign_core::CandidateRecord,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let candidate = candidate.derive().map_err(|error| format!("{error:?}"))?;
    crate::suite::ingress::support::forged_batch(
        state,
        vec![candidate],
        crate::suite::ingress::support::REJECT,
    )
}

fn replay_with(
    events: &[campaign_core::CampaignControlEvent],
    event: campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::FrontierState> {
    let mut forged = events.to_vec();
    forged.push(event);
    campaign_core::reconstruct(&forged)
}

#[test]
fn roots_start_at_depth_zero_and_children_take_the_selected_candidate_depth() -> Result<(), String>
{
    let (events, state) = crate::suite::admission::initial_state()?;
    let root_id = state.admission.roots[0].moment_id;
    assert_eq!(state.known_moment_depth(root_id), Some(ROOT_DEPTH));
    let settled = settle_with_child(events, &state)?;
    assert_eq!(
        settled.state.known_moment_depth(settled.child_moment_id),
        Some(crate::suite::constants::DEPTH_ROOT_CHILD)
    );
    assert_eq!(
        settled.state.settled,
        vec![campaign_core::Settlement {
            selection_id: settled.selection_id,
            settlement_id: settled.result.result_id,
        }]
    );
    assert_eq!(
        settled.state.reservations,
        vec![settled.result.children[0].reservation_id]
    );
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.valid]
fn a_candidate_under_a_discovered_child_is_admitted_at_parent_depth_plus_one() -> Result<(), String>
{
    let (events, state) = crate::suite::admission::initial_state()?;
    let settled = settle_with_child(events, &state)?;
    let event = admit(
        &settled.state,
        under(&settled.state, settled.child_moment_id, GRANDCHILD_DEPTH),
    )
    .map_err(|error| format!("{error:?}"))?;
    let mut events = settled.events;
    events.push(event);
    let state = campaign_core::reconstruct(&events).map_err(|error| format!("{error:?}"))?;
    assert!(state.candidates.iter().any(|entry| {
        entry.candidate.parent_moment_id == settled.child_moment_id
            && entry.candidate.depth == GRANDCHILD_DEPTH
    }));
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.invalid]
fn intake_rejects_a_depth_that_is_not_parent_depth_plus_one() -> Result<(), String> {
    let (_events, state) = crate::suite::admission::initial_state()?;
    let root_id = state.admission.roots[0].moment_id;
    for depth in [ROOT_DEPTH, GRANDCHILD_DEPTH] {
        assert_eq!(
            admit(&state, under(&state, root_id, depth)),
            Err(campaign_core::CampaignError::InvalidDepth)
        );
    }
    let (events, state) = crate::suite::admission::initial_state()?;
    let settled = settle_with_child(events, &state)?;
    let state_id = settled.state.state_id;
    for depth in [crate::suite::constants::DEPTH_ROOT_CHILD, SKIPPED_DEPTH] {
        assert_eq!(
            admit(
                &settled.state,
                under(&settled.state, settled.child_moment_id, depth)
            ),
            Err(campaign_core::CampaignError::InvalidDepth)
        );
    }
    assert_eq!(settled.state.state_id, state_id);
    Ok(())
}

#[test]
fn replay_rejects_a_depth_that_is_not_parent_depth_plus_one() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let root_id = state.admission.roots[0].moment_id;
    assert_eq!(
        replay_with(
            &events,
            forged_batch(&state, under(&state, root_id, GRANDCHILD_DEPTH))?
        ),
        Err(campaign_core::CampaignError::InvalidDepth)
    );
    let settled = settle_with_child(events, &state)?;
    assert_eq!(
        replay_with(
            &settled.events,
            forged_batch(
                &settled.state,
                under(
                    &settled.state,
                    settled.child_moment_id,
                    crate::suite::constants::DEPTH_ROOT_CHILD
                )
            )?
        ),
        Err(campaign_core::CampaignError::InvalidDepth)
    );
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.invalid]
fn intake_and_replay_reject_a_candidate_deeper_than_the_bound() -> Result<(), String> {
    let mut input = crate::suite::admission::default_input()?;
    input.bounds.max_depth = SHALLOW_MAX_DEPTH;
    input.bounds = input
        .bounds
        .derive()
        .map_err(|error| format!("{error:?}"))?;
    let (events, state) = crate::suite::admission::admitted(input)?;
    let settled = settle_with_child(events, &state)?;
    let too_deep = under(&settled.state, settled.child_moment_id, GRANDCHILD_DEPTH);
    assert_eq!(
        admit(&settled.state, too_deep.clone()),
        Err(campaign_core::CampaignError::CandidateOverDepth)
    );
    assert_eq!(
        replay_with(&settled.events, forged_batch(&settled.state, too_deep)?),
        Err(campaign_core::CampaignError::CandidateOverDepth)
    );
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.invalid]
// r[verify campaign.selection.invalid]
fn an_over_depth_candidate_is_never_selected() -> Result<(), String> {
    let mut input = crate::suite::admission::default_input()?;
    input.bounds.max_depth = SHALLOW_MAX_DEPTH;
    input.bounds = input
        .bounds
        .derive()
        .map_err(|error| format!("{error:?}"))?;
    let (events, state) = crate::suite::admission::admitted(input)?;
    let settled = settle_with_child(events, &state)?;
    let too_deep = under(&settled.state, settled.child_moment_id, GRANDCHILD_DEPTH)
        .derive()
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        admit(&settled.state, too_deep.clone()).map(|_| ()),
        Err(campaign_core::CampaignError::CandidateOverDepth)
    );
    let rebuilt =
        campaign_core::reconstruct(&settled.events).map_err(|error| format!("{error:?}"))?;
    assert_eq!(rebuilt.state_id, settled.state.state_id);
    let selection = crate::suite::selection::plan(&settled.state)?;
    assert_ne!(selection.candidate_id, too_deep.candidate_id);
    let forged = campaign_core::SelectionDecision {
        candidate_id: too_deep.candidate_id,
        ..selection
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        replay_with(&settled.events, crate::suite::selection::event(forged)?).map(|_| ()),
        Err(campaign_core::CampaignError::MissingCandidate)
    );
    Ok(())
}

#[test]
fn intake_and_replay_reject_an_unadmitted_operation_schema() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let foreign = campaign_core::CandidateRecord {
        operation_schema_id: crate::suite::admission::xid(
            campaign_core::IdKind::OperationSchema,
            b"foreign-operation-schema",
        ),
        ..crate::suite::candidate::record(&state, b"foreign", crate::suite::constants::COST_ONE)
    };
    assert_eq!(
        admit(&state, foreign.clone()),
        Err(campaign_core::CampaignError::UnsupportedSchema)
    );
    assert_eq!(
        replay_with(&events, forged_batch(&state, foreign)?),
        Err(campaign_core::CampaignError::UnsupportedSchema)
    );
    Ok(())
}
