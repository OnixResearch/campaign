//! The events a writer publishes to deepen a fork before its head: after a stop, closures of the
//! outstanding selections; otherwise strict guidance improvements
//! of admitted standard operations while the guidance-record bound admits them, then
//! single-candidate batches while the candidate bound admits them, then equal-rank coalesced
//! duplicates of a guided candidate's latest guidance record, which use no record room (after the
//! first guidance of a fill candidate when none is guided). A fork is
//! therefore limited by the projection bound, the control-event room, and the graph room, not by
//! a record bound.

/// Operations a writer improves on a deeper fork, in rotation.
const IMPROVED_OPERATIONS: [crate::vocabulary::OperationHandle; 3] = [
    super::super::super::super::common::OP_C,
    super::super::super::super::common::OP_B,
    super::super::super::super::common::OP_A,
];

/// Levels of the first improvement, and the rise of both levels per later improvement: every
/// improvement of an operation strictly exceeds its earlier levels.
const FIRST_IMPROVED_LEVELS: [u32; 2] = [40, 4];
const IMPROVEMENT_RISE: [u32; 2] = [10, 1];

/// Handle of the operation a writer admits at the first fill step that admits a candidate; the
/// operation of a later step follows it by the step.
const FIRST_ADDED_OPERATION: u32 = 100;

/// The guidance-record room of `state`.
// r[impl campaign.conformance.bounds]
fn guidance_room(state: &campaign_core::FrontierState) -> Option<usize> {
    state
        .admission
        .bounds
        .max_guidance_records
        .checked_sub(state.guidance_record_count)
        .and_then(|room| usize::try_from(room).ok())
}

/// Fill events a running `base` admits (`usize::MAX` when unbounded): once some retained
/// candidate is guided, coalesced duplicates of its latest record continue the fill without end,
/// and a fill with record room reaches one (an improvement or first guidance of a retained
/// candidate, after a batch when none is retained); without record room and a guided candidate
/// only single-candidate batches, up to the candidate room, fill.
pub(super) fn room(base: &campaign_core::FrontierState) -> usize {
    let mut retained = base
        .candidates
        .iter()
        .filter(|candidate| candidate.status == campaign_core::CandidateStatus::Retained);
    let has_guided = retained
        .clone()
        .any(|candidate| candidate.latest_guidance.is_some());
    let has_retained = retained.next().is_some();
    let candidate_room = u64::try_from(base.candidates.len())
        .ok()
        .and_then(|count| base.admission.bounds.max_candidates.checked_sub(count))
        .and_then(|room| usize::try_from(room).ok())
        .unwrap_or(0);
    let has_record_room = guidance_room(base).is_some_and(|room| room > 0);
    if has_guided || (has_record_room && (has_retained || candidate_room > 0)) {
        return usize::MAX;
    }
    debug_assert!(!has_guided);
    candidate_room
}

/// The `step`-th strict guidance improvement a writer publishes on a deeper fork (zero-based):
/// the operations rotate C, B, A, and each round publishes the next guidance generation at higher
/// levels.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` for a step whose generation or levels are unrepresentable.
fn improvement(
    step: usize,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::ScenarioPreference> {
    let overflow = crate::verdicts::ConformanceError::Campaign(
        campaign_core::CampaignError::ArithmeticOverflow,
    );
    let rotation = IMPROVED_OPERATIONS.len();
    let operation = step
        .checked_rem(rotation)
        .and_then(|position| IMPROVED_OPERATIONS.get(position))
        .copied()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the improved operation rotates over the standard operations",
        })?;
    let round = step
        .checked_div(rotation)
        .and_then(|round| u64::try_from(round).ok())
        .ok_or(overflow.clone())?;
    let generation = super::super::super::super::common::SECOND_GENERATION
        .checked_add(round)
        .ok_or(overflow.clone())?;
    let rises = u32::try_from(step).map_err(|_| overflow.clone())?;
    let mut levels = FIRST_IMPROVED_LEVELS;
    for (level, rise) in levels.iter_mut().zip(IMPROVEMENT_RISE) {
        *level = rise
            .checked_mul(rises)
            .and_then(|raised| level.checked_add(raised))
            .ok_or(overflow.clone())?;
    }
    debug_assert!(generation >= super::super::super::super::common::SECOND_GENERATION);
    Ok(super::super::super::super::common::preference(
        operation, generation, levels,
    ))
}

/// The `step`-th strict improvement admitted against `state`, when the step lies within the
/// guidance room of `base` and `state` retains the improved operation's candidate; its generation
/// follows the candidate's latest record when that is already at or past the step's generation.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the improvement.
pub(super) fn improved(
    session: &mut crate::harness::Session<'_, '_>,
    base: &campaign_core::FrontierState,
    state: &campaign_core::FrontierState,
    step: usize,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    if guidance_room(base).is_none_or(|room| step >= room) {
        return Ok(None);
    }
    let mut preference = improvement(step)?;
    let retained = session
        .candidate_id(preference.operation)
        .ok()
        .and_then(|candidate_id| {
            state.candidates.iter().find(|candidate| {
                candidate.candidate.candidate_id == candidate_id
                    && candidate.status == campaign_core::CandidateStatus::Retained
            })
        });
    let Some(retained) = retained else {
        return Ok(None);
    };
    if let Some(latest) = retained.latest_guidance.as_ref()
        && latest.generation >= preference.generation
    {
        preference.generation = latest
            .generation
            .checked_add(1)
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    }
    debug_assert!(preference.generation >= super::super::super::super::common::SECOND_GENERATION);
    debug_assert!(state.stopped.is_none());
    let record = session.guidance_record(&preference)?;
    Ok(Some(campaign_core::admit_guidance(state, record)?))
}

