//! Discovery fixtures: shared, conflicting, and unreserved children of results.

/// Requires the accepted result to report exactly the children the suite reserved for the
/// selection, each with its own payload and reservation, and the state to record each
/// reservation and each child's payload in its stable discovery.
///
/// # Errors
///
/// Returns `Diverged` when a child is missing, extra, cross-wired, or unrecorded.
pub(super) fn require_bound(
    session: &crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    accepted: &campaign_core::ResultObservation,
    handles: &[crate::vocabulary::MomentHandle],
) -> crate::verdicts::ConformanceResult<()> {
    let reserved = session.reserve_children(selection, handles)?;
    let is_bound = accepted.children.len() == reserved.len()
        && reserved
            .iter()
            .all(|child| accepted.children.contains(child));
    super::super::common::require(
        is_bound,
        "each accepted child binds its own reserved payload and reservation",
    )?;
    debug_assert_eq!(accepted.children.len(), reserved.len());
    let is_recorded = reserved.iter().all(|child| {
        session.state.reservations.contains(&child.reservation_id)
            && session
                .state
                .discovery(child.child_moment_id)
                .is_some_and(|discovery| discovery.event_payload_id == child.event_payload_id)
    });
    super::super::common::require(
        is_recorded,
        "the state records each accepted reservation and each child's payload",
    )?;
    debug_assert!(
        reserved
            .iter()
            .all(|child| session.state.reservations.contains(&child.reservation_id))
    );
    Ok(())
}

pub(super) fn shared_discovery(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    let first = session.execute(None)?;
    let first_result = session.settle(
        &first,
        &super::super::common::outcome(&[super::super::common::CHILD_ONE]),
    )?;
    let second = session.execute(None)?;
    let second_result = session.settle(
        &second,
        &super::super::common::outcome(&[super::super::common::CHILD_ONE]),
    )?;
    let child = session
        .moment(super::super::common::CHILD_ONE)?
        .identity
        .moment_id();
    super::super::common::require(
        first.candidate_id == second.candidate_id && session.state.accepted_results.len() == 2,
        "both executions of one candidate settled",
    )?;
    debug_assert_eq!(first.candidate_id, second.candidate_id);
    debug_assert_eq!(session.state.accepted_results.len(), 2);
    super::super::common::require(
        session.state.discoveries.len() == 1,
        "two paths share one discovery",
    )?;
    super::super::common::require(
        session
            .state
            .discovery(child)
            .is_some_and(|discovery| discovery.first_result_id == first_result.result_id),
        "both paths observe the first stable result",
    )?;
    require_bound(
        &session,
        &first,
        &first_result,
        &[super::super::common::CHILD_ONE],
    )?;
    require_bound(
        &session,
        &second,
        &second_result,
        &[super::super::common::CHILD_ONE],
    )?;
    super::super::common::require(
        session.state.reservations.len() == 2,
        "the second path binds its own reservation of the shared discovery",
    )
}

pub(super) fn conflicting_rediscovery(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    session.add_moment(super::super::common::child(super::super::common::CHILD_TWO))?;
    let first = session.execute(None)?;
    session.settle(
        &first,
        &super::super::common::outcome(&[super::super::common::CHILD_ONE]),
    )?;
    let second = session.execute(None)?;
    let crate::port::MomentIdentity::Child {
        event_payload_id, ..
    } = session.moment(super::super::common::CHILD_ONE)?.identity
    else {
        return Err(crate::verdicts::ConformanceError::InvalidIdentities);
    };
    let mut observation = session.observation(
        &second,
        &super::super::common::outcome(&[super::super::common::CHILD_TWO]),
    )?;
    for child in &mut observation.children {
        child.event_payload_id = event_payload_id;
        *child = child
            .clone()
            .derive(session.state.admission.campaign_id, second.selection_id)?;
    }
    super::super::common::require(
        observation.children.len() == 1 && session.state.discoveries.len() == 1,
        "the rediscovery reports its one reserved child after one discovery",
    )?;
    debug_assert_eq!(observation.children.len(), 1);
    debug_assert_eq!(session.state.discoveries.len(), 1);
    let rediscovered = campaign_core::admit_result(&session.state, observation);
    session.harness.deny(
        rediscovered.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::ConflictingResult,
        "a rediscovered payload became a second stable result",
    )
}

pub(super) fn unreserved_child(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    let mut observation = session.observation(
        &selection,
        &super::super::common::outcome(&[super::super::common::CHILD_ONE]),
    )?;
    for child in &mut observation.children {
        child.reservation_id = campaign_core::Id::absent();
    }
    let committed = campaign_core::admit_result(&session.state, observation);
    session.harness.deny(
        committed.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::InvalidChildReservation,
        "a parent result committed before its child was reserved",
    )
}
