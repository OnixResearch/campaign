//! Explicit guidance revision naming the invalidated record and a bounded reason.

const SECOND_GENERATION: u64 = 2;
const FIRST_GENERATION: u64 = 1;

fn reason() -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::Reason, b"domain-decay")
}

fn request(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    invalidated_guidance_id: campaign_core::Id,
) -> campaign_core::GuidanceRevisionRequest {
    campaign_core::GuidanceRevisionRequest {
        candidate_id,
        successor: crate::suite::ingress::support::guidance(
            state,
            candidate_id,
            SECOND_GENERATION,
            vec![crate::suite::constants::RANK_LOW],
        ),
        invalidated_guidance_id,
        reason_id: reason(),
    }
}

#[test]
// r[verify campaign.guidance.revision]
fn a_non_improving_successor_is_admitted_as_an_explicit_revision() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let revision = request(&state, candidate_id, previous.guidance_id);
    assert_eq!(
        campaign_core::admit_guidance(&state, revision.successor.clone()),
        Err(campaign_core::CampaignError::NonImprovingRelaxation)
    );
    let event =
        crate::suite::ingress::support::text(campaign_core::revise_guidance(&state, revision))?;
    let revised = crate::suite::ingress::support::guidance_of(&event)?.clone();
    let invalidation_id = crate::suite::ingress::support::text(
        campaign_core::derive_revision_invalidation_id(previous.guidance_id, reason()),
    )?;
    assert_eq!(invalidation_id.kind, campaign_core::IdKind::Invalidation);
    assert_eq!(
        revised.dynamics,
        campaign_core::GuidanceDynamics::Revision {
            invalidated_guidance_id: previous.guidance_id,
            invalidation_id,
            reason_id: reason(),
        }
    );
    let (_events, next) = crate::suite::ingress::support::accept(events, event)?;
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?,
        revised.guidance
    );
    assert_eq!(
        crate::suite::ingress::support::latest(&next, candidate_id)?.ranks,
        vec![crate::suite::constants::RANK_LOW]
    );
    assert_eq!(
        next.guidance_record_count,
        state.guidance_record_count.saturating_add(1)
    );
    let selection = crate::suite::selection::plan(&next)?;
    assert_eq!(selection.guidance_id, revised.guidance.guidance_id);
    Ok(())
}

#[test]
fn invalid_revisions_are_denied_without_mutation() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let stale_id = crate::suite::ingress::support::text(
        crate::suite::ingress::support::guidance(
            &state,
            candidate_id,
            FIRST_GENERATION,
            vec![crate::suite::constants::RANK_LOW],
        )
        .derive(),
    )?
    .guidance_id;
    let mut not_latest = request(&state, candidate_id, stale_id);
    not_latest.successor.generation = SECOND_GENERATION;
    let mut same_generation = request(&state, candidate_id, previous.guidance_id);
    same_generation.successor.generation = FIRST_GENERATION;
    let mut missing_reason = request(&state, candidate_id, previous.guidance_id);
    missing_reason.reason_id = campaign_core::Id::absent();
    let mut crossed_reason = request(&state, candidate_id, previous.guidance_id);
    crossed_reason.reason_id =
        crate::suite::admission::xid(campaign_core::IdKind::Policy, b"not-a-reason");
    let mut crossed_candidate = request(&state, candidate_id, previous.guidance_id);
    crossed_candidate.candidate_id =
        crate::suite::admission::xid(campaign_core::IdKind::Candidate, b"other-candidate");
    let mut invalid_integer = request(&state, candidate_id, previous.guidance_id);
    invalid_integer.successor.ranks = vec![i64::MIN];
    for (revision, expected) in [
        (not_latest, campaign_core::CampaignError::StaleGuidance),
        (same_generation, campaign_core::CampaignError::StaleGuidance),
        (
            missing_reason,
            campaign_core::CampaignError::MissingIdentity,
        ),
        (
            crossed_reason,
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (
            crossed_candidate,
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (
            invalid_integer,
            campaign_core::CampaignError::InvalidGuidanceShape,
        ),
    ] {
        assert_eq!(
            campaign_core::revise_guidance(&state, revision),
            Err(expected)
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let (pending, unguided) = crate::suite::admission::initial_state()?;
    let (_events, unguided, fresh) = crate::suite::candidate::admit(
        pending,
        &unguided,
        b"unguided",
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        campaign_core::revise_guidance(&unguided, request(&unguided, fresh, previous.guidance_id)),
        Err(campaign_core::CampaignError::MissingGuidance)
    );
    Ok(())
}

#[test]
fn replay_rechecks_the_invalidated_record_and_invalidation_identity() -> Result<(), String> {
    let (events, state, candidate_id) =
        crate::suite::ingress::support::guided(crate::suite::constants::RANK_HIGH)?;
    let previous = crate::suite::ingress::support::latest(&state, candidate_id)?;
    let successor = crate::suite::ingress::support::text(
        crate::suite::ingress::support::guidance(
            &state,
            candidate_id,
            SECOND_GENERATION,
            vec![crate::suite::constants::RANK_LOW],
        )
        .derive(),
    )?;
    let invalidation_id = crate::suite::ingress::support::text(
        campaign_core::derive_revision_invalidation_id(previous.guidance_id, reason()),
    )?;
    let forged_invalidation = campaign_core::GuidanceDynamics::Revision {
        invalidated_guidance_id: previous.guidance_id,
        invalidation_id: crate::suite::admission::xid(
            campaign_core::IdKind::Invalidation,
            b"forged-invalidation",
        ),
        reason_id: reason(),
    };
    let wrong_record = campaign_core::GuidanceDynamics::Revision {
        invalidated_guidance_id: successor.guidance_id,
        invalidation_id: crate::suite::ingress::support::text(
            campaign_core::derive_revision_invalidation_id(successor.guidance_id, reason()),
        )?,
        reason_id: reason(),
    };
    for dynamics in [forged_invalidation, wrong_record] {
        assert_eq!(
            crate::suite::ingress::support::replay_with(
                &events,
                crate::suite::ingress::support::forged_guidance(
                    &state,
                    successor.clone(),
                    dynamics
                )?
            ),
            Err(campaign_core::CampaignError::StaleGuidance)
        );
        crate::suite::ingress::support::assert_unchanged(&events, &state)?;
    }
    let valid = campaign_core::GuidanceDynamics::Revision {
        invalidated_guidance_id: previous.guidance_id,
        invalidation_id,
        reason_id: reason(),
    };
    let rebuilt = crate::suite::ingress::support::replay_with(
        &events,
        crate::suite::ingress::support::forged_guidance(&state, successor.clone(), valid)?,
    );
    assert_eq!(
        rebuilt
            .and_then(|next| next.candidate(candidate_id).cloned())
            .map(|entry| entry.latest_guidance),
        Ok(Some(successor))
    );
    Ok(())
}
