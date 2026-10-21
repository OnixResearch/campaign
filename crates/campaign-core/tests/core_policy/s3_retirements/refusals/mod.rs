//! Stale, unknown, crossed, and non-retained retirement requests fail without mutation.

fn edited(
    base: &campaign_core::RetirementRequest,
    edit: impl FnOnce(&mut campaign_core::RetirementRequest),
) -> campaign_core::RetirementRequest {
    let mut request = base.clone();
    edit(&mut request);
    request
}

fn other(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

#[test]
// r[verify campaign.retirement.invalid]
fn stale_unknown_crossed_and_non_retained_requests_are_denied_without_mutation()
-> Result<(), String> {
    let (events, admitted) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &admitted,
        b"target",
        crate::suite::constants::RANK_HIGH,
    )?;
    let valid = crate::suite::s3_fixture::retirement(
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Consumer,
    );
    let crossed = campaign_core::CampaignError::CrossedIdentity;
    let cases = [
        (
            edited(&valid, |request| request.state_id = admitted.state_id),
            campaign_core::CampaignError::StaleState,
        ),
        (
            edited(&valid, |request| {
                request.candidate_id = other(campaign_core::IdKind::Candidate, b"unknown");
            }),
            campaign_core::CampaignError::MissingCandidate,
        ),
        (
            edited(&valid, |request| {
                request.campaign_id = other(campaign_core::IdKind::Campaign, b"other-campaign");
            }),
            crossed.clone(),
        ),
        (
            edited(&valid, |request| {
                request.policy_id = other(campaign_core::IdKind::Policy, b"other-policy");
            }),
            crossed.clone(),
        ),
        (
            edited(&valid, |request| {
                request.adapter_id = other(campaign_core::IdKind::Adapter, b"other-adapter");
            }),
            crossed,
        ),
    ];
    for (request, expected) in cases {
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::retire_candidate(&state, request))?,
            expected
        );
        crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    }
    let (events, retired, _retirement) = crate::suite::s3_retirements::retire(
        events,
        &state,
        candidate_id,
        campaign_core::RetirementOrigin::Consumer,
    )?;
    let again = crate::suite::s3_fixture::retirement(
        &retired,
        candidate_id,
        campaign_core::RetirementOrigin::Policy,
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::retire_candidate(&retired, again))?,
        campaign_core::CampaignError::InvalidRetirement
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &retired)
}

#[test]
// r[verify campaign.retirement.invalid]
fn replay_rejects_a_crossed_or_forged_retirement() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"forged",
        crate::suite::constants::RANK_HIGH,
    )?;
    let event = campaign_core::retire_candidate(
        &state,
        crate::suite::s3_fixture::retirement(
            &state,
            candidate_id,
            campaign_core::RetirementOrigin::Consumer,
        ),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidateRetired(valid) = event else {
        return Err(String::from("expected a retirement event"));
    };
    let mut crossed_policy = valid.clone();
    crossed_policy.policy_id = other(campaign_core::IdKind::Policy, b"forged-policy");
    crossed_policy.invalidation_id =
        campaign_core::derive_retirement_invalidation_id(&crossed_policy)
            .map_err(crate::suite::s3_fixture::err)?;
    crossed_policy.event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::CandidateRetired,
        crossed_policy.campaign_id,
        crossed_policy.state_before_id,
        &[
            crossed_policy.candidate_id,
            crossed_policy.reason_id,
            crossed_policy.invalidation_id,
        ],
    )
    .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            campaign_core::CampaignControlEvent::CandidateRetired(crossed_policy)
        )?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut forged_invalidation = valid;
    forged_invalidation.origin = campaign_core::RetirementOrigin::Policy;
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            campaign_core::CampaignControlEvent::CandidateRetired(forged_invalidation)
        )?,
        campaign_core::CampaignError::MalformedIdentity
    );
    Ok(())
}
