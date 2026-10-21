//! `campaign.receipts.non_claims` controls and event receipts.

#[test]
// r[verify campaign.receipts.non_claims]
fn all_eight_non_claims_are_listed_and_each_overclaim_is_rejected() -> Result<(), String> {
    let fixture = crate::suite::world::selecting()?;
    let projected = crate::suite::world::observed(crate::suite::world::project(&fixture))?;
    assert_eq!(projected.non_claims, campaign_core::NonClaim::ALL.to_vec());
    assert_eq!(projected.non_claims.len(), campaign_core::NON_CLAIM_COUNT);
    let no_claims = campaign_core::ReceiptClaims::no_claims();
    crate::suite::world::observed(campaign_core::validate_receipt(&projected, &no_claims))?;
    let overclaims: [fn(&mut campaign_core::ReceiptClaims); campaign_core::NON_CLAIM_COUNT] = [
        |claims| claims.exhaustive_search = true,
        |claims| claims.event_truth = true,
        |claims| claims.operation_success = true,
        |claims| claims.replay_success = true,
        |claims| claims.finding_correctness = true,
        |claims| claims.authorization = true,
        |claims| claims.release_readiness = true,
        |claims| claims.safe_deletion = true,
    ];
    for overclaim in overclaims {
        let mut claims = campaign_core::ReceiptClaims::no_claims();
        overclaim(&mut claims);
        assert_ne!(claims, no_claims);
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_receipt(&projected, &claims))?,
            campaign_core::CampaignError::InvalidReceiptClaim
        );
    }
    for dropped in campaign_core::NonClaim::ALL {
        let mut omitted = projected.clone();
        omitted.non_claims.retain(|non_claim| non_claim != &dropped);
        assert_eq!(
            omitted.non_claims.len().checked_add(1),
            Some(campaign_core::NON_CLAIM_COUNT)
        );
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_receipt(&omitted, &no_claims))?,
            campaign_core::CampaignError::InvalidReceiptClaim
        );
    }
    let mut forged = projected;
    forged.state_id = crate::suite::admission::xid(campaign_core::IdKind::State, b"forged-state");
    assert_eq!(
        crate::suite::world::denial(campaign_core::validate_receipt(&forged, &no_claims))?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}

#[test]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.stop.non_claim]
fn a_stop_keeps_the_exhaustive_search_non_claim() -> Result<(), String> {
    let (_events, empty) = crate::suite::admission::initial_state()?;
    let exhausted = crate::suite::world::planned(empty)?;
    let campaign_core::NextDecision::Stop(stop) = &exhausted.plan.decision else {
        return Err(String::from("an empty frontier must stop"));
    };
    assert_eq!(stop.reason, campaign_core::StopReason::FrontierExhausted);
    let stop_receipt = crate::suite::world::observed(crate::suite::world::project(&exhausted))?;
    assert_eq!(stop_receipt.decision_id, stop.stop_id);
    assert!(
        stop_receipt
            .non_claims
            .contains(&campaign_core::NonClaim::ExhaustiveSearch)
    );
    let world = crate::suite::world::populated()?;
    let mut events = world.events;
    events.push(crate::suite::world::observed(campaign_core::admit_stop(
        &world.state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"receipt-stop"),
        },
    ))?);
    let stopped = crate::suite::world::planned(crate::suite::world::observed(
        campaign_core::reconstruct(&events),
    )?)?;
    let campaign_core::NextDecision::Stop(recorded) = &stopped.plan.decision else {
        return Err(String::from("a stopped campaign must return its stop"));
    };
    assert_ne!(recorded.state_before_id, stopped.state.state_id);
    let recorded_receipt = crate::suite::world::observed(crate::suite::world::project(&stopped))?;
    assert_eq!(recorded_receipt.decision_id, recorded.stop_id);
    assert!(
        recorded_receipt
            .non_claims
            .contains(&campaign_core::NonClaim::ExhaustiveSearch)
    );
    Ok(())
}
