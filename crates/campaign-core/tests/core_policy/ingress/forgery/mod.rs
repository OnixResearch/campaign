//! Replay re-derives every recorded candidate admission outcome.

type Coalesced = (
    crate::suite::ingress::support::Events,
    campaign_core::FrontierState,
    campaign_core::CampaignControlEvent,
    campaign_core::CandidateBatchEvent,
);

fn coalesced_fixture() -> Result<Coalesced, String> {
    let (events, state, existing) = super::dedupe::fixture()?;
    let fresh = super::dedupe::fresh(&state)?;
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![fresh, existing],
        crate::suite::ingress::support::COALESCE,
    ))?;
    let valid = crate::suite::ingress::support::batch_of(&event)?.clone();
    assert_eq!(valid.outcomes.len(), 2);
    assert_eq!(valid.candidates.len(), 1);
    Ok((events, state, event, valid))
}

#[test]
fn replay_rechecks_every_recorded_admission_outcome() -> Result<(), String> {
    let (events, state, _event, valid) = coalesced_fixture()?;
    assert_eq!(
        valid.admission_rule,
        crate::suite::ingress::support::COALESCE
    );
    let mut claimed_admitted = valid.clone();
    for outcome in &mut claimed_admitted.outcomes {
        outcome.outcome = campaign_core::AdmissionDecision::Admitted;
    }
    let mut claimed_coalesced = valid.clone();
    claimed_coalesced.candidates.clear();
    for outcome in &mut claimed_coalesced.outcomes {
        outcome.outcome = campaign_core::AdmissionDecision::CoalescedDuplicate {
            existing_candidate_id: outcome.candidate_id,
        };
    }
    let mut hidden_candidate = valid.clone();
    hidden_candidate.candidates.clear();
    let mut rule_swapped = valid.clone();
    rule_swapped.admission_rule = crate::suite::ingress::support::REJECT;
    let mut policy_forged = valid.clone();
    policy_forged.admission_policy_id = crate::suite::ingress::support::text(
        crate::suite::ingress::support::REJECT.admission_policy_id(),
    )?;
    for (forged, expected) in [
        (
            claimed_admitted,
            campaign_core::CampaignError::MalformedIdentity,
        ),
        (
            claimed_coalesced,
            campaign_core::CampaignError::MalformedIdentity,
        ),
        (
            hidden_candidate,
            campaign_core::CampaignError::MalformedIdentity,
        ),
        (
            rule_swapped,
            campaign_core::CampaignError::MalformedIdentity,
        ),
        (
            policy_forged,
            campaign_core::CampaignError::MalformedIdentity,
        ),
    ] {
        assert_eq!(
            crate::suite::ingress::support::replay_with(
                &events,
                crate::suite::ingress::support::resealed_batch(forged)?
            ),
            Err(expected)
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    Ok(())
}

#[test]
fn replay_rechecks_the_rule_and_the_sealed_outcome_identity() -> Result<(), String> {
    let (events, state, event, valid) = coalesced_fixture()?;
    let mut rejected = valid.clone();
    rejected.admission_rule = crate::suite::ingress::support::REJECT;
    rejected.admission_policy_id = crate::suite::ingress::support::text(
        crate::suite::ingress::support::REJECT.admission_policy_id(),
    )?;
    assert_eq!(
        crate::suite::ingress::support::replay_with(
            &events,
            crate::suite::ingress::support::resealed_batch(rejected)?
        ),
        Err(campaign_core::CampaignError::DuplicateIdentity)
    );
    let mut unsealed = valid.clone();
    unsealed.admission_outcomes_id =
        crate::suite::ingress::support::text(campaign_core::derive_admission_outcomes_id(&[]))?;
    assert_eq!(
        crate::suite::ingress::support::replay_with(
            &events,
            campaign_core::CampaignControlEvent::CandidatesAdmitted(unsealed)
        ),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    let mut coalescing_forged = valid.clone();
    coalescing_forged.coalescing_policy_id =
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"forged");
    assert_eq!(
        crate::suite::ingress::support::replay_with(
            &events,
            campaign_core::CampaignControlEvent::CandidatesAdmitted(coalescing_forged)
        ),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    assert!(
        crate::suite::ingress::support::replay_with(&events, event)
            .is_ok_and(|next| next.state_id != state.state_id)
    );
    Ok(())
}
