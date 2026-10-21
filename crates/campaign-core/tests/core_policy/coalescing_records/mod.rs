//! `campaign.fd.admission.policy`: receipts of accepted batches and guidance bind the outcome list
//! derived from the accepted event itself, never a caller-supplied identity.

fn project_for(
    state: &campaign_core::FrontierState,
    event: &campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::CampaignReceipt> {
    let profile = crate::suite::selection::ranked_profile()
        .map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    let binding = crate::suite::admission::current_projection(state)
        .map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    campaign_core::project_event_receipt(
        state,
        &campaign_core::EventReceiptRequest {
            request_id: crate::suite::admission::xid(campaign_core::IdKind::Request, b"event"),
            event: event.clone(),
            profile,
        },
        &binding,
    )
}

/// One admitted candidate and the two batches that could follow it: a plain batch admitting a
/// fresh candidate, and a coalescing batch that also resubmits the admitted one.
struct Batches {
    existing: campaign_core::CandidateRecord,
    plain_state: campaign_core::FrontierState,
    plain_event: campaign_core::CampaignControlEvent,
    coalesced_state: campaign_core::FrontierState,
    coalesced_event: campaign_core::CampaignControlEvent,
}

fn batches() -> Result<Batches, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, existing_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"existing",
        crate::suite::constants::COST_ONE,
    )?;
    let existing = crate::suite::world::observed(state.candidate(existing_id))?
        .candidate
        .clone();
    let fresh = crate::suite::world::observed(
        crate::suite::candidate::record(&state, b"fresh", crate::suite::constants::COST_ONE)
            .derive(),
    )?;
    let plain_event = crate::suite::world::observed(crate::suite::ingress::support::admit_batch(
        &state,
        vec![fresh.clone()],
        crate::suite::ingress::support::COALESCE,
    ))?;
    let coalesced_event =
        crate::suite::world::observed(crate::suite::ingress::support::admit_batch(
            &state,
            vec![fresh, existing.clone()],
            crate::suite::ingress::support::COALESCE,
        ))?;
    let (_plain, plain_state) =
        crate::suite::ingress::support::accept(events.clone(), plain_event.clone())?;
    let (_coalesced, coalesced_state) =
        crate::suite::ingress::support::accept(events, coalesced_event.clone())?;
    assert_eq!(plain_state.candidates, coalesced_state.candidates);
    assert_ne!(plain_state.state_id, coalesced_state.state_id);
    Ok(Batches {
        existing,
        plain_state,
        plain_event,
        coalesced_state,
        coalesced_event,
    })
}

#[test]
// r[verify campaign.fd.admission.policy]
fn a_coalesced_duplicate_binds_its_reason_into_the_receipt() -> Result<(), String> {
    let batches = batches()?;
    let plain =
        crate::suite::world::observed(project_for(&batches.plain_state, &batches.plain_event))?;
    let coalesced = crate::suite::world::observed(project_for(
        &batches.coalesced_state,
        &batches.coalesced_event,
    ))?;
    let batch = crate::suite::ingress::support::batch_of(&batches.coalesced_event)?;
    let recorded = campaign_core::AdmissionOutcome {
        candidate_id: batches.existing.candidate_id,
        outcome: campaign_core::AdmissionDecision::CoalescedDuplicate {
            existing_candidate_id: batches.existing.candidate_id,
        },
    };
    assert!(batch.outcomes.contains(&recorded));
    assert_eq!(
        coalesced.admission_outcomes_id,
        Some(crate::suite::world::observed(
            campaign_core::derive_admission_outcomes_id(&batch.outcomes)
        )?)
    );
    assert_ne!(plain.admission_outcomes_id, coalesced.admission_outcomes_id);
    assert_ne!(plain.receipt_id, coalesced.receipt_id);
    assert_eq!(
        coalesced.admission_policy_id,
        crate::suite::world::observed(
            crate::suite::ingress::support::COALESCE.admission_policy_id()
        )?
    );
    assert_eq!(
        coalesced.coalescing_policy_id,
        crate::suite::ingress::support::coalescing_policy()
    );
    assert_eq!(coalesced.decision_id, batches.coalesced_event.event_id());
    Ok(())
}

