pub(crate) mod checks;

pub(super) fn apply_result(
    state: crate::frontier::FrontierState,
    event: &crate::model::ResultEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if event.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let observation = event.observation.clone().derive()?;
    if observation != event.observation {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let outstanding_index =
        crate::frontier::settlement::checks::validate_observation(&state, &observation)?;
    let expected = crate::model::derive_event_id(
        &crate::model::ControlEventKind::ResultObserved,
        event.campaign_id,
        event.state_before_id,
        &[observation.result_id],
    )?;
    if expected != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let candidate_id = state.outstanding[outstanding_index].candidate_id;
    let child_depth = state.candidate(candidate_id)?.candidate.depth;
    debug_assert!(outstanding_index < state.outstanding.len());
    Ok(record_observation(
        state,
        &observation,
        outstanding_index,
        child_depth,
    ))
}

/// Records one validated result. Lookups search the canonical (sorted) entries the state held
/// before the result, so each costs logarithmic time; the result's children, payloads and findings
/// are unique within it (validated), so new entries are appended and sorted once by
/// `with_identity`.
// r[impl campaign.fd.identity]
// r[impl campaign.fd.reservation]
// r[impl campaign.fork]
fn record_observation(
    mut state: crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
    outstanding_index: usize,
    child_depth: u64,
) -> crate::frontier::FrontierState {
    debug_assert_eq!(
        state
            .outstanding
            .get(outstanding_index)
            .map(|selection| selection.selection_id),
        Some(observation.selection_id)
    );
    debug_assert!(
        state
            .accepted_results
            .binary_search(&observation.result_id)
            .is_err()
    );
    state = record_children(state, observation, child_depth);
    state = record_findings(state, observation);
    state
        .payload_bindings
        .push(crate::frontier::PayloadBinding {
            payload_id: observation.payload_id,
            owner_id: observation.result_id,
        });
    state.accepted_results.push(observation.result_id);
    let settled = state.outstanding.remove(outstanding_index);
    state.settled.push(crate::frontier::Settlement {
        selection_id: settled.selection_id,
        settlement_id: observation.result_id,
    });
    debug_assert_eq!(settled.selection_id, observation.selection_id);
    state
}

/// Records the result's child moments, reservations, child payload bindings, and first
/// discoveries; a rediscovered child keeps its known moment, payload binding, and discovery.
fn record_children(
    mut state: crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
    child_depth: u64,
) -> crate::frontier::FrontierState {
    let known_moments = state.known_moments.len();
    let payload_bindings = state.payload_bindings.len();
    let discoveries = state.discoveries.len();
    for child in &observation.children {
        let is_known = crate::identity::collection::sorted_position(
            state.known_moments.get(..known_moments).unwrap_or_default(),
            child.child_moment_id,
            |moment| moment.moment_id,
        )
        .is_some();
        if !is_known {
            state.known_moments.push(crate::frontier::KnownMoment {
                moment_id: child.child_moment_id,
                depth: child_depth,
            });
        }
        state.reservations.push(child.reservation_id);
        let is_bound = crate::identity::collection::sorted_position(
            state
                .payload_bindings
                .get(..payload_bindings)
                .unwrap_or_default(),
            child.event_payload_id,
            |binding| binding.payload_id,
        )
        .is_some();
        if !is_bound {
            state
                .payload_bindings
                .push(crate::frontier::PayloadBinding {
                    payload_id: child.event_payload_id,
                    owner_id: child.child_moment_id,
                });
        }
        let is_discovered = crate::identity::collection::sorted_position(
            state.discoveries.get(..discoveries).unwrap_or_default(),
            child.child_moment_id,
            |discovery| discovery.child_moment_id,
        )
        .is_some();
        if !is_discovered {
            state.discoveries.push(crate::frontier::StableDiscovery {
                child_moment_id: child.child_moment_id,
                discovery_id: child.discovery_id,
                event_payload_id: child.event_payload_id,
                first_result_id: observation.result_id,
            });
        }
    }
    debug_assert!(state.known_moments.len() >= known_moments);
    debug_assert!(state.discoveries.len() >= discoveries);
    state
}

/// Records the result's findings that are not accepted yet; a re-reported finding repeats its
/// accepted source facts (validated) and is recorded once.
fn record_findings(
    mut state: crate::frontier::FrontierState,
    observation: &crate::model::ResultObservation,
) -> crate::frontier::FrontierState {
    let accepted_findings = state.findings.len();
    for finding in &observation.findings {
        let is_accepted = crate::identity::collection::sorted_position(
            state.findings.get(..accepted_findings).unwrap_or_default(),
            finding.finding_id,
            |existing| existing.finding_id,
        )
        .is_some();
        if !is_accepted {
            state.findings.push(finding.clone());
        }
    }
    debug_assert!(state.findings.len() >= accepted_findings);
    debug_assert!(
        state.findings.len() <= accepted_findings.saturating_add(observation.findings.len())
    );
    state
}

pub(super) fn apply_closure(
    mut state: crate::frontier::FrontierState,
    event: &crate::model::ClosureEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    if event.campaign_id != state.admission.campaign_id {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    let outstanding_index =
        crate::frontier::settlement::checks::validate_closure(&state, &event.closure)?;
    let expected = crate::model::derive_event_id(
        &crate::model::ControlEventKind::UnobservedClosed,
        event.campaign_id,
        event.state_before_id,
        &[
            event.closure.closure_id,
            event.closure.selection_id,
            event.closure.reason_id,
        ],
    )?;
    if expected != event.event_id {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let removed = state.outstanding.remove(outstanding_index);
    state.settled.push(crate::frontier::Settlement {
        selection_id: removed.selection_id,
        settlement_id: event.closure.closure_id,
    });
    debug_assert_eq!(removed.selection_id, event.closure.selection_id);
    debug_assert!(event.closure.uncertainty_acknowledged);
    Ok(state)
}
