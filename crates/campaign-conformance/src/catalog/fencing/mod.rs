//! Publication-fence and execution-gate fixtures.

mod matrix;
mod publication;

/// Publication-fence fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "fencing.published_execution",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: published_execution,
    },
    super::Fixture {
        name: "fencing.unpublished_execution",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: unpublished_execution,
    },
    super::Fixture {
        name: "fencing.non_durable_execution",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: non_durable_execution,
    },
    super::Fixture {
        name: "fencing.stale_branch",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: stale_branch,
    },
    super::Fixture {
        name: "fencing.concurrent_planner",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: concurrent_planner,
    },
    super::Fixture {
        name: "fencing.foreign_selection",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: publication::foreign_selection,
    },
    super::Fixture {
        name: "fencing.stale_state_selection",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: publication::stale_state_selection,
    },
    super::Fixture {
        name: "fencing.gate_matrix_ranked",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: matrix::ranked,
    },
    super::Fixture {
        name: "fencing.gate_matrix_exploratory",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: matrix::exploratory,
    },
];

fn is_fence_mismatch(error: &crate::verdicts::ConformanceError) -> bool {
    matches!(
        error,
        crate::verdicts::ConformanceError::Adapter(
            campaign_choregraph::AdapterError::PublicationFenceMismatch
        )
    )
}

fn published_execution(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let is_outstanding = session
        .state
        .outstanding
        .iter()
        .any(|outstanding| outstanding.selection_id == selection.selection_id);
    super::common::require(
        is_outstanding,
        "the published selection is outstanding work",
    )
}

fn unpublished_execution(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.select(None)?;
    let never_planned = session.gate(
        &selection,
        None,
        None,
        "a planned but unpublished selection was executed",
    )?;
    super::common::forbid(
        never_planned == crate::port::ExecutionAuthority::Granted,
        "a planned but unpublished selection was executed",
    )?;
    super::common::require(
        session.state.outstanding.is_empty() && selection.state_before_id == session.state.state_id,
        "the planned selection is unpublished and binds the current state",
    )?;
    debug_assert!(session.state.outstanding.is_empty());
    debug_assert_eq!(selection.state_before_id, session.state.state_id);
    let planned = session.plan_publication(session.selection_event(&selection)?)?;
    let uncommitted = crate::harness::decisions::durable_evidence(
        &planned,
        crate::harness::decisions::Durability::PENDING,
    );
    let before_commit = session.gate(
        &selection,
        Some(&planned),
        Some(&uncommitted),
        "a selection executed before its publication committed",
    )?;
    super::common::forbid(
        before_commit == crate::port::ExecutionAuthority::Granted,
        "a selection executed before its publication committed",
    )
}

fn non_durable_execution(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.select(None)?;
    let publication = session.publish_selection(&selection)?;
    for (is_event_durable, is_branch_update_durable) in [(true, false), (false, true)] {
        let durability = crate::harness::decisions::Durability {
            is_event_durable,
            is_branch_update_durable,
        };
        let evidence = crate::harness::decisions::durable_evidence(&publication, durability);
        debug_assert!(!(evidence.durable_event && evidence.durable_branch_update));
        debug_assert_eq!(evidence.history_event_id, publication.history_event_id());
        let authority = session.gate(
            &selection,
            Some(&publication),
            Some(&evidence),
            "a selection executed without durable publication",
        )?;
        super::common::forbid(
            authority == crate::port::ExecutionAuthority::Granted,
            "a selection executed without durable publication",
        )?;
    }
    Ok(())
}

fn stale_branch(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.select(None)?;
    let stale = session.plan_publication(session.selection_event(&selection)?)?;
    session.guide(&super::common::preference(
        super::common::OP_C,
        super::common::SECOND_GENERATION,
        [40, 4],
    ))?;
    let published = session.publish_selection(&selection);
    session.harness.deny_with(
        published,
        "a selection published on a stale branch",
        is_fence_mismatch,
    )?;
    let mut forged = crate::harness::decisions::durable_evidence(
        &stale,
        crate::harness::decisions::Durability::COMMITTED,
    );
    forged.branch_before = session.branch.clone();
    debug_assert_ne!(stale.branch_plan().before, session.branch);
    debug_assert_eq!(forged.branch_before, session.branch);
    let authority = session.gate(
        &selection,
        Some(&stale),
        Some(&forged),
        "stale branch evidence authorized execution",
    )?;
    super::common::forbid(
        authority == crate::port::ExecutionAuthority::Granted,
        "stale branch evidence authorized execution",
    )
}

fn concurrent_planner(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let first = session.plan(None)?.selection()?;
    let second = session.plan(None)?.selection()?;
    super::common::require(
        first == second && first.state_before_id == session.state.state_id,
        "two planners on one projection plan the same selection of the current state",
    )?;
    debug_assert_eq!(first, second);
    debug_assert_eq!(first.state_before_id, session.state.state_id);
    session.publish_selection(&first)?;
    let raced = session.publish_selection(&second);
    session.harness.deny_with(
        raced,
        "a concurrent plan published over the winner",
        is_fence_mismatch,
    )?;
    let replanned = session.select(None)?;
    let next_ordinal = first.ordinal.checked_add(1);
    super::common::require(
        Some(replanned.ordinal) == next_ordinal,
        "replanning from the fresh projection takes the next ordinal",
    )?;
    super::common::require(
        replanned.operation_id != first.operation_id,
        "replanning yields a distinct operation",
    )
}
