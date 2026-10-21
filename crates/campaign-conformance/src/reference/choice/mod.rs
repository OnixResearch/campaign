//! Reference eligibility, entropy, and choice rules.

/// One eligible candidate with its latest guidance and decayed ranks.
pub(super) struct Eligible<'a> {
    candidate: &'a campaign_core::CandidateState,
    guidance: &'a campaign_core::GuidanceRecord,
    effective_ranks: alloc::vec::Vec<i64>,
}

/// An exploration order names only dimensions the campaign's rank shape carries: every index is
/// below `max_rank_dimensions` and, once a guidance record fixes the campaign's one rank length,
/// below that length. Checked before any draw, so the refusal never depends on the ticket.
///
/// # Errors
///
/// Returns `InvalidConfig` for an index at or beyond either bound.
pub(super) fn check_rank_order(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
) -> campaign_core::CampaignResult<()> {
    let campaign_core::SelectionMode::Exploratory {
        exploration_order, ..
    } = &request.profile.mode
    else {
        return Ok(());
    };
    let fixed_length = state
        .candidates
        .iter()
        .find_map(|candidate| candidate.latest_guidance.as_ref())
        .map(|guidance| guidance.ranks.len());
    let is_within = |dimension: &u32| {
        u64::from(*dimension) < state.admission.bounds.max_rank_dimensions
            && fixed_length.is_none_or(|length| {
                usize::try_from(*dimension).is_ok_and(|dimension| dimension < length)
            })
    };
    if !exploration_order.iter().all(is_within) {
        return Err(campaign_core::CampaignError::InvalidConfig);
    }
    debug_assert!(exploration_order.iter().all(is_within));
    Ok(())
}

/// Ranked mode takes no ticket; exploratory mode takes one exact ticket from the admitted source.
pub(super) fn check_entropy(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
) -> campaign_core::CampaignResult<()> {
    let campaign_core::SelectionMode::Exploratory {
        entropy_source_id, ..
    } = &request.profile.mode
    else {
        return if request.entropy.is_none() {
            Ok(())
        } else {
            Err(campaign_core::CampaignError::InvalidEntropy)
        };
    };
    let ticket = request
        .entropy
        .as_ref()
        .ok_or(campaign_core::CampaignError::InvalidEntropy)?;
    if *entropy_source_id != state.admission.entropy_profile_id
        || ticket.source_id != *entropy_source_id
    {
        return Err(campaign_core::CampaignError::InvalidEntropy);
    }
    debug_assert_eq!(ticket.source_id, state.admission.entropy_profile_id);
    debug_assert!(request.entropy.is_some());
    ticket.validate(
        state.admission.campaign_id,
        state.state_id,
        request.profile.profile_id,
    )
}

/// Pass one: every candidate is examined; retained, guided candidates whose costs fit are
/// eligible. Returns the eligible candidates and the examined count.
pub(super) fn eligible<'a>(
    state: &'a campaign_core::FrontierState,
    decay: &campaign_core::SelectionCountDecay,
) -> campaign_core::CampaignResult<(alloc::vec::Vec<Eligible<'a>>, u64)> {
    let mut eligible = alloc::vec::Vec::with_capacity(state.candidates.len());
    let mut examined: u64 = 0;
    for candidate in &state.candidates {
        examined = examined
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
        let Some(guidance) = candidate.latest_guidance.as_ref() else {
            continue;
        };
        let is_retained = candidate.status == campaign_core::CandidateStatus::Retained;
        if is_retained && super::budget::blocking_dimensions(state, candidate)?.is_empty() {
            eligible.push(Eligible {
                candidate,
                guidance,
                effective_ranks: decayed(&guidance.ranks, candidate.selection_count, decay)?,
            });
        }
    }
    debug_assert!(eligible.len() <= state.candidates.len());
    Ok((eligible, examined))
}

/// Declared selection-count decay: the first rank drops by the penalty per prior selection; a
/// decayed guidance without a first rank has no declared shape.
fn decayed(
    ranks: &[i64],
    selections: u64,
    decay: &campaign_core::SelectionCountDecay,
) -> campaign_core::CampaignResult<alloc::vec::Vec<i64>> {
    let mut ranks = ranks.to_vec();
    let campaign_core::SelectionCountDecay::SubtractFromFirstRank {
        penalty_per_selection,
    } = decay
    else {
        return Ok(ranks);
    };
    if selections == 0 {
        return Ok(ranks);
    }
    let penalty = penalty_per_selection
        .checked_mul(selections)
        .and_then(|penalty| i64::try_from(penalty).ok())
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    debug_assert!(selections > 0);
    debug_assert!(penalty >= 0);
    let first = ranks
        .first_mut()
        .ok_or(campaign_core::CampaignError::InvalidGuidanceShape)?;
    *first = first
        .checked_sub(penalty)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    Ok(ranks)
}

