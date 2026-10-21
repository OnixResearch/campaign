mod bounded;
pub(crate) mod checks;

/// Plans the next decision against the caller-derived current projection binding.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
#[expect(
    clippy::needless_pass_by_value,
    reason = "selection planning consumes an exact request value at the public boundary"
)]
pub fn plan_next(
    state: &crate::frontier::FrontierState,
    request: crate::model::SelectionRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    crate::policy::validation::assert_state_identity_current(state)?;
    checks::validate_selection_request(state, &request, current_projection)?;
    plan_validated(
        state,
        &PlanningInputs::of_request(&request, current_projection),
    )
}

/// Declared policy inputs of one planned decision: profile, explicit entropy, and the branch fence
/// (control branch, head, generation) the selection is planned against.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlanningInputs<'a> {
    pub(crate) profile: &'a crate::model::SelectionProfile,
    pub(crate) entropy: Option<&'a crate::model::EntropyTicket>,
    pub(crate) control_branch_id: crate::identity::Id,
    pub(crate) head_id: crate::identity::Id,
    pub(crate) generation: u64,
}

impl<'a> PlanningInputs<'a> {
    /// Inputs of a request planned against the current projection binding.
    pub(crate) fn of_request(
        request: &'a crate::model::SelectionRequest,
        projection: &crate::model::ProjectionBinding,
    ) -> Self {
        Self {
            profile: &request.profile,
            entropy: request.entropy.as_ref(),
            control_branch_id: projection.control_branch_id,
            head_id: projection.head_id,
            generation: projection.generation,
        }
    }

    /// Inputs a published selection event declares: its profile, ticket, and planned fence.
    pub(crate) fn of_event(event: &'a crate::model::SelectionEvent) -> Self {
        let plan = &event.decision.publication_plan;
        Self {
            profile: &event.profile,
            entropy: event.entropy.as_ref(),
            control_branch_id: plan.control_branch_id,
            head_id: plan.expected_head_id,
            generation: plan.expected_generation,
        }
    }
}

/// Re-plans the decision the declared policy makes for `state` under `inputs`: the exact
/// production planner, re-run by replay for a published selection and by receipts and
/// scheduling explanations for a supplied plan.
// r[impl campaign.guidance.declared_order]
pub(crate) fn replan(
    state: &crate::frontier::FrontierState,
    inputs: &PlanningInputs<'_>,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    checks::validate_profile(state, inputs.profile)?;
    plan_validated(state, inputs)
}

// r[impl campaign.wait]
// r[impl campaign.stop.precedence]
// r[impl campaign.stop.recorded]
// r[impl campaign.selection.valid]
// r[impl campaign.wait.capacity]
fn plan_validated(
    state: &crate::frontier::FrontierState,
    inputs: &PlanningInputs<'_>,
) -> crate::error::CampaignResult<crate::model::DecisionPlan> {
    let profile = inputs.profile;
    checks::validate_entropy(state, profile, inputs.entropy)?;
    if let Some(stop) = &state.stopped {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Stop(alloc::boxed::Box::new(stop.clone())),
            crate::model::PlanningWork::none(),
        ));
    }
    if state.selection_count >= state.admission.bounds.max_selections {
        return bounded::plan_at_selection_limit(state, profile);
    }
    if !crate::frontier::reserve::has_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::SELECTION,
    )? {
        return bounded::plan_at_control_event_limit(state, profile);
    }
    if let Some(plan) = bounded::plan_at_finding_limit(state, profile)? {
        return Ok(plan);
    }
    let eligible = crate::policy::ranking::eligible_candidates(state, profile)?;
    let work = crate::model::PlanningWork {
        candidates_examined: crate::identity::usize_to_u64(state.candidates.len())?,
        eligible_candidates: crate::identity::usize_to_u64(eligible.len())?,
    };
    if eligible.is_empty() {
        return bounded::plan_without_eligible(state, profile, work);
    }
    if crate::identity::usize_to_u64(state.outstanding.len())?
        >= state.admission.bounds.max_outstanding
    {
        return Ok(crate::policy::plans::decision_plan(
            state,
            profile,
            crate::model::NextDecision::Wait(crate::model::WaitReason::AwaitingCapacity),
            work,
        ));
    }
    let selected_index = crate::policy::ranking::choose_index(&eligible, profile, inputs.entropy)?;
    let selected = eligible
        .get(selected_index)
        .ok_or(crate::error::CampaignError::MissingCandidate)?;
    let decision = crate::policy::plans::build_selection(
        state,
        inputs,
        selected.candidate,
        selected.guidance,
    )?;
    debug_assert_eq!(decision.state_before_id, state.state_id);
    Ok(crate::policy::plans::decision_plan(
        state,
        profile,
        crate::model::NextDecision::Select(alloc::boxed::Box::new(decision)),
        work,
    ))
}
