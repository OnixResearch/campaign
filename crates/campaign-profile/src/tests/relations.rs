//! `campaign.configuration.invalid`: cross-field relations and the closed non-claim list.

#[test]
fn finding_limit_must_not_exceed_the_finding_record_bound() -> super::TestResult {
    const FINDING_RECORDS: u64 = 4_096;
    const OVER_FINDING_RECORDS: u64 = 4_097;
    assert_eq!(
        super::rejected(&super::with_key(
            "/stopping/finding_limit",
            Some(serde_json::json!(OVER_FINDING_RECORDS)),
        )?)?,
        crate::ProfileError::FindingLimitExceedsFindingBound
    );
    let at_bound = super::rejection::loaded(&super::with_key(
        "/stopping/finding_limit",
        Some(serde_json::json!(FINDING_RECORDS)),
    )?)?;
    assert_eq!(at_bound.bounds.max_findings, FINDING_RECORDS);
    assert_eq!(at_bound.stopping.finding_limit, Some(FINDING_RECORDS));
    let mut raised = super::with_key(
        "/stopping/finding_limit",
        Some(serde_json::json!(OVER_FINDING_RECORDS)),
    )?;
    *raised
        .pointer_mut("/bounds/maximum_finding_records")
        .ok_or("default finding bound is absent")? = serde_json::json!(OVER_FINDING_RECORDS);
    let raised = super::rejection::loaded(&raised)?;
    assert_eq!(raised.bounds.max_findings, OVER_FINDING_RECORDS);
    assert_eq!(raised.stopping.finding_limit, Some(OVER_FINDING_RECORDS));
    Ok(())
}

#[test]
fn receipt_non_claims_must_list_each_non_claim_once() -> super::TestResult {
    let mut missing = super::default_document()?;
    missing
        .pointer_mut("/receipt_non_claims")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("default non-claims are absent")?
        .retain(|tag| tag != "replay_success");
    assert_eq!(
        super::rejected(&missing)?,
        crate::ProfileError::MissingNonClaim(campaign_core::NonClaim::ReplaySuccess)
    );
    // The list holds one tag per non-claim, so a repeat displaces a later one; canonical order names the repeat.
    let mut duplicate = super::default_document()?;
    let duplicated = duplicate
        .pointer_mut("/receipt_non_claims")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("default non-claims are absent")?;
    duplicated.retain(|tag| tag != "safe_deletion");
    duplicated.push(serde_json::json!("authorization"));
    assert_eq!(duplicated.len(), campaign_core::NON_CLAIM_COUNT);
    assert_eq!(
        super::rejected(&duplicate)?,
        crate::ProfileError::DuplicateNonClaim(campaign_core::NonClaim::Authorization)
    );
    assert_eq!(
        super::rejected(&super::with("/receipt_non_claims", serde_json::json!([]))?)?,
        crate::ProfileError::MissingNonClaim(campaign_core::NonClaim::ExhaustiveSearch)
    );
    Ok(())
}

#[test]
fn control_events_must_reserve_the_admission_event_and_a_stop() -> super::TestResult {
    const NO_STOP_RESERVE: u64 = 1;
    let error = super::rejected(&super::with(
        "/bounds/maximum_control_events",
        serde_json::json!(NO_STOP_RESERVE),
    )?)?;
    assert_eq!(
        error,
        crate::ProfileError::Campaign(campaign_core::CampaignError::InvalidConfig)
    );
    let reserved = super::rejection::loaded(&super::with(
        "/bounds/maximum_control_events",
        serde_json::json!(campaign_core::MIN_CONTROL_EVENTS),
    )?)?;
    assert_eq!(
        reserved.bounds.max_control_events,
        campaign_core::MIN_CONTROL_EVENTS
    );
    Ok(())
}
