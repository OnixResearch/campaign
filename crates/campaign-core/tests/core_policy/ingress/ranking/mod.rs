//! Opaque deterministic guidance: identity, successor updates, invalid shapes, and opacity.

type Edit = fn(&mut campaign_core::GuidanceRecord);

const ONE_DIMENSION: u64 = 1;
const FIRST_GENERATION: u64 = 1;
const SECOND_GENERATION: u64 = 2;
const THIRD_GENERATION: u64 = 3;
const TOP_RANK: i64 = 30;

fn xid(kind: campaign_core::IdKind, label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(kind, label)
}

const IDENTITY_EDITS: [Edit; 7] = [
    |record| record.campaign_id = xid(campaign_core::IdKind::Campaign, b"other-campaign"),
    |record| record.candidate_id = xid(campaign_core::IdKind::Candidate, b"other-candidate"),
    |record| record.generation = THIRD_GENERATION,
    |record| record.ranks = vec![TOP_RANK],
    |record| record.source_facts_id = xid(campaign_core::IdKind::SourceFacts, b"other-facts"),
    |record| {
        record.ordering_profile_id = xid(campaign_core::IdKind::GuidanceProfile, b"other-order");
    },
    |record| record.adapter_id = xid(campaign_core::IdKind::Adapter, b"other-adapter"),
];

#[test]
// r[verify campaign.guidance]
fn guidance_identity_binds_every_record_fact() -> Result<(), String> {
    let (_events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_LOW)?;
    let base = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![crate::suite::constants::RANK_HIGH],
    );
    let base_id = crate::suite::ingress::support::text(base.clone().derive())?.guidance_id;
    let mut seen = vec![base_id];
    for edit in IDENTITY_EDITS {
        let mut changed = base.clone();
        edit(&mut changed);
        let changed_id = crate::suite::ingress::support::text(changed.derive())?.guidance_id;
        assert!(!seen.contains(&changed_id));
        seen.push(changed_id);
    }
    assert_eq!(
        base_id,
        crate::suite::ingress::support::text(base.derive())?.guidance_id
    );
    Ok(())
}

#[test]
// r[verify campaign.guidance.update]
fn a_successor_generation_keeps_the_candidate_and_drives_later_selection() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_LOW)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let candidate = crate::suite::ingress::support::text(state.candidate(candidate_id))?
        .candidate
        .clone();
    let successor = crate::suite::ingress::support::guidance(
        &state,
        candidate_id,
        SECOND_GENERATION,
        vec![crate::suite::constants::RANK_HIGH],
    );
    let event =
        crate::suite::ingress::support::text(campaign_core::admit_guidance(&state, successor))?;
    let admitted = crate::suite::ingress::support::guidance_of(&event)?.clone();
    assert_eq!(
        admitted.dynamics,
        campaign_core::GuidanceDynamics::StrictImprovement {
            previous_guidance_id: previous.guidance_id,
        }
    );
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(
        crate::suite::ingress::support::text(next.candidate(candidate_id))?.candidate,
        candidate
    );
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?,
        admitted.guidance
    );
    let selection = crate::suite::selection::plan(&next)?;
    assert_eq!(selection.candidate_id, candidate_id);
    assert_eq!(selection.guidance_id, admitted.guidance.guidance_id);
    assert_ne!(selection.guidance_id, previous.guidance_id);
    Ok(())
}

const INVALID: [(&str, Edit, campaign_core::CampaignError); 10] = [
    (
        "too many dimensions",
        |record| record.ranks.push(crate::suite::constants::RANK_HIGH),
        campaign_core::CampaignError::BoundExceeded,
    ),
    (
        "stale equal generation",
        |record| record.generation = FIRST_GENERATION,
        campaign_core::CampaignError::StaleGuidance,
    ),
    (
        "stale older generation",
        |record| record.generation = campaign_core::NO_SELECTIONS,
        campaign_core::CampaignError::StaleGuidance,
    ),
    (
        "unsupported ordering profile",
        |record| {
            record.ordering_profile_id =
                xid(campaign_core::IdKind::GuidanceProfile, b"unsupported-order");
        },
        campaign_core::CampaignError::CrossedIdentity,
    ),
    (
        "rank above the magnitude bound",
        |record| record.ranks = vec![i64::MAX],
        campaign_core::CampaignError::InvalidGuidanceShape,
    ),
    (
        "rank below the magnitude bound",
        |record| record.ranks = vec![i64::MIN],
        campaign_core::CampaignError::InvalidGuidanceShape,
    ),
    (
        "crossed campaign",
        |record| record.campaign_id = xid(campaign_core::IdKind::Campaign, b"other-campaign"),
        campaign_core::CampaignError::CrossedIdentity,
    ),
    (
        "unknown candidate",
        |record| record.candidate_id = xid(campaign_core::IdKind::Candidate, b"unknown"),
        campaign_core::CampaignError::MissingCandidate,
    ),
    (
        "mismatched adapter",
        |record| record.adapter_id = xid(campaign_core::IdKind::Adapter, b"other-adapter"),
        campaign_core::CampaignError::CrossedIdentity,
    ),
    (
        "empty ranks",
        |record| record.ranks.clear(),
        campaign_core::CampaignError::InvalidGuidanceShape,
    ),
];

