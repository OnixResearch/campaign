//! Stateful fast-forward: a host holding a validated projection advances only onto a lineage that
//! extends it, and a recorded selection's fence is the branch value at its lineage position.

mod ceiling;
mod host_paths;
mod lineage;
mod sibling;

/// Moves `history`'s branch to `head` in `graph` with one generation-fenced compare-and-set, as a
/// host would, presenting `payloads` as the new lineage.
fn moved_to(
    history: &crate::suite::history::Fixture,
    graph: choregraph_history::HistoryGraph,
    head: choregraph_history::EventId,
    payloads: Vec<campaign_choregraph::ControlPayloadEnvelope>,
) -> Result<crate::suite::history::Fixture, String> {
    let moved = crate::suite::identity::observed(choregraph_history::plan_branch_advance(
        &graph,
        &history.branch,
        history.branch.generation,
        history.branch.head,
        head,
    ))?;
    Ok(crate::suite::history::Fixture {
        graph,
        branch: moved.after,
        payloads,
    })
}

#[test]
fn an_honest_multi_step_advance_equals_the_cold_start_projection() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let previous = history.current()?;
    let selection = crate::suite::selection::select_from_current(&previous)?;
    let (selected, _p) = history.append(crate::suite::selection::event(&selection))?;
    let selected_view = selected.advance(&previous).map_err(|e| format!("{e:?}"))?;
    assert_eq!(selected_view, selected.current()?);
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        selected_view.state(),
        crate::suite::selection::result_for(&selected_view, &selection),
    ))?;
    let (settled, _p) = selected.clone().append(result)?;
    let stop =
        crate::suite::integrity::consumer_stop(settled.current()?.state(), b"advance-honest-stop")?;
    let (stopped, _p) = settled.clone().append(stop)?;

    let advanced = crate::suite::identity::observed(stopped.advance(&previous))?;
    assert_eq!(advanced, stopped.current()?);
    assert!(advanced.state().stopped.is_some());
    assert_eq!(
        advanced
            .control_events()
            .get(..previous.control_events().len()),
        Some(previous.control_events())
    );
    let stepwise = crate::suite::identity::observed(settled.advance(&selected_view)).and_then(
        |settled_view| crate::suite::identity::observed(stopped.advance(&settled_view)),
    )?;
    assert_eq!(stepwise, advanced);
    assert_eq!(stopped.advance(&advanced), Ok(advanced.clone()));
    Ok(())
}

/// A renamed branch is refused as another branch, before the recorded selection fences (which
/// name the admission's branch) are compared.
#[test]
fn a_renamed_branch_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let previous = history.current()?;
    let selection = crate::suite::selection::select_from_current(&previous)?;
    let (selected, _p) = history.append(crate::suite::selection::event(&selection))?;
    let renamed = crate::suite::history::Fixture {
        branch: choregraph_history::BranchRef {
            name: String::from("foreign-control"),
            ..selected.branch.clone()
        },
        ..selected.clone()
    };
    assert_eq!(
        renamed.validate(),
        Err(campaign_choregraph::AdapterError::ControlBranchMismatch)
    );
    assert_eq!(
        renamed.advance(&previous),
        Err(campaign_choregraph::AdapterError::ControlBranchMismatch)
    );
    let stop =
        crate::suite::integrity::consumer_stop(selected.current()?.state(), b"renamed-stop")?;
    assert_eq!(
        renamed.publish_from(&previous, stop.clone()),
        Err(campaign_choregraph::AdapterError::ControlBranchMismatch)
    );
    selected
        .publish_from(&previous, stop)
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

#[test]
// r[verify campaign.history_projection.stale]
fn a_rewound_branch_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let before_stop = history.current()?;
    let pre_stop_head = history.branch.head;
    let stop = crate::suite::integrity::consumer_stop(before_stop.state(), b"rewind-stop")?;
    let (stopped, _p) = history.append(stop)?;
    let stopped_view = stopped.current()?;
    let mut payloads = stopped.payloads.clone();
    payloads.pop();
    let rewound = moved_to(&stopped, stopped.graph.clone(), pre_stop_head, payloads)?;
    assert!(rewound.branch.generation > stopped.branch.generation);
    assert_eq!(
        rewound.advance(&stopped_view),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let reopened = crate::suite::selection::select_from_current(&before_stop)?;
    assert_eq!(
        rewound.publish_from(&stopped_view, crate::suite::selection::event(&reopened)),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert!(stopped_view.state().stopped.is_some());
    Ok(())
}

#[test]
fn a_lagging_or_rewritten_payload_list_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let previous = history.current()?;
    let selection = crate::suite::selection::select_from_current(&previous)?;
    let (selected, _p) = history
        .clone()
        .append(crate::suite::selection::event(&selection))?;
    let lagging = crate::suite::history::Fixture {
        payloads: history.payloads.clone(),
        ..selected.clone()
    };
    assert_eq!(
        lagging.advance(&previous),
        Err(campaign_choregraph::AdapterError::ProjectionPayloadMismatch)
    );
    let mut rewritten = selected.clone();
    let accepted = rewritten
        .payloads
        .get(1)
        .map(|payload| payload.control_event.clone())
        .ok_or_else(|| String::from("an accepted event after the admission"))?;
    let first = rewritten
        .payloads
        .first_mut()
        .ok_or_else(|| String::from("the admission payload"))?;
    first.control_event = accepted;
    assert_eq!(
        rewritten.advance(&previous),
        Err(campaign_choregraph::AdapterError::PayloadIdentityMismatch)
    );
    assert_eq!(selected.advance(&previous), Ok(selected.current()?));
    Ok(())
}

/// Admits `selection`'s event directly as the linear child of the head and advances the branch
/// onto it, bypassing the publication planner's fence check.
fn injected(
    history: &crate::suite::history::Fixture,
    selection: &campaign_core::SelectionDecision,
) -> Result<crate::suite::history::Fixture, String> {
    let event = crate::suite::selection::event(selection);
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &event,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        Some(history.branch.head),
    ))?;
    let admission = crate::suite::identity::observed(history.graph.admit(draft))?;
    let mut payloads = history.payloads.clone();
    payloads.push(campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: admission.event,
        control_event: event,
    });
    moved_to(history, admission.graph, admission.event, payloads)
}

#[test]
fn a_recorded_selection_fence_is_the_branch_value_at_its_lineage_position() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let honest = crate::suite::selection::select_from_current(&current)?;
    let admission_fence = current.state().admission.projection.clone();
    let mut foreign = honest.clone();
    foreign.publication_plan.control_branch_id = admission_fence.control_branch_id;
    foreign.publication_plan.expected_head_id = admission_fence.head_id;
    foreign.publication_plan.expected_generation = admission_fence.generation;
    let foreign = crate::suite::identity::observed(foreign.derive())?;
    assert_eq!(
        history.publish(crate::suite::selection::event(&foreign)),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    let forged = injected(&history, &foreign)?;
    assert_eq!(
        forged.validate(),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    assert_eq!(
        forged.advance(&current),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    let recorded = injected(&history, &honest)?;
    assert_eq!(
        recorded.validate().map(|view| view.state().selection_count),
        Ok(1)
    );
    assert_eq!(recorded.advance(&current), recorded.validate());
    Ok(())
}
