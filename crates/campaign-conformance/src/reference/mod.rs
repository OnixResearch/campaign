//! Readable reference planner for differential evidence only.
//!
//! This planner restates the documented planning rules in two plain passes (eligibility, then
//! choice) using only the public Campaign model. It is reachable only through the explicit
//! `campaign_conformance::reference` path; no production crate depends on this crate, and
//! production dispatch refuses `PolicyImplementation::ReferenceEvidence`. It reports its own
//! semantic work: every candidate examined for eligibility plus every eligible candidate examined
//! again while choosing.

mod budget;
mod choice;

/// Control events one more selection needs: its publication, its settlement, and the stop.
const SELECTION_EVENTS: u64 = 3;

/// Control events a candidate awaiting guidance needs: its guidance, then one selection's events.
const GUIDED_SELECTION_EVENTS: u64 = 4;

/// Plans the next decision with the readable reference rules, in production's check order.
///
/// # Errors
///
/// Returns `StaleState` for a state whose identity is not its re-derived identity, the model's
/// kind error for a request identity of another kind, the profile's typed error for a profile
/// that does not validate, `InvalidConfig` for a decay outside the admitted bounds' domain or an
/// exploration order naming a dimension beyond the campaign's rank shape, the binding's typed
/// error for a projection binding that does not validate, `CrossedIdentity` for a binding of
/// another projection schema, adapter, or control domain than the admission's, `StaleState` or
/// `StaleProjection` for a request of another state or history, `InvalidEntropy` for a missing,
/// crossed, or unexpected ticket (also on a stopped campaign), `InvalidGuidanceShape` for
/// guidance without a projected or decayed dimension, `ArithmeticOverflow` for unrepresentable
/// arithmetic, and the model's typed identity errors.
// r[impl campaign.fd.oracle]
pub fn plan_next(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
    current_projection: &campaign_core::ProjectionBinding,
) -> campaign_core::CampaignResult<campaign_core::DecisionPlan> {
    check_request(state, request, current_projection)?;
    choice::check_entropy(state, request)?;
    if let Some(stop) = &state.stopped {
        return Ok(decided(state, request, stopped(stop.clone()), no_work()));
    }
    if let Some(plan) = plan_at_limits(state, request)? {
        return Ok(plan);
    }
    let (eligible, examined) = choice::eligible(state, &request.profile.decay)?;
    let eligible_count = count(eligible.len())?;
    let mut work = campaign_core::PlanningWork {
        candidates_examined: examined,
        eligible_candidates: eligible_count,
    };
    if eligible.is_empty() {
        return Ok(decided(state, request, without_eligible(state)?, work));
    }
    if count(state.outstanding.len())? >= state.admission.bounds.max_outstanding {
        let wait = campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity);
        return Ok(decided(state, request, wait, work));
    }
    let chosen = choice::choose(&eligible, request)?;
    work.candidates_examined = examined
        .checked_add(eligible_count)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let selection = choice::selection(state, request, current_projection, chosen)?;
    debug_assert!(work.candidates_examined >= work.eligible_candidates);
    debug_assert_eq!(selection.state_before_id, state.state_id);
    Ok(decided(
        state,
        request,
        campaign_core::NextDecision::Select(alloc::boxed::Box::new(selection)),
        work,
    ))
}

/// The request checks production runs before planning, in its order: the state's identity is its
/// re-derived identity, the request identity is a request, the profile validates within the
/// admitted bounds and rank shape, the binding validates and names the admission's envelope
/// identities, and the request names the current state and history.
///
/// # Errors
///
/// Returns the typed error of the first check that fails, as `plan_next` documents.
// r[impl campaign.fd.oracle]
fn check_request(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
    current_projection: &campaign_core::ProjectionBinding,
) -> campaign_core::CampaignResult<()> {
    let mut canonical = state.clone();
    canonical.recompute_identity()?;
    if canonical != *state {
        return Err(campaign_core::CampaignError::StaleState);
    }
    request
        .request_id
        .require_kind(campaign_core::IdKind::Request)?;
    request.profile.validate()?;
    request
        .profile
        .decay
        .validate_within_bounds(&state.admission.bounds)?;
    choice::check_rank_order(state, request)?;
    current_projection.validate()?;
    let admitted = &state.admission;
    if current_projection.projection_schema_id != admitted.projection.projection_schema_id
        || current_projection.adapter_id != admitted.adapter_id
        || current_projection.control_domain_id != admitted.control_domain_id
    {
        return Err(campaign_core::CampaignError::CrossedIdentity);
    }
    if request.campaign_id != admitted.campaign_id || request.state_id != state.state_id {
        return Err(campaign_core::CampaignError::StaleState);
    }
    if request.expected_generation != current_projection.generation
        || request.expected_head_id != current_projection.head_id
    {
        return Err(campaign_core::CampaignError::StaleProjection);
    }
    debug_assert_eq!(request.state_id, state.state_id);
    Ok(())
}

