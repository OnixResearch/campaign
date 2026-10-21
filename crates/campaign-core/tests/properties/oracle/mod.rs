//! Independent reference model of eligibility, canonical ranking, exploration order, and the
//! planner's decision kind, computed only from public state fields.

/// Decision kind the reference model expects for one state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Select,
    Wait(campaign_core::WaitReason),
    Stop,
}

pub(crate) fn kind(decision: &campaign_core::NextDecision) -> Kind {
    match decision {
        campaign_core::NextDecision::Select(_) => Kind::Select,
        campaign_core::NextDecision::Wait(reason) => Kind::Wait(reason.clone()),
        campaign_core::NextDecision::Stop(_) => Kind::Stop,
    }
}

pub(crate) fn amount_in(
    amounts: &[campaign_core::ResourceAmount],
    dimension_id: campaign_core::Id,
) -> u64 {
    amounts
        .iter()
        .find(|amount| amount.dimension_id == dimension_id)
        .map_or(0, |amount| amount.amount)
}

/// Reserved amount of `dimension_id`, zero when nothing was reserved.
pub(crate) fn reserved(
    state: &campaign_core::FrontierState,
    dimension_id: campaign_core::Id,
) -> u64 {
    amount_in(&state.reserved_resources, dimension_id)
}

fn fits(state: &campaign_core::FrontierState, candidate: &campaign_core::CandidateState) -> bool {
    candidate.candidate.declared_costs.iter().all(|cost| {
        reserved(state, cost.dimension_id)
            .checked_add(cost.amount)
            .is_some_and(|next| {
                next <= amount_in(&state.admission.resource_budgets, cost.dimension_id)
            })
    })
}

/// Retained, guided candidates whose declared costs fit every remaining budget.
pub(crate) fn eligible(
    state: &campaign_core::FrontierState,
) -> Vec<&campaign_core::CandidateState> {
    state
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(candidate.status, campaign_core::CandidateStatus::Retained)
                && candidate.latest_guidance.is_some()
                && fits(state, candidate)
        })
        .collect()
}

fn latest_ranks(candidate: &campaign_core::CandidateState) -> Vec<i64> {
    candidate
        .latest_guidance
        .as_ref()
        .map(|guidance| guidance.ranks.clone())
        .unwrap_or_default()
}

fn decayed(candidate: &campaign_core::CandidateState) -> Result<Vec<i64>, String> {
    let mut ranks = latest_ranks(candidate);
    let penalty = crate::suite::requests::DECAY_PENALTY
        .checked_mul(candidate.selection_count)
        .and_then(|product| i64::try_from(product).ok())
        .ok_or_else(|| String::from("decay penalty overflow"))?;
    if let Some(first) = ranks.first_mut() {
        *first = first
            .checked_sub(penalty)
            .ok_or_else(|| String::from("decayed rank overflow"))?;
    }
    Ok(ranks)
}

/// Eligible identities by decayed ranks descending, identity ascending.
pub(crate) fn ranked_order(
    state: &campaign_core::FrontierState,
) -> Result<Vec<campaign_core::Id>, String> {
    let candidates = eligible(state);
    let mut keyed = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        keyed.push((decayed(candidate)?, candidate.candidate.candidate_id));
    }
    keyed.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    Ok(keyed
        .into_iter()
        .map(|(_, candidate_id)| candidate_id)
        .collect())
}

/// Eligible identities by undecayed ranks projected onto `order` descending, identity ascending.
pub(crate) fn exploration_order(
    state: &campaign_core::FrontierState,
    order: &[u32],
) -> Vec<campaign_core::Id> {
    let mut keyed: Vec<(Vec<i64>, campaign_core::Id)> = eligible(state)
        .into_iter()
        .map(|candidate| {
            let ranks = latest_ranks(candidate);
            let projected = order
                .iter()
                .filter_map(|dimension| usize::try_from(*dimension).ok())
                .filter_map(|dimension| ranks.get(dimension).copied())
                .collect();
            (projected, candidate.candidate.candidate_id)
        })
        .collect();
    keyed.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    keyed
        .into_iter()
        .map(|(_, candidate_id)| candidate_id)
        .collect()
}

