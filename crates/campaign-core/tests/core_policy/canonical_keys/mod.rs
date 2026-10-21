//! D79: a caller-built state is current only when every keyed vector has strictly increasing
//! keys; canonical order alone (a stable sort) leaves a repeated key in place.

mod inner_vectors;
mod scans;

type History = crate::suite::s3_fixture::History;
type Forge = (
    &'static str,
    fn(&mut campaign_core::FrontierState) -> Result<(), String>,
);

/// A state with an entry in every keyed vector: the first selection settled with one child and
/// one finding, the second selection outstanding.
fn populated() -> Result<
    (
        History,
        campaign_core::FrontierState,
        campaign_core::ResultObservation,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|_| {})?;
    let (events, state, first) = crate::suite::s3_fixture::selected(events, &state, b"keys-a")?;
    let (events, state, second) = crate::suite::s3_fixture::selected(events, &state, b"keys-b")?;
    let child =
        crate::suite::s3_fixture::child(&state, &first, b"keys-child", b"keys-child-payload")?;
    let mut observation = crate::suite::s3_fixture::observation(&state, &first, vec![child]);
    observation.findings = vec![crate::suite::s3_fixture::finding(b"keys-finding")];
    let result =
        campaign_core::admit_result(&state, observation).map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, result)?;
    let pending = crate::suite::s3_fixture::observation(&state, &second, Vec::new());
    Ok((events, state, pending))
}

fn first<T: Clone>(entries: &[T]) -> Result<T, String> {
    entries
        .first()
        .cloned()
        .ok_or_else(|| String::from("the populated state lacks an entry"))
}

/// One forge per keyed vector of the admission and the first frontier vectors.
fn leading_forges() -> [Forge; 7] {
    [
        ("roots", |state| {
            let entry = first(&state.admission.roots)?;
            state.admission.roots.insert(0, entry);
            Ok(())
        }),
        ("resource_budgets", |state| {
            let entry = first(&state.admission.resource_budgets)?;
            state.admission.resource_budgets.insert(0, entry);
            Ok(())
        }),
        ("operation_schema_ids", |state| {
            let entry = first(&state.admission.operation_schema_ids)?;
            state.admission.operation_schema_ids.insert(0, entry);
            Ok(())
        }),
        ("candidates", |state| {
            let entry = first(&state.candidates)?;
            state.candidates.insert(0, entry);
            Ok(())
        }),
        ("outstanding", |state| {
            let entry = first(&state.outstanding)?;
            state.outstanding.insert(0, entry);
            Ok(())
        }),
        ("accepted_results", |state| {
            let entry = first(&state.accepted_results)?;
            state.accepted_results.insert(0, entry);
            Ok(())
        }),
        ("discoveries", |state| {
            let entry = first(&state.discoveries)?;
            state.discoveries.insert(0, entry);
            Ok(())
        }),
    ]
}

/// One forge per remaining keyed vector of `FrontierState`.
fn trailing_forges() -> [Forge; 7] {
    [
        ("known_moments", |state| {
            let entry = first(&state.known_moments)?;
            state.known_moments.insert(0, entry);
            Ok(())
        }),
        ("settled", |state| {
            let entry = first(&state.settled)?;
            state.settled.insert(0, entry);
            Ok(())
        }),
        ("reservations", |state| {
            let entry = first(&state.reservations)?;
            state.reservations.insert(0, entry);
            Ok(())
        }),
        ("payload_bindings", |state| {
            let entry = first(&state.payload_bindings)?;
            state.payload_bindings.insert(0, entry);
            Ok(())
        }),
        ("findings", |state| {
            let entry = first(&state.findings)?;
            state.findings.insert(0, entry);
            Ok(())
        }),
        ("event_ids", |state| {
            let entry = first(&state.event_ids)?;
            state.event_ids.insert(0, entry);
            Ok(())
        }),
        ("reserved_resources", |state| {
            let entry = first(&state.reserved_resources)?;
            state.reserved_resources.insert(0, entry);
            Ok(())
        }),
    ]
}

/// The typed answers of intake, replay, and retention planning for one state.
struct Answers {
    admitted: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
    applied: campaign_core::CampaignResult<campaign_core::FrontierState>,
    planned: campaign_core::CampaignResult<Option<campaign_core::PruneRequest>>,
}