fn one_dimension_fixture() -> Result<
    (
        crate::suite::ingress::support::Events,
        campaign_core::FrontierState,
        campaign_core::Id,
    ),
    String,
> {
    let (events, state) = crate::suite::ingress::support::admitted_with(|bounds| {
        bounds.max_rank_dimensions = ONE_DIMENSION;
    })?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"crate::suite::ingress::support::guided",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_LOW,
        FIRST_GENERATION,
    )?;
    Ok((events, state, candidate_id))
}

#[test]
// r[verify campaign.guidance.invalid]
// r[verify campaign.guidance.non_improving]
fn invalid_guidance_is_denied_at_intake_without_mutation() -> Result<(), String> {
    let (events, state, candidate_id) = one_dimension_fixture()?;
    let selection = crate::suite::selection::plan(&state)?;
    for (name, edit, expected) in INVALID {
        let mut record = crate::suite::ingress::support::guidance(
            &state,
            candidate_id,
            SECOND_GENERATION,
            vec![crate::suite::constants::RANK_HIGH],
        );
        edit(&mut record);
        assert_eq!(
            campaign_core::admit_guidance(&state, record),
            Err(expected.clone()),
            "{name}"
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
        assert_eq!(crate::suite::selection::plan(&state)?, selection);
    }
    Ok(())
}

#[test]
// r[verify campaign.guidance.invalid]
fn invalid_guidance_is_denied_at_replay_without_mutation() -> Result<(), String> {
    let (events, state, candidate_id) = one_dimension_fixture()?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let claimed = campaign_core::GuidanceDynamics::StrictImprovement {
        previous_guidance_id: previous.guidance_id,
    };
    let valid = crate::suite::ingress::support::text(
        crate::suite::ingress::support::guidance(
            &state,
            candidate_id,
            SECOND_GENERATION,
            vec![crate::suite::constants::RANK_HIGH],
        )
        .derive(),
    )?;
    for (name, edit, expected) in INVALID {
        let mut record = valid.clone();
        edit(&mut record);
        let forged = match record.clone().derive() {
            Ok(derived) => derived,
            Err(_) => record,
        };
        assert_eq!(
            crate::suite::ingress::support::replay_with(
                &events,
                crate::suite::ingress::support::forged_guidance(&state, forged, claimed.clone())?
            ),
            Err(expected.clone()),
            "{name}"
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let mut mislabeled = valid.clone();
    mislabeled.guidance_id = previous.guidance_id;
    assert_eq!(
        crate::suite::ingress::support::replay_with(
            &events,
            crate::suite::ingress::support::forged_guidance(&state, mislabeled, claimed.clone())?
        ),
        Err(campaign_core::CampaignError::MalformedIdentity)
    );
    let mut crossed_event =
        crate::suite::ingress::support::forged_guidance(&state, valid.clone(), claimed.clone())?;
    if let campaign_core::CampaignControlEvent::GuidanceAdmitted(event) = &mut crossed_event {
        event.campaign_id = xid(campaign_core::IdKind::Campaign, b"other-campaign");
    }
    assert_eq!(
        crate::suite::ingress::support::replay_with(&events, crossed_event),
        Err(campaign_core::CampaignError::CrossedIdentity)
    );
    assert!(
        crate::suite::ingress::support::replay_with(
            &events,
            crate::suite::ingress::support::forged_guidance(&state, valid, claimed)?
        )
        .is_ok()
    );
    Ok(())
}
