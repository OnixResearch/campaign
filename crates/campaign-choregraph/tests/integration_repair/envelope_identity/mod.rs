//! Control roots and envelopes are bound to the projection schema, adapter, and control domain
//! the campaign's admission names, on cold load, advance, publication, and cache validation.

/// Every crossing of the canonical envelope identities: each one alone, and all three.
fn crossings() -> [crate::suite::configured::Envelope; 4] {
    let canonical = crate::suite::configured::Envelope::canonical();
    let foreign = crate::suite::configured::Envelope::foreign();
    [
        crate::suite::configured::Envelope {
            schema: foreign.schema,
            ..canonical
        },
        crate::suite::configured::Envelope {
            adapter: foreign.adapter,
            ..canonical
        },
        crate::suite::configured::Envelope {
            domain: foreign.domain,
            ..canonical
        },
        foreign,
    ]
}

fn crossed() -> campaign_choregraph::AdapterError {
    campaign_choregraph::AdapterError::Campaign(campaign_core::CampaignError::CrossedIdentity)
}

/// The suite's admission drafted as a second control root of its graph under `envelope`, without
/// the adapter's root admission, as a host writing history directly could.
fn drafted_root(
    envelope: crate::suite::configured::Envelope,
) -> Result<crate::suite::configured::Hosted, String> {
    let canonical = crate::suite::admission::initialized()?;
    let admission = canonical.payloads[0].control_event.clone();
    let draft = crate::suite::identity::observed(campaign_choregraph::control_event_draft(
        &admission,
        envelope.schema,
        envelope.adapter,
        envelope.domain,
        None,
    ))?;
    let root = crate::suite::identity::observed(canonical.graph.admit(draft))?;
    assert_ne!(root.event, canonical.branch.head);
    assert!(root.graph.contains(canonical.branch.head));
    Ok(crate::suite::configured::Hosted {
        graph: root.graph,
        branch: choregraph_history::BranchRef {
            name: canonical.branch.name,
            head: root.event,
            generation: crate::suite::constants::ROOT_BRANCH_GENERATION,
        },
        payloads: vec![campaign_choregraph::ControlPayloadEnvelope {
            history_event_id: root.event,
            control_event: admission,
        }],
        envelope,
        maximum_events: crate::suite::constants::MAX_EVENTS,
    })
}

/// The suite's selectable history under its canonical host configuration.
fn canonical_selectable() -> Result<crate::suite::configured::Hosted, String> {
    let history = crate::suite::candidate::selectable()?;
    Ok(crate::suite::configured::Hosted {
        graph: history.graph,
        branch: history.branch,
        payloads: history.payloads,
        envelope: crate::suite::configured::Envelope::canonical(),
        maximum_events: crate::suite::constants::MAX_EVENTS,
    })
}

/// A control root is admitted only for a reconstructible `CampaignAdmitted` event under exactly
/// the identities its admission binds; one campaign therefore has one control root per graph.
#[test]
// r[verify campaign.initial_admission.invalid]
fn a_control_root_is_admitted_only_under_its_admissions_identities() -> Result<(), String> {
    let canonical = crate::suite::admission::initialized()?;
    let admission = canonical.payloads[0].control_event.clone();
    let rooted = crate::suite::configured::campaign::admitted_at(
        crate::suite::constants::ROOT_GENERATION,
        crate::suite::constants::MAX_EVENTS,
    )?;
    assert_eq!(rooted.branch, canonical.branch);
    assert_eq!(rooted.payloads, canonical.payloads);
    let graph = crate::suite::candidate::selectable()?.graph;
    let admit = |event: campaign_core::CampaignControlEvent,
                 envelope: crate::suite::configured::Envelope| {
        campaign_choregraph::admit_control_root(
            &graph,
            event,
            envelope.schema,
            envelope.adapter,
            envelope.domain,
        )
        .map(|root| root.envelope)
    };
    for envelope in crossings() {
        assert_eq!(
            admit(admission.clone(), envelope),
            Err(crossed()),
            "{envelope:?}"
        );
    }
    let batch = crate::suite::candidate::selectable()?.payloads[1]
        .control_event
        .clone();
    assert_eq!(
        admit(batch, crate::suite::configured::Envelope::canonical()),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::MissingAdmission
        ))
    );
    let campaign_core::CampaignControlEvent::CampaignAdmitted {
        admission: admitted,
        ..
    } = admission.clone()
    else {
        return Err(String::from("the suite's root is its admission"));
    };
    let forged = campaign_core::CampaignControlEvent::CampaignAdmitted {
        event_id: crate::suite::identity::derive(campaign_core::IdKind::Event, b"forged-admission"),
        admission: admitted,
    };
    assert_eq!(
        admit(forged, crate::suite::configured::Envelope::canonical()),
        Err(campaign_choregraph::AdapterError::Campaign(
            campaign_core::CampaignError::MalformedIdentity
        ))
    );
    Ok(())
}

