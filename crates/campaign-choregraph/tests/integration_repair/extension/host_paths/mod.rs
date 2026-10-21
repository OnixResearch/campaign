//! A host that holds a validated projection publishes and validates caches only through it, so a
//! branch moved onto a fork that drops an accepted stop is refused on every host path.

fn cache_request<'a>(
    history: &'a crate::suite::history::Fixture,
    choices: &'a choregraph_history::ParentChoices,
    previous: Option<&'a campaign_choregraph::ValidatedControlProjection>,
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
        previous,
    }
}

/// Post-stop work planned on the fork never publishes: publication always advances the held stop
/// projection (the cold-start `previous: None` path no longer exists), which refuses the fork.
#[test]
fn post_stop_work_on_a_fork_jump_cannot_publish_through_the_held_stop() -> Result<(), String> {
    let fork = super::sibling::fork_over_stop(b"held-publication-stop")?;
    let cold = fork.jumped.current()?;
    assert!(cold.state().stopped.is_none());
    let post_stop = crate::suite::selection::select_from_current(&cold)?;
    assert_eq!(
        fork.jumped.publish_from(
            &fork.stopped_view,
            crate::suite::selection::event(&post_stop)
        ),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let on_stopped = crate::suite::selection::event(&post_stop);
    assert_eq!(
        fork.stopped.publish_from(&fork.stopped_view, on_stopped),
        Err(campaign_choregraph::AdapterError::PublicationFenceMismatch)
    );
    assert!(fork.stopped_view.state().stopped.is_some());
    Ok(())
}

/// A cache of the forked state never validates through the held stop projection; only a
/// cold-start validation (a host without a held projection) reconstructs the fork.
#[test]
fn cache_validation_through_the_held_stop_refuses_the_fork_jump() -> Result<(), String> {
    let fork = super::sibling::fork_over_stop(b"held-cache-stop")?;
    let choices = choregraph_history::ParentChoices::new();
    let binding = crate::suite::identity::observed(fork.jumped.derived_binding())?;
    let forked_state = fork.jumped.current()?.state().clone();
    let cache =
        crate::suite::identity::observed(campaign_core::derive_cache(&forked_state, &binding))?;
    assert_eq!(
        campaign_choregraph::validate_cache_against_history(
            &cache_request(&fork.jumped, &choices, Some(&fork.stopped_view)),
            &cache,
        ),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    let cold = campaign_choregraph::validate_cache_against_history(
        &cache_request(&fork.jumped, &choices, None),
        &cache,
    );
    assert_eq!(cold.map(|state| state.stopped.is_none()), Ok(true));

    let stopped_binding = crate::suite::identity::observed(fork.stopped.derived_binding())?;
    let stopped_cache = crate::suite::identity::observed(campaign_core::derive_cache(
        fork.stopped_view.state(),
        &stopped_binding,
    ))?;
    assert_eq!(
        campaign_choregraph::validate_cache_against_history(
            &cache_request(&fork.stopped, &choices, Some(&fork.stopped_view)),
            &stopped_cache,
        ),
        Ok(fork.stopped_view.state().clone())
    );
    Ok(())
}

/// A cache of an honest extension validates through an older held projection and equals the
/// cold-start answer.
#[test]
fn cache_validation_advances_an_older_held_projection() -> Result<(), String> {
    let fork = super::sibling::fork_over_stop(b"held-older-cache-stop")?;
    let choices = choregraph_history::ParentChoices::new();
    let binding = crate::suite::identity::observed(fork.stopped.derived_binding())?;
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        fork.stopped_view.state(),
        &binding,
    ))?;
    let advanced = campaign_choregraph::validate_cache_against_history(
        &cache_request(&fork.stopped, &choices, Some(&fork.before_view)),
        &cache,
    );
    assert_eq!(advanced, Ok(fork.stopped_view.state().clone()));
    assert_eq!(
        advanced,
        campaign_choregraph::validate_cache_against_history(
            &cache_request(&fork.stopped, &choices, None),
            &cache,
        )
    );
    Ok(())
}

/// Fast-forward is relative to what the host validated: a projection that predates the dropped
/// stop is extended by the fork, while the projection that holds the stop refuses it.
#[test]
fn only_the_projection_holding_the_stop_refuses_the_fork() -> Result<(), String> {
    let fork = super::sibling::fork_over_stop(b"held-older-stop")?;
    let advanced = fork.jumped.advance(&fork.before_view);
    assert_eq!(
        advanced.map(|view| view.state().stopped.is_none()),
        Ok(true)
    );
    assert_eq!(
        fork.jumped.advance(&fork.stopped_view),
        Err(campaign_choregraph::AdapterError::NonFastForwardBranch)
    );
    assert_eq!(
        fork.forked.advance(&fork.before_view),
        Ok(fork.forked.current()?)
    );
    Ok(())
}
