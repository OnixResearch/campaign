//! Cache fixtures: current states under moved bindings, and the cache oracle matrix.

/// Cold presentation of the session's current history.
const COLD: crate::harness::oracle::Presentation = crate::harness::oracle::Presentation::Cold;

/// Presentation through the projection the session last loaded.
const HELD: crate::harness::oracle::Presentation =
    crate::harness::oracle::Presentation::ThroughHeld;

/// Presents two caches of the unchanged current state: one bound to the graph before a new
/// exploration moment was admitted, and one bound to another head of the current graph. The
/// adapter's cache validation must reject both bindings even though the state identity matches.
pub(super) fn moved_binding(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let before_moment = campaign_core::derive_cache(&session.state, &session.binding()?)?;
    let moment =
        session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    super::super::common::require(
        before_moment.state_id == session.state.state_id,
        "admitting an exploration moment leaves the Campaign state unchanged",
    )?;
    debug_assert_eq!(before_moment.state_id, session.state.state_id);
    let forbidden = "a cache bound to a superseded graph was used as current state";
    let moved_graph = super::validate_cache(&mut session, &before_moment, forbidden);
    session.harness.deny(
        moved_graph,
        campaign_core::CampaignError::InvalidCache,
        forbidden,
    )?;
    let other_head = campaign_choregraph::BranchRef {
        head: moment.event,
        ..session.branch.clone()
    };
    let other_binding = campaign_choregraph::derive_projection_binding(
        &session.graph,
        &other_head,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    let moved_head = campaign_core::derive_cache(&session.state, &other_binding)?;
    super::super::common::require(
        moved_head.state_id == session.state.state_id && moved_head.binding != session.binding()?,
        "the moved-head cache binds the current state to another head",
    )?;
    debug_assert_eq!(moved_head.state_id, session.state.state_id);
    let forbidden = "a cache bound to another head was used as current state";
    let validated = super::validate_cache(&mut session, &moved_head, forbidden);
    session.harness.deny(
        validated,
        campaign_core::CampaignError::InvalidCache,
        forbidden,
    )
}

/// Typed denial Campaign answers for a presented cache, and the forbidden effect it names.
type Declared = (campaign_choregraph::AdapterError, &'static str);

/// Presents one cache Campaign refuses with the declared denial; the adapter must refuse it
/// identically.
fn refuse(
    session: &mut crate::harness::Session<'_, '_>,
    history: &crate::harness::oracle::HistoryView,
    cache: &campaign_core::CacheProjection,
    presentation: crate::harness::oracle::Presentation,
    declared: Declared,
) -> crate::verdicts::ConformanceResult<()> {
    let (expected, forbidden) = declared;
    let validated = session.present_cache(history, cache, presentation, forbidden)?;
    session.harness.deny_with(
        validated.map_err(crate::verdicts::ConformanceError::from),
        forbidden,
        super::forged::adapter_denial(expected),
    )
}

/// Presents a stale state under its own and under the current binding and the current state
/// under a superseded binding; a fresh and a stale cache of the current history through a held
/// projection one event older than the head (the fresh one must stand for the current state);
/// and caches of the current, earlier, and replayed states of every history in the shared
/// catalog, cold, through the held projection one event older than the head (H0), and through the
/// held projection of the current head (H1); then caches of every presentation of the
/// held-lineage matrix through its held prefix.
pub(super) fn validation_matrix(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let (mut session, earlier) = super::histories::prepared(harness)?;
    let stale_binding = campaign_choregraph::derive_projection_binding(
        &earlier.head.history.graph,
        &earlier.head.history.branch,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    let current_binding = session.binding()?;
    debug_assert_ne!(current_binding, stale_binding);
    let current = session.history();
    let invalid =
        campaign_choregraph::AdapterError::Campaign(campaign_core::CampaignError::InvalidCache);
    let rebound = campaign_core::derive_cache(&earlier.head.state, &current_binding)?;
    let cases = [
        (
            campaign_core::derive_cache(&earlier.head.state, &stale_binding)?,
            "a cache of a stale state was used as current state",
        ),
        (
            rebound.clone(),
            "a stale state under the current binding was used as current state",
        ),
        (
            campaign_core::derive_cache(&session.state, &stale_binding)?,
            "the current state under a superseded binding was used as current state",
        ),
    ];
    for (cache, forbidden) in &cases {
        let declared = (invalid.clone(), *forbidden);
        refuse(&mut session, &current, cache, COLD, declared)?;
    }
    let fresh = campaign_core::derive_cache(&session.state, &current_binding)?;
    let validated = session.present_cache(&current, &fresh, HELD, FRESH)??;
    super::super::common::require(
        validated == session.state,
        "a fresh cache through a held projection older than the head stands for the current state",
    )?;
    refuse(
        &mut session,
        &current,
        &rebound,
        HELD,
        (
            invalid,
            "a stale state was validated through a held projection older than the head",
        ),
    )?;
    let catalog = super::histories::assembled(&mut session, &earlier)?;
    super::histories::presented::caches(&mut session, &catalog, &earlier)?;
    super::histories::held::caches(&mut session, &earlier)
}

/// Forbidden effect of a fresh cache the adapter validates to another answer than Campaign.
pub(super) const FRESH: &str = "a fresh cache stood for another state than Campaign's";
