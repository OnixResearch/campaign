//! Host control capacity: the Choregraph projection bound, the control branch generation range,
//! and the Choregraph graph's own event limit.
//!
//! Campaign core reserves one event per outstanding selection and one stop against
//! `max_control_events`. The host bounds the same lineage three more times: a projection holds at
//! most `maximum_events` events, the control branch advances one generation per event from the
//! admission's binding without passing `u64::MAX`, and the graph holding the lineage admits at
//! most `HistoryLimits::max_events` events. Publication keeps the same reserve against every host
//! bound, the control root is admitted only with room for its own stop (and only on a graph
//! holding no other Campaign campaign under its envelope, see [`crate::occupancy`]), and planning
//! through a published projection uses the same effective capacity, so every outstanding
//! selection can settle and the stop can always be published.
//!
//! The graph reserve holds one control event per outstanding selection (a result without
//! children, or a closure) and the stop. Every Campaign control draft pending below the branch
//! head (a publication whose branch update has not committed, the drafts chained below it, and
//! racing siblings) is part of the reserve, each counted as a selection: its lineage event and one
//! settlement (see [`reserve`]). The validated projection exposes the reserve and the room left
//! outside it ([`crate::ValidatedControlProjection::graph_room`]). The graph is shared:
//! exploration moments, reported children, and every other writer consume that room, so hosts
//! must leave the reserve free. When another writer has consumed it, the planner refuses every
//! plan with the typed capacity error, as publication refuses every event that cannot shrink the
//! reserve, instead of planning a wait or a stop that could never be published. Pending drafts
//! are completed in chain order before anything else is planned or published there; a
//! republication counts its own draft once and the drafts chained below it as selections.
//!
//! When the host capacity leaves no room for another selection with nothing outstanding, the
//! adapter plans a consumer stop under the adapter-owned [`control_capacity_reason_id`]. Campaign
//! core's `ControlEventLimitReached` stop is not used: replay derives that reason from
//! `max_control_events` alone and cannot see the host's bounds, so such a stop would be refused as
//! `InvalidStop` on publication and on every later replay. A wait for guidance is kept only while
//! the guidance event, then a selection, its settlement, and the stop still fit, so the planner
//! never waits for guidance that could not lead to a selection.

pub(crate) mod limits;
pub(crate) mod reserve;

/// Framing label of the adapter-owned host-capacity stop reason.
const CAPACITY_REASON_LABEL: &[u8] = b"campaign-choregraph/control-capacity-stop";

/// The adapter-owned `Reason` identity of the consumer stop planned when the host's control
/// capacity (projection bound, branch generation range, or graph event limit) leaves no room for
/// another selection.
///
/// It is derived deterministically from a fixed adapter label, so every host and replay names the
/// same reason.
#[must_use]
// r[impl campaign.stop.consumer]
pub fn control_capacity_reason_id() -> campaign_core::Id {
    campaign_core::derive_id(campaign_core::IdKind::Reason, CAPACITY_REASON_LABEL, &[])
}

/// Lineage events of a freshly admitted control root.
const ROOT_LINEAGE: usize = 1;

/// Requires a control root admitted to `graph` to leave room for its own stop: `2 <=
/// max_projection_events`, the admission generation plus two without overflow, and `graph.len() +
/// 2 <= max_events`. A root draft the graph already holds (`is_held`, a retried admission) counts
/// once.
///
/// # Errors
///
/// Returns `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or
/// `GraphCapacityExhausted` for the first bound the root and its stop exceed.
// r[impl campaign.initial_admission.stop_reserve]
// r[impl campaign.initial_admission.root_retry]
pub(crate) fn check_root(
    state: &campaign_core::FrontierState,
    graph: &choregraph_history::HistoryGraph,
    is_held: bool,
) -> crate::contracts::AdapterResult<()> {
    let history_limits = graph.limits();
    let extent = limits::Extent {
        lineage: ROOT_LINEAGE,
        graph: graph
            .len()
            .checked_add(u32::from(!is_held))
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
    };
    debug_assert!(state.outstanding.is_empty());
    let bounds = limits::HostBounds {
        maximum_events: history_limits.max_projection_events,
        admitted_generation: state.admission.projection.generation,
        graph_max_events: history_limits.max_events,
    };
    bounds.check(limits::Demand {
        extent,
        outstanding: state.outstanding.len(),
        open: state.stopped.is_none(),
    })
}

/// Requires the control history after one publication, its outstanding settlements, the stop
/// reserve of an open campaign, and the `chained` Campaign control drafts the graph holds below
/// the published event (each counted as a selection: its lineage event and one settlement, exact
/// for a chained selection and conservative for any other chained draft, the pipelined-chain
/// residual of [`reserve`]) to fit the host control capacity of `current`: its projection bound,
/// the branch generation range, and its graph's event limit.
///
/// # Errors
///
/// Returns `ProjectionCapacityExhausted` beyond the projection bound,
/// `GenerationCapacityExhausted` beyond the branch generation range, and
/// `GraphCapacityExhausted` beyond the graph's event limit.
// r[impl campaign.publication_fence.capacity]
// r[impl campaign.publication_fence.pending_reserve]
pub(crate) fn check_publication(
    state_after: &campaign_core::FrontierState,
    after: limits::Extent,
    chained: usize,
    current: &crate::contracts::ControlProjectionRequest<'_>,
) -> crate::contracts::AdapterResult<()> {
    let bounds = limits::HostBounds {
        maximum_events: current.maximum_events,
        admitted_generation: state_after.admission.projection.generation,
        graph_max_events: current.graph.limits().max_events,
    };
    bounds.check(reserve::demand(after, state_after, chained))
}