/// Pass two: the best decayed ranks win (candidate identity breaks ties), unless the explicit
/// ticket takes the explore branch of the declared exploration order.
///
/// # Errors
///
/// Returns `InvalidGuidanceShape` when the explore branch projects a candidate's guidance onto a
/// dimension it does not carry.
pub(super) fn choose<'e, 'a>(
    eligible: &'e [Eligible<'a>],
    request: &campaign_core::SelectionRequest,
) -> campaign_core::CampaignResult<&'e Eligible<'a>> {
    let (ordered, explored) = match (&request.profile.mode, request.entropy.as_ref()) {
        (
            campaign_core::SelectionMode::Exploratory {
                threshold_parts_per_million,
                exploration_order,
                ..
            },
            Some(ticket),
        ) if ticket.explore_draw() % campaign_core::ENTROPY_SCALE
            < *threshold_parts_per_million =>
        {
            let mut keyed = alloc::vec::Vec::with_capacity(eligible.len());
            for item in eligible {
                keyed.push((projected(item, exploration_order)?, item));
            }
            keyed.sort_by(|left, right| {
                right
                    .0
                    .cmp(&left.0)
                    .then_with(|| id(left.1).cmp(&id(right.1)))
            });
            let ordered: alloc::vec::Vec<&Eligible<'a>> =
                keyed.into_iter().map(|(_, item)| item).collect();
            (ordered, ticket.index_draw())
        }
        (
            campaign_core::SelectionMode::Ranked | campaign_core::SelectionMode::Exploratory { .. },
            _,
        ) => {
            let mut ordered: alloc::vec::Vec<&Eligible<'a>> = eligible.iter().collect();
            ordered.sort_by(|left, right| {
                right
                    .effective_ranks
                    .cmp(&left.effective_ranks)
                    .then_with(|| id(left).cmp(&id(right)))
            });
            (ordered, 0)
        }
    };
    debug_assert_eq!(ordered.len(), eligible.len());
    debug_assert!(!ordered.is_empty());
    let len = u64::try_from(ordered.len())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let position = explored
        .checked_rem(len)
        .and_then(|position| usize::try_from(position).ok())
        .ok_or(campaign_core::CampaignError::MissingCandidate)?;
    ordered
        .get(position)
        .copied()
        .ok_or(campaign_core::CampaignError::MissingCandidate)
}

fn id(eligible: &Eligible<'_>) -> campaign_core::Id {
    eligible.candidate.candidate.candidate_id
}

/// The candidate's undecayed guidance ranks projected onto the declared dimensions, in order; a
/// dimension the guidance does not carry has no projection.
fn projected(
    eligible: &Eligible<'_>,
    order: &[u32],
) -> campaign_core::CampaignResult<alloc::vec::Vec<i64>> {
    let mut ranks = alloc::vec::Vec::with_capacity(order.len());
    for dimension in order {
        let rank = usize::try_from(*dimension)
            .ok()
            .and_then(|dimension| eligible.guidance.ranks.get(dimension))
            .ok_or(campaign_core::CampaignError::InvalidGuidanceShape)?;
        ranks.push(*rank);
    }
    debug_assert_eq!(ranks.len(), order.len());
    Ok(ranks)
}

/// Builds the selection of the chosen candidate against the current projection.
pub(super) fn selection(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
    current_projection: &campaign_core::ProjectionBinding,
    chosen: &Eligible<'_>,
) -> campaign_core::CampaignResult<campaign_core::SelectionDecision> {
    let record = &chosen.candidate.candidate;
    let selection = campaign_core::SelectionDecision {
        selection_id: campaign_core::Id::absent(),
        ordinal: state
            .selection_count
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidate_id: record.candidate_id,
        parent_moment_id: record.parent_moment_id,
        operation_id: campaign_core::Id::absent(),
        guidance_id: chosen.guidance.guidance_id,
        profile_id: request.profile.profile_id,
        entropy_id: request.entropy.as_ref().map(|ticket| ticket.entropy_id),
        reserved_costs: record.declared_costs.clone(),
        publication_plan: campaign_core::PublicationPlan {
            event_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            control_branch_id: current_projection.control_branch_id,
            expected_head_id: current_projection.head_id,
            expected_generation: current_projection.generation,
            state_before_id: state.state_id,
            selection_id: campaign_core::Id::absent(),
        },
    }
    .derive()?;
    debug_assert_eq!(
        selection.publication_plan.selection_id,
        selection.selection_id
    );
    debug_assert!(selection.ordinal > state.selection_count);
    Ok(selection)
}
