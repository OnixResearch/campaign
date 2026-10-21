//! `campaign.observations.duplicate`: once a result or closure settles a selection, a second
//! settlement that passes the checks preceding the selection lookup is refused with
//! `DuplicateResult` (identical) or `InvalidRetryBoundary` (unequal); one that fails an earlier
//! check answers that check's typed error instead. Entry and replay agree, without mutation.

/// Unequal second settlements that reach the settled-selection lookup.
const UNEQUAL: usize = 2;
/// Unequal second settlements that fail an earlier check.
const EARLY_FAILURES: usize = 5;

/// A second settlement presented on the settled state: its entry answer and its replayed event.
struct Second {
    entry: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
    forged: campaign_core::CampaignControlEvent,
}

/// A settled selection with the history and state after its settlement.
struct Settled {
    events: crate::suite::s3_fixture::History,
    state: campaign_core::FrontierState,
    selection: campaign_core::SelectionDecision,
}

fn second_result(
    state: &campaign_core::FrontierState,
    observation: campaign_core::ResultObservation,
) -> Result<Second, String> {
    Ok(Second {
        entry: campaign_core::admit_result(state, observation.clone()),
        forged: crate::suite::s3_fixture::forged_result(state, observation)?,
    })
}

fn second_closure(
    state: &campaign_core::FrontierState,
    closure: campaign_core::UnobservedClosure,
) -> Result<Second, String> {
    Ok(Second {
        entry: campaign_core::close_unobserved(state, closure.clone()),
        forged: crate::suite::s3_fixture::forged_closure(state, closure)?,
    })
}

/// Appends the accepted settlement `event` of the published `selection`.
fn settle(
    events: crate::suite::s3_fixture::History,
    selection: campaign_core::SelectionDecision,
    event: campaign_core::CampaignControlEvent,
) -> Result<Settled, String> {
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert!(state.outstanding.is_empty());
    let settled_ids: Vec<campaign_core::Id> = state
        .settled
        .iter()
        .map(|settled| settled.selection_id)
        .collect();
    assert_eq!(settled_ids, vec![selection.selection_id]);
    Ok(Settled {
        events,
        state,
        selection,
    })
}

/// A selection settled by an exact result, and the identical result presented again.
fn by_result() -> Result<(Settled, Second), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"retry-by-result")?;
    let observation = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    let event = campaign_core::admit_result(&state, observation.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let settled = settle(events, selection, event)?;
    let identical = second_result(&settled.state, observation)?;
    Ok((settled, identical))
}

/// A selection settled by an acknowledged closure, and the identical closure presented again.
fn by_closure() -> Result<(Settled, Second), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"retry-by-closure")?;
    let closure = crate::suite::s3_fixture::closure(&state, &selection, b"retry-closed", true)?;
    let event = campaign_core::close_unobserved(&state, closure.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let settled = settle(events, selection, event)?;
    let identical = second_closure(&settled.state, closure)?;
    Ok((settled, identical))
}

/// The typed refusal of `second` at entry, after asserting that replay refuses it equally and
/// that the refusal left the settled state unchanged.
fn refusal(settled: &Settled, second: Second) -> Result<campaign_core::CampaignError, String> {
    let entry = crate::suite::s3_fixture::denial(second.entry)?;
    let replayed = crate::suite::s3_fixture::replay_denial(&settled.events, second.forged)?;
    assert_eq!(replayed, entry);
    crate::suite::s3_fixture::assert_unchanged(&settled.events, &settled.state)?;
    Ok(entry)
}

/// Unequal second settlements that reach the settled-selection lookup: another result payload
/// and another acknowledged closure reason.
fn unequal(settled: &Settled) -> Result<[Second; UNEQUAL], String> {
    let (state, selection) = (&settled.state, &settled.selection);
    let other_payload = campaign_core::ResultObservation {
        payload_id: crate::suite::admission::xid(campaign_core::IdKind::EventPayload, b"retry"),
        ..crate::suite::s3_fixture::observation(state, selection, Vec::new())
    };
    let other_reason = crate::suite::s3_fixture::closure(state, selection, b"retry-other", true)?;
    assert!(other_reason.uncertainty_acknowledged);
    assert_eq!(other_reason.campaign_id, state.admission.campaign_id);
    Ok([
        second_result(state, other_payload)?,
        second_closure(state, other_reason)?,
    ])
}

/// Unequal second settlements that fail a check preceding the selection lookup, with that
/// check's typed error: results of another adapter or campaign and a closure of another campaign
/// (`CrossedIdentity`), an unacknowledged closure, and a closure with a forged identity.
fn early_failures(
    settled: &Settled,
) -> Result<[(Second, campaign_core::CampaignError); EARLY_FAILURES], String> {
    let (state, selection) = (&settled.state, &settled.selection);
    let exact = crate::suite::s3_fixture::observation(state, selection, Vec::new());
    let other_adapter = campaign_core::ResultObservation {
        adapter_id: crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"retry-other"),
        ..exact.clone()
    };
    let other_campaign_id =
        crate::suite::admission::xid(campaign_core::IdKind::Campaign, b"retry-other");
    assert_ne!(other_campaign_id, state.admission.campaign_id);
    let other_campaign = campaign_core::ResultObservation {
        campaign_id: other_campaign_id,
        ..exact
    };
    let mut closure_of_other =
        crate::suite::s3_fixture::closure(state, selection, b"retry-foreign", true)?;
    closure_of_other.campaign_id = other_campaign_id;
    closure_of_other.closure_id = campaign_core::derive_closure_id(&closure_of_other)
        .map_err(crate::suite::s3_fixture::err)?;
    let unacknowledged =
        crate::suite::s3_fixture::closure(state, selection, b"retry-unacknowledged", false)?;
    assert!(!unacknowledged.uncertainty_acknowledged);
    let mut forged_identity =
        crate::suite::s3_fixture::closure(state, selection, b"retry-forged", true)?;
    forged_identity.closure_id = unacknowledged.closure_id;
    let crossed = campaign_core::CampaignError::CrossedIdentity;
    Ok([
        (second_result(state, other_adapter)?, crossed.clone()),
        (second_result(state, other_campaign)?, crossed.clone()),
        (second_closure(state, closure_of_other)?, crossed),
        (
            second_closure(state, unacknowledged)?,
            campaign_core::CampaignError::MissingUncertaintyAcknowledgement,
        ),
        (
            second_closure(state, forged_identity)?,
            campaign_core::CampaignError::MalformedIdentity,
        ),
    ])
}

/// Every second settlement of a result-settled or closure-settled selection: the identical one is
/// a duplicate, unequal ones that reach the selection lookup cross the retry boundary, and those
/// failing an earlier check answer that check.
#[test]
// r[verify campaign.observations.duplicate]
fn a_settled_selection_refuses_second_settlements_after_their_earlier_checks() -> Result<(), String>
{
    for (settled, identical) in [by_result()?, by_closure()?] {
        assert_eq!(
            refusal(&settled, identical)?,
            campaign_core::CampaignError::DuplicateResult
        );
        for second in unequal(&settled)? {
            assert_eq!(
                refusal(&settled, second)?,
                campaign_core::CampaignError::InvalidRetryBoundary
            );
        }
        for (second, expected) in early_failures(&settled)? {
            assert_eq!(refusal(&settled, second)?, expected);
        }
    }
    Ok(())
}
