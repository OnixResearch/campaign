const LABEL_CACHE: &[u8] = b"derived-cache-v2";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Derived frontier cache bound to the exact Choregraph projection it was computed from.
// r[impl campaign.cache]
pub struct CacheProjection {
    /// Content-derived cache identity.
    pub cache_id: crate::identity::Id,
    /// Complete Choregraph projection binding (graph, branch, generation, head, schema, adapter, domain).
    pub binding: super::projection::ProjectionBinding,
    /// Campaign identity of the cached state.
    pub campaign_id: crate::identity::Id,
    /// Campaign policy identity of the cached state.
    pub policy_id: crate::identity::Id,
    /// Campaign adapter identity of the cached state.
    pub adapter_id: crate::identity::Id,
    /// Cached frontier state identity.
    pub state_id: crate::identity::Id,
    /// Number of control events folded into the cached state.
    pub control_event_count: u64,
}

/// Derives the cache identity from every bound cache fact.
///
/// # Errors
///
/// Returns a typed `CampaignError` when a bound identity is missing or has the wrong kind.
pub fn derive_cache_id(
    cache: &CacheProjection,
) -> crate::error::CampaignResult<crate::identity::Id> {
    cache
        .binding
        .projection_id
        .require_kind(crate::identity::IdKind::Projection)?;
    cache
        .campaign_id
        .require_kind(crate::identity::IdKind::Campaign)?;
    cache
        .policy_id
        .require_kind(crate::identity::IdKind::Policy)?;
    cache
        .adapter_id
        .require_kind(crate::identity::IdKind::Adapter)?;
    cache
        .state_id
        .require_kind(crate::identity::IdKind::State)?;
    debug_assert_eq!(
        cache.binding.projection_id.kind,
        crate::identity::IdKind::Projection
    );
    debug_assert_eq!(cache.state_id.kind, crate::identity::IdKind::State);
    crate::identity::derive_id_with(crate::identity::IdKind::Cache, LABEL_CACHE, |writer| {
        let binding = &cache.binding;
        Ok(writer
            .id(binding.projection_id)
            .id(binding.graph_id)
            .id(binding.control_branch_id)
            .u64(binding.generation)
            .id(binding.head_id)
            .id(binding.projection_schema_id)
            .id(binding.adapter_id)
            .id(binding.control_domain_id)
            .id(cache.campaign_id)
            .id(cache.policy_id)
            .id(cache.adapter_id)
            .id(cache.state_id)
            .u64(cache.control_event_count))
    })
}
