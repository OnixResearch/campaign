//! One Campaign campaign per graph and envelope: the adapter bounds one campaign's reserve against
//! the graph's event limit, so a second campaign's control root under the same envelope is
//! refused at root admission and by validation (`SharedControlGraph`), while campaigns under
//! other envelopes are other writers bound by the exposed room.

type Limits = crate::suite::configured::campaign::root::Limits;
type Unadmitted = crate::suite::configured::campaign::root::Unadmitted;

fn graph_of(graph_events: u32) -> Limits {
    Limits {
        graph_events,
        ..Limits::standard(
            crate::suite::constants::ROOT_GENERATION,
            crate::suite::constants::MAX_EVENTS,
        )
    }
}

fn exposed(held: &campaign_choregraph::ValidatedControlProjection) -> (u32, u32, u32) {
    (
        held.reserved_control_events(),
        held.reserved_graph_events(),
        held.graph_room(),
    )
}

/// Campaign B: the suite's admission at generation 1, so another campaign identity.
fn second_admission(limits: &Limits) -> Result<Unadmitted, String> {
    Unadmitted::bound(Limits {
        generation: crate::suite::constants::BRANCH_EVENT_INCREMENT,
        ..limits.clone()
    })
}

fn shared<T>(event_id: choregraph_history::EventId) -> campaign_choregraph::AdapterResult<T> {
    Err(campaign_choregraph::AdapterError::SharedControlGraph { event_id })
}

/// The round-9 reproducer: with campaign A's root on a graph of 4 events (A's exposed room is 1),
/// campaign B's root used to be admitted into that room, both reserves claimed the same stop
/// slot, and once A stopped B could never stop. B's root is now refused with
/// `SharedControlGraph` naming A's root, at graph limits 4 and 5 alike, while on its own graph it
/// is admitted and stops. A root drafted onto A's graph outside `admit_control_root` makes A's
/// cold load, its advance, and its publication refuse with `SharedControlGraph` naming B's root;
/// on its own graph A's stop publishes.
#[test]
// r[verify campaign.initial_admission.one_per_graph]
// r[verify campaign.initial_admission.stop_reserve]
fn a_second_campaigns_root_is_refused_on_a_shared_graph() -> Result<(), String> {
    for limit in [4_u32, 5] {
        let limits = graph_of(limit);
        let a = Unadmitted::bound(limits.clone())?;
        let a_root = crate::suite::identity::observed(a.root())?;
        let hosted_a = a.hosted()?;
        let held_a = crate::suite::identity::observed(hosted_a.validate())?;
        let room = limit.checked_sub(3).ok_or("limit underflow")?;
        assert_eq!(exposed(&held_a), (1, 1, room), "A's room at {limit}");
        let b = Unadmitted {
            graph: a_root.graph_after_admission.clone(),
            ..second_admission(&limits)?
        };
        let a_root_id = a_root.envelope.history_event_id;
        assert_eq!(b.root(), shared(a_root_id), "limit {limit}");
        let alone = second_admission(&limits)?.hosted()?;
        let held_b = crate::suite::identity::observed(alone.validate())?;
        let (stopped_b, _) = alone.append(crate::suite::integrity::consumer_stop(
            held_b.state(),
            b"single-b-stop",
        )?)?;
        assert!(
            crate::suite::identity::observed(stopped_b.validate())?
                .state()
                .stopped
                .is_some()
        );
        let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
            &b.event,
            hosted_a.envelope.schema,
            hosted_a.envelope.adapter,
            hosted_a.envelope.domain,
            None,
        ))?;
        let forged = crate::suite::identity::observed(hosted_a.graph.admit(draft))?;
        let on_shared = crate::suite::configured::Hosted {
            graph: forged.graph,
            ..hosted_a.clone()
        };
        assert_eq!(on_shared.validate(), shared(forged.event));
        assert_eq!(on_shared.advance(&held_a), shared(forged.event));
        let stop = crate::suite::integrity::consumer_stop(held_a.state(), b"single-a-stop")?;
        assert_eq!(
            on_shared.publish(&held_a, stop.clone()).map(|_| ()),
            shared(forged.event)
        );
        assert_eq!(hosted_a.publish(&held_a, stop).map(|_| ()), Ok(()));
    }
    Ok(())
}

