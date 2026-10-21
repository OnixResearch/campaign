/// Public Campaign core API operation.
///
/// # Errors
///
/// Returns a typed `CampaignError` when explicit values are missing, crossed, stale, malformed, over bounds, or would violate policy invariants.
// r[impl campaign.frontier]
pub fn reconstruct(
    events: &[crate::model::CampaignControlEvent],
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    let (first, rest) = events
        .split_first()
        .ok_or(crate::error::CampaignError::MissingAdmission)?;
    let (event_id, admission) = match first {
        crate::model::CampaignControlEvent::CampaignAdmitted {
            event_id,
            admission,
        } => (*event_id, admission.clone()),
        _ => return Err(crate::error::CampaignError::MissingAdmission),
    };
    admission.validate()?;
    let expected = crate::model::derive_campaign_event_id(&admission)?;
    if event_id != expected {
        return Err(crate::error::CampaignError::MalformedIdentity);
    }
    let mut state = crate::frontier::FrontierState {
        admission: admission.clone(),
        candidates: alloc::vec::Vec::new(),
        outstanding: alloc::vec::Vec::new(),
        accepted_results: alloc::vec::Vec::new(),
        discoveries: alloc::vec::Vec::new(),
        known_moments: admission
            .roots
            .iter()
            .map(|root| crate::frontier::KnownMoment {
                moment_id: root.moment_id,
                depth: crate::frontier::ROOT_DEPTH,
            })
            .collect(),
        settled: alloc::vec::Vec::new(),
        reservations: alloc::vec::Vec::new(),
        payload_bindings: alloc::vec::Vec::new(),
        findings: alloc::vec::Vec::new(),
        stopped: None,
        event_ids: alloc::vec![event_id],
        reserved_resources: alloc::vec::Vec::new(),
        selection_count: crate::model::NO_SELECTIONS,
        guidance_record_count: crate::model::NO_SELECTIONS,
        control_event_count: crate::model::UNIT_COST,
        state_id: crate::identity::Id::absent(),
    };
    state = state.with_identity()?;
    debug_assert_eq!(state.event_ids.len(), 1);
    debug_assert_eq!(state.known_moments.len(), admission.roots.len());
    for event in rest {
        state = apply_control_event(state, event)?;
    }
    Ok(state)
}

