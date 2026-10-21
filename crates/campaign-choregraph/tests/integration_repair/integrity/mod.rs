//! Publication integrity: a control event is only published when it applies to the current
//! Campaign state, and execution is only granted for the exact published decision.

fn fresh_binding(
    history: &crate::suite::history::Fixture,
) -> Result<campaign_core::ProjectionBinding, String> {
    crate::suite::identity::observed(history.derived_binding())
}

/// A planner that refreshes only the branch fence, but keeps its stale Campaign state.
fn stale_selection(
    stale: &campaign_choregraph::ValidatedControlProjection,
    fresh: &campaign_core::ProjectionBinding,
) -> Result<campaign_core::SelectionDecision, String> {
    let mut request = crate::suite::selection::request_for(stale)?;
    request.expected_generation = fresh.generation;
    request.expected_head_id = fresh.head_id;
    let plan =
        crate::suite::identity::observed(campaign_core::plan_next(stale.state(), request, fresh))?;
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Err(String::from("stale planner must select"));
    };
    Ok(*selection)
}

pub(crate) fn consumer_stop(
    state: &campaign_core::FrontierState,
    label: &[u8],
) -> Result<campaign_core::CampaignControlEvent, String> {
    crate::suite::identity::observed(campaign_core::admit_stop(
        state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::identity::derive(campaign_core::IdKind::Reason, label),
        },
    ))
}

#[test]
fn a_stale_state_selection_under_a_refreshed_fence_cannot_publish() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let stale = history.current()?;
    let winner = crate::suite::selection::select_from_current(&stale)?;
    let (history, winner_publication) = history.append(crate::suite::selection::event(&winner))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &winner,
            &winner_publication,
            &crate::suite::selection::exact_durable_evidence(&winner_publication),
        ),
        Ok(true)
    );
    let rival = stale_selection(&stale, &fresh_binding(&history)?)?;
    assert_eq!(rival.ordinal, winner.ordinal);
    assert_eq!(rival.candidate_id, winner.candidate_id);
    assert_ne!(rival.operation_id, winner.operation_id);
    let before = history.current()?;
    assert_eq!(
        history.publish(crate::suite::selection::event(&rival)),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::StaleState
        ))
    );
    assert_eq!(history.current()?, before);
    assert_eq!(before.state().outstanding.len(), 1);
    Ok(())
}

#[test]
// r[verify campaign.history_projection.caller_binding]
// r[verify campaign.history_projection.stale]
fn a_stale_projection_cannot_plan_at_a_refreshed_fence() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let stale = history.current()?;
    let winner = crate::suite::selection::select_from_current(&stale)?;
    let (history, _publication) = history.append(crate::suite::selection::event(&winner))?;
    let fresh = history.current()?;
    let mut request = crate::suite::selection::request_for(&stale)?;
    request.expected_generation = fresh.binding().generation;
    request.expected_head_id = fresh.binding().head_id;
    assert_eq!(
        campaign_choregraph::plan_next_from_published_projection(
            &stale,
            campaign_choregraph::PublishedSelectionRequest { request },
        ),
        Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
    );
    let replanned = crate::suite::selection::select_from_current(&fresh)?;
    assert_eq!(replanned.ordinal, crate::suite::constants::SECOND_ORDINAL);
    assert_eq!(replanned.state_before_id, fresh.state().state_id);
    Ok(())
}

#[test]
// r[verify campaign.stop.terminal]
fn a_selection_after_an_accepted_stop_cannot_publish() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let stale = history.current()?;
    let stop = consumer_stop(stale.state(), b"audit-consumer-stop")?;
    let (history, _stop_publication) = history.append(stop)?;
    let stopped = history.current()?;
    assert!(stopped.state().stopped.is_some());
    let rival = stale_selection(&stale, &fresh_binding(&history)?)?;
    assert_eq!(
        history.publish(crate::suite::selection::event(&rival)),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::CampaignStopped
        ))
    );
    assert_eq!(history.current()?, stopped);
    Ok(())
}

#[test]
fn a_retired_candidate_selection_cannot_publish() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let stale = history.current()?;
    let candidate_id = stale
        .state()
        .candidates
        .first()
        .map(|candidate| candidate.candidate.candidate_id)
        .ok_or_else(|| String::from("one candidate"))?;
    let retirement = crate::suite::identity::observed(campaign_core::retire_candidate(
        stale.state(),
        campaign_core::RetirementRequest {
            campaign_id: stale.state().admission.campaign_id,
            state_id: stale.state().state_id,
            policy_id: stale.state().admission.policy_id,
            adapter_id: stale.state().admission.adapter_id,
            candidate_id,
            reason_id: crate::suite::identity::derive(
                campaign_core::IdKind::Reason,
                b"audit-retire",
            ),
            origin: campaign_core::RetirementOrigin::Consumer,
        },
    ))?;
    let (history, _retire_publication) = history.append(retirement)?;
    let retired = history.current()?;
    let rival = stale_selection(&stale, &fresh_binding(&history)?)?;
    assert_eq!(rival.candidate_id, candidate_id);
    assert_eq!(
        history.publish(crate::suite::selection::event(&rival)),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::StaleState
        ))
    );
    assert_eq!(history.current()?, retired);
    assert!(retired.state().outstanding.is_empty());
    Ok(())
}

#[test]
fn the_execution_gate_requires_the_exact_published_decision() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let selection = crate::suite::selection::select_from_current(&history.current()?)?;
    let (_history, publication) = history.append(crate::suite::selection::event(&selection))?;
    let evidence = crate::suite::selection::exact_durable_evidence(&publication);
    let mut tampered = selection.clone();
    tampered.operation_id =
        crate::suite::identity::derive(campaign_core::IdKind::Operation, b"audit-unpublished-op");
    tampered.candidate_id =
        crate::suite::identity::derive(campaign_core::IdKind::Candidate, b"audit-other-candidate");
    tampered.ordinal = crate::suite::constants::TAMPERED_ORDINAL;
    assert_eq!(tampered.selection_id, selection.selection_id);
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &tampered,
            &publication,
            &evidence
        ),
        Err(campaign_choregraph::AdapterError::MissingControlEvent)
    );
    let rederived = crate::suite::identity::observed(tampered.derive())?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &rederived,
            &publication,
            &evidence
        ),
        Err(campaign_choregraph::AdapterError::MissingControlEvent)
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

/// A second planner that declares a different profile, so its selection is a distinct event.
fn rival_of(
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

#[test]
// r[verify campaign.publication_fence.unpublished]
fn an_unpublished_rival_cannot_execute_under_the_winners_publication() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let winner = crate::suite::selection::select_from_current(&current)?;
    let rival = rival_of(&current)?;
    assert_ne!(winner.selection_id, rival.selection_id);
    let (history, winner_publication) = history.append(crate::suite::selection::event(&winner))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &rival,
            &winner_publication,
            &crate::suite::selection::exact_durable_evidence(&winner_publication),
        ),
        Err(campaign_choregraph::AdapterError::MissingControlEvent)
    );
    assert!(
        history
            .current()?
            .state()
            .outstanding
            .iter()
            .all(|outstanding| outstanding.selection_id != rival.selection_id)
    );
    Ok(())
}
