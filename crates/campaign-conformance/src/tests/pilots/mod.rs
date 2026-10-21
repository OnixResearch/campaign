//! `campaign.conformance.positive`, `campaign.consumer_pilots.valid`,
//! `campaign.non_vm_candidate_fixture.reproducible`, and `campaign.guidance.float`.

#[test]
// r[verify campaign.conformance.positive]
// r[verify campaign.consumer_pilots.valid]
// r[verify campaign.fd.verification.matrix]
// r[verify campaign.non_vm_candidate_fixture]
fn the_animus_shaped_adapter_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let report = crate::catalog::run_suite(&mut adapter, &profile);
    assert_eq!(super::failures(&report), alloc::vec::Vec::new());
    assert_eq!(report.outcomes.len(), crate::catalog::fixture_names().len());
    let has_both_polarities = report
        .outcomes
        .iter()
        .any(|outcome| outcome.polarity == crate::verdicts::FixturePolarity::Negative);
    assert!(has_both_polarities);
    Ok(())
}

#[test]
// r[verify campaign.conformance.positive]
// r[verify campaign.fd.verification.matrix]
// r[verify campaign.guidance.float]
fn the_float_scoring_adapter_passes_every_fixture() -> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::float_scores::FixedPointAdapter::new();
    let report = crate::catalog::run_suite(&mut adapter, &profile);
    assert_eq!(super::failures(&report), alloc::vec::Vec::new());
    assert!(!report.transcript.entries.is_empty());
    Ok(())
}

#[test]
// r[verify campaign.consumer_pilots.valid]
// r[verify campaign.guidance.opaque]
fn two_conforming_consumers_agree_on_every_decision() -> Result<(), campaign_core::CampaignError> {
    let profile =
        super::default_profile().map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    let mut animus_shaped = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut float_scores = crate::fixtures::float_scores::FixedPointAdapter::new();
    let left = crate::catalog::run_suite(&mut animus_shaped, &profile);
    let right = crate::catalog::run_suite(&mut float_scores, &profile);
    let left_ids = crate::port::MomentMapping::identities(&animus_shaped)
        .map_err(|_| campaign_core::CampaignError::MissingIdentity)?;
    let right_ids = crate::port::MomentMapping::identities(&float_scores)
        .map_err(|_| campaign_core::CampaignError::MissingIdentity)?;
    assert_ne!(left_ids.adapter_id, right_ids.adapter_id);
    assert_eq!(left.transcript, right.transcript);
    assert_eq!(left.transcript.digest()?, right.transcript.digest()?);
    Ok(())
}

#[test]
// r[verify campaign.non_vm_candidate_fixture.reproducible]
fn equal_inputs_reproduce_equal_decisions() -> Result<(), campaign_core::CampaignError> {
    let profile =
        super::default_profile().map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    let mut first = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut second = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let left = crate::catalog::run_suite(&mut first, &profile);
    let right = crate::catalog::run_suite(&mut second, &profile);
    assert_eq!(left, right);
    assert_eq!(left.transcript.digest()?, right.transcript.digest()?);
    Ok(())
}

#[test]
fn a_changed_profile_changes_the_decisions_not_the_agreement()
-> Result<(), campaign_core::CampaignError> {
    let mut profile =
        super::default_profile().map_err(|_| campaign_core::CampaignError::InvalidConfig)?;
    let default_digest = crate::catalog::run_suite(
        &mut crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
        &profile,
    )
    .transcript
    .digest()?;
    profile.exploration_order = alloc::vec![1];
    profile.retention_rank_order = alloc::vec![1];
    let left = crate::catalog::run_suite(
        &mut crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
        &profile,
    );
    let right = crate::catalog::run_suite(
        &mut crate::fixtures::float_scores::FixedPointAdapter::new(),
        &profile,
    );
    assert_eq!(super::failures(&left), alloc::vec::Vec::new());
    assert_ne!(left.transcript.digest()?, default_digest);
    assert_eq!(left.transcript.digest()?, right.transcript.digest()?);
    Ok(())
}

#[test]
fn an_unusable_profile_fails_every_fixture_by_name() -> crate::verdicts::ConformanceResult<()> {
    let mut profile = super::default_profile()?;
    profile.exploratory_threshold_parts_per_million = 0;
    let report = crate::catalog::run_suite(
        &mut crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new(),
        &profile,
    );
    assert_eq!(report.failures().len(), report.outcomes.len());
    assert!(report.outcomes.iter().all(|outcome| matches!(
        outcome.verdict,
        crate::verdicts::FixtureVerdict::PositiveFixtureFailed(
            crate::verdicts::ConformanceError::InvalidProfile { .. }
        ) | crate::verdicts::FixtureVerdict::NegativeFixtureFailed(
            crate::verdicts::ConformanceError::InvalidProfile { .. }
        )
    )));
    Ok(())
}

#[test]
// r[verify campaign.guidance.float]
fn float_scores_convert_to_bounded_monotone_integer_ranks()
-> Result<(), campaign_core::CampaignError> {
    let rank = crate::fixtures::float_scores::fixed_point_rank;
    assert_eq!(rank(0.0)?, 0);
    assert_eq!(rank(-0.0)?, 0);
    assert_eq!(
        rank(1.0)?,
        1 << crate::fixtures::float_scores::FRACTION_BITS
    );
    assert_eq!(rank(0.25)?.checked_mul(4), Some(rank(1.0)?));
    assert!(rank(7.5)? > rank(7.25)?);
    assert_eq!(rank(f64::MIN_POSITIVE)?, 0);
    for invalid in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            rank(invalid),
            Err(campaign_core::CampaignError::InvalidGuidanceShape)
        );
    }
    assert_eq!(
        rank(f64::MAX),
        Err(campaign_core::CampaignError::ArithmeticOverflow)
    );
    let conversion = crate::fixtures::float_scores::FixedPointAdapter::conversion_profile_id()
        .map_err(|_| campaign_core::CampaignError::MissingIdentity)?;
    let identities = crate::port::MomentMapping::identities(
        &crate::fixtures::float_scores::FixedPointAdapter::new(),
    )
    .map_err(|_| campaign_core::CampaignError::MissingIdentity)?;
    assert_eq!(identities.guidance_profile_id, conversion);
    Ok(())
}