/// The round-9 reproducer's second shape: campaign A's root used to be admitted to the graph of
/// campaign B, whose selection executes; A's stop then left B's result, closure, and stop without
/// room. A's root is now refused with `SharedControlGraph` naming B's first control event in
/// Choregraph event-identity order, and so is a retry of B's own root once B has published after
/// it; B then settles and stops at graph limit 8.
#[test]
// r[verify campaign.initial_admission.one_per_graph]
// r[verify campaign.initial_admission.root_retry]
fn a_root_is_refused_on_a_graph_holding_another_campaigns_lineage() -> Result<(), String> {
    let limits = graph_of(8);
    let b = second_admission(&limits)?;
    let b_event = b.event.clone();
    let hosted_b = b.hosted()?;
    let (hosted_b, ids) = crate::suite::configured::campaign::with_candidates(hosted_b, 1)?;
    let held_b = crate::suite::identity::observed(hosted_b.validate())?;
    let candidate = *ids.first().ok_or("one candidate")?;
    let (hosted_b, _) = hosted_b.append(crate::suite::configured::campaign::guidance_for(
        &held_b, candidate,
    )?)?;
    let held_b = crate::suite::identity::observed(hosted_b.validate())?;
    let selection = crate::suite::selection::select_from_current(&held_b)?;
    let (hosted_b, publication) = hosted_b.append(crate::suite::selection::event(&selection))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &crate::suite::selection::exact_durable_evidence(&publication),
        ),
        Ok(true)
    );
    let first_b = hosted_b
        .payloads
        .iter()
        .map(|payload| payload.history_event_id)
        .min()
        .ok_or("B's lineage")?;
    let a = Unadmitted {
        graph: hosted_b.graph.clone(),
        ..Unadmitted::bound(limits.clone())?
    };
    assert_eq!(a.root(), shared(first_b));
    let root_b = hosted_b.payloads[0].history_event_id;
    let first_other = hosted_b
        .payloads
        .iter()
        .map(|payload| payload.history_event_id)
        .filter(|event_id| *event_id != root_b)
        .min()
        .ok_or("B's published events")?;
    let retried = Unadmitted {
        graph: hosted_b.graph.clone(),
        event: b_event,
        ..second_admission(&limits)?
    };
    assert_eq!(retried.root(), shared(first_other));
    let held_b = crate::suite::identity::observed(hosted_b.validate())?;
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        held_b.state(),
        crate::suite::selection::result_for(&held_b, &selection),
    ))?;
    let (settled, _) = hosted_b.append(result)?;
    let held_b = crate::suite::identity::observed(settled.validate())?;
    let (stopped, _) = settled.append(crate::suite::integrity::consumer_stop(
        held_b.state(),
        b"single-b-settled-stop",
    )?)?;
    assert_eq!(stopped.graph.len(), 7);
    Ok(())
}

/// A campaign under another envelope is another writer: its root on the graph does not stop a
/// root under the suite's envelope from being admitted, the suite campaign validates, and the
/// foreign root takes one event of the exposed room like any exploration event.
#[test]
// r[verify campaign.initial_admission.one_per_graph]
fn a_root_under_another_envelope_is_another_writer() -> Result<(), String> {
    let limits = graph_of(crate::suite::constants::MAX_EVENTS);
    let exploration = Unadmitted::bound(limits.clone())?.graph;
    let alone =
        crate::suite::identity::observed(Unadmitted::bound(limits.clone())?.hosted()?.validate())?
            .graph_room();
    let b = second_admission(&limits)?;
    let foreign = crate::suite::configured::Envelope::foreign();
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &b.event,
        foreign.schema,
        foreign.adapter,
        foreign.domain,
        None,
    ))?;
    let with_foreign = crate::suite::identity::observed(exploration.admit(draft))?;
    let a = Unadmitted {
        graph: with_foreign.graph,
        ..Unadmitted::bound(limits)?
    };
    let hosted = a.hosted()?;
    let held = crate::suite::identity::observed(hosted.validate())?;
    assert_eq!(
        held.graph_room(),
        alone.checked_sub(1).ok_or("room underflow")?
    );
    assert!(hosted.graph.contains(with_foreign.event));
    Ok(())
}
