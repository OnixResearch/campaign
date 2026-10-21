//! Result fixtures.

mod children;
mod counts;
mod crossed;
mod reports;

/// Result fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "results.fork_children",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: fork_children,
    },
    super::Fixture {
        name: "results.crossed_selection",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: crossed::older_first,
    },
    super::Fixture {
        name: "results.out_of_order_settlement",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: crossed::newer_first,
    },
    super::Fixture {
        name: "results.retry_boundary",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: retry_boundary,
    },
    super::Fixture {
        name: "results.shared_discovery",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: children::shared_discovery,
    },
    super::Fixture {
        name: "results.conflicting_rediscovery",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: children::conflicting_rediscovery,
    },
    super::Fixture {
        name: "results.unreserved_child",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: children::unreserved_child,
    },
    super::Fixture {
        name: "results.child_counts",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: counts::presented_children,
    },
    super::Fixture {
        name: "results.findings",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: reports::findings,
    },
];

/// Operation discovered below the first child moment.
const OP_DEEP: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(4);

/// Levels of the deep operation: best in every dimension.
const LEVELS_DEEP: [u32; 2] = [50, 5];

/// Depth of a candidate below a child of the root.
const CHILD_CANDIDATE_DEPTH: u64 = 2;

/// Session with a guided frontier, a forked result, a candidate below a child, and a closure.
///
/// # Errors
///
/// Returns the error of any accepted step.
pub(super) fn varied_session<'s, 'a>(
    harness: &'s mut crate::harness::Driver<'a>,
) -> crate::verdicts::ConformanceResult<crate::harness::Session<'s, 'a>> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    forked(&mut session)?;
    let deep = session.execute(None)?;
    session.close(&deep)?;
    super::common::require(
        session.state.outstanding.is_empty() && session.state.settled.len() == 2,
        "the forked and the closed selections are both settled",
    )?;
    debug_assert!(session.state.outstanding.is_empty());
    debug_assert_eq!(session.state.settled.len(), 2);
    Ok(session)
}

/// Executes the best candidate, settles it with two children, and admits and guides one
/// operation below the first child.
fn forked(session: &mut crate::harness::Session<'_, '_>) -> crate::verdicts::ConformanceResult<()> {
    let selection = session.execute(None)?;
    session.add_moment(super::common::child(super::common::CHILD_ONE))?;
    session.add_moment(super::common::child(super::common::CHILD_TWO))?;
    let forked_children = [super::common::CHILD_ONE, super::common::CHILD_TWO];
    let accepted = session.settle(&selection, &super::common::outcome(&forked_children))?;
    session.admit(&[super::common::operation(
        OP_DEEP,
        super::common::CHILD_ONE,
        super::common::UNIT_COST,
    )])?;
    session.guide(&super::common::preference(
        OP_DEEP,
        super::common::FIRST_GENERATION,
        LEVELS_DEEP,
    ))?;
    super::common::require(
        session.state.discoveries.len() == 2 && session.state.reservations.len() == 2,
        "the forked result committed both reserved children",
    )?;
    debug_assert_eq!(session.state.discoveries.len(), 2);
    debug_assert_eq!(session.state.reservations.len(), 2);
    children::require_bound(session, &selection, &accepted, &forked_children)
}

pub(super) fn reserved_units(session: &crate::harness::Session<'_, '_>) -> u64 {
    let dimension = session.identities.resource_dimension_ids.first().copied();
    session
        .state
        .reserved_resources
        .iter()
        .filter(|reserved| Some(reserved.dimension_id) == dimension)
        .map(|reserved| reserved.amount)
        .sum()
}

fn fork_children(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    forked(&mut session)?;
    let selection = session.select(None)?;
    let child = session
        .moment(super::common::CHILD_ONE)?
        .identity
        .moment_id();
    super::common::require(
        selection.ordinal == 2 && session.state.has_known_moment(child),
        "the second selection follows the committed fork child",
    )?;
    debug_assert_eq!(selection.ordinal, 2);
    debug_assert!(session.state.has_known_moment(child));
    super::common::require(
        super::common::selected(&session, &selection)? == OP_DEEP,
        "a candidate below a discovered child is selectable",
    )?;
    super::common::require(
        selection.parent_moment_id == child,
        "the fork child is the parent",
    )?;
    let depth = session
        .state
        .candidate(selection.candidate_id)?
        .candidate
        .depth;
    super::common::require(
        depth == CHILD_CANDIDATE_DEPTH,
        "the child candidate is one deeper",
    )
}

fn retry_boundary(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let accepted = session.settle(&selection, &super::common::outcome(&[]))?;
    let before = session.state.state_id;
    let identical = campaign_core::admit_result(&session.state, accepted);
    session.harness.deny(
        identical.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::DuplicateResult,
        "an identical resubmission changed the frontier",
    )?;
    session.add_moment(super::common::child(super::common::CHILD_ONE))?;
    let retried = session.observation(
        &selection,
        &super::common::outcome(&[super::common::CHILD_ONE]),
    )?;
    super::common::require(
        session.state.outstanding.is_empty() && retried.selection_id == selection.selection_id,
        "the retried observation names the settled selection",
    )?;
    debug_assert!(session.state.outstanding.is_empty());
    debug_assert_eq!(retried.selection_id, selection.selection_id);
    let different = campaign_core::admit_result(&session.state, retried);
    session.harness.deny(
        different.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::InvalidRetryBoundary,
        "a retried execution introduced new exploration",
    )?;
    super::common::require(
        session.state.state_id == before,
        "denied retries left the state unchanged",
    )
}
