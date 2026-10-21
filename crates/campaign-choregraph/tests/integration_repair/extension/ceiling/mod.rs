//! A campaign admitted one generation below `u64::MAX`: the generation range is a typed capacity
//! bound, never a wrap.

/// A campaign admitted on a branch bound one generation below `u64::MAX`, published at
/// `u64::MAX`, and its admission event.
fn admitted_at_saturation()
-> Result<(crate::suite::history::Fixture, choregraph_history::EventId), String> {
    let history = crate::suite::admission::initialized()?;
    let campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. } =
        history.payloads[0].control_event.clone()
    else {
        return Err(String::from("admission"));
    };
    let exploration = crate::suite::admission::exploration_root()?;
    let before = choregraph_history::BranchRef {
        name: history.branch.name.clone(),
        head: exploration.event,
        generation: u64::MAX.saturating_sub(1),
    };
    let projection =
        crate::suite::identity::observed(campaign_choregraph::derive_projection_binding(
            &exploration.graph,
            &before,
            crate::suite::identity::schema(),
            crate::suite::identity::adapter(),
            crate::suite::identity::domain(),
        ))?;
    let mut high = admission.clone();
    high.campaign_id = campaign_core::Id::absent();
    high.roots[0].projection_id = projection.projection_id;
    high.projection = projection;
    let event = crate::suite::identity::observed(campaign_core::admit_campaign(high))?;
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &event,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        None,
    ))?;
    let root = crate::suite::identity::observed(exploration.graph.admit(draft))?;
    let high_history = crate::suite::history::Fixture {
        graph: root.graph,
        branch: choregraph_history::BranchRef {
            name: history.branch.name.clone(),
            head: root.event,
            generation: u64::MAX,
        },
        payloads: vec![campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: root.event,
            control_event: event,
        }],
    };
    assert_eq!(high_history.branch.generation, u64::MAX);
    assert_eq!(high_history.payloads.len(), 1);
    Ok((high_history, root.event))
}

/// An admission bound one generation below `u64::MAX` and drafted outside `admit_control_root`
/// (which refuses it) validates, cannot publish (the adapter refuses the generation the stop
/// would need), the planner refuses the same way instead of planning that stop, and a crafted
/// longer lineage at the saturated generation is a typed error on cold start and advance, never
/// a wrap or panic.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn generation_saturation_is_a_typed_error_never_a_wrap() -> Result<(), String> {
    let (high_history, root_event) = admitted_at_saturation()?;
    let view = high_history.current()?;
    assert_eq!(high_history.advance(&view), Ok(view.clone()));
    let stop = crate::suite::integrity::consumer_stop(view.state(), b"lineage-overflow-stop")?;
    assert_eq!(
        high_history.publish_from(&view, stop.clone()),
        Err(campaign_choregraph::AdapterError::GenerationCapacityExhausted)
    );
    assert_eq!(
        campaign_choregraph::plan_next_from_published_projection(
            &view,
            campaign_choregraph::PublishedSelectionRequest {
                request: crate::suite::selection::request_for(&view)?,
            },
        ),
        Err(campaign_choregraph::AdapterError::GenerationCapacityExhausted)
    );
    // A stop admitted directly as the linear child and presented at the saturated generation.
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &stop,
        crate::suite::identity::schema(),
        crate::suite::identity::adapter(),
        crate::suite::identity::domain(),
        Some(root_event),
    ))?;
    let child = crate::suite::identity::observed(high_history.graph.admit(draft))?;
    let mut payloads = high_history.payloads.clone();
    payloads.push(campaign_choregraph::ControlPayloadEnvelope {
        history_event_id: child.event,
        control_event: stop,
    });
    let crafted = crate::suite::history::Fixture {
        graph: child.graph,
        branch: choregraph_history::BranchRef {
            head: child.event,
            ..high_history.branch.clone()
        },
        payloads,
    };
    let cold = crafted.validate();
    let advanced = crafted.advance(&view);
    assert_eq!(
        cold,
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::ArithmeticOverflow
        ))
    );
    assert_eq!(cold, advanced);
    Ok(())
}