#[test]
// r[verify campaign.fd.admission.policy]
fn a_hidden_duplicate_outcome_is_rejected() -> Result<(), String> {
    let batches = batches()?;
    let mut hidden = batches.coalesced_event.clone();
    if let campaign_core::CampaignControlEvent::CandidatesAdmitted(forged) = &mut hidden {
        for outcome in &mut forged.outcomes {
            outcome.outcome = campaign_core::AdmissionDecision::Admitted;
        }
    }
    assert_ne!(hidden, batches.coalesced_event);
    assert_eq!(
        crate::suite::world::denial(project_for(&batches.coalesced_state, &hidden))?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}

#[test]
fn a_rejected_duplicate_produces_no_event_and_no_event_receipt() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, existing_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"existing",
        crate::suite::constants::COST_ONE,
    )?;
    let existing = crate::suite::world::observed(state.candidate(existing_id))?
        .candidate
        .clone();
    assert_eq!(
        crate::suite::world::denial(crate::suite::ingress::support::admit_batch(
            &state,
            vec![existing.clone()],
            crate::suite::ingress::support::REJECT,
        ))?,
        campaign_core::CampaignError::DuplicateIdentity
    );
    crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    let unaccepted = crate::suite::world::observed(crate::suite::ingress::support::admit_batch(
        &state,
        vec![existing],
        crate::suite::ingress::support::COALESCE,
    ))?;
    assert_eq!(
        crate::suite::world::denial(project_for(&state, &unaccepted))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}

#[test]
fn a_guidance_receipt_binds_the_recorded_dynamic() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let equal = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        crate::suite::constants::GENERATION_NEXT,
        vec![crate::suite::constants::RANK_HIGH],
    );
    let event = crate::suite::world::observed(campaign_core::admit_guidance(&state, equal))?;
    let guidance = crate::suite::ingress::support::guidance_of(&event)?.clone();
    let (_events, next) = crate::suite::ingress::support::accept(events, event.clone())?;
    let bound = crate::suite::world::observed(project_for(&next, &event))?;
    assert_eq!(
        bound.admission_outcomes_id,
        Some(crate::suite::world::observed(
            guidance.dynamics.outcomes_id(guidance.guidance.guidance_id)
        )?)
    );
    assert!(matches!(
        guidance.dynamics,
        campaign_core::GuidanceDynamics::CoalescedDuplicate { .. }
    ));
    let admission = crate::suite::admission::initial_state()?.0;
    let plain = crate::suite::world::observed(project_for(&next, &admission[0]))?;
    assert_eq!(plain.decision_id, admission[0].event_id());
    assert_eq!(plain.admission_outcomes_id, None);
    let mut forged = event;
    if let campaign_core::CampaignControlEvent::GuidanceAdmitted(recorded) = &mut forged {
        recorded.dynamics = campaign_core::GuidanceDynamics::Discovery;
    }
    assert_eq!(
        crate::suite::world::denial(project_for(&next, &forged))?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}

#[test]
fn an_event_of_another_history_is_crossed() -> Result<(), String> {
    let (_events, state, _candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let mut foreign = crate::suite::admission::initial_state()?.0[0].clone();
    if let campaign_core::CampaignControlEvent::CampaignAdmitted { event_id, .. } = &mut foreign {
        *event_id = crate::suite::admission::xid(campaign_core::IdKind::Event, b"foreign");
    }
    assert_eq!(
        crate::suite::world::denial(project_for(&state, &foreign))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    assert!(foreign.event_id().kind == campaign_core::IdKind::Event);
    Ok(())
}

/// The event id of an accepted coalescing batch, relabelled into another event variant, does not
/// authenticate: the receipt re-derives each event's identity from its own content.
#[test]
fn a_relabelled_event_variant_projects_no_receipt() -> Result<(), String> {
    let batches = batches()?;
    let honest = crate::suite::world::observed(project_for(
        &batches.coalesced_state,
        &batches.coalesced_event,
    ))?;
    assert!(honest.admission_outcomes_id.is_some());
    let (admission_events, _state) = crate::suite::admission::initial_state()?;
    let mut disguised = admission_events[0].clone();
    if let campaign_core::CampaignControlEvent::CampaignAdmitted { event_id, .. } = &mut disguised {
        *event_id = batches.coalesced_event.event_id();
    }
    assert_eq!(
        crate::suite::world::denial(project_for(&batches.coalesced_state, &disguised))?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}
