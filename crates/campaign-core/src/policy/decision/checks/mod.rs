//! Request, profile, and entropy checks that precede every planned decision.

/// Checks the declared profile against the state: content identity, production implementation,
/// decay representable under the admitted bounds, and exploration dimensions within the rank bound
/// and the campaign's fixed rank shape.
// r[impl campaign.fd.oracle.isolated]
pub(crate) fn validate_profile(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
) -> crate::error::CampaignResult<()> {
    profile.validate()?;
    crate::discipline::select_production_policy(&profile.implementation)?;
    profile
        .decay
        .validate_within_bounds(&state.admission.bounds)?;
    validate_exploration_order(state, &profile.mode)
}

// r[impl campaign.history_projection]
// r[impl campaign.history_projection.caller_binding]
pub(super) fn validate_selection_request(
    state: &crate::frontier::FrontierState,
    request: &crate::model::SelectionRequest,
    current_projection: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<()> {
    request
        .request_id
        .require_kind(crate::identity::IdKind::Request)?;
    validate_profile(state, &request.profile)?;
    current_projection.validate()?;
    if current_projection.projection_schema_id != state.admission.projection.projection_schema_id
        || current_projection.adapter_id != state.admission.adapter_id
        || current_projection.control_domain_id != state.admission.control_domain_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    if request.campaign_id != state.admission.campaign_id || request.state_id != state.state_id {
        return Err(crate::error::CampaignError::StaleState);
    }
    if request.expected_generation != current_projection.generation
        || request.expected_head_id != current_projection.head_id
    {
        return Err(crate::error::CampaignError::StaleProjection);
    }
    debug_assert_eq!(request.state_id, state.state_id);
    debug_assert_eq!(request.expected_generation, current_projection.generation);
    Ok(())
}

// r[impl campaign.guidance.declared_order]
fn validate_exploration_order(
    state: &crate::frontier::FrontierState,
    mode: &crate::model::SelectionMode,
) -> crate::error::CampaignResult<()> {
    let crate::model::SelectionMode::Exploratory {
        exploration_order, ..
    } = mode
    else {
        return Ok(());
    };
    crate::policy::validation::validate_declared_rank_dimensions(state, exploration_order)
}

/// Ranked mode accepts no ticket; exploratory mode requires one exact ticket from the campaign's
/// admitted entropy profile.
// r[impl campaign.entropy]
// r[impl campaign.entropy.invalid]
pub(super) fn validate_entropy(
    state: &crate::frontier::FrontierState,
    profile: &crate::model::SelectionProfile,
    entropy: Option<&crate::model::EntropyTicket>,
) -> crate::error::CampaignResult<()> {
    let crate::model::SelectionMode::Exploratory {
        entropy_source_id, ..
    } = profile.mode
    else {
        if entropy.is_some() {
            return Err(crate::error::CampaignError::InvalidEntropy);
        }
        return Ok(());
    };
    if entropy_source_id != state.admission.entropy_profile_id {
        return Err(crate::error::CampaignError::InvalidEntropy);
    }
    let entropy = entropy.ok_or(crate::error::CampaignError::InvalidEntropy)?;
    entropy.validate(
        state.admission.campaign_id,
        state.state_id,
        profile.profile_id,
    )?;
    if entropy.source_id != entropy_source_id {
        return Err(crate::error::CampaignError::InvalidEntropy);
    }
    debug_assert_eq!(entropy.source_id, state.admission.entropy_profile_id);
    debug_assert_eq!(entropy.state_id, state.state_id);
    Ok(())
}
