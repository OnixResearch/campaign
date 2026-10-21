//! `campaign.receipts.crossed` (D72): receipt projections check the state identity before the
//! history binding, and a state without its admission event (zero control events) has no history.

/// A state holding one admitted candidate, the refusal of its resubmission under `Reject`, and
/// the admitted candidate event.
fn refused() -> Result<
    (
        campaign_core::FrontierState,
        campaign_core::RefusalReceiptRequest,
        campaign_core::CampaignControlEvent,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|_| {})?;
    let (events, state, _candidate) = crate::suite::candidate::admit(
        events,
        &state,
        b"history-refused",
        crate::suite::constants::COST_ONE,
    )?;
    let refusal = campaign_core::explain_admission_refusal(
        &state,
        vec![
            crate::suite::candidate::record(
                &state,
                b"history-refused",
                crate::suite::constants::COST_ONE,
            )
            .derive()
            .map_err(crate::suite::s3_fixture::err)?,
        ],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or("the resubmission is refused")?;
    let request = campaign_core::RefusalReceiptRequest {
        request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"history"),
        refusal,
        profile: crate::suite::selection::ranked_profile()?,
    };
    let admitted = events.last().cloned().ok_or("the candidate event")?;
    assert_eq!(state.control_event_count, 2);
    assert_eq!(request.refusal.state_id, state.state_id);
    Ok((state, request, admitted))
}

/// The genuine state projects its refusal receipt; with its control-event count zeroed but its
/// identity not re-derived, every build answers `StaleState` (a debug build used to panic on the
/// binding's generation assertion first).
#[test]
// r[verify campaign.receipts.crossed]
fn a_refusal_receipt_checks_the_state_identity_first() -> Result<(), String> {
    let (state, request, _admitted) = refused()?;
    let binding = crate::suite::admission::current_projection(&state)?;
    let receipt = campaign_core::project_refusal_receipt(&state, &request, &binding)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(receipt.decision_id, request.refusal.refusal_id);
    let mut zeroed = state.clone();
    zeroed.control_event_count = 0;
    for projection in [&binding, &zeroed.admission.projection] {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::project_refusal_receipt(
                &zeroed, &request, projection
            ))?,
            campaign_core::CampaignError::StaleState
        );
    }
    Ok(())
}

/// A caller-built state with zero control events and a re-derived identity has no admission
/// event: refusal, plan, and event receipts all refuse it with `MissingAdmission`.
#[test]
// r[verify campaign.receipts.crossed]
fn every_receipt_refuses_a_state_without_its_admission_event() -> Result<(), String> {
    let (state, request, admitted) = refused()?;
    let profile = crate::suite::selection::ranked_profile()?;
    let plan_request = crate::suite::selection::request(&state, profile.clone(), None);
    let plan = campaign_core::plan_next(&state, plan_request.clone(), &state.admission.projection)
        .map_err(crate::suite::s3_fixture::err)?;
    let mut unadmitted = state.clone();
    unadmitted.control_event_count = 0;
    unadmitted
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    let binding = unadmitted.admission.projection.clone();
    let event_request = campaign_core::EventReceiptRequest {
        request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"history-event"),
        event: admitted,
        profile,
    };
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_refusal_receipt(
            &unadmitted,
            &request,
            &binding
        ))?,
        campaign_core::CampaignError::MissingAdmission
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_receipt(
            &unadmitted,
            &plan,
            &plan_request,
            &binding
        ))?,
        campaign_core::CampaignError::MissingAdmission
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_event_receipt(
            &unadmitted,
            &event_request,
            &binding
        ))?,
        campaign_core::CampaignError::MissingAdmission
    );
    Ok(())
}