/// True when the ticket's explore draw takes the exploratory branch under `threshold`.
pub(crate) fn explores(ticket: &campaign_core::EntropyTicket, threshold: u64) -> bool {
    ticket.explore_draw() % campaign_core::ENTROPY_SCALE < threshold
}

/// Candidate the planner must choose for `request` over `state`, when it selects.
pub(crate) fn expected_choice(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
) -> Result<Option<campaign_core::Id>, String> {
    let ranked = ranked_order(state)?;
    let exploration = match (&request.profile.mode, &request.entropy) {
        (
            campaign_core::SelectionMode::Exploratory {
                threshold_parts_per_million,
                exploration_order: order,
                ..
            },
            Some(ticket),
        ) if explores(ticket, *threshold_parts_per_million) => {
            Some((exploration_order(state, order), ticket.index_draw()))
        }
        _ => None,
    };
    let Some((explored, index_draw)) = exploration else {
        return Ok(ranked.first().copied());
    };
    assert_eq!(explored.len(), ranked.len());
    let count = u64::try_from(explored.len()).map_err(|error| format!("{error}"))?;
    let Some(position) = index_draw.checked_rem(count) else {
        return Ok(None);
    };
    assert!(position < count);
    let position = usize::try_from(position).map_err(|error| format!("{error}"))?;
    Ok(explored.get(position).copied())
}

/// A retained unguided candidate awaits guidance only while one more record fits the bound and
/// its declared costs fit the remaining budgets.
fn awaiting_guidance(state: &campaign_core::FrontierState) -> bool {
    state.guidance_record_count < state.admission.bounds.max_guidance_records
        && state.candidates.iter().any(|candidate| {
            matches!(candidate.status, campaign_core::CandidateStatus::Retained)
                && candidate.latest_guidance.is_none()
                && fits(state, candidate)
        })
}

/// Control events an open campaign must still be able to record after `events` more: every
/// outstanding settlement plus one stop.
fn control_event_demand(state: &campaign_core::FrontierState, events: u64) -> Result<u64, String> {
    let outstanding = u64::try_from(state.outstanding.len()).map_err(|error| format!("{error}"))?;
    state
        .control_event_count
        .checked_add(outstanding)
        .and_then(|demand| demand.checked_add(events))
        .and_then(|demand| demand.checked_add(1))
        .ok_or_else(|| String::from("control-event demand overflow"))
}

/// Whether one more new-work event keeps headroom for outstanding settlements and a stop.
pub(crate) fn has_new_work_headroom(state: &campaign_core::FrontierState) -> Result<bool, String> {
    Ok(control_event_demand(state, 1)? <= state.admission.bounds.max_control_events)
}

/// Whether a selection (and its later settlement) keeps headroom for a stop.
fn has_selection_headroom(state: &campaign_core::FrontierState) -> Result<bool, String> {
    Ok(control_event_demand(state, 2)? <= state.admission.bounds.max_control_events)
}

/// Decision kind for a live campaign without a finding limit.
pub(crate) fn expected_kind(state: &campaign_core::FrontierState) -> Result<Kind, String> {
    let outstanding = u64::try_from(state.outstanding.len()).map_err(|error| format!("{error}"))?;
    let bounds = &state.admission.bounds;
    assert!(outstanding <= bounds.max_outstanding);
    assert!(state.selection_count <= bounds.max_selections);
    if state.selection_count >= bounds.max_selections {
        if outstanding > 0 {
            return Ok(Kind::Wait(campaign_core::WaitReason::AwaitingResults));
        }
        return Ok(Kind::Stop);
    }
    if !has_selection_headroom(state)? {
        if outstanding > 0 {
            return Ok(Kind::Wait(campaign_core::WaitReason::AwaitingResults));
        }
        return Ok(Kind::Stop);
    }
    if eligible(state).is_empty() {
        if awaiting_guidance(state) {
            return Ok(Kind::Wait(campaign_core::WaitReason::AwaitingGuidance));
        }
        if outstanding > 0 {
            return Ok(Kind::Wait(campaign_core::WaitReason::AwaitingResults));
        }
        return Ok(Kind::Stop);
    }
    if outstanding >= bounds.max_outstanding {
        return Ok(Kind::Wait(campaign_core::WaitReason::AwaitingCapacity));
    }
    Ok(Kind::Select)
}
