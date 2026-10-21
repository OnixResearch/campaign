//! The control-event reserve: every outstanding selection can still settle and an open campaign
//! can still record its stop.

/// Effect of one control event (or a planned run of events) on the control-event budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ControlEventEffect {
    /// Events recorded beyond the reserve: a settlement's event was reserved by its outstanding
    /// selection and records none.
    unreserved_events: u64,
    /// Outstanding selections added, each of which must still be able to settle.
    added_outstanding: u64,
    /// Whether the event consumes the stop reserve.
    records_stop: bool,
}
impl ControlEventEffect {
    /// Candidate, guidance, revision, retirement, or prune event: keeps outstanding work and the
    /// stop reserve.
    pub(crate) const NEW_WORK: Self = Self {
        unreserved_events: 1,
        added_outstanding: 0,
        records_stop: false,
    };
    /// Selection: adds one outstanding selection that must still be able to settle.
    pub(crate) const SELECTION: Self = Self {
        unreserved_events: 1,
        added_outstanding: 1,
        records_stop: false,
    };
    /// Guidance and then a selection of the guided candidate: a candidate awaits guidance only
    /// while the control events admit its guidance, a selection, that selection's settlement, and
    /// the stop.
    // r[impl campaign.wait.guidance_room]
    pub(crate) const GUIDED_SELECTION: Self = Self {
        unreserved_events: 2,
        added_outstanding: 1,
        records_stop: false,
    };
    /// Result or closure: consumes the event reserved by the settled selection.
    pub(crate) const SETTLEMENT: Self = Self {
        unreserved_events: 0,
        added_outstanding: 0,
        records_stop: false,
    };
    /// Stop: consumes the stop reserve.
    pub(crate) const STOP: Self = Self {
        unreserved_events: 1,
        added_outstanding: 0,
        records_stop: true,
    };
}
/// Whether the state keeps `control_event_count + outstanding + (stopped ? 0 : 1) <=
/// max_control_events` after one event with `effect`: every outstanding selection can still
/// settle and an open campaign can still record its stop.
// r[impl campaign.budgets.control_event_reserve]
pub(crate) fn has_control_event_headroom(
    state: &crate::frontier::FrontierState,
    effect: ControlEventEffect,
) -> crate::error::CampaignResult<bool> {
    let outstanding = crate::identity::usize_to_u64(state.outstanding.len())?;
    let is_open_after = state.stopped.is_none() && !effect.records_stop;
    let demand = state
        .control_event_count
        .checked_add(effect.unreserved_events)
        .and_then(|count| count.checked_add(outstanding))
        .and_then(|count| count.checked_add(effect.added_outstanding))
        .and_then(|count| count.checked_add(u64::from(is_open_after)))
        .ok_or(crate::error::CampaignError::ArithmeticOverflow)?;
    debug_assert!(demand >= state.control_event_count);
    Ok(demand <= state.admission.bounds.max_control_events)
}
/// Denies an event with `effect` that would leave no headroom for outstanding settlements and
/// the stop reserve.
pub(crate) fn ensure_control_event_headroom(
    state: &crate::frontier::FrontierState,
    effect: ControlEventEffect,
) -> crate::error::CampaignResult<()> {
    if !has_control_event_headroom(state, effect)? {
        return Err(crate::error::CampaignError::ControlEventLimitReached);
    }
    Ok(())
}
