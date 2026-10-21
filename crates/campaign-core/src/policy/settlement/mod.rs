/// Admits one exact result for a published outstanding selection.
///
/// The entry point applies every check that replay applies, so an admitted event always replays.
///
/// # Errors
///
/// Returns `CrossedIdentity` for another campaign, adapter, parent, or operation;
/// `MissingSelection` for an unknown selection; `DuplicateResult` for the identical settled result
/// or a result the state already lists as accepted; `StaleState` for a non-current state (one
/// whose identity does not re-derive or whose keyed vectors do not strictly increase);
/// `InvalidRetryBoundary` for a different settlement of a settled selection; `BoundExceeded` for
/// child, derived-record, or finding bounds; `DuplicateChildMoment` and `InvalidFinding` for
/// duplicates within the result; `ConflictingResult` when the result payload is already bound, a
/// child's exploration payload already belongs to another child moment or result, or a re-reported
/// finding names different source facts; and `ControlEventLimitReached` past the control-event
/// bound.
// r[impl campaign.observations]
pub fn admit_result(
    state: &crate::frontier::FrontierState,
    observation: crate::model::ResultObservation,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::frontier::reserve::ensure_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::SETTLEMENT,
    )?;
    let observation = observation.derive()?;
    let outstanding_index =
        crate::frontier::settlement::checks::validate_observation(state, &observation)?;
    debug_assert!(outstanding_index < state.outstanding.len());
    let event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::ResultObserved,
        state.admission.campaign_id,
        state.state_id,
        &[observation.result_id],
    )?;
    Ok(crate::model::CampaignControlEvent::ResultObserved(
        crate::model::ResultEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            observation,
        },
    ))
}

/// Admits one explicit unobserved closure for a published outstanding selection.
///
/// # Errors
///
/// Returns `MissingUncertaintyAcknowledgement` without an uncertainty acknowledgement,
/// `CrossedIdentity` for another campaign, `MalformedIdentity` for a forged closure identity,
/// `MissingSelection` for an unknown selection, and `DuplicateResult` or `InvalidRetryBoundary`
/// for a selection that is already settled.
// r[impl campaign.unobserved]
pub fn close_unobserved(
    state: &crate::frontier::FrontierState,
    closure: crate::model::UnobservedClosure,
) -> crate::error::CampaignResult<crate::model::CampaignControlEvent> {
    crate::policy::validation::assert_state_identity_current(state)?;
    crate::frontier::reserve::ensure_control_event_headroom(
        state,
        crate::frontier::reserve::ControlEventEffect::SETTLEMENT,
    )?;
    let outstanding_index = crate::frontier::settlement::checks::validate_closure(state, &closure)?;
    debug_assert!(outstanding_index < state.outstanding.len());
    debug_assert!(closure.uncertainty_acknowledged);
    let event_id = crate::model::derive_event_id(
        &crate::model::ControlEventKind::UnobservedClosed,
        state.admission.campaign_id,
        state.state_id,
        &[closure.closure_id, closure.selection_id, closure.reason_id],
    )?;
    Ok(crate::model::CampaignControlEvent::UnobservedClosed(
        crate::model::ClosureEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            closure,
        },
    ))
}
