//! Explicit duplicate-candidate admission policy recorded in admission events.

pub(super) fn fixture() -> Result<
    (
        crate::suite::ingress::support::Events,
        campaign_core::FrontierState,
        campaign_core::CandidateRecord,
    ),
    String,
> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, existing) = crate::suite::candidate::admit(
        events,
        &state,
        b"existing",
        crate::suite::constants::COST_ONE,
    )?;
    let record = crate::suite::ingress::support::text(state.candidate(existing))?
        .candidate
        .clone();
    Ok((events, state, record))
}

pub(super) fn fresh(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::CandidateRecord, String> {
    crate::suite::ingress::support::text(
        crate::suite::candidate::record(state, b"fresh", crate::suite::constants::COST_ONE)
            .derive(),
    )
}

#[test]
fn the_admission_rule_derives_distinct_policy_identities() -> Result<(), String> {
    let reject = crate::suite::ingress::support::text(
        crate::suite::ingress::support::REJECT.admission_policy_id(),
    )?;
    let coalesce = crate::suite::ingress::support::text(
        crate::suite::ingress::support::COALESCE.admission_policy_id(),
    )?;
    assert_eq!(reject.kind, campaign_core::IdKind::AdmissionPolicy);
    assert_eq!(coalesce.kind, campaign_core::IdKind::AdmissionPolicy);
    assert_ne!(reject, coalesce);
    assert_eq!(
        reject,
        crate::suite::ingress::support::text(
            crate::suite::ingress::support::REJECT.admission_policy_id()
        )?
    );
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.invalid]
fn reject_denies_the_whole_batch_on_any_duplicate() -> Result<(), String> {
    let (events, state, existing) = fixture()?;
    let fresh = fresh(&state)?;
    for batch in [
        vec![fresh.clone(), existing.clone()],
        vec![fresh.clone(), fresh.clone()],
    ] {
        assert_eq!(
            crate::suite::ingress::support::admit_batch(
                &state,
                batch,
                crate::suite::ingress::support::REJECT
            ),
            Err(campaign_core::CampaignError::DuplicateIdentity)
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![fresh.clone()],
        crate::suite::ingress::support::REJECT,
    ))?;
    let batch = crate::suite::ingress::support::batch_of(&event)?;
    assert_eq!(
        batch.outcomes,
        vec![campaign_core::AdmissionOutcome {
            candidate_id: fresh.candidate_id,
            outcome: campaign_core::AdmissionDecision::Admitted,
        }]
    );
    assert_eq!(
        batch.admission_policy_id,
        crate::suite::ingress::support::text(
            crate::suite::ingress::support::REJECT.admission_policy_id()
        )?
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.admission]
// r[verify campaign.candidate_admission.coalesce]
fn coalesce_removes_duplicates_and_records_the_reason() -> Result<(), String> {
    let (events, state, existing) = fixture()?;
    let fresh = fresh(&state)?;
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![fresh.clone(), existing.clone(), fresh.clone()],
        crate::suite::ingress::support::COALESCE,
    ))?;
    let batch = crate::suite::ingress::support::batch_of(&event)?;
    assert_eq!(batch.candidates, vec![fresh.clone()]);
    assert_eq!(
        batch.admission_rule,
        crate::suite::ingress::support::COALESCE
    );
    assert_eq!(
        batch.admission_policy_id,
        crate::suite::ingress::support::text(
            crate::suite::ingress::support::COALESCE.admission_policy_id()
        )?
    );
    let mut expected = vec![
        campaign_core::AdmissionOutcome {
            candidate_id: existing.candidate_id,
            outcome: campaign_core::AdmissionDecision::CoalescedDuplicate {
                existing_candidate_id: existing.candidate_id,
            },
        },
        campaign_core::AdmissionOutcome {
            candidate_id: fresh.candidate_id,
            outcome: campaign_core::AdmissionDecision::Admitted,
        },
        campaign_core::AdmissionOutcome {
            candidate_id: fresh.candidate_id,
            outcome: campaign_core::AdmissionDecision::CoalescedDuplicate {
                existing_candidate_id: fresh.candidate_id,
            },
        },
    ];
    expected.sort_by_key(|outcome| outcome.candidate_id);
    assert_eq!(batch.outcomes, expected);
    assert_eq!(
        batch.admission_outcomes_id,
        crate::suite::ingress::support::text(campaign_core::derive_admission_outcomes_id(
            &expected
        ))?
    );
    assert_eq!(
        batch.admission_outcomes_id.kind,
        campaign_core::IdKind::AdmissionOutcomes
    );
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(
        next.candidates.len(),
        state.candidates.len().saturating_add(1)
    );
    Ok(())
}

#[test]
// r[verify campaign.candidate_admission.coalesce]
fn a_fully_coalesced_batch_is_recorded_without_adding_candidates() -> Result<(), String> {
    let (events, state, existing) = fixture()?;
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![existing.clone()],
        crate::suite::ingress::support::COALESCE,
    ))?;
    let batch = crate::suite::ingress::support::batch_of(&event)?;
    assert!(batch.candidates.is_empty());
    assert_eq!(
        batch.outcomes[0].outcome,
        campaign_core::AdmissionDecision::CoalescedDuplicate {
            existing_candidate_id: existing.candidate_id,
        }
    );
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(next.candidates, state.candidates);
    Ok(())
}

#[test]
// r[verify campaign.guidance.coalesced]
fn equal_rank_guidance_is_recorded_as_a_coalesced_duplicate() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let equal = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        crate::suite::constants::GENERATION_NEXT,
        vec![crate::suite::constants::RANK_HIGH],
    );
    let event = crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, equal))?;
    let guidance = crate::suite::ingress::support::guidance_of(&event)?;
    assert_eq!(
        guidance.dynamics,
        campaign_core::GuidanceDynamics::CoalescedDuplicate {
            existing_guidance_id: previous.guidance_id,
        }
    );
    assert_eq!(
        guidance.admission_outcomes_id,
        crate::suite::ingress::support::text(
            guidance.dynamics.outcomes_id(guidance.guidance.guidance_id)
        )?
    );
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?,
        previous
    );
    assert_eq!(next.guidance_record_count, state.guidance_record_count);
    Ok(())
}
