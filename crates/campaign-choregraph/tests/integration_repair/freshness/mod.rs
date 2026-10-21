//! `campaign.cache` controls against authoritative Choregraph history.

const OTHER_GENERATION: u64 = 99;

fn request<'a>(
    history: &'a crate::suite::history::Fixture,
    choices: &'a choregraph_history::ParentChoices,
) -> campaign_choregraph::CacheValidationRequest<'a> {
    campaign_choregraph::CacheValidationRequest {
        graph: &history.graph,
        branch: &history.branch,
        parent_choices: choices,
        maximum_events: crate::suite::constants::MAX_EVENTS,
        projection_schema_id: crate::suite::identity::schema(),
        adapter_id: crate::suite::identity::adapter(),
        control_domain_id: crate::suite::identity::domain(),
        payloads: &history.payloads,
        previous: None,
    }
}

fn rejected(
    history: &crate::suite::history::Fixture,
    cache: &campaign_core::CacheProjection,
) -> Option<campaign_choregraph::AdapterError> {
    let choices = choregraph_history::ParentChoices::new();
    campaign_choregraph::validate_cache_against_history(&request(history, &choices), cache).err()
}

fn derived_at(
    current: &campaign_choregraph::ValidatedControlProjection,
    change: impl FnOnce(&mut campaign_core::ProjectionBinding),
) -> Result<campaign_core::CacheProjection, String> {
    let mut binding = current.binding().clone();
    change(&mut binding);
    let binding = crate::suite::identity::observed(binding.derive())?;
    crate::suite::identity::observed(campaign_core::derive_cache(current.state(), &binding))
}

fn rederived(
    mut cache: campaign_core::CacheProjection,
) -> Result<campaign_core::CacheProjection, String> {
    cache.cache_id = crate::suite::identity::observed(campaign_core::derive_cache_id(&cache))?;
    Ok(cache)
}

const INVALID_CACHE: Option<campaign_choregraph::AdapterError> = Some(
    campaign_choregraph::AdapterError::Campaign(campaign_core::CampaignError::InvalidCache),
);

#[test]
// r[verify campaign.cache.valid]
fn matching_fresh_history_is_accepted() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        current.state(),
        current.binding(),
    ))?;
    let choices = choregraph_history::ParentChoices::new();
    let state = crate::suite::identity::observed(
        campaign_choregraph::validate_cache_against_history(&request(&history, &choices), &cache),
    )?;
    assert_eq!(state, *current.state());
    assert_eq!(rejected(&history, &cache), None);
    Ok(())
}

#[test]
fn a_stale_head_generation_or_graph_is_rejected_separately() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let stale_head = derived_at(&current, |binding| {
        binding.head_id =
            crate::suite::identity::derive(campaign_core::IdKind::Head, b"stale-head");
    })?;
    let stale_generation = derived_at(&current, |binding| binding.generation = OTHER_GENERATION)?;
    let other_graph = derived_at(&current, |binding| {
        binding.graph_id =
            crate::suite::identity::derive(campaign_core::IdKind::Graph, b"other-graph");
    })?;
    for stale in [stale_head, stale_generation, other_graph] {
        assert_ne!(stale.binding, *current.binding());
        assert_eq!(stale.state_id, current.state().state_id);
        assert_eq!(rejected(&history, &stale), INVALID_CACHE);
    }
    Ok(())
}

#[test]
fn another_policy_or_adapter_is_rejected_separately() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let current = history.current()?;
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        current.state(),
        current.binding(),
    ))?;
    let mut other_policy = cache.clone();
    other_policy.policy_id =
        crate::suite::identity::derive(campaign_core::IdKind::Policy, b"other-policy");
    let mut other_adapter = cache;
    other_adapter.adapter_id =
        crate::suite::identity::derive(campaign_core::IdKind::Adapter, b"other-adapter");
    for crossed in [rederived(other_policy)?, rederived(other_adapter)?] {
        assert_eq!(crossed.binding, *current.binding());
        assert_eq!(rejected(&history, &crossed), INVALID_CACHE);
    }
    Ok(())
}

#[test]
// r[verify campaign.cache.invalid]
fn a_missing_published_control_event_is_rejected() -> Result<(), String> {
    let history = crate::suite::candidate::selectable()?;
    let before = history.current()?;
    let old_cache = crate::suite::identity::observed(campaign_core::derive_cache(
        before.state(),
        before.binding(),
    ))?;
    let selection = crate::suite::selection::select_from_current(&before)?;
    let (history, _publication) = history.append(crate::suite::selection::event(&selection))?;
    let after = history.current()?;
    assert_eq!(rejected(&history, &old_cache), INVALID_CACHE);
    let missing_event = crate::suite::identity::observed(campaign_core::derive_cache(
        before.state(),
        after.binding(),
    ))?;
    assert_eq!(missing_event.binding, *after.binding());
    assert_ne!(
        missing_event.control_event_count,
        after.state().control_event_count
    );
    assert_eq!(rejected(&history, &missing_event), INVALID_CACHE);
    let fresh = crate::suite::identity::observed(campaign_core::derive_cache(
        after.state(),
        after.binding(),
    ))?;
    assert_eq!(rejected(&history, &fresh), None);
    Ok(())
}
