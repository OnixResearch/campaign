/// Derives a cache for `state` computed from the exact Choregraph projection `binding`.
///
/// # Errors
///
/// Returns `StaleState` when the state identity is not current, the binding's typed error when it
/// is malformed, or `CrossedIdentity` when the binding names another schema, adapter, or domain.
pub fn derive_cache(
    state: &crate::frontier::FrontierState,
    binding: &crate::model::ProjectionBinding,
) -> crate::error::CampaignResult<crate::model::CacheProjection> {
    crate::policy::validation::assert_state_identity_current(state)?;
    binding.validate()?;
    if binding.projection_schema_id != state.admission.projection.projection_schema_id
        || binding.adapter_id != state.admission.adapter_id
        || binding.control_domain_id != state.admission.control_domain_id
    {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let mut cache = crate::model::CacheProjection {
        cache_id: crate::identity::Id::absent(),
        binding: binding.clone(),
        campaign_id: state.admission.campaign_id,
        policy_id: state.admission.policy_id,
        adapter_id: state.admission.adapter_id,
        state_id: state.state_id,
        control_event_count: state.control_event_count,
    };
    cache.cache_id = crate::model::derive_cache_id(&cache)?;
    debug_assert_eq!(cache.cache_id.kind, crate::identity::IdKind::Cache);
    debug_assert_eq!(&cache.binding, binding);
    Ok(cache)
}

/// Validates a cache against a fresh authoritative binding and the authoritative control events.
///
/// Returns the reconstructed state the cache stands for.
///
/// # Errors
///
/// Returns `InvalidCache` when the cache binds another projection (head, generation, graph,
/// branch, schema, adapter, or domain), another campaign, policy, adapter, or state, another event
/// count, or a forged identity; history errors from reconstruction are returned unchanged.
// r[impl campaign.cache]
pub fn validate_cache(
    cache: &crate::model::CacheProjection,
    fresh_binding: &crate::model::ProjectionBinding,
    events: &[crate::model::CampaignControlEvent],
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    fresh_binding.validate()?;
    if &cache.binding != fresh_binding {
        return Err(crate::error::CampaignError::InvalidCache);
    }
    let state = crate::frontier::reconstruct(events)?;
    validate_cache_for_state(cache, fresh_binding, &state)?;
    Ok(state)
}

/// Validates a cache against a fresh binding and a state already reconstructed from authoritative
/// history (for adapters that reconstructed while validating the history projection).
///
/// # Errors
///
/// Returns `InvalidCache` for any cache fact that differs from the fresh derivation.
// r[impl campaign.cache.invalid]
pub fn validate_cache_for_state(
    cache: &crate::model::CacheProjection,
    fresh_binding: &crate::model::ProjectionBinding,
    authoritative_state: &crate::frontier::FrontierState,
) -> crate::error::CampaignResult<()> {
    if &cache.binding != fresh_binding {
        return Err(crate::error::CampaignError::InvalidCache);
    }
    let expected = derive_cache(authoritative_state, fresh_binding)?;
    if cache != &expected {
        return Err(crate::error::CampaignError::InvalidCache);
    }
    debug_assert_eq!(cache.state_id, authoritative_state.state_id);
    debug_assert_eq!(
        cache.control_event_count,
        authoritative_state.control_event_count
    );
    Ok(())
}
