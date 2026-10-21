#[test]
// r[verify campaign.history_projection.valid]
fn current_published_history_can_plan_the_next_selection() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    assert_ne!(
        current.binding().generation,
        current.state().admission.projection.generation
    );
    let decision = crate::suite::identity::observed(
        campaign_choregraph::plan_next_from_published_projection(
            &current,
            campaign_choregraph::PublishedSelectionRequest {
                request: crate::suite::selection::request_for(&current)?,
            },
        ),
    )?;
    let campaign_core::NextDecision::Select(selection) = decision.decision else {
        return Err(String::from(
            "the current eligible candidate must produce a selection",
        ));
    };
    assert_eq!(
        selection.publication_plan.expected_generation,
        current.binding().generation
    );
    assert_eq!(
        selection.publication_plan.expected_head_id,
        current.binding().head_id
    );
    assert_eq!(
        selection.publication_plan.control_branch_id,
        current.binding().control_branch_id
    );
    Ok(())
}
#[test]
fn stale_published_history_cannot_plan_a_selection() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let valid_request = crate::suite::selection::request_for(&current)?;
    let plan = campaign_choregraph::plan_next_from_published_projection(
        &current,
        campaign_choregraph::PublishedSelectionRequest {
            request: valid_request.clone(),
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    assert!(matches!(
        plan.decision,
        campaign_core::NextDecision::Select(_)
    ));
    let mut stale_request = valid_request;
    stale_request.expected_generation = 0;
    let before = current.state().clone();
    assert_eq!(
        campaign_choregraph::plan_next_from_published_projection(
            &current,
            campaign_choregraph::PublishedSelectionRequest {
                request: stale_request,
            },
        ),
        Err(campaign_choregraph::AdapterError::ProjectionBindingMismatch)
    );
    assert_eq!(*current.state(), before);
    Ok(())
}
