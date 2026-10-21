//! Publication fence adapter matrix (`campaign.publication_fence`, D2).

const IMPROVED_RANK: i64 = 20;
const BEST_RANK: i64 = 30;
const SECOND_GUIDANCE_GENERATION: u64 = 2;

fn only_candidate(state: &campaign_core::FrontierState) -> Result<campaign_core::Id, String> {
    state
        .candidates
        .first()
        .map(|candidate| candidate.candidate.candidate_id)
        .ok_or_else(|| String::from("the fixture admits one candidate"))
}

fn improved_guidance(
    state: &campaign_core::FrontierState,
    rank: i64,
) -> Result<campaign_core::CampaignControlEvent, String> {
    crate::suite::identity::observed(campaign_core::admit_guidance(
        state,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            candidate_id: only_candidate(state)?,
            generation: SECOND_GUIDANCE_GENERATION,
            ranks: vec![rank],
            source_facts_id: crate::suite::identity::derive(
                campaign_core::IdKind::SourceFacts,
                b"repair-fence-facts",
            ),
            ordering_profile_id: state.admission.guidance_profile_id,
            adapter_id: crate::suite::identity::adapter(),
        },
    ))
}

/// A second planner that declares a different profile, so its selection is a distinct event.
fn rival_selection(
    current: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::SelectionDecision, String> {
    let mut request = crate::suite::selection::request_for(current)?;
    request.profile.decay = campaign_core::SelectionCountDecay::None;
    request.profile = crate::suite::identity::observed(request.profile.derive())?;
    let plan = crate::suite::identity::observed(
        campaign_choregraph::plan_next_from_published_projection(
            current,
            campaign_choregraph::PublishedSelectionRequest { request },
        ),
    )?;
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Err(String::from("the rival planner must select"));
    };
    Ok(*selection)
}

fn publish_against(
    history: &crate::suite::history::Fixture,
    previous: &campaign_choregraph::ValidatedControlProjection,
    expected_binding: campaign_core::ProjectionBinding,
    event: campaign_core::CampaignControlEvent,
) -> campaign_choregraph::AdapterResult<campaign_choregraph::ControlPublication> {
    campaign_choregraph::plan_control_event_publication(
        campaign_choregraph::ControlPublicationRequest {
            current: history.projection_request(expected_binding),
            previous,
            control_event: event,
        },
    )
}

#[test]
// r[verify campaign.publication_fence.stale]
fn a_selection_planned_on_a_stale_branch_cannot_publish() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let stale = history.current()?;
    let selection = crate::suite::selection::select_from_current(&stale)?;
    let (history, _guidance) = history.append(improved_guidance(stale.state(), IMPROVED_RANK)?)?;
    let fresh = history.current()?;
    assert_eq!(
        publish_against(
            &history,
            &stale,
            fresh.binding().clone(),
            crate::suite::selection::event(&selection)
        ),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    assert_eq!(
        publish_against(
            &history,
            &stale,
            stale.binding().clone(),
            crate::suite::selection::event(&selection)
        ),
        Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
    );
    let replanned = crate::suite::selection::select_from_current(&fresh)?;
    assert_eq!(
        replanned.publication_plan.expected_generation,
        fresh.binding().generation
    );
    let publication = crate::suite::identity::observed(publish_against(
        &history,
        &fresh,
        fresh.binding().clone(),
        crate::suite::selection::event(&replanned),
    ))?;
    assert_eq!(publication.branch_plan().before, history.branch);
    Ok(())
}

#[test]
fn a_republished_control_event_cannot_publish_as_a_duplicate() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let guidance = history
        .payloads
        .last()
        .map(|payload| payload.control_event.clone())
        .ok_or_else(|| String::from("the fixture publishes guidance last"))?;
    let before = history.current()?;
    assert_eq!(
        history.publish(guidance),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::DuplicateControlEvent
        ))
    );
    assert_eq!(history.current()?, before);
    Ok(())
}

