//! D94: state currency re-derives and validates the admission. A caller-built state whose
//! admission no longer re-derives (an edited bound, re-derived bounds under the admitted campaign
//! identity, or an edited admitted binding) is refused with `StaleState` by every entry point that
//! checks state currency, before any policy reads the edited admission.

type History = crate::suite::s3_fixture::History;

/// An admitted outstanding bound of one with one selection outstanding and a second guided
/// candidate: the honest planner waits for capacity.
fn at_capacity() -> Result<
    (
        History,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_outstanding = 1;
    })?;
    let (events, state, first) =
        crate::suite::s3_fixture::selected(events, &state, b"d94-admission-first")?;
    let (events, state, _second) = crate::suite::planning_support::guided(
        events,
        &state,
        b"d94-admission-second",
        crate::suite::planning_support::RANK_TOP,
    )?;
    assert_eq!(state.outstanding.len(), 1);
    Ok((events, state, first))
}

/// The plan, request, and binding a receipt for `state` is projected under.
fn receipt_inputs(
    state: &campaign_core::FrontierState,
) -> Result<
    (
        campaign_core::SelectionRequest,
        campaign_core::ProjectionBinding,
    ),
    String,
> {
    let binding = crate::suite::admission::current_projection(state)?;
    let request = crate::suite::selection::request_on(
        state,
        crate::suite::selection::ranked_profile()?,
        None,
        &binding,
    );
    Ok((request, binding))
}

/// Every currency-checking entry point refuses `forged` with `StaleState`: planning, plan-receipt
/// projection (with the honest state's plan), stop, result, and closure intake, cache derivation,
/// and replay of a selection re-planned on the forged state.
fn stale_everywhere(
    honest: &campaign_core::FrontierState,
    forged: &campaign_core::FrontierState,
    first: &campaign_core::SelectionDecision,
) -> Result<(), String> {
    let stale = Err(campaign_core::CampaignError::StaleState);
    let profile = crate::suite::selection::ranked_profile()?;
    let plan = crate::suite::planning_support::plan_with(forged, profile.clone(), None);
    assert_eq!(plan.map(|_| ()), stale);
    let (honest_request, honest_binding) = receipt_inputs(honest)?;
    let honest_plan = campaign_core::plan_next(honest, honest_request, &honest_binding)
        .map_err(crate::suite::s3_fixture::err)?;
    let (request, binding) = receipt_inputs(forged)?;
    let receipt = campaign_core::project_receipt(forged, &honest_plan, &request, &binding);
    assert_eq!(receipt.map(|_| ()), stale);
    let stop = campaign_core::StopReason::ConsumerRequested {
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"d94-stop"),
    };
    assert_eq!(campaign_core::admit_stop(forged, stop).map(|_| ()), stale);
    let observation = crate::suite::s3_fixture::observation(forged, first, Vec::new());
    assert_eq!(
        campaign_core::admit_result(forged, observation).map(|_| ()),
        stale
    );
    assert_eq!(
        campaign_core::derive_cache(forged, &binding).map(|_| ()),
        stale
    );
    let replanned = crate::suite::s3_fixture::replanned_on(forged, first)?;
    assert_eq!(
        campaign_core::apply_control_event(forged.clone(), &replanned).map(|_| ()),
        stale
    );
    Ok(())
}

/// The round-10 reproducer: raising the admitted outstanding bound from one to two without
/// re-deriving `bounds_id` or the campaign identity, then recomputing the state identity, used to
/// yield a current state on which the planner selected a second candidate and replay accepted it
/// (two outstanding under an admitted bound of one). Every currency check now refuses it.
#[test]
// r[verify campaign.identity.admission_current]
fn an_edited_admitted_bound_is_stale_state_at_every_entry_point() -> Result<(), String> {
    let (_events, state, first) = at_capacity()?;
    let forged = crate::suite::planning_support::rebuilt(state.clone(), |state| {
        state.admission.bounds.max_outstanding = 2;
    })?;
    assert_eq!(forged.admission.campaign_id, state.admission.campaign_id);
    assert_ne!(forged.state_id, state.state_id);
    assert_eq!(
        forged.admission.validate(),
        Err(campaign_core::CampaignError::InvalidConfig)
    );
    stale_everywhere(&state, &forged, &first)
}