/// Applies one accepted control event to a reconstructed state: the exact per-event step of
/// `reconstruct`, exposed so hosts can advance state incrementally instead of rebuilding it.
///
/// The state must be current (its identity re-derives, its admission re-derives and validates,
/// and every keyed vector has strictly increasing keys). The event must be new, must not be an
/// admission, must be admissible while the campaign is open (or settle prior outstanding work
/// after a stop), and must name the state identity as its `state_before_id`. Every event also
/// counts against `max_control_events`: after it, every outstanding selection must still be able
/// to settle and an open campaign must still be able to record its stop.
///
/// Folding this function over the events after the admission equals `reconstruct` of the same
/// prefix. The input state is consumed; a caller that keeps a copy observes it unchanged on error.
///
/// # Errors
///
/// Returns `StaleState` for a non-current state (a caller-built state with an edited admission
/// included), an event built against another state, or a
/// (caller-built) state the event would leave with a repeated key;
/// `DuplicateControlEvent` for an already-applied event, `InvalidControlEventOrder` for a second
/// admission, `CampaignStopped` for new work after a stop, `ControlEventLimitReached` past the
/// control-event bound, and the typed replay error of the event's own validation.
// r[impl campaign.frontier]
// r[impl campaign.budgets.control_event_reserve]
// r[impl campaign.budgets.overflow]
pub fn apply_control_event(
    state: crate::frontier::FrontierState,
    event: &crate::model::CampaignControlEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    crate::policy::validation::assert_state_identity_current(&state)?;
    crate::frontier::replay::check_event_not_seen(&state, event.event_id())?;
    if matches!(
        event,
        crate::model::CampaignControlEvent::CampaignAdmitted { .. }
    ) {
        return Err(crate::error::CampaignError::InvalidControlEventOrder);
    }
    crate::frontier::retention::ensure_open_for_event(&state, event)?;
    if Some(state.state_id) != event.state_before_id() {
        return Err(crate::error::CampaignError::StaleState);
    }
    crate::frontier::reserve::ensure_control_event_headroom(&state, control_event_effect(event))?;
    let before_count = state.control_event_count;
    let mut next = crate::frontier::replay::apply_event(state, event)?;
    next.event_ids.push(event.event_id());
    next.control_event_count = before_count
        .checked_add(crate::model::UNIT_COST)
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert!(crate::frontier::reserve::has_control_event_headroom(
        &next,
        crate::frontier::reserve::ControlEventEffect::SETTLEMENT
    )?);
    let next = next.with_identity()?;
    crate::policy::validation::assert_keys_strictly_increasing(&next)?;
    debug_assert!(next.event_ids.binary_search(&event.event_id()).is_ok());
    debug_assert!(next.control_event_count > before_count);
    Ok(next)
}
const fn control_event_effect(
    event: &crate::model::CampaignControlEvent,
) -> crate::frontier::reserve::ControlEventEffect {
    match event {
        crate::model::CampaignControlEvent::SelectionPublished(_) => {
            crate::frontier::reserve::ControlEventEffect::SELECTION
        }
        crate::model::CampaignControlEvent::ResultObserved(_)
        | crate::model::CampaignControlEvent::UnobservedClosed(_) => {
            crate::frontier::reserve::ControlEventEffect::SETTLEMENT
        }
        crate::model::CampaignControlEvent::StopAccepted(_) => {
            crate::frontier::reserve::ControlEventEffect::STOP
        }
        crate::model::CampaignControlEvent::CampaignAdmitted { .. }
        | crate::model::CampaignControlEvent::CandidatesAdmitted(_)
        | crate::model::CampaignControlEvent::GuidanceAdmitted(_)
        | crate::model::CampaignControlEvent::CandidateRetired(_)
        | crate::model::CampaignControlEvent::CandidatesPruned(_) => {
            crate::frontier::reserve::ControlEventEffect::NEW_WORK
        }
    }
}
pub(super) fn apply_event(
    state: crate::frontier::FrontierState,
    event: &crate::model::CampaignControlEvent,
) -> crate::error::CampaignResult<crate::frontier::FrontierState> {
    match event {
        crate::model::CampaignControlEvent::CampaignAdmitted { .. } => {
            Err(crate::error::CampaignError::InvalidControlEventOrder)
        }
        crate::model::CampaignControlEvent::CandidatesAdmitted(batch) => {
            crate::frontier::admission::apply_candidates(state, batch)
        }
        crate::model::CampaignControlEvent::GuidanceAdmitted(event) => {
            crate::frontier::admission::apply_guidance(state, event)
        }
        crate::model::CampaignControlEvent::SelectionPublished(event) => {
            crate::frontier::dispatch::apply_selection(state, event)
        }
        crate::model::CampaignControlEvent::ResultObserved(event) => {
            crate::frontier::settlement::apply_result(state, event)
        }
        crate::model::CampaignControlEvent::UnobservedClosed(event) => {
            crate::frontier::settlement::apply_closure(state, event)
        }
        crate::model::CampaignControlEvent::CandidateRetired(event) => {
            crate::frontier::retention::apply_retirement(state, event)
        }
        crate::model::CampaignControlEvent::CandidatesPruned(event) => {
            crate::frontier::retention::apply_prune(state, event)
        }
        crate::model::CampaignControlEvent::StopAccepted(event) => {
            crate::frontier::retention::apply_stop(state, event)
        }
    }
}
pub(super) fn check_event_not_seen(
    state: &crate::frontier::FrontierState,
    event_id: crate::identity::Id,
) -> crate::error::CampaignResult<()> {
    event_id.require_kind(crate::identity::IdKind::Event)?;
    if state.event_ids.binary_search(&event_id).is_ok() {
        return Err(crate::error::CampaignError::DuplicateControlEvent);
    }
    Ok(())
}