fn answers(
    state: &campaign_core::FrontierState,
    pending: &campaign_core::ResultObservation,
) -> Result<Answers, String> {
    let forged = crate::suite::s3_fixture::forged_result(state, pending.clone())?;
    Ok(Answers {
        admitted: campaign_core::admit_result(state, pending.clone()),
        applied: campaign_core::apply_control_event(state.clone(), &forged),
        planned: campaign_core::plan_pruning(
            state,
            crate::suite::s3_pruning::plan_request(
                state,
                vec![crate::suite::s3_pruning::FIRST_DIMENSION],
            ),
        ),
    })
}

/// A state that repeats the key of any keyed vector, with its identity re-derived, is
/// `StaleState` at intake, at replay, and at retention planning; the same state without the
/// repeat is current at all three.
#[test]
// r[verify campaign.cache]
fn a_state_repeating_any_key_is_stale_at_intake_replay_and_planning() -> Result<(), String> {
    let (events, state, pending) = populated()?;
    let mut rederived = state.clone();
    rederived
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(rederived, state);
    let Answers {
        admitted,
        applied,
        planned,
    } = answers(&state, &pending)?;
    let admitted = admitted.map_err(crate::suite::s3_fixture::err)?;
    let (_events, settled) = crate::suite::s3_fixture::append(events.clone(), admitted)?;
    assert_eq!(applied, Ok(settled));
    assert_eq!(planned, Ok(None));
    for (vector, forge) in leading_forges().into_iter().chain(trailing_forges()) {
        let mut forged = state.clone();
        forge(&mut forged)?;
        forged
            .recompute_identity()
            .map_err(crate::suite::s3_fixture::err)?;
        let Answers {
            admitted,
            applied,
            planned,
        } = answers(&forged, &pending)?;
        assert_eq!(
            admitted.map(|_| ()),
            Err(campaign_core::CampaignError::StaleState),
            "{vector}"
        );
        assert_eq!(
            applied.map(|_| ()),
            Err(campaign_core::CampaignError::StaleState),
            "{vector}"
        );
        assert_eq!(
            planned,
            Err(campaign_core::CampaignError::StaleState),
            "{vector}"
        );
    }
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

/// A caller-built state that already lists the pending result as accepted (keys still strictly
/// increasing) refuses the result as `DuplicateResult` at intake and at replay; it used to be
/// admitted, and applying it tripped a debug assertion (a release build listed the result twice).
#[test]
// r[verify campaign.observations.duplicate]
fn a_state_already_listing_the_result_as_accepted_refuses_it_as_a_duplicate() -> Result<(), String>
{
    let (_events, state, pending) = populated()?;
    let result_id = pending
        .clone()
        .derive()
        .map_err(crate::suite::s3_fixture::err)?
        .result_id;
    let mut forged = state.clone();
    forged.accepted_results.push(result_id);
    forged
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    let Answers {
        admitted, applied, ..
    } = answers(&forged, &pending)?;
    assert_eq!(
        admitted.map(|_| ()),
        Err(campaign_core::CampaignError::DuplicateResult)
    );
    assert_eq!(
        applied.map(|_| ()),
        Err(campaign_core::CampaignError::DuplicateResult)
    );
    Ok(())
}

/// A caller-built state that lists the outstanding selection as settled as well cannot be
/// advanced by its result: replay would repeat the settled key, so it answers `StaleState`
/// instead of returning a state that lists the selection twice.
#[test]
// r[verify campaign.cache]
fn a_state_listing_the_outstanding_selection_as_settled_is_not_advanced() -> Result<(), String> {
    let (_events, state, pending) = populated()?;
    let mut forged = state.clone();
    forged.settled.push(campaign_core::Settlement {
        selection_id: pending.selection_id,
        settlement_id: crate::suite::admission::xid(
            campaign_core::IdKind::Event,
            b"keys-forged-settlement",
        ),
    });
    forged
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    assert!(
        forged
            .outstanding
            .iter()
            .any(|selection| selection.selection_id == pending.selection_id)
    );
    assert!(
        forged
            .settled
            .iter()
            .any(|settlement| settlement.selection_id == pending.selection_id)
    );
    let Answers { applied, .. } = answers(&forged, &pending)?;
    assert_eq!(
        applied.map(|_| ()),
        Err(campaign_core::CampaignError::StaleState)
    );
    Ok(())
}