#[test]
fn a_conflicting_control_event_from_the_same_state_cannot_publish() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let state = history.current()?.state().clone();
    let first = improved_guidance(&state, IMPROVED_RANK)?;
    let conflicting = improved_guidance(&state, BEST_RANK)?;
    assert_ne!(first.event_id(), conflicting.event_id());
    let (history, _publication) = history.append(first)?;
    let before = history.current()?;
    assert_eq!(
        history.publish(conflicting),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::StaleState
        ))
    );
    assert_eq!(history.current()?, before);
    Ok(())
}

#[test]
// r[verify campaign.publication_fence.stale]
fn a_host_compare_and_set_rejection_forces_replanning_from_the_fresh_projection()
-> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let winner = crate::suite::selection::select_from_current(&current)?;
    let rival = rival_selection(&current)?;
    assert_ne!(winner.selection_id, rival.selection_id);
    let rival_publication = crate::suite::identity::observed(publish_against(
        &history,
        &current,
        current.binding().clone(),
        crate::suite::selection::event(&rival),
    ))?;
    let (history, winner_publication) = history.append(crate::suite::selection::event(&winner))?;
    assert_eq!(history.branch, winner_publication.branch_plan().after);
    assert!(matches!(
        choregraph_history::plan_branch_advance(
            &history.graph,
            &history.branch,
            rival_publication.branch_plan().before.generation,
            rival_publication.branch_plan().before.head,
            rival_publication.history_event_id(),
        ),
        Err(choregraph_history::HistoryError::StaleGeneration { .. })
    ));
    let mut rejected = crate::suite::selection::exact_durable_evidence(&rival_publication);
    rejected.durable_branch_update = false;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &rival,
            &rival_publication,
            &rejected
        ),
        Ok(false)
    );
    let fresh = history.current()?;
    assert_eq!(
        publish_against(
            &history,
            &current,
            fresh.binding().clone(),
            crate::suite::selection::event(&rival)
        ),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    let replanned = rival_selection(&fresh)?;
    assert_eq!(
        replanned.ordinal,
        rival
            .ordinal
            .checked_add(crate::suite::constants::BRANCH_EVENT_INCREMENT)
            .ok_or_else(|| String::from("ordinal must stay in range"))?
    );
    assert_ne!(replanned.operation_id, rival.operation_id);
    assert_ne!(replanned.operation_id, winner.operation_id);
    assert_eq!(
        replanned.publication_plan.expected_generation,
        winner_publication.branch_plan().after.generation
    );
    let (_history, replanned_publication) =
        history.append(crate::suite::selection::event(&replanned))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &replanned,
            &replanned_publication,
            &crate::suite::selection::exact_durable_evidence(&replanned_publication),
        ),
        Ok(true)
    );
    Ok(())
}

#[test]
fn forged_after_head_evidence_cannot_enable_execution() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let selection = crate::suite::selection::select_from_current(&history.current()?)?;
    let (_history, publication) = history.append(crate::suite::selection::event(&selection))?;
    let evidence = crate::suite::selection::exact_durable_evidence(&publication);
    let mut forged_head = evidence.clone();
    forged_head.branch_after.head = publication.branch_plan().before.head;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &forged_head
        ),
        Err(campaign_choregraph::AdapterError::PublicationEvidenceMismatch)
    );
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &evidence
        ),
        Ok(true)
    );
    Ok(())
}

#[test]
// r[verify campaign.publication_fence.valid]
fn only_durable_event_and_branch_evidence_enables_execution() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let selection = crate::suite::selection::select_from_current(&history.current()?)?;
    let (_history, publication) = history.append(crate::suite::selection::event(&selection))?;
    let durable = crate::suite::selection::exact_durable_evidence(&publication);
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &durable
        ),
        Ok(true)
    );
    let mut event_not_durable = durable;
    event_not_durable.durable_event = false;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &event_not_durable
        ),
        Ok(false)
    );
    Ok(())
}
