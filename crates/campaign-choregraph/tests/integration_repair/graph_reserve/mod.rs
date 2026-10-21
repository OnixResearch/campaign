//! Graph reserve: the Choregraph graph holding a control lineage is shared with exploration and
//! other writers, and its own event limit keeps room for every outstanding settlement and the
//! stop in publication and in planning.

fn adapter_plan(
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_choregraph::AdapterResult<campaign_core::DecisionPlan>, String> {
    Ok(campaign_choregraph::plan_next_from_published_projection(
        held,
        campaign_choregraph::PublishedSelectionRequest {
            request: crate::suite::selection::request_for(held)?,
        },
    ))
}

fn capacity_reason() -> campaign_core::StopReason {
    campaign_core::StopReason::ConsumerRequested {
        reason_id: campaign_choregraph::control_capacity_reason_id(),
    }
}

/// `hosted` after another writer admitted `count` exploration events to its graph.
pub(crate) fn after_foreign_writes(
    hosted: crate::suite::configured::Hosted,
    count: u32,
) -> Result<crate::suite::configured::Hosted, String> {
    let before = hosted.graph.len();
    let graph = (0..count).try_fold(hosted.graph.clone(), |graph, index| {
        let draft = choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            choregraph_history::Digest::from_bytes(
                crate::suite::identity::derive(
                    campaign_core::IdKind::EventPayload,
                    &index.to_le_bytes(),
                )
                .digest,
            ),
            Vec::new(),
            choregraph_history::Digest::from_bytes(crate::suite::identity::schema().digest),
        );
        crate::suite::identity::observed(graph.admit(draft)).map(|admission| admission.graph)
    })?;
    assert_eq!(
        graph.len(),
        before.checked_add(count).ok_or("graph length overflow")?
    );
    assert!(graph.contains(hosted.branch.head));
    Ok(crate::suite::configured::Hosted { graph, ..hosted })
}

/// Under the suite's standard host configuration the exploration root shares the graph with the
/// control lineage, so the graph limit binds before the projection bound: every selection is
/// granted and settles, and where no further selection, settlement, and stop fit the graph the
/// planner plans the capacity stop, which publishes. The validated projection records the graph
/// length and limit it plans against.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn the_planner_stops_within_shared_history_under_standard_limits() -> Result<(), String> {
    let mut hosted = crate::suite::configured::campaign::guided(
        crate::suite::configured::campaign::admitted_at(
            crate::suite::constants::ROOT_GENERATION,
            crate::suite::constants::MAX_EVENTS,
        )?,
        1,
    )?;
    let mut rounds = 0_u32;
    let (held, stop) = loop {
        let held = crate::suite::identity::observed(hosted.validate())?;
        assert_eq!(held.graph_events(), hosted.graph.len());
        assert_eq!(held.graph_max_events(), crate::suite::constants::MAX_EVENTS);
        let plan = crate::suite::identity::observed(adapter_plan(&held)?)?;
        match plan.decision {
            campaign_core::NextDecision::Select(selection) => {
                let publication = crate::suite::identity::observed(
                    hosted.publish(&held, crate::suite::selection::event(&selection)),
                )?;
                assert_eq!(
                    campaign_choregraph::can_execute_selection_after_publication(
                        &selection,
                        &publication,
                        &crate::suite::selection::exact_durable_evidence(&publication),
                    ),
                    Ok(true)
                );
                let next = hosted.committed(&publication);
                let granted = crate::suite::identity::observed(next.validate())?;
                let mut observation = crate::suite::selection::result_for(&granted, &selection);
                observation.payload_id = campaign_core::derive_id(
                    campaign_core::IdKind::EventPayload,
                    b"graph-reserve-result",
                    &[selection.selection_id],
                );
                let result = crate::suite::identity::observed(campaign_core::admit_result(
                    granted.state(),
                    observation,
                ))?;
                hosted = next.append(result)?.0;
                rounds = rounds
                    .checked_add(1)
                    .ok_or_else(|| String::from("bounded rounds"))?;
            }
            campaign_core::NextDecision::Stop(stop) => break (held, *stop),
            campaign_core::NextDecision::Wait(wait) => {
                return Err(format!("unexpected wait {wait:?}"));
            }
        }
    };
    assert_eq!(rounds, 5);
    assert_eq!(stop.reason, capacity_reason());
    assert!(held.state().outstanding.is_empty());
    let event = crate::suite::identity::observed(campaign_core::admit_stop(
        held.state(),
        stop.reason.clone(),
    ))?;
    let (stopped, _) = hosted.append(event)?;
    let stopped_held = crate::suite::identity::observed(stopped.validate())?;
    assert!(stopped_held.state().stopped.is_some());
    assert_eq!(stopped.graph.len(), 15);
    Ok(())
}

/// Another writer that consumes the graph reserve leaves no room for the stop: the planner then
/// refuses with `GraphCapacityExhausted` instead of planning a stop publication refuses, and
/// publication refuses the stop the same way. One event less and the capacity stop is planned and
/// publishes as the graph's last event.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn a_foreign_writer_consuming_the_stop_room_refuses_planning_and_publication() -> Result<(), String>
{
    let hosted = crate::suite::configured::campaign::guided(
        crate::suite::configured::campaign::admitted_at(
            crate::suite::constants::ROOT_GENERATION,
            crate::suite::constants::MAX_EVENTS,
        )?,
        1,
    )?;
    let room = crate::suite::constants::MAX_EVENTS
        .checked_sub(hosted.graph.len())
        .ok_or_else(|| String::from("the guided graph fits its limit"))?;
    let full = after_foreign_writes(hosted.clone(), room)?;
    let held = crate::suite::identity::observed(full.validate())?;
    assert_eq!(held.graph_events(), crate::suite::constants::MAX_EVENTS);
    assert_eq!(
        adapter_plan(&held)?,
        Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
    );
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"graph-reserve-stop")?;
    assert_eq!(
        full.publish(&held, stop).map(|_| ()),
        Err(campaign_choregraph::AdapterError::GraphCapacityExhausted)
    );
    let almost = after_foreign_writes(
        hosted,
        room.checked_sub(1)
            .ok_or_else(|| String::from("room for the stop"))?,
    )?;
    let held = crate::suite::identity::observed(almost.validate())?;
    let plan = crate::suite::identity::observed(adapter_plan(&held)?)?;
    let campaign_core::NextDecision::Stop(stop) = plan.decision else {
        return Err(format!("expected the capacity stop, planned {plan:?}"));
    };
    assert_eq!(stop.reason, capacity_reason());
    let event = crate::suite::identity::observed(campaign_core::admit_stop(
        held.state(),
        stop.reason.clone(),
    ))?;
    let (stopped, _) = almost.append(event)?;
    assert_eq!(stopped.graph.len(), crate::suite::constants::MAX_EVENTS);
    Ok(())
}
