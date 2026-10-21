//! Root reserve: a control root is admitted only with room for its own stop on every host bound,
//! and a campaign is never bound to a control branch Choregraph cannot advance.

type Limits = crate::suite::configured::campaign::root::Limits;

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

/// For each host bound in turn (the graph's projection limit, the admission generation, and the
/// graph event limit, whose graph also holds the exploration root), limits with room for the
/// control root and exactly `spare` more control events.
fn limits_with(spare: u32) -> Result<[Limits; 3], String> {
    let standard = Limits::standard(
        crate::suite::constants::ROOT_GENERATION,
        crate::suite::constants::MAX_EVENTS,
    );
    let lineage = spare
        .checked_add(1)
        .ok_or_else(|| String::from("a bounded spare"))?;
    let generation = u64::MAX
        .checked_sub(u64::from(lineage))
        .ok_or_else(|| String::from("a generation below u64::MAX"))?;
    let graph_events = lineage
        .checked_add(1)
        .ok_or_else(|| String::from("a bounded graph"))?;
    assert!(generation < u64::MAX);
    assert!(graph_events > lineage);
    Ok([
        Limits {
            projection_events: lineage,
            maximum_events: lineage,
            ..standard.clone()
        },
        Limits {
            generation,
            ..standard.clone()
        },
        Limits {
            graph_events,
            ..standard
        },
    ])
}

/// A root whose lineage leaves no room for its stop is refused with the typed capacity error of
/// the bound it exceeds, before Choregraph admits anything.
#[test]
// r[verify campaign.initial_admission.stop_reserve]
fn an_admission_without_room_for_its_stop_is_refused() -> Result<(), String> {
    let exhausted = [
        campaign_choregraph::AdapterError::ProjectionCapacityExhausted,
        campaign_choregraph::AdapterError::GenerationCapacityExhausted,
        campaign_choregraph::AdapterError::GraphCapacityExhausted,
    ];
    for (limits, exhausted) in limits_with(0)?.into_iter().zip(exhausted) {
        let unadmitted = crate::suite::configured::campaign::root::Unadmitted::bound(limits)?;
        assert_eq!(unadmitted.root(), Err(exhausted.clone()), "{exhausted:?}");
        assert_eq!(unadmitted.graph.len(), 1);
    }
    Ok(())
}

/// With exactly room for the root and its stop, the root loads, the planner plans the stop of the
/// empty frontier, and that stop (or a consumer stop) publishes as the last event the bound
/// holds.
#[test]
// r[verify campaign.initial_admission.stop_reserve]
fn an_admission_with_room_for_exactly_its_stop_publishes_the_stop() -> Result<(), String> {
    for limits in limits_with(1)? {
        let hosted = crate::suite::configured::campaign::admitted_under(limits.clone())?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let plan = crate::suite::identity::observed(adapter_plan(&held)?)?;
        let campaign_core::NextDecision::Stop(stop) = &plan.decision else {
            return Err(format!("expected the frontier stop, planned {plan:?}"));
        };
        assert_eq!(stop.reason, campaign_core::StopReason::FrontierExhausted);
        let consumer = crate::suite::integrity::consumer_stop(held.state(), b"root-reserve-stop")?;
        let published = crate::suite::identity::observed(hosted.publish(&held, consumer.clone()))?;
        assert_eq!(published.control_event(), &consumer, "{limits:?}");
        let planned = crate::suite::identity::observed(campaign_core::admit_stop(
            held.state(),
            stop.reason.clone(),
        ))?;
        let (stopped, _) = hosted.append(planned)?;
        let stopped = crate::suite::identity::observed(stopped.validate())?;
        assert_eq!(stopped.control_events().len(), 2, "{limits:?}");
        assert!(stopped.state().stopped.is_some());
    }
    Ok(())
}

