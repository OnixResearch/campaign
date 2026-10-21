//! Guidance heads: the first guidance of the first retained unguided candidate, and a strict
//! improvement, an equal-rank update, or a revision of the first retained guided candidate's
//! latest record.

/// Levels of a head's first guidance.
const DISCOVERED_LEVELS: [u32; 2] = [15, 8];

/// Levels of a head's strict improvement: above every level a recipe or a fill publishes.
const IMPROVED_LEVELS: [u32; 2] = [1_000, 1_000];

/// Levels of a head's revision.
const REVISED_LEVELS: [u32; 2] = [2, 9];

/// Whether `state` admits one more counted guidance record.
const fn has_room(state: &campaign_core::FrontierState) -> bool {
    state.guidance_record_count < state.admission.bounds.max_guidance_records
}

/// The guidance head of `kind` admitted against `state`; `None` when no retained candidate fits
/// or a counted record finds no room.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the guidance.
pub(super) fn event(
    session: &mut crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
    kind: super::SubKind,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let is_discovery = kind == super::SubKind::Discovery;
    let chosen = super::first_in_suite_order(session, state, |candidate| {
        candidate.status == campaign_core::CandidateStatus::Retained
            && candidate.latest_guidance.is_some() != is_discovery
    })?;
    let Some((operation, candidate)) = chosen else {
        return Ok(None);
    };
    if kind != super::SubKind::CoalescedGuidance && !has_room(state) {
        return Ok(None);
    }
    let Some(latest) = candidate.latest_guidance.as_ref() else {
        debug_assert!(is_discovery);
        let record = session.guidance_record(&crate::catalog::common::preference(
            operation,
            crate::catalog::common::FIRST_GENERATION,
            DISCOVERED_LEVELS,
        ))?;
        return Ok(Some(campaign_core::admit_guidance(state, record)?));
    };
    debug_assert!(!is_discovery);
    if kind == super::SubKind::CoalescedGuidance {
        return Ok(Some(campaign_core::admit_guidance(state, latest.clone())?));
    }
    let generation = latest
        .generation
        .checked_add(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    if kind == super::SubKind::Improvement {
        let record = session.guidance_record(&crate::catalog::common::preference(
            operation,
            generation,
            IMPROVED_LEVELS,
        ))?;
        return Ok(Some(campaign_core::admit_guidance(state, record)?));
    }
    debug_assert_eq!(kind, super::SubKind::Revision);
    let context = crate::port::RevisionContext {
        campaign_id: state.admission.campaign_id,
        candidate_id: candidate.candidate.candidate_id,
        invalidated_guidance_id: latest.guidance_id,
    };
    let request = session.harness.adapter.revision(
        &crate::catalog::common::preference(operation, generation, REVISED_LEVELS),
        &context,
    )?;
    Ok(Some(campaign_core::revise_guidance(state, request)?))
}
