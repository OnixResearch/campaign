#[test]
fn second_round_selection_uses_the_current_published_fence() -> Result<(), String> {
    let mut history = crate::suite::candidate::selectable()?;
    let first_current = history.current()?;
    let first = crate::suite::selection::select_from_current(&first_current)?;
    assert_eq!(first.ordinal, crate::suite::constants::FIRST_ORDINAL);
    assert_eq!(
        first.publication_plan.expected_generation,
        first_current.binding().generation
    );
    let (next_history, selection_publication) =
        history.append(crate::suite::selection::event(&first))?;
    history = next_history;
    let evidence = crate::suite::selection::exact_durable_evidence(&selection_publication);
    assert!(crate::suite::identity::observed(
        campaign_choregraph::can_execute_selection_after_publication(
            &first,
            &selection_publication,
            &evidence,
        )
    )?);
    let selected_current = history.current()?;
    assert!(
        selected_current
            .state()
            .outstanding
            .iter()
            .any(|selection| selection.selection_id == first.selection_id)
    );
    let result_event = crate::suite::identity::observed(campaign_core::admit_result(
        selected_current.state(),
        crate::suite::selection::result_for(&selected_current, &first),
    ))?;
    history = history.append(result_event)?.0;
    let settled_current = history.current()?;
    assert!(settled_current.state().outstanding.is_empty());
    let second = crate::suite::selection::select_from_current(&settled_current)?;
    assert_eq!(second.ordinal, crate::suite::constants::SECOND_ORDINAL);
    assert_ne!(first.selection_id, second.selection_id);
    assert_eq!(
        second.publication_plan.expected_generation,
        settled_current.binding().generation
    );
    assert_eq!(
        second.publication_plan.expected_head_id,
        settled_current.binding().head_id
    );
    assert_eq!(
        second.publication_plan.control_branch_id,
        settled_current.binding().control_branch_id
    );
    Ok(())
}
#[test]
// r[verify campaign.history_projection.stale]
fn stale_generation_or_head_cannot_plan_from_published_history() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let mut stale_generation = crate::suite::selection::request_for(&current)?;
    stale_generation.expected_generation = current
        .binding()
        .generation
        .checked_sub(crate::suite::constants::BRANCH_EVENT_INCREMENT)
        .ok_or_else(|| String::from("fixture generation must permit a stale value"))?;
    assert!(matches!(
        campaign_choregraph::plan_next_from_published_projection(
            &current,
            campaign_choregraph::PublishedSelectionRequest {
                request: stale_generation,
            },
        ),
        Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
    ));
    let mut stale_head = crate::suite::selection::request_for(&current)?;
    stale_head.expected_head_id =
        crate::suite::identity::derive(campaign_core::IdKind::Head, b"repair-forged-head");
    assert!(matches!(
        campaign_choregraph::plan_next_from_published_projection(
            &current,
            campaign_choregraph::PublishedSelectionRequest {
                request: stale_head,
            },
        ),
        Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
    ));
    Ok(())
}
#[test]
fn forged_evidence_cannot_enable_execution() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let selection = crate::suite::selection::select_from_current(&current)?;
    let (_history, publication) = history.append(crate::suite::selection::event(&selection))?;
    let evidence = crate::suite::selection::exact_durable_evidence(&publication);
    assert!(crate::suite::identity::observed(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &evidence,
        )
    )?);
    let mut forged_event = evidence.clone();
    forged_event.history_event_id =
        choregraph_history::EventId::new(choregraph_history::Digest::from_bytes(
            crate::suite::identity::derive(
                campaign_core::IdKind::EventPayload,
                b"repair-forged-publication",
            )
            .digest,
        ));
    assert!(matches!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &forged_event
        ),
        Err(campaign_choregraph::AdapterError::PublicationEvidenceMismatch)
    ));
    let mut forged_generation = evidence;
    forged_generation.branch_after.generation = forged_generation
        .branch_after
        .generation
        .checked_add(crate::suite::constants::BRANCH_EVENT_INCREMENT)
        .ok_or_else(|| String::from("fixture generation must stay in range"))?;
    assert!(matches!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &forged_generation
        ),
        Err(campaign_choregraph::AdapterError::PublicationEvidenceMismatch)
    ));
    Ok(())
}
#[test]
// r[verify campaign.publication_fence.stale]
fn a_concurrent_planner_is_rejected_by_the_branch_fence() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let winner = crate::suite::selection::select_from_current(&current)?;
    let loser = crate::suite::selection::select_from_current(&current)?;
    let (history, publication) = history.append(crate::suite::selection::event(&winner))?;
    let fresh = history.current()?;
    assert_eq!(
        campaign_choregraph::plan_control_event_publication(
            campaign_choregraph::ControlPublicationRequest {
                current: history.projection_request(fresh.binding().clone()),
                previous: &current,
                control_event: crate::suite::selection::event(&loser),
            }
        ),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    let replanned = crate::suite::selection::select_from_current(&fresh)?;
    assert_eq!(replanned.ordinal, crate::suite::constants::SECOND_ORDINAL);
    assert_ne!(replanned.operation_id, winner.operation_id);
    assert_eq!(
        replanned.publication_plan.expected_head_id,
        fresh.binding().head_id
    );
    let mut non_durable = crate::suite::selection::exact_durable_evidence(&publication);
    non_durable.durable_branch_update = false;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &winner,
            &publication,
            &non_durable
        ),
        Ok(false)
    );
    Ok(())
}

/// A selection planned under a `ReferenceEvidence` profile (the readable reference planner's
/// answer) is refused at adapter publication with `ReferencePolicySelected` before any draft is
/// admitted, while the production selection it mirrors publishes.
#[test]
// r[verify campaign.fd.oracle.isolated]
fn a_reference_planned_selection_is_refused_by_the_adapter() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let production = crate::suite::selection::select_from_current(&current)?;
    let campaign_core::CampaignControlEvent::SelectionPublished(published) =
        crate::suite::selection::event(&production)
    else {
        return Err(String::from("a selection publishes a selection event"));
    };
    let mut profile = *published.profile;
    profile.implementation = campaign_core::PolicyImplementation::ReferenceEvidence {
        reference_id: crate::suite::identity::derive(
            campaign_core::IdKind::Policy,
            b"repair-reference-policy",
        ),
    };
    let profile = crate::suite::identity::observed(profile.derive())?;
    let reference = crate::suite::identity::observed(
        campaign_core::SelectionDecision {
            profile_id: profile.profile_id,
            ..production.clone()
        }
        .derive(),
    )?;
    let event =
        campaign_core::CampaignControlEvent::SelectionPublished(crate::suite::identity::observed(
            campaign_core::SelectionEvent::new(reference, profile, None),
        )?);
    assert_eq!(
        history.publish_from(&current, event),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::ReferencePolicySelected
        ))
    );
    assert_eq!(history.current()?, current);
    let publication = crate::suite::identity::observed(
        history.publish_from(&current, crate::suite::selection::event(&production)),
    )?;
    assert_eq!(
        publication.control_event(),
        &crate::suite::selection::event(&production)
    );
    Ok(())
}
