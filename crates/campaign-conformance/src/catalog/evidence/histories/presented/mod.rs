//! Presentation of the shared history catalog to the adapter's load and cache paths.

/// The projections the catalog presents through: the earlier held projection (H0) and the held
/// projection of the current head (H1).
struct Held<'p> {
    earlier: &'p campaign_choregraph::ValidatedControlProjection,
    current: Option<campaign_choregraph::ValidatedControlProjection>,
}

impl<'p> Held<'p> {
    /// H0 from `earlier` and H1 from the projection the session last loaded.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` when the session never loaded a projection.
    fn of(
        session: &crate::harness::Session<'_, '_>,
        earlier: &'p super::super::lineages::forks::Earlier,
    ) -> crate::verdicts::ConformanceResult<Self> {
        let current = session
            .presented_projection(crate::harness::oracle::Presentation::ThroughHeld)?
            .cloned();
        debug_assert!(current.is_some());
        Ok(Self {
            earlier: &earlier.head.view,
            current,
        })
    }

    /// The host presenting `entry` under `holder`.
    const fn host(
        &self,
        entry: &super::Entry,
        holder: super::Holder,
    ) -> crate::harness::oracle::hosted::Host<'_> {
        let previous = if matches!(holder, super::Holder::Cold) {
            None
        } else if matches!(holder, super::Holder::Earlier) {
            Some(self.earlier)
        } else {
            self.current.as_ref()
        };
        crate::harness::oracle::hosted::Host {
            maximum_events: entry.maximum_events,
            previous,
        }
    }
}

/// Presents every catalog history to the adapter's load path under each of its holders (cold,
/// through H0, and through H1); each answer must equal Campaign's declared answer.
///
/// # Errors
///
/// Returns the oracle mismatch of a presentation and `Diverged` when Campaign's own answer is
/// not the declared one.
pub(in crate::catalog::evidence) fn loads(
    session: &mut crate::harness::Session<'_, '_>,
    catalog: &[super::Entry],
    earlier: &super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(!catalog.is_empty());
    let held = Held::of(session, earlier)?;
    for entry in catalog {
        for holder in entry.holders.iter().copied() {
            present_load(
                session,
                entry,
                held.host(entry, holder),
                entry.answer(holder),
            )?;
        }
    }
    Ok(())
}

/// Presents one history to the adapter's load path under `host`; the answer must equal
/// Campaign's declared answer.
///
/// # Errors
///
/// Returns the oracle mismatch of the presentation and `Diverged` when Campaign's own answer is
/// not the declared one.
pub(in crate::catalog::evidence) fn present_load(
    session: &mut crate::harness::Session<'_, '_>,
    entry: &super::Entry,
    host: crate::harness::oracle::hosted::Host<'_>,
    declared: &super::Answer,
) -> crate::verdicts::ConformanceResult<()> {
    let loaded = session.present_hosted_history(
        &entry.history,
        entry.binding.clone(),
        host,
        entry.loaded,
    )?;
    super::super::super::common::require(
        declared.is_answer(&loaded),
        "Campaign answers every catalog history as declared",
    )?;
    debug_assert!(declared.is_answer(&loaded));
    if loaded.is_err() {
        session.harness.deny_with(
            loaded.map_err(crate::verdicts::ConformanceError::from),
            entry.loaded,
            |error| match error {
                crate::verdicts::ConformanceError::Adapter(error) => {
                    declared.is_answer::<()>(&Err(error.clone()))
                }
                _ => false,
            },
        )?;
    }
    Ok(())
}

/// Presents caches of the state each history's payloads replay to, of the current state, and of
/// the earlier state, each bound to the history's binding, to the adapter's cache validation
/// under each of the history's holders (cold, through H0, and through H1); each answer must
/// equal Campaign's, and Campaign accepts a cache exactly when the history loads to the cached
/// state.
///
/// # Errors
///
/// Returns the oracle mismatch of a presentation and `Diverged` when Campaign's own answer
/// contradicts its load answer.
pub(in crate::catalog::evidence) fn caches(
    session: &mut crate::harness::Session<'_, '_>,
    catalog: &[super::Entry],
    earlier: &super::super::lineages::forks::Earlier,
) -> crate::verdicts::ConformanceResult<()> {
    let held = Held::of(session, earlier)?;
    for entry in catalog {
        let anchors = [
            &earlier.before.view,
            &earlier.head.view,
            held.current.as_ref().unwrap_or(held.earlier),
        ];
        let states = super::cached::states(session, entry, &earlier.head.state, &anchors);
        for holder in entry.holders.iter().copied() {
            let host = held.host(entry, holder);
            present_caches(session, entry, host, entry.answer(holder), &states)?;
        }
    }
    Ok(())
}

