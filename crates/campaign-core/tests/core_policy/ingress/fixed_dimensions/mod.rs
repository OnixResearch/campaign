//! `campaign.guidance.shape`: the first admitted guidance record fixes the campaign's rank-vector
//! length; the admitted rank-dimension bound is checked before that fixed length.

const FIRST_GENERATION: u64 = 1;
const SECOND_GENERATION: u64 = 2;
const RANK: i64 = 20;
const ONE_DIMENSION: usize = 1;
const TWO_DIMENSIONS: usize = 2;
/// Admission (discovery, successor), revision, and two replay answers of one guidance record.
const ANSWERS: usize = 5;

/// Rank vectors of another length than one dimension with their typed refusal once one dimension
/// is fixed: within the rank-dimension bound `InvalidGuidanceShape`, one past it `BoundExceeded`.
fn mismatched_lengths(
    state: &campaign_core::FrontierState,
) -> Result<Vec<(Vec<i64>, campaign_core::CampaignError)>, String> {
    let bound = usize::try_from(state.admission.bounds.max_rank_dimensions)
        .map_err(crate::suite::s3_fixture::err)?;
    assert!(
        bound > TWO_DIMENSIONS,
        "two dimensions stay within the bound"
    );
    let over = bound
        .checked_add(1)
        .ok_or("rank-dimension bound overflow")?;
    assert!(over > bound);
    let invalid_shape = campaign_core::CampaignError::InvalidGuidanceShape;
    Ok(vec![
        (vec![RANK; TWO_DIMENSIONS], invalid_shape.clone()),
        (vec![RANK; bound], invalid_shape),
        (
            vec![RANK; over],
            campaign_core::CampaignError::BoundExceeded,
        ),
    ])
}

/// Replay answer of a guidance event carrying `record` under `dynamics`.
fn replayed(
    events: &crate::suite::ingress::support::Events,
    state: &campaign_core::FrontierState,
    record: campaign_core::GuidanceRecord,
    dynamics: campaign_core::GuidanceDynamics,
) -> Result<campaign_core::CampaignResult<()>, String> {
    let record = crate::suite::ingress::support::text(record.derive())?;
    let forged = crate::suite::ingress::support::forged_guidance(state, record, dynamics)?;
    Ok(crate::suite::ingress::support::replay_with(events, forged).map(|_| ()))
}

/// Guidance admission (discovery and successor), revision, and replay answers of `ranks`.
fn answers(
    events: &crate::suite::ingress::support::Events,
    state: &campaign_core::FrontierState,
    (guided, fixed_id, other): (campaign_core::Id, campaign_core::Id, campaign_core::Id),
    ranks: Vec<i64>,
) -> Result<[campaign_core::CampaignResult<()>; ANSWERS], String> {
    assert_ne!(ranks.len(), ONE_DIMENSION);
    let discovery =
        crate::suite::ingress::support::guidance(state, other, FIRST_GENERATION, ranks.clone());
    let successor =
        crate::suite::ingress::support::guidance(state, guided, SECOND_GENERATION, ranks);
    assert_eq!(successor.ranks, discovery.ranks);
    let revision = campaign_core::GuidanceRevisionRequest {
        candidate_id: guided,
        successor: successor.clone(),
        invalidated_guidance_id: fixed_id,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"relength"),
    };
    let improvement = campaign_core::GuidanceDynamics::StrictImprovement {
        previous_guidance_id: fixed_id,
    };
    Ok([
        campaign_core::admit_guidance(state, discovery.clone()).map(|_| ()),
        campaign_core::admit_guidance(state, successor.clone()).map(|_| ()),
        campaign_core::revise_guidance(state, revision).map(|_| ()),
        replayed(
            events,
            state,
            discovery,
            campaign_core::GuidanceDynamics::Discovery,
        )?,
        replayed(events, state, successor, improvement)?,
    ])
}

/// Once a one-dimension record fixes the length, a discovery, a successor, a revision and a
/// replayed guidance event of another length within the bound fail with `InvalidGuidanceShape`,
/// and one longer than the bound fails with `BoundExceeded`, all without state mutation; before
/// the length is fixed only the bound applies, and a record of the fixed length is admitted.
#[test]
// r[verify campaign.guidance.shape]
// r[verify campaign.guidance.invalid]
fn another_rank_length_is_refused_and_an_over_bound_length_exceeds_the_bound() -> Result<(), String>
{
    let (events, state) = crate::suite::admission::initial_state()?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, guided) = crate::suite::candidate::admit(events, &state, b"fixed", cost)?;
    let (events, state, other) = crate::suite::candidate::admit(events, &state, b"unfixed", cost)?;
    for (ranks, expected) in mismatched_lengths(&state)? {
        let record =
            crate::suite::ingress::support::guidance(&state, other, FIRST_GENERATION, ranks);
        let unfixed = campaign_core::admit_guidance(&state, record).map(|_| ());
        let is_over_bound = expected == campaign_core::CampaignError::BoundExceeded;
        assert_eq!(unfixed, if is_over_bound { Err(expected) } else { Ok(()) });
    }
    let (events, state, fixed_id) =
        crate::suite::candidate::guide(events, &state, guided, RANK, FIRST_GENERATION)?;
    for (ranks, expected) in mismatched_lengths(&state)? {
        for answer in answers(&events, &state, (guided, fixed_id, other), ranks)? {
            assert_eq!(answer, Err(expected.clone()));
        }
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let fitting = vec![RANK; ONE_DIMENSION];
    let fitting =
        crate::suite::ingress::support::guidance(&state, other, FIRST_GENERATION, fitting);
    let admitted = campaign_core::admit_guidance(&state, fitting);
    assert!(admitted.is_ok(), "{admitted:?}");
    Ok(())
}
