//! Child-count fixture: one result reports exactly the child moments its execution discovered, at
//! one child, an intermediate count, and the most one result may carry (`max_child_moments`,
//! capped by the room the session graph leaves). A mapping that reports a fixed number of
//! children per result drops a reserved child moment at one of these counts.

/// Intermediate child count: more than any other result fixture reports.
const INTERMEDIATE_CHILDREN: u64 = 3;

/// Handle of the first child moment a counted result discovers; later children follow in order.
const FIRST_COUNTED_CHILD: u32 = 100;

/// Graph events the session still publishes after the children: the result, and the stop the
/// adapter keeps in reserve.
const EVENTS_AFTER_CHILDREN: u32 = 2;

/// Child counts the fixture presents, in order: one child, the intermediate count, and the most
/// the session admits (`None`), each at most the most.
const PRESENTED: [Option<u64>; 3] = [Some(1), Some(INTERMEDIATE_CHILDREN), None];

/// The most children one result of the session can report: the profile's `max_child_moments`,
/// capped by the graph room left beside the result and the stop reserve.
///
/// # Errors
///
/// Returns `Diverged` when the graph leaves no room for one child.
fn most_children(
    session: &crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<u64> {
    let room = session
        .graph
        .limits()
        .max_events
        .checked_sub(session.graph.len())
        .and_then(|room| room.checked_sub(EVENTS_AFTER_CHILDREN))
        .filter(|room| *room > 0)
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the session graph has room for a child moment",
        })?;
    let most = session
        .state
        .admission
        .bounds
        .max_child_moments
        .min(u64::from(room));
    debug_assert!(most > 0);
    Ok(most)
}

/// Executes the best candidate, discovers `requested` children (the most when `None`), and
/// settles the execution with all of them. Returns the count presented.
///
/// # Errors
///
/// Returns the error of the scenario and `Diverged` when the result or the state does not carry
/// exactly the reserved children.
fn settle_with_children(
    harness: &mut crate::harness::Driver<'_>,
    requested: Option<u64>,
) -> crate::verdicts::ConformanceResult<u64> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let most = most_children(&session)?;
    let count = requested.map_or(most, |requested| requested.min(most));
    let mut children = alloc::vec::Vec::with_capacity(
        usize::try_from(count).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?,
    );
    for offset in 0..count {
        let handle = u32::try_from(offset)
            .ok()
            .and_then(|offset| FIRST_COUNTED_CHILD.checked_add(offset))
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
        let handle = crate::vocabulary::MomentHandle(handle);
        session.add_moment(super::super::common::child(handle))?;
        children.push(handle);
    }
    let accepted = session.settle(&selection, &super::super::common::outcome(&children))?;
    let expected =
        usize::try_from(count).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    super::super::common::require(
        accepted.children.len() == expected
            && session.state.discoveries.len() == expected
            && session.state.reservations.len() == expected,
        "the result reports and the state records every discovered child",
    )?;
    debug_assert_eq!(children.len(), expected);
    super::children::require_bound(&session, &selection, &accepted, &children)?;
    Ok(count)
}

/// Results with one child, an intermediate count, and the most children one result may carry
/// report exactly the reserved children, each once.
// r[impl campaign.conformance.bounds]
pub(super) fn presented_children(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut presented = alloc::vec::Vec::with_capacity(PRESENTED.len());
    for requested in PRESENTED {
        let count = settle_with_children(harness, requested)?;
        presented.push(count);
    }
    super::super::common::require(
        presented.windows(2).all(|pair| pair.first() <= pair.last()),
        "child counts rise to the most one result carries",
    )?;
    debug_assert_eq!(presented.len(), PRESENTED.len());
    Ok(())
}