/// Presents caches of each of `states` bound to the entry's binding under `host`; Campaign's
/// load answer must be the declared one, and each cache answer must equal Campaign's.
///
/// # Errors
///
/// Returns the oracle mismatch of a presentation and `Diverged` when Campaign's own answer
/// contradicts its declared load answer.
pub(in crate::catalog::evidence) fn present_caches(
    session: &mut crate::harness::Session<'_, '_>,
    entry: &super::Entry,
    host: crate::harness::oracle::hosted::Host<'_>,
    declared: &super::Answer,
    states: &[campaign_core::FrontierState],
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(!states.is_empty());
    let loaded = crate::harness::oracle::campaign_load(&load_request(entry), host.previous);
    super::super::super::common::require(
        declared.is_answer(&loaded),
        "Campaign answers every catalog history as declared",
    )?;
    let loaded_state = loaded
        .as_ref()
        .ok()
        .map(|projection| projection.state().state_id);
    for state in states {
        present_cache(session, entry, host, state, loaded_state)?;
    }
    Ok(())
}

/// Presents one cache of `state` bound to the entry's binding under `host`; Campaign accepts it
/// exactly when the history loads (to `loaded_state`) and `state` is that state, and the adapter
/// must answer as Campaign.
///
/// # Errors
///
/// Returns the oracle mismatch of the presentation and `Diverged` when Campaign's own answer
/// contradicts its load answer.
fn present_cache(
    session: &mut crate::harness::Session<'_, '_>,
    entry: &super::Entry,
    host: crate::harness::oracle::hosted::Host<'_>,
    state: &campaign_core::FrontierState,
    loaded_state: Option<campaign_core::Id>,
) -> crate::verdicts::ConformanceResult<()> {
    let cache = cache_of(session, entry, state)?;
    let request = cache_request(entry, host);
    let validated = session.present_cache_request(&request, &cache, entry.cached)?;
    let is_consistent = validated.is_ok() == (loaded_state == Some(state.state_id));
    super::super::super::common::require(
        is_consistent,
        "Campaign accepts a catalog cache exactly when the history loads to its state",
    )?;
    debug_assert!(is_consistent);
    if let Err(error) = validated {
        let expected = crate::verdicts::ConformanceError::Adapter(error.clone());
        session.harness.deny_with(
            Err::<(), _>(crate::verdicts::ConformanceError::Adapter(error)),
            entry.cached,
            |denial| *denial == expected,
        )?;
    }
    Ok(())
}

/// The cache of `state` a host holding the entry's binding presents: derived by Campaign when
/// the binding names the state's admitted envelope identities, otherwise Campaign's cache of the
/// same history under the admitted identities rebound to the entry's binding (a host that
/// believes the foreign identities can only forge it).
///
/// # Errors
///
/// Returns the Campaign or adapter error of the derivation.
fn cache_of(
    session: &crate::harness::Session<'_, '_>,
    entry: &super::Entry,
    state: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<campaign_core::CacheProjection> {
    let admitted = &state.admission.projection;
    let is_admitted = entry.binding.projection_schema_id == admitted.projection_schema_id
        && entry.binding.adapter_id == admitted.adapter_id
        && entry.binding.control_domain_id == admitted.control_domain_id;
    if is_admitted {
        return Ok(campaign_core::derive_cache(state, &entry.binding)?);
    }
    let own = campaign_choregraph::derive_projection_binding(
        &entry.history.graph,
        &entry.history.branch,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    let mut cache = campaign_core::derive_cache(state, &own)?;
    cache.binding = entry.binding.clone();
    cache.cache_id = campaign_core::derive_cache_id(&cache)?;
    debug_assert_eq!(cache.state_id, state.state_id);
    debug_assert_eq!(cache.binding, entry.binding);
    Ok(cache)
}

/// The cache validation request of one catalog entry under `host`: the envelope identities the
/// entry's binding names.
fn cache_request<'r>(
    entry: &'r super::Entry,
    host: crate::harness::oracle::hosted::Host<'r>,
) -> campaign_choregraph::CacheValidationRequest<'r> {
    campaign_choregraph::CacheValidationRequest {
        graph: &entry.history.graph,
        branch: &entry.history.branch,
        parent_choices: &entry.history.parent_choices,
        maximum_events: host.maximum_events,
        projection_schema_id: entry.binding.projection_schema_id,
        adapter_id: entry.binding.adapter_id,
        control_domain_id: entry.binding.control_domain_id,
        payloads: &entry.history.payloads,
        previous: host.previous,
    }
}

/// Campaign's projection request of one catalog entry.
fn load_request(entry: &super::Entry) -> campaign_choregraph::ControlProjectionRequest<'_> {
    campaign_choregraph::ControlProjectionRequest {
        graph: &entry.history.graph,
        branch: &entry.history.branch,
        parent_choices: &entry.history.parent_choices,
        maximum_events: entry.maximum_events,
        expected_binding: entry.binding.clone(),
        payloads: &entry.history.payloads,
    }
}