/// The host bounds of the published projection.
const fn bounds_of(current: &crate::contracts::ValidatedControlProjection) -> limits::HostBounds {
    limits::HostBounds {
        maximum_events: current.maximum_events,
        admitted_generation: current.state.admission.projection.generation,
        graph_max_events: current.graph_max_events,
    }
}

/// The demand of the published projection (see [`reserve::held`]) after `events` more control
/// events.
// r[impl campaign.publication_fence.pending_reserve]
fn after_more(
    current: &crate::contracts::ValidatedControlProjection,
    events: u32,
) -> crate::contracts::AdapterResult<limits::Demand> {
    let held = reserve::held(current);
    let lineage_events =
        usize::try_from(events).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let extent = limits::Extent {
        lineage: held
            .extent
            .lineage
            .checked_add(lineage_events)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        graph: held
            .extent
            .graph
            .checked_add(events)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
    };
    debug_assert!(extent.lineage >= current.control_events.len());
    debug_assert!(extent.graph >= current.graph_events);
    Ok(limits::Demand { extent, ..held })
}

/// Whether `preceding` control events, then one more selection, its settlement, and the stop
/// still fit the host control capacity of the published projection.
// r[impl campaign.stop.host_capacity]
// r[impl campaign.publication_fence.pending_reserve]
fn fits_another_selection(
    current: &crate::contracts::ValidatedControlProjection,
    preceding: u32,
) -> crate::contracts::AdapterResult<bool> {
    let events = preceding
        .checked_add(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let demand = after_more(current, events)?;
    bounds_of(current).fits(limits::Demand {
        outstanding: demand
            .outstanding
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        ..demand
    })
}

/// Requires the recorded reserve of the published projection (its lineage, outstanding
/// settlements, and the stop of an open campaign) to fit its host control capacity. Every control
/// event adds one lineage and one graph event and settles at most one reserved event, so when the
/// reserve no longer fits no publication can succeed.
// r[impl campaign.publication_fence.pending_reserve]
fn check_reserve(
    current: &crate::contracts::ValidatedControlProjection,
) -> crate::contracts::AdapterResult<()> {
    debug_assert!(current.pending.drafts.is_empty());
    bounds_of(current).check(reserve::held(current))
}

/// The control events a decision towards a new selection still needs published before that
/// selection: none for a selection or a wait for outstanding capacity, the guidance event for a
/// wait for guidance. `None` for decisions that lead to no new selection.
// r[impl campaign.stop.host_capacity]
const fn events_before_selection(decision: &campaign_core::NextDecision) -> Option<u32> {
    match decision {
        campaign_core::NextDecision::Select(_)
        | campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity) => Some(0),
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance) => Some(1),
        campaign_core::NextDecision::Stop(_)
        | campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults) => None,
    }
}

/// Bounds the core plan by the published projection's host control capacity.
///
/// Without room for another selection, its settlement, and the stop (after the guidance event a
/// wait for guidance still needs), a plan towards a new selection becomes a wait for outstanding
/// results or, with nothing outstanding, the consumer stop under [`control_capacity_reason_id`];
/// stops and result waits are kept. Every plan requires the recorded reserve to fit, as every
/// publication does.
///
/// # Errors
///
/// Returns `ProjectionCapacityExhausted`, `GenerationCapacityExhausted`, or
/// `GraphCapacityExhausted` when the lineage, its outstanding settlements, and the stop of an
/// open campaign no longer fit (the reserve was consumed outside Campaign publication), so no
/// settlement, stop, or other event could be published.
// r[impl campaign.stop.host_capacity]
// r[impl campaign.publication_fence.capacity]
pub(crate) fn plan_within_capacity(
    current: &crate::contracts::ValidatedControlProjection,
    plan: campaign_core::DecisionPlan,
) -> crate::contracts::AdapterResult<campaign_core::DecisionPlan> {
    check_reserve(current)?;
    bounded_plan(current, plan)
}

/// The core plan, or its wait or capacity stop when no further selection fits.
// r[impl campaign.stop.host_capacity]
fn bounded_plan(
    current: &crate::contracts::ValidatedControlProjection,
    plan: campaign_core::DecisionPlan,
) -> crate::contracts::AdapterResult<campaign_core::DecisionPlan> {
    let Some(preceding) = events_before_selection(&plan.decision) else {
        return Ok(plan);
    };
    if fits_another_selection(current, preceding)? {
        return Ok(plan);
    }
    let state = &current.state;
    let decision = if state.outstanding.is_empty() {
        campaign_core::NextDecision::Stop(alloc::boxed::Box::new(capacity_stop(state)?))
    } else {
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    };
    debug_assert_eq!(plan.state_id, state.state_id);
    Ok(campaign_core::DecisionPlan {
        decision,
        work: campaign_core::PlanningWork::none(),
        ..plan
    })
}

/// The consumer stop Campaign core admits for the host-capacity reason on `state`.
// r[impl campaign.receipts.host_capacity]
// r[impl campaign.stop.consumer]
fn capacity_stop(
    state: &campaign_core::FrontierState,
) -> crate::contracts::AdapterResult<campaign_core::StopDecision> {
    let event = campaign_core::admit_stop(
        state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: control_capacity_reason_id(),
        },
    )?;
    let campaign_core::CampaignControlEvent::StopAccepted(event) = event else {
        return Err(crate::contracts::AdapterError::MissingControlEvent);
    };
    debug_assert!(event.stop.incomplete_search_acknowledged);
    Ok(event.stop)
}
