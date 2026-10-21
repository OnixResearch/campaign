//! `campaign.cache` controls at the core level.

const ONE_EVENT: u64 = 1;

fn rebinding(
    binding: &campaign_core::ProjectionBinding,
    change: impl FnOnce(&mut campaign_core::ProjectionBinding),
) -> Result<campaign_core::ProjectionBinding, String> {
    let mut changed = binding.clone();
    change(&mut changed);
    crate::suite::world::observed(changed.derive())
}

fn rederived(
    mut cache: campaign_core::CacheProjection,
) -> Result<campaign_core::CacheProjection, String> {
    cache.cache_id = crate::suite::world::observed(campaign_core::derive_cache_id(&cache))?;
    Ok(cache)
}

#[test]
// r[verify campaign.cache.valid]
fn a_fresh_binding_is_accepted() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let binding = world.state.admission.projection.clone();
    let cache = crate::suite::world::observed(campaign_core::derive_cache(&world.state, &binding))?;
    assert_eq!(cache.cache_id.kind, campaign_core::IdKind::Cache);
    assert_eq!(cache.binding, binding);
    assert_eq!(cache.state_id, world.state.state_id);
    assert_eq!(cache.policy_id, world.state.admission.policy_id);
    assert_eq!(cache.adapter_id, world.state.admission.adapter_id);
    assert_eq!(cache.control_event_count, world.state.control_event_count);
    let validated = crate::suite::world::observed(campaign_core::validate_cache(
        &cache,
        &binding,
        &world.events,
    ))?;
    assert_eq!(validated, world.state);
    assert_eq!(
        crate::suite::world::observed(campaign_core::derive_cache(&world.state, &binding))?,
        cache
    );
    Ok(())
}

#[test]
// r[verify campaign.cache.invalid]
fn a_stale_head_generation_or_graph_is_rejected_separately() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let fresh = world.state.admission.projection.clone();
    let stale_head = rebinding(&fresh, |binding| {
        binding.head_id = crate::suite::admission::xid(campaign_core::IdKind::Head, b"stale-head");
    })?;
    let stale_generation = rebinding(&fresh, |binding| {
        binding.generation = crate::suite::constants::GENERATION_NEXT;
    })?;
    let other_graph = rebinding(&fresh, |binding| {
        binding.graph_id =
            crate::suite::admission::xid(campaign_core::IdKind::Graph, b"other-graph");
    })?;
    for stale in [stale_head, stale_generation, other_graph] {
        assert_ne!(stale, fresh);
        let cache =
            crate::suite::world::observed(campaign_core::derive_cache(&world.state, &stale))?;
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_cache(
                &cache,
                &fresh,
                &world.events
            ))?,
            campaign_core::CampaignError::InvalidCache
        );
    }
    Ok(())
}

#[test]
// r[verify campaign.cache.invalid]
fn another_policy_or_adapter_is_rejected_separately() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let fresh = world.state.admission.projection.clone();
    let cache = crate::suite::world::observed(campaign_core::derive_cache(&world.state, &fresh))?;
    let mut other_policy = cache.clone();
    other_policy.policy_id =
        crate::suite::admission::xid(campaign_core::IdKind::Policy, b"other-policy");
    let mut other_adapter = cache.clone();
    other_adapter.adapter_id =
        crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"other-adapter");
    for crossed in [rederived(other_policy)?, rederived(other_adapter)?] {
        assert_ne!(crossed.cache_id, cache.cache_id);
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_cache(
                &crossed,
                &fresh,
                &world.events
            ))?,
            campaign_core::CampaignError::InvalidCache
        );
    }
    let mut policy_input = crate::suite::admission::default_input()?;
    policy_input.policy_id =
        crate::suite::admission::xid(campaign_core::IdKind::Policy, b"other-campaign-policy");
    let (_other_events, other_state) = crate::suite::admission::admitted(policy_input)?;
    let other_campaign_cache =
        crate::suite::world::observed(campaign_core::derive_cache(&other_state, &fresh))?;
    let (initial_events, _initial_state) = crate::suite::admission::initial_state()?;
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_cache(
            &other_campaign_cache,
            &fresh,
            &initial_events
        ))?,
        campaign_core::CampaignError::InvalidCache
    );
    Ok(())
}

#[test]
fn a_missing_control_event_is_rejected() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let fresh = world.state.admission.projection.clone();
    let behind =
        crate::suite::world::observed(campaign_core::derive_cache(&world.settled_state, &fresh))?;
    assert_eq!(
        behind.control_event_count.checked_add(ONE_EVENT),
        Some(world.state.control_event_count)
    );
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_cache(
            &behind,
            &fresh,
            &world.events
        ))?,
        campaign_core::CampaignError::InvalidCache
    );
    let mut forged_count =
        crate::suite::world::observed(campaign_core::derive_cache(&world.settled_state, &fresh))?;
    forged_count.control_event_count = world.state.control_event_count;
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_cache(
            &rederived(forged_count)?,
            &fresh,
            &world.events
        ))?,
        campaign_core::CampaignError::InvalidCache
    );
    Ok(())
}

#[test]
fn a_forged_identity_or_binding_is_rejected() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    let fresh = world.state.admission.projection.clone();
    let mut forged_id =
        crate::suite::world::observed(campaign_core::derive_cache(&world.state, &fresh))?;
    forged_id.cache_id = crate::suite::admission::xid(campaign_core::IdKind::Cache, b"forged");
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_cache(
            &forged_id,
            &fresh,
            &world.events
        ))?,
        campaign_core::CampaignError::InvalidCache
    );
    let mut forged_binding = fresh.clone();
    forged_binding.generation = crate::suite::constants::GENERATION_NEXT;
    let cache = crate::suite::world::observed(campaign_core::derive_cache(&world.state, &fresh))?;
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_cache(
            &cache,
            &forged_binding,
            &world.events
        ))?,
        campaign_core::CampaignError::StaleProjection
    );
    let crossed_adapter = rebinding(&fresh, |binding| {
        binding.adapter_id =
            crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"crossed-adapter");
    })?;
    assert_eq!(
        crate::suite::world::denial(campaign_core::derive_cache(&world.state, &crossed_adapter))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut stale_state = world.state.clone();
    stale_state.state_id = world.settled_state.state_id;
    assert_eq!(
        crate::suite::world::denial(campaign_core::derive_cache(&stale_state, &fresh))?,
        campaign_core::CampaignError::StaleState
    );
    Ok(())
}