/// A single-candidate batch of the operation `handle` admitted against `state`, when `state`
/// admits one more candidate.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the batch.
pub(in crate::catalog::evidence) fn batch(
    session: &mut crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
    handle: u32,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let has_room = u64::try_from(state.candidates.len())
        .is_ok_and(|count| count < state.admission.bounds.max_candidates);
    if !has_room {
        return Ok(None);
    }
    let records = session.candidates(&[super::super::super::super::common::operation(
        crate::vocabulary::OperationHandle(handle),
        crate::harness::history::ROOT,
        super::super::super::super::common::UNIT_COST,
    )])?;
    Ok(Some(campaign_core::admit_candidates(
        state,
        records,
        session.rule,
        crate::harness::records::coalescing_policy_id()?,
    )?))
}

/// The latest guidance record of the first retained guided candidate of `state`, admitted again
/// as an equal-rank coalesced duplicate (it uses no record room).
///
/// # Errors
///
/// Returns the Campaign error of the admission.
pub(super) fn coalesced(
    state: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    let latest = state
        .candidates
        .iter()
        .filter(|candidate| candidate.status == campaign_core::CandidateStatus::Retained)
        .find_map(|candidate| candidate.latest_guidance.clone());
    let Some(record) = latest else {
        return Ok(None);
    };
    let event = campaign_core::admit_guidance(state, record)?;
    debug_assert!(matches!(
        &event,
        campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance)
            if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::CoalescedDuplicate { .. })
    ));
    Ok(Some(event))
}

/// The `step`-th fill event (zero-based) of a fork over `base`, admitted against `state` (the
/// fork's state after the earlier fill events, at the head of `history`): after a stop, the
/// closure of the latest outstanding selection; otherwise an improvement within the guidance room
/// of `base`, else a single-candidate batch within the candidate room of `state`, else an
/// equal-rank coalesced duplicate, else the first guidance of an unguided candidate.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the event, and `Diverged` when `state` admits none
/// of them.
// r[impl campaign.conformance.bounds]
pub(super) fn event(
    session: &mut crate::harness::Session<'_, '_>,
    base: &campaign_core::FrontierState,
    (history, state): (
        &crate::harness::oracle::HistoryView,
        &campaign_core::FrontierState,
    ),
    step: usize,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignControlEvent> {
    let exhausted = crate::verdicts::ConformanceError::Diverged {
        expected: "a fork's fill admits an improvement, a candidate, a coalesced duplicate, or a \
                   settlement",
    };
    debug_assert!(u64::try_from(history.payloads.len()).ok() == Some(state.control_event_count));
    debug_assert!(state.admission == base.admission);
    if state.stopped.is_some() {
        return super::heads::settlement(
            session,
            (history, state),
            super::heads::HeadKind::Closure,
        )?
        .ok_or(exhausted);
    }
    if let Some(improved) = improved(session, base, state, step)? {
        return Ok(improved);
    }
    let handle = u32::try_from(step)
        .ok()
        .and_then(|step| FIRST_ADDED_OPERATION.checked_add(step))
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    if let Some(added) = batch(session, state, handle)? {
        return Ok(added);
    }
    if let Some(duplicate) = coalesced(state)? {
        return Ok(duplicate);
    }
    discovered(session, state)?.ok_or(exhausted)
}

/// Levels of the first guidance a writer publishes for a fill candidate, once the candidate
/// bound is used and no candidate is guided yet.
const DISCOVERED_LEVELS: [u32; 2] = [1, 1];

/// The first guidance of the first retained unguided candidate of `state`, when the
/// guidance-record bound admits one more record (a coalesced duplicate of it then continues the
/// fill).
///
/// # Errors
///
/// Returns the adapter or Campaign error of the guidance.
fn discovered(
    session: &mut crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
    if guidance_room(state).is_none_or(|room| room == 0) {
        return Ok(None);
    }
    let unguided = state.candidates.iter().find(|candidate| {
        candidate.status == campaign_core::CandidateStatus::Retained
            && candidate.latest_guidance.is_none()
    });
    let Some(candidate) = unguided else {
        return Ok(None);
    };
    let operation = session.operation_of(candidate.candidate.candidate_id)?;
    let record = session.guidance_record(&super::super::super::super::common::preference(
        operation,
        super::super::super::super::common::FIRST_GENERATION,
        DISCOVERED_LEVELS,
    ))?;
    Ok(Some(campaign_core::admit_guidance(state, record)?))
}