/// A root drafted under crossed envelope identities never loads cold, as a cache, or as a
/// publication base, while the canonical root does.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_root_under_crossed_identities_never_loads_or_caches() -> Result<(), String> {
    let canonical = crate::suite::admission::initialized()?;
    let honest = crate::suite::configured::Hosted {
        graph: canonical.graph,
        branch: canonical.branch,
        payloads: canonical.payloads,
        envelope: crate::suite::configured::Envelope::canonical(),
        maximum_events: crate::suite::constants::MAX_EVENTS,
    };
    let held = crate::suite::identity::observed(honest.validate())?;
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        held.state(),
        held.binding(),
    ))?;
    assert_eq!(
        honest.validate_cache(None, &cache).as_ref(),
        Ok(held.state())
    );
    for envelope in crossings() {
        let crossed_root = drafted_root(envelope)?;
        assert_eq!(crossed_root.validate(), Err(crossed()), "{envelope:?}");
        let crossed_cache = campaign_core::CacheProjection {
            binding: crate::suite::identity::observed(crossed_root.binding())?,
            ..cache.clone()
        };
        assert_eq!(
            crossed_root.validate_cache(None, &crossed_cache),
            Err(crossed()),
            "{envelope:?}"
        );
    }
    Ok(())
}

/// A held canonical projection never advances, publishes, or validates a cache under crossed
/// envelope identities, so no selection is published or granted under them.
#[test]
// r[verify campaign.history_projection.stale]
// r[verify campaign.cache.invalid]
fn a_held_projection_never_advances_publishes_or_caches_under_crossed_identities()
-> Result<(), String> {
    let honest = canonical_selectable()?;
    let held = crate::suite::identity::observed(honest.validate())?;
    let selection = crate::suite::selection::select_from_current(&held)?;
    let event = crate::suite::selection::event(&selection);
    let cache = crate::suite::identity::observed(campaign_core::derive_cache(
        held.state(),
        held.binding(),
    ))?;
    assert_eq!(honest.advance(&held).as_ref(), Ok(&held));
    assert_eq!(
        honest.validate_cache(Some(&held), &cache).as_ref(),
        Ok(held.state())
    );
    for envelope in crossings() {
        let crossed_history = honest.under(envelope);
        assert_eq!(
            crossed_history.advance(&held),
            Err(crossed()),
            "{envelope:?}"
        );
        assert_eq!(
            crossed_history.publish(&held, event.clone()),
            Err(crossed()),
            "{envelope:?}"
        );
        let crossed_cache = campaign_core::CacheProjection {
            binding: crate::suite::identity::observed(crossed_history.binding())?,
            ..cache.clone()
        };
        assert_eq!(
            crossed_history.validate_cache(Some(&held), &crossed_cache),
            Err(crossed()),
            "{envelope:?}"
        );
    }
    let publication = crate::suite::identity::observed(honest.publish(&held, event))?;
    assert_eq!(
        campaign_choregraph::can_execute_selection_after_publication(
            &selection,
            &publication,
            &crate::suite::selection::exact_durable_evidence(&publication),
        ),
        Ok(true)
    );
    Ok(())
}
