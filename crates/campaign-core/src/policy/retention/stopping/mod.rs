/// Admits one terminal stop decision against the exact current state.
///
/// Consumer and policy stops need a Reason-kind identity and are admissible while the campaign is
/// not stopped. Frontier, budget, selection-limit, and control-event-limit stops are admissible only
/// when the planner would produce exactly that reason for the current state.
///
/// # Errors
///
/// Returns `CampaignStopped` after an accepted stop, `ControlEventLimitReached` when the stop would
/// leave an outstanding selection unable to settle, and `InvalidStop` when a bound or frontier stop
/// does not match the current state.
// r[impl campaign.stop]
// r[impl campaign.stop.control_event_limit]
pub fn admit_stop(
    state: &crate::frontier::FrontierState,
    mut reason: crate::model::StopReason,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    if state.stopped.is_some() {
        return Err(crate::error::CampaignError::CampaignStopped);
    }
    crate::frontier::reserve::ensure_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::STOP,
    )?;
    if let crate::model::StopReason::BudgetExhausted { dimensions } = &mut reason {
        dimensions.sort();
    }
    crate::policy::retention::stopping::validate_stop_reason(state, &reason)?;
    let stop = crate::policy::plans::build_stop(state, reason)?;
    debug_assert!(state.stopped.is_none());
    debug_assert!(stop.incomplete_search_acknowledged);
    let event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::StopAccepted,
        state.admission.campaign_id,
        state.state_id,
        &[stop.stop_id],
    )?;
    Ok(crate::model::CampaignControlEvent::StopAccepted(
        crate::model::StopEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            stop,
        },
    ))
}

/// Checks the admission precondition of one stop reason for the current state.
// r[impl campaign.stop.consumer]
// r[impl campaign.stop.control_event_limit]
pub(crate) fn validate_stop_reason(
    state: &crate::frontier::FrontierState,
    reason: &crate::model::StopReason,
) -> crate::error::CampaignResult<()> {
    if let crate::model::StopReason::ConsumerRequested { reason_id }
    | crate::model::StopReason::PolicyRequested { reason_id } = reason
    {
        reason_id.require_kind(crate::identity::IdKind::Reason)?;
        return Ok(());
    }
    if bound_stop_reason(state)?.as_ref() != Some(reason) {
        return Err(crate::error::CampaignError::InvalidStop);
    }
    Ok(())
}

/// Returns the frontier, budget, selection-limit, or control-event-limit stop the planner produces
/// for this state.
// r[impl campaign.stop.budget]
// r[impl campaign.stop.frontier]
// r[impl campaign.stop.control_event_limit]
// r[impl campaign.wait.guidance_room]
fn bound_stop_reason(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<Option<crate::model::StopReason>> {
    if state.selection_count >= state.admission.bounds.max_selections {
        return Ok(state
            .outstanding
            .is_empty()
            .then_some(crate::model::StopReason::SelectionLimitReached));
    }
    if !crate::frontier::reserve::has_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::SELECTION,
    )? {
        return Ok(state
            .outstanding
            .is_empty()
            .then_some(crate::model::StopReason::ControlEventLimitReached));
    }
    let guidance_wait = crate::policy::capacity::guidance_wait(state)?;
    if has_eligible_candidate(state)?
        || guidance_wait == crate::policy::capacity::GuidanceWait::Awaiting
        || !state.outstanding.is_empty()
    {
        return Ok(None);
    }
    debug_assert!(state.outstanding.is_empty());
    if guidance_wait == crate::policy::capacity::GuidanceWait::ControlEventLimited {
        return Ok(Some(crate::model::StopReason::ControlEventLimitReached));
    }
    let dimensions = crate::policy::capacity::exhausted_dimensions(state)?;
    if dimensions.is_empty() {
        return Ok(Some(crate::model::StopReason::FrontierExhausted));
    }
    Ok(Some(crate::model::StopReason::BudgetExhausted {
        dimensions,
    }))
}

fn has_eligible_candidate(
    state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<bool> {
    for candidate in &state.candidates {
        if matches!(candidate.status, crate::model::CandidateStatus::Retained)
            && candidate.latest_guidance.is_some()
            && crate::policy::capacity::costs_fit(state, &candidate.candidate.declared_costs)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}