/// Whether the control-event bound still admits `events` more events beside one settlement per
/// outstanding selection (`count + outstanding + events <= max_control_events`).
fn has_room(
    state: &campaign_core::FrontierState,
    events: u64,
) -> campaign_core::CampaignResult<bool> {
    let demand = count(state.outstanding.len())?
        .checked_add(state.control_event_count)
        .and_then(|demand| demand.checked_add(events))
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    Ok(demand <= state.admission.bounds.max_control_events)
}

/// The selection bound, the control-event bound (one selection, its settlement, and the stop no
/// longer fit, D29), and the declared finding limit, in that order, checked before any
/// candidate: each waits for outstanding results, then stops.
fn plan_at_limits(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
) -> campaign_core::CampaignResult<Option<campaign_core::DecisionPlan>> {
    let is_finding_bound =
        request.profile.stopping.finding_limit.is_some_and(|limit| {
            u64::try_from(state.findings.len()).is_ok_and(|found| found >= limit)
        });
    // `None` names the finding limit, whose policy reason is derived only once it stops.
    let bound_reason = if state.selection_count >= state.admission.bounds.max_selections {
        Some(campaign_core::StopReason::SelectionLimitReached)
    } else if !has_room(state, SELECTION_EVENTS)? {
        Some(campaign_core::StopReason::ControlEventLimitReached)
    } else if is_finding_bound {
        None
    } else {
        return Ok(None);
    };
    if !state.outstanding.is_empty() {
        let wait = campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults);
        return Ok(Some(decided(state, request, wait, no_work())));
    }
    debug_assert!(state.outstanding.is_empty());
    let reason = if let Some(reason) = bound_reason {
        reason
    } else {
        campaign_core::StopReason::PolicyRequested {
            reason_id: request
                .profile
                .stopping
                .policy_reason_id()?
                .ok_or(campaign_core::CampaignError::InvalidConfig)?,
        }
    };
    Ok(Some(decided(
        state,
        request,
        stopped(stop(state, reason)?),
        no_work(),
    )))
}

/// Wait or stop when no candidate is eligible. A retained unguided candidate is awaited only
/// while one more guidance record fits `max_guidance_records`, its declared costs fit the
/// remaining budgets, and the control events admit its guidance, one selection, that selection's
/// settlement, and the stop (D69); without that control-event room it stops the campaign with
/// `ControlEventLimitReached` once nothing is outstanding. The budget stop names every dimension
/// that blocks some retained candidate.
// r[impl campaign.wait.guidance_room]
// r[impl campaign.wait.guidance]
// r[impl campaign.stop.control_event_limit]
fn without_eligible(
    state: &campaign_core::FrontierState,
) -> campaign_core::CampaignResult<campaign_core::NextDecision> {
    let is_guidable = budget::has_guidable_candidate(state)?;
    if is_guidable && has_room(state, GUIDED_SELECTION_EVENTS)? {
        return Ok(campaign_core::NextDecision::Wait(
            campaign_core::WaitReason::AwaitingGuidance,
        ));
    }
    if !state.outstanding.is_empty() {
        return Ok(campaign_core::NextDecision::Wait(
            campaign_core::WaitReason::AwaitingResults,
        ));
    }
    debug_assert!(state.outstanding.is_empty());
    let reason = if is_guidable {
        campaign_core::StopReason::ControlEventLimitReached
    } else {
        let dimensions = budget::exhausted_dimensions(state)?;
        if dimensions.is_empty() {
            campaign_core::StopReason::FrontierExhausted
        } else {
            campaign_core::StopReason::BudgetExhausted { dimensions }
        }
    };
    Ok(stopped(stop(state, reason)?))
}

fn stop(
    state: &campaign_core::FrontierState,
    reason: campaign_core::StopReason,
) -> campaign_core::CampaignResult<campaign_core::StopDecision> {
    let mut decision = campaign_core::StopDecision {
        stop_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        reason,
        incomplete_search_acknowledged: true,
    };
    decision.stop_id = campaign_core::derive_stop_id(&decision)?;
    Ok(decision)
}

fn stopped(decision: campaign_core::StopDecision) -> campaign_core::NextDecision {
    campaign_core::NextDecision::Stop(alloc::boxed::Box::new(decision))
}

const fn no_work() -> campaign_core::PlanningWork {
    campaign_core::PlanningWork {
        candidates_examined: 0,
        eligible_candidates: 0,
    }
}

fn count(len: usize) -> campaign_core::CampaignResult<u64> {
    u64::try_from(len).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)
}

fn decided(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
    decision: campaign_core::NextDecision,
    work: campaign_core::PlanningWork,
) -> campaign_core::DecisionPlan {
    campaign_core::DecisionPlan {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        decision,
        scheduling_policy_id: request.profile.scheduling_policy_id,
        admission_policy_id: request.profile.admission_policy_id,
        coalescing_policy_id: request.profile.coalescing_policy_id,
        work,
    }
}
