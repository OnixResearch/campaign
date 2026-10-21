//! The guidance record bound holds at intake and replay; a coalesced duplicate adds no record.

const ONE_RECORD: u64 = 1;
const TWO_RECORDS: u64 = 2;
const FIRST_GENERATION: u64 = 1;
const SECOND_GENERATION: u64 = 2;
const THIRD_GENERATION: u64 = 3;
const TOP_RANK: i64 = 30;

/// Every explicit revision adds a guidance record whatever its ranks, so with the bound used up
/// both a lower-rank and an equal-rank revision are `BoundExceeded`.
fn assert_revisions_exceed_the_bound(
    state: &campaign_core::FrontierState,
    guided: campaign_core::Id,
    existing_id: campaign_core::Id,
    rank: i64,
) {
    for successor_rank in [crate::suite::constants::RANK_LOW, rank] {
        let revision = campaign_core::GuidanceRevisionRequest {
            candidate_id: guided,
            successor: crate::suite::ingress::support::guidance(
                state,
                guided,
                THIRD_GENERATION,
                vec![successor_rank],
            ),
            invalidated_guidance_id: existing_id,
            reason_id: crate::suite::admission::xid(
                campaign_core::IdKind::Reason,
                b"full-revision",
            ),
        };
        assert_eq!(
            campaign_core::revise_guidance(state, revision),
            Err(campaign_core::CampaignError::BoundExceeded)
        );
    }
}

#[test]
// r[verify campaign.guidance.invalid]
fn the_guidance_record_bound_is_checked_at_intake_and_replay() -> Result<(), String> {
    let (events, state) = crate::suite::ingress::support::admitted_with(|bounds| {
        bounds.max_guidance_records = TWO_RECORDS;
    })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"bounded",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _first) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_LOW,
        FIRST_GENERATION,
    )?;
    let (events, state, second) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        SECOND_GENERATION,
    )?;
    let third = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        THIRD_GENERATION,
        vec![TOP_RANK],
    );
    assert_eq!(
        campaign_core::admit_guidance(&state, third.clone()),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    let forged = crate::suite::ingress::support::forged_guidance(
        &state,
        crate::suite::ingress::support::text(third.derive())?,
        campaign_core::GuidanceDynamics::StrictImprovement {
            previous_guidance_id: second,
        },
    )?;
    assert_eq!(
        crate::suite::ingress::support::replay_with(&events, forged),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    crate::suite::ingress::support::assert_unchanged(&events, &state)
}

/// `campaign.guidance.coalesced` and `campaign.guidance.invalid`: with the guidance-record bound
/// used up, an equal-rank successor whose generation is not lower is admitted as a coalesced
/// duplicate naming the existing record, leaving the latest record, the record count and later
/// selection unchanged; a lower generation is stale, and every record-adding update (discovery,
/// strict improvement, revision, including an equal-rank revision) fails with `BoundExceeded`
/// without state mutation.
#[test]
// r[verify campaign.guidance.coalesced]
// r[verify campaign.guidance.invalid]
fn an_equal_rank_successor_coalesces_when_the_guidance_record_bound_is_used_up()
-> Result<(), String> {
    let (events, state) = crate::suite::ingress::support::admitted_with(|bounds| {
        bounds.max_guidance_records = ONE_RECORD;
    })?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, guided) = crate::suite::candidate::admit(events, &state, b"full", cost)?;
    let (events, state, unguided) =
        crate::suite::candidate::admit(events, &state, b"full-unguided", cost)?;
    let rank = crate::suite::constants::RANK_HIGH;
    let (events, state, existing_id) =
        crate::suite::candidate::guide(events, &state, guided, rank, SECOND_GENERATION)?;
    assert_eq!(state.guidance_record_count, ONE_RECORD);
    let existing = crate::suite::ingress::support::latest(&state, guided)?;
    assert_eq!(existing.guidance_id, existing_id);
    let coalesced = campaign_core::GuidanceDynamics::CoalescedDuplicate {
        existing_guidance_id: existing_id,
    };
    for generation in [SECOND_GENERATION, THIRD_GENERATION] {
        let equal =
            crate::suite::ingress::support::guidance(&state, guided, generation, vec![rank]);
        let event =
            crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, equal))?;
        let admitted = crate::suite::ingress::support::guidance_of(&event)?;
        assert_eq!(admitted.dynamics, coalesced);
        let (_events, next) = crate::suite::ingress::support::accept(events.clone(), event)?;
        assert_eq!(next.guidance_record_count, ONE_RECORD);
        assert_eq!(
            crate::suite::ingress::support::latest(&next, guided)?,
            existing
        );
        assert_eq!(
            crate::suite::selection::plan(&next)?.guidance_id,
            existing_id
        );
    }
    let lower =
        crate::suite::ingress::support::guidance(&state, guided, FIRST_GENERATION, vec![rank]);
    assert_eq!(
        campaign_core::admit_guidance(&state, lower),
        Err(campaign_core::CampaignError::StaleGuidance)
    );
    let bound = Err(campaign_core::CampaignError::BoundExceeded);
    let better =
        crate::suite::ingress::support::guidance(&state, guided, THIRD_GENERATION, vec![TOP_RANK]);
    assert_eq!(campaign_core::admit_guidance(&state, better), bound);
    let discovery =
        crate::suite::ingress::support::guidance(&state, unguided, FIRST_GENERATION, vec![rank]);
    assert_eq!(campaign_core::admit_guidance(&state, discovery), bound);
    assert_revisions_exceed_the_bound(&state, guided, existing_id, rank);
    crate::suite::ingress::support::assert_unchanged(&events, &state)
}

