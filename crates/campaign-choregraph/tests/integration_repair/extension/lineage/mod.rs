//! Lineages that never extend a held projection: another campaign's lineage under the same
//! branch name (refused outright on a shared graph), a control event merged with a foreign
//! parent, and a foreign event spliced onto the branch.

/// Another campaign's admission under the suite's branch name and host identities (another
/// budget profile), and its root draft.
fn other_admission(
    history: &crate::suite::history::Fixture,
) -> Result<
    (
        campaign_core::CampaignControlEvent,
        choregraph_history::EventDraft,
    ),
    String,
> {
    let campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. } =
        history.payloads[0].control_event.clone()
    else {
        return Err(String::from("admission"));
    };
    let mut other = admission.clone();
    other.campaign_id = campaign_core::Id::absent();
    other.budget_profile_id =
        crate::suite::identity::derive(campaign_core::IdKind::BudgetProfile, b"lineage-other");
    let other_event = crate::suite::identity::observed(campaign_core::admit_campaign(other))?;
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &other_event,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        None,
    ))?;
    assert_ne!(other_event, history.payloads[0].control_event);
    assert!(draft.parents.is_empty());
    Ok((other_event, draft))
}

/// A projection of another campaign admitted under the same branch name and host identities
/// never advances onto this campaign's lineage, nor this one onto it. On one graph the second
/// control root is refused outright (one Campaign campaign per graph and envelope), naming the
/// other campaign's root, cold and on advance.
#[test]
// r[verify campaign.initial_admission.one_per_graph]
fn another_campaigns_projection_on_the_same_branch_does_not_advance() -> Result<(), String> {
    let history = crate::suite::admission::initialized()?;
    let (other_event, draft) = other_admission(&history)?;
    let shared = crate::suite::identity::observed(history.graph.admit(draft.clone()))?;
    let this_on_shared = crate::suite::history::Fixture {
        graph: shared.graph.clone(),
        ..history.clone()
    };
    let second_root = Err(campaign_choregraph::AdapterError::SharedControlGraph {
        event_id: shared.event,
    });
    assert_eq!(this_on_shared.validate(), second_root);
    assert_eq!(this_on_shared.advance(&history.current()?), second_root);
    let payloads = vec![campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: shared.event,
        control_event: other_event,
    }];
    let other_on_shared = crate::suite::history::Fixture {
        graph: shared.graph,
        branch: choregraph_history::BranchRef {
            head: shared.event,
            ..history.branch.clone()
        },
        payloads: payloads.clone(),
    };
    assert_eq!(
        other_on_shared.validate(),
        Err(campaign_choregraph::AdapterError::SharedControlGraph {
            event_id: history.branch.head,
        })
    );
    let own = crate::suite::identity::observed(
        crate::suite::admission::exploration_root()?
            .graph
            .admit(draft),
    )?;
    assert_eq!(own.event, shared.event);
    let other_history = crate::suite::history::Fixture {
        graph: own.graph,
        branch: other_on_shared.branch,
        payloads,
    };
    let other_view = other_history.current()?;
    let stop_a = crate::suite::integrity::consumer_stop(history.current()?.state(), b"lineage-a")?;
    let (a2, _p) = history.clone().append(stop_a)?;
    let stop_b = crate::suite::integrity::consumer_stop(other_view.state(), b"lineage-b")?;
    let (b2, _p) = other_history.append(stop_b)?;
    let a2_view = a2.current()?;
    assert_ne!(
        a2_view.state().admission.campaign_id,
        b2.current()?.state().admission.campaign_id
    );
    assert_eq!(
        b2.advance(&a2_view),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert_eq!(
        b2.advance(&history.current()?),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    Ok(())
}

/// A control event carrying a second (non-control) parent is refused on advance and cold
/// start, with or without an explicit parent choice selecting its control parent.
#[test]
fn a_merged_control_event_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let held = history.current()?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"lineage-merge-stop")?;
    let orphan =
        crate::suite::identity::observed(history.graph.admit(choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            choregraph_history::Digest::derive(b"lineage", b"orphan"),
            Vec::new(),
            choregraph_history::Digest::derive(b"lineage", b"schema"),
        )))?;
    let mut draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &stop,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        Some(history.branch.head),
    ))?;
    draft.parents.push(choregraph_history::ParentLink {
        role: String::from("lineage-foreign"),
        event: orphan.event,
    });
    draft.parents.sort();
    let merged = crate::suite::identity::observed(orphan.graph.admit(draft))?;
    let mut payloads = history.payloads.clone();
    payloads.push(campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: merged.event,
        control_event: stop,
    });
    let moved = super::moved_to(&history, merged.graph, merged.event, payloads)?;
    let ambiguous = moved.advance(&held);
    assert!(matches!(
        ambiguous,
        Err(campaign_choregraph::AdapterError::History(
            choregraph_history::HistoryError::AmbiguousLineage { .. }
        ))
    ));
    let mut choices = choregraph_history::ParentChoices::new();
    choices.insert(merged.event, history.branch.head);
    let binding = crate::suite::identity::observed(moved.derived_binding())?;
    let request = campaign_choregraph::ControlProjectionRequest {
        graph: &moved.graph,
        branch: &moved.branch,
        parent_choices: &choices,
        maximum_events: crate::suite::constants::MAX_EVENTS,
        expected_binding: binding,
        payloads: &moved.payloads,
    };
    let chosen = campaign_choregraph::validate_control_advance(&held, &request);
    assert_eq!(
        chosen,
        Err(campaign_choregraph::AdapterError::ParentLineageMismatch)
    );
    assert_eq!(
        campaign_choregraph::validate_control_projection(&request),
        Err(campaign_choregraph::AdapterError::ParentLineageMismatch)
    );
    Ok(())
}

/// A non-control event spliced between the held head and a new control event is refused on
/// advance and on cold start, whichever control payload claims it.
#[test]
fn a_foreign_event_spliced_onto_the_branch_does_not_advance() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let held = history.current()?;
    let foreign =
        crate::suite::identity::observed(history.graph.admit(choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            choregraph_history::Digest::derive(b"lineage", b"foreign"),
            vec![choregraph_history::ParentLink {
                role: String::from("campaign-control-parent"),
                event: history.branch.head,
            }],
            choregraph_history::Digest::from_bytes(crate::suite::identity::schema().digest),
        )))?;
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"lineage-foreign-stop")?;
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &stop,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        Some(foreign.event),
    ))?;
    let after = crate::suite::identity::observed(foreign.graph.admit(draft))?;
    for claimed in [stop.clone(), history.payloads[0].control_event.clone()] {
        let mut payloads = history.payloads.clone();
        payloads.push(campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: foreign.event,
            control_event: claimed,
        });
        payloads.push(campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: after.event,
            control_event: stop.clone(),
        });
        let moved = crate::suite::history::Fixture {
            graph: after.graph.clone(),
            branch: choregraph_history::BranchRef {
                head: after.event,
                generation: history.branch.generation.saturating_add(2),
                ..history.branch.clone()
            },
            payloads,
        };
        assert_eq!(
            moved.advance(&held),
            Err(campaign_choregraph::AdapterError::PayloadIdentityMismatch)
        );
        assert_eq!(
            moved.validate(),
            Err(campaign_choregraph::AdapterError::PayloadIdentityMismatch)
        );
    }
    Ok(())
}