/// Bounds whose identity is re-derived with the raised value, under the admitted campaign
/// identity (which frames the old bounds identity), do not re-derive the admission either.
#[test]
// r[verify campaign.identity.admission_current]
fn rederived_bounds_under_the_admitted_campaign_identity_are_stale_state() -> Result<(), String> {
    let (_events, state, first) = at_capacity()?;
    let mut bounds = state.admission.bounds.clone();
    bounds.max_outstanding = 2;
    let bounds = bounds.derive().map_err(crate::suite::s3_fixture::err)?;
    let forged = crate::suite::planning_support::rebuilt(state.clone(), |state| {
        state.admission.bounds = bounds;
    })?;
    assert_eq!(forged.admission.bounds.validate(), Ok(()));
    assert_eq!(
        forged.admission.validate(),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    stale_everywhere(&state, &forged, &first)
}

/// An admitted projection binding replaced by another re-derived binding (another head at a later
/// generation), with the admitted roots rebound to it, under the admitted campaign identity (which
/// frames the original binding) is refused as not current.
#[test]
// r[verify campaign.identity.admission_current]
fn an_edited_admitted_binding_is_stale_state() -> Result<(), String> {
    let (_events, state, first) = at_capacity()?;
    let mut projection = state.admission.projection.clone();
    projection.projection_id = campaign_core::Id::absent();
    projection.head_id = crate::suite::admission::xid(campaign_core::IdKind::Head, b"d94-head");
    projection.generation = projection.generation.saturating_add(1);
    let projection = projection.derive().map_err(crate::suite::s3_fixture::err)?;
    let forged = crate::suite::planning_support::rebuilt(state.clone(), |state| {
        for root in &mut state.admission.roots {
            root.projection_id = projection.projection_id;
        }
        state.admission.projection = projection;
    })?;
    assert_eq!(
        forged.admission.validate(),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    stale_everywhere(&state, &forged, &first)
}

/// The honest path: the reconstructed state at capacity, whose admission re-derives, is current
/// at every currency check (the planner waits for capacity, the receipt projects, the stop and
/// the result are admitted, the cache derives), and replaying its history reproduces it.
#[test]
// r[verify campaign.identity.admission_current]
fn an_honest_reconstructed_state_is_current() -> Result<(), String> {
    let (events, state, first) = at_capacity()?;
    assert_eq!(state.admission.validate(), Ok(()));
    assert_eq!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity)
    );
    let (request, binding) = receipt_inputs(&state)?;
    let plan = campaign_core::plan_next(&state, request.clone(), &binding)
        .map_err(crate::suite::s3_fixture::err)?;
    campaign_core::project_receipt(&state, &plan, &request, &binding)
        .map_err(crate::suite::s3_fixture::err)?;
    campaign_core::derive_cache(&state, &binding).map_err(crate::suite::s3_fixture::err)?;
    let stop = campaign_core::admit_stop(
        &state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"d94-stop"),
        },
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let result = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &first, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let settled = campaign_core::apply_control_event(state.clone(), &result)
        .map_err(crate::suite::s3_fixture::err)?;
    assert!(settled.outstanding.is_empty());
    let stopped = campaign_core::apply_control_event(state.clone(), &stop)
        .map_err(crate::suite::s3_fixture::err)?;
    assert!(stopped.stopped.is_some());
    assert_eq!(
        campaign_core::reconstruct(&events).map_err(crate::suite::s3_fixture::err)?,
        state
    );
    Ok(())
}