/// `campaign.guidance.invalid` below the bound: with room for one more guidance record, an
/// explicit revision whose ranks equal the latest record's is admitted as a revision (neither
/// refused as non-improving nor coalesced), adds a record, and becomes the candidate's latest
/// guidance; `an_equal_rank_successor_coalesces_when_the_guidance_record_bound_is_used_up` shows
/// the same revision is `BoundExceeded` once the bound is used up.
#[test]
// r[verify campaign.guidance.invalid]
// r[verify campaign.guidance.revision]
fn an_equal_rank_revision_below_the_bound_is_admitted_as_a_revision() -> Result<(), String> {
    let (events, state) = crate::suite::ingress::support::admitted_with(|bounds| {
        bounds.max_guidance_records = TWO_RECORDS;
    })?;
    let cost = crate::suite::constants::COST_ONE;
    let (events, state, guided) = crate::suite::candidate::admit(events, &state, b"below", cost)?;
    let rank = crate::suite::constants::RANK_HIGH;
    let (events, state, existing_id) =
        crate::suite::candidate::guide(events, &state, guided, rank, SECOND_GENERATION)?;
    assert_eq!(state.guidance_record_count, ONE_RECORD);
    let reason_id = crate::suite::admission::xid(campaign_core::IdKind::Reason, b"below-revision");
    let revision = campaign_core::GuidanceRevisionRequest {
        candidate_id: guided,
        successor: crate::suite::ingress::support::guidance(
            &state,
            guided,
            THIRD_GENERATION,
            vec![rank],
        ),
        invalidated_guidance_id: existing_id,
        reason_id,
    };
    let event =
        crate::suite::ingress::support::text(campaign_core::revise_guidance(&state, revision))?;
    let admitted = crate::suite::ingress::support::guidance_of(&event)?.clone();
    assert!(
        matches!(
            admitted.dynamics,
            campaign_core::GuidanceDynamics::Revision {
                invalidated_guidance_id,
                reason_id: revised_for,
                ..
            } if invalidated_guidance_id == existing_id && revised_for == reason_id
        ),
        "{:?}",
        admitted.dynamics
    );
    assert_eq!(admitted.guidance.ranks, vec![rank]);
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(next.guidance_record_count, TWO_RECORDS);
    let latest = crate::suite::ingress::support::latest(&next, guided)?;
    assert_eq!(latest.guidance_id, admitted.guidance.guidance_id);
    assert_ne!(latest.guidance_id, existing_id);
    Ok(())
}