/// After a partial commit of the control root (the draft admitted, the branch never
/// created), retrying the identical admission returns the same admission with the unchanged graph
/// on every bound, even where the root and its stop fill the bound (the held root counts once).
/// Another campaign's root on that graph under the same envelope is refused with
/// `SharedControlGraph` naming the held root, before any capacity check (one Campaign campaign
/// per graph and envelope), whether or not the graph has room for it and its stop.
#[test]
// r[verify campaign.initial_admission.root_retry]
// r[verify campaign.initial_admission.one_per_graph]
fn retrying_the_identical_admission_returns_it_unchanged() -> Result<(), String> {
    for limits in limits_with(1)? {
        let unadmitted =
            crate::suite::configured::campaign::root::Unadmitted::bound(limits.clone())?;
        let first = crate::suite::identity::observed(unadmitted.root())?;
        let crashed = crate::suite::configured::campaign::root::Unadmitted {
            graph: first.graph_after_admission.clone(),
            ..unadmitted
        };
        assert_eq!(crashed.root(), Ok(first.clone()), "{limits:?}");
        let hosted = crashed.hosted()?;
        assert_eq!(hosted.graph, first.graph_after_admission);
        assert!(
            crate::suite::identity::observed(hosted.validate())?
                .state()
                .stopped
                .is_none()
        );
    }
    for graph_events in [3, crate::suite::constants::MAX_EVENTS] {
        let limits = Limits {
            graph_events,
            ..Limits::standard(
                crate::suite::constants::ROOT_GENERATION,
                crate::suite::constants::MAX_EVENTS,
            )
        };
        let first = crate::suite::identity::observed(
            crate::suite::configured::campaign::root::Unadmitted::bound(limits.clone())?.root(),
        )?;
        let other = crate::suite::configured::campaign::root::Unadmitted::bound(Limits {
            generation: crate::suite::constants::BRANCH_EVENT_INCREMENT,
            ..limits
        })?;
        assert_eq!(
            crate::suite::identity::observed(other.root())?
                .graph_after_admission
                .len(),
            2,
            "on its own graph the other root is admitted"
        );
        let shared = crate::suite::configured::campaign::root::Unadmitted {
            graph: first.graph_after_admission.clone(),
            ..other
        };
        assert_eq!(
            shared.root(),
            Err(campaign_choregraph::AdapterError::SharedControlGraph {
                event_id: first.envelope.history_event_id,
            })
        );
    }
    Ok(())
}

/// A host projection bound below the graph's projection limit admits the root, but its lineage
/// leaves no room for a stop: the planner refuses with the typed capacity error instead of
/// planning a stop that publication refuses, and publication refuses every stop the same way.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn a_host_bound_without_room_for_the_stop_refuses_planning_and_publication() -> Result<(), String> {
    let limits = Limits {
        maximum_events: 1,
        ..Limits::standard(
            crate::suite::constants::ROOT_GENERATION,
            crate::suite::constants::MAX_EVENTS,
        )
    };
    let hosted = crate::suite::configured::campaign::admitted_under(limits)?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    assert_eq!(
        adapter_plan(&held)?,
        Err(campaign_choregraph::AdapterError::ProjectionCapacityExhausted)
    );
    let stop = crate::suite::integrity::consumer_stop(held.state(), b"root-reserve-stop")?;
    assert_eq!(
        hosted.publish(&held, stop).map(|_| ()),
        Err(campaign_choregraph::AdapterError::ProjectionCapacityExhausted)
    );
    Ok(())
}

/// A control branch name Choregraph refuses (empty, or longer than the graph's
/// `max_label_bytes`) can neither bind an admission nor load a history; a name at the limit
/// binds.
#[test]
// r[verify campaign.history_projection.stale]
fn a_control_branch_name_choregraph_refuses_binds_nothing() -> Result<(), String> {
    let hosted = crate::suite::configured::campaign::admitted_at(
        crate::suite::constants::ROOT_GENERATION,
        crate::suite::constants::MAX_EVENTS,
    )?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    let limit_bytes = usize::try_from(crate::suite::constants::MAX_LABEL_BYTES)
        .map_err(|error| error.to_string())?;
    let over_bytes = limit_bytes
        .checked_add(1)
        .ok_or_else(|| String::from("a bounded label"))?;
    let envelope = crate::suite::configured::Envelope::canonical();
    let named = |name: String| choregraph_history::BranchRef {
        name,
        ..hosted.branch.clone()
    };
    let bind = |branch: &choregraph_history::BranchRef| {
        campaign_choregraph::derive_projection_binding(
            &hosted.graph,
            branch,
            envelope.schema,
            envelope.adapter,
            envelope.domain,
        )
    };
    assert_eq!(
        bind(&named("c".repeat(limit_bytes))).map(|binding| binding.generation),
        Ok(hosted.branch.generation)
    );
    let invalid = campaign_choregraph::AdapterError::History(
        choregraph_history::HistoryError::InvalidBranchName,
    );
    for name in [String::new(), "c".repeat(over_bytes)] {
        let branch = named(name);
        assert_eq!(bind(&branch), Err(invalid.clone()));
        let request = campaign_choregraph::ControlProjectionRequest {
            branch: &branch,
            expected_binding: held.binding().clone(),
            ..crate::suite::identity::observed(hosted.request())?
        };
        assert_eq!(
            campaign_choregraph::validate_control_projection(&request),
            Err(invalid.clone())
        );
        assert_eq!(
            campaign_choregraph::validate_control_advance(&held, &request),
            Err(invalid.clone())
        );
    }
    Ok(())
}
