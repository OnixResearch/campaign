//! `campaign.frontier.equal`: incremental control-event application equals reconstruction.

/// A varied accepted history: admission, candidate, guidance, selection, forked result,
/// reselection, unobserved closure, retirement, and a consumer stop.
fn varied_history() -> Result<Vec<campaign_core::CampaignControlEvent>, String> {
    let world = crate::suite::world::populated()?;
    let (events, state) = crate::suite::s3_fixture::append(
        world.events,
        crate::suite::world::observed(campaign_core::close_unobserved(
            &world.state,
            crate::suite::s3_fixture::closure(
                &world.state,
                &world.second_selection,
                b"incremental-closure",
                true,
            )?,
        ))?,
    )?;
    let (events, state) = crate::suite::s3_fixture::append(
        events,
        crate::suite::world::observed(campaign_core::retire_candidate(
            &state,
            crate::suite::s3_fixture::retirement(
                &state,
                world.candidate_id,
                campaign_core::RetirementOrigin::Consumer,
            ),
        ))?,
    )?;
    let (events, stopped) = crate::suite::s3_fixture::append(
        events,
        crate::suite::world::observed(campaign_core::admit_stop(
            &state,
            campaign_core::StopReason::ConsumerRequested {
                reason_id: crate::suite::admission::xid(
                    campaign_core::IdKind::Reason,
                    b"incremental-stop",
                ),
            },
        ))?,
    )?;
    assert!(stopped.stopped.is_some());
    assert_eq!(stopped.settled.len(), 2);
    Ok(events)
}

#[test]
// r[verify campaign.frontier.equal]
fn folding_every_prefix_equals_reconstruction() -> Result<(), String> {
    let events = varied_history()?;
    let (admission, rest) = events
        .split_first()
        .ok_or_else(|| String::from("empty history"))?;
    let mut folded = crate::suite::world::observed(campaign_core::reconstruct(
        core::slice::from_ref(admission),
    ))?;
    for (index, event) in rest.iter().enumerate() {
        folded = crate::suite::world::observed(campaign_core::apply_control_event(folded, event))?;
        let prefix_end = index.checked_add(2).ok_or("prefix overflow")?;
        let rebuilt =
            crate::suite::world::observed(campaign_core::reconstruct(&events[..prefix_end]))?;
        assert_eq!(folded, rebuilt);
    }
    assert!(folded.stopped.is_some());
    assert_eq!(
        folded.control_event_count,
        u64::try_from(events.len()).map_err(|error| format!("{error:?}"))?
    );
    Ok(())
}

fn derived(
    state: &campaign_core::FrontierState,
    label: &'static [u8],
) -> Result<campaign_core::CandidateRecord, String> {
    crate::suite::world::observed(
        crate::suite::candidate::record(state, label, crate::suite::constants::COST_ONE).derive(),
    )
}

/// Applies `event` to the state rebuilt from `history`, asserts the typed denial, and asserts
/// that a kept copy of the input still equals the authoritative rebuild.
fn assert_denied(
    history: &[campaign_core::CampaignControlEvent],
    event: &campaign_core::CampaignControlEvent,
    expected: &campaign_core::CampaignError,
) -> Result<(), String> {
    let state = crate::suite::world::observed(campaign_core::reconstruct(history))?;
    let kept = state.clone();
    assert_eq!(
        &crate::suite::world::denial(campaign_core::apply_control_event(state, event))?,
        expected
    );
    assert_eq!(
        kept,
        crate::suite::world::observed(campaign_core::reconstruct(history))?
    );
    Ok(())
}

#[test]
fn replayed_reordered_and_superseded_events_are_rejected_without_mutation() -> Result<(), String> {
    let events = varied_history()?;
    let open_end = events.len().checked_sub(1).ok_or("empty history")?;
    let open = &events[..open_end];
    let stop = &events[open_end];
    let stale_end = open_end.checked_sub(1).ok_or("short history")?;
    assert!(stale_end < open_end);
    assert_ne!(stop.event_id(), events[1].event_id());
    assert_denied(
        open,
        &events[1],
        &campaign_core::CampaignError::DuplicateControlEvent,
    )?;
    assert_denied(
        &events,
        stop,
        &campaign_core::CampaignError::DuplicateControlEvent,
    )?;
    let (other_admission, _other) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_findings = bounds.max_findings.saturating_add(1);
    })?;
    assert_denied(
        open,
        &other_admission[0],
        &campaign_core::CampaignError::InvalidControlEventOrder,
    )?;
    assert_denied(
        &events[..stale_end],
        stop,
        &campaign_core::CampaignError::StaleState,
    )
}

#[test]
// r[verify campaign.budgets.control_event_reserve]
// r[verify campaign.stop.terminal]
fn stopped_malformed_over_bound_and_tampered_inputs_are_rejected() -> Result<(), String> {
    let events = varied_history()?;
    let open_end = events.len().checked_sub(1).ok_or("empty history")?;
    let open = &events[..open_end];
    let stopped = crate::suite::world::observed(campaign_core::reconstruct(&events))?;
    assert!(stopped.stopped.is_some());
    assert!(open.len() < events.len());
    let after_stop = crate::suite::ingress::support::forged_batch(
        &stopped,
        vec![derived(&stopped, b"after-stop")?],
        crate::suite::ingress::support::REJECT,
    )?;
    assert_denied(
        &events,
        &after_stop,
        &campaign_core::CampaignError::CampaignStopped,
    )?;
    let open_state = crate::suite::world::observed(campaign_core::reconstruct(open))?;
    let mut unsealed = crate::suite::ingress::support::forged_batch(
        &open_state,
        vec![derived(&open_state, b"unsealed")?],
        crate::suite::ingress::support::REJECT,
    )?;
    if let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = &mut unsealed {
        batch.admission_outcomes_id = crate::suite::admission::xid(
            campaign_core::IdKind::AdmissionOutcomes,
            b"unsealed-outcomes",
        );
        batch.event_id = crate::suite::world::observed(batch.expected_event_id())?;
    }
    assert_denied(
        open,
        &unsealed,
        &campaign_core::CampaignError::MalformedIdentity,
    )?;
    let (bounded, bounded_state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_control_events = campaign_core::MIN_CONTROL_EVENTS;
    })?;
    let over_bound = crate::suite::ingress::support::forged_batch(
        &bounded_state,
        vec![derived(&bounded_state, b"over-bound")?],
        crate::suite::ingress::support::REJECT,
    )?;
    assert_denied(
        &bounded,
        &over_bound,
        &campaign_core::CampaignError::ControlEventLimitReached,
    )?;
    let mut tampered = open_state;
    tampered.selection_count = tampered.selection_count.saturating_add(1);
    assert_eq!(
        crate::suite::world::denial(campaign_core::apply_control_event(
            tampered,
            &events[open_end]
        ))?,
        campaign_core::CampaignError::StaleState
    );
    Ok(())
}
