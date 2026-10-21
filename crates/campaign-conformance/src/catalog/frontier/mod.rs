//! Wait and stop fixtures.

/// Wait and stop fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "waits.guidance",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: awaiting_guidance,
    },
    super::Fixture {
        name: "waits.results",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: awaiting_results,
    },
    super::Fixture {
        name: "waits.capacity",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: awaiting_capacity,
    },
    super::Fixture {
        name: "stops.frontier_exhausted",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: exhausted_without_candidates,
    },
    super::Fixture {
        name: "stops.budget_exhausted",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: budget_exhausted,
    },
    super::Fixture {
        name: "stops.consumer",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: consumer,
    },
    super::Fixture {
        name: "stops.selection_limit",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: selection_limit,
    },
    super::Fixture {
        name: "stops.terminal",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: terminal,
    },
];

fn decision(planned: &crate::harness::decisions::Planned) -> &campaign_core::NextDecision {
    &planned.plan.decision
}

pub(super) fn wait_reason(
    planned: &crate::harness::decisions::Planned,
) -> Option<campaign_core::WaitReason> {
    let campaign_core::NextDecision::Wait(reason) = decision(planned) else {
        return None;
    };
    Some(reason.clone())
}

pub(super) fn stop_reason(
    planned: &crate::harness::decisions::Planned,
) -> Option<campaign_core::StopReason> {
    let campaign_core::NextDecision::Stop(stop) = decision(planned) else {
        return None;
    };
    Some(stop.reason.clone())
}

fn awaiting_guidance(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    let planned = session.plan(None)?;
    super::common::require(
        wait_reason(&planned) == Some(campaign_core::WaitReason::AwaitingGuidance),
        "an unguided candidate waits for guidance and never stops",
    )
}

fn awaiting_results(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(harness, super::common::UNIT_COST, |_| {})?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    session.execute(None)?;
    let planned = session.plan(None)?;
    super::common::require(
        wait_reason(&planned) == Some(campaign_core::WaitReason::AwaitingResults),
        "outstanding work that can change the frontier is awaited, not a stop",
    )
}

fn awaiting_capacity(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::common::AMPLE_BUDGET, |bounds| {
            bounds.max_outstanding = 1;
        })?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    session.execute(None)?;
    let reserved = session.state.reserved_resources.clone();
    let planned = session.plan(None)?;
    super::common::require(
        wait_reason(&planned) == Some(campaign_core::WaitReason::AwaitingCapacity),
        "a full outstanding bound waits for capacity",
    )?;
    super::common::require(
        session.state.reserved_resources == reserved,
        "a capacity wait reserves no budget",
    )
}

fn exhausted_without_candidates(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    let planned = session.plan(None)?;
    super::common::require(
        stop_reason(&planned) == Some(campaign_core::StopReason::FrontierExhausted),
        "an empty frontier stops as exhausted",
    )?;
    session.stop(campaign_core::StopReason::FrontierExhausted)?;
    let replanned = session.plan(None)?;
    super::common::require(
        stop_reason(&replanned) == Some(campaign_core::StopReason::FrontierExhausted),
        "the accepted stop is returned by later plans",
    )
}

fn budget_exhausted(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = crate::harness::Session::open(harness, super::common::UNIT_COST, |_| {})?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    session.settle(&selection, &super::common::outcome(&[]))?;
    let planned = session.plan(None)?;
    let reason = stop_reason(&planned).ok_or(crate::verdicts::ConformanceError::Diverged {
        expected: "an exhausted budget stops the campaign",
    })?;
    super::common::require(
        matches!(reason, campaign_core::StopReason::BudgetExhausted { .. }),
        "the stop names the exhausted budget",
    )?;
    session.stop(reason)?;
    super::common::require(
        session.state.stopped.is_some(),
        "the budget stop is accepted",
    )
}

fn consumer(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let reason = session.consumer_stop()?;
    session.stop(reason.clone())?;
    let planned = session.plan(None)?;
    super::common::require(
        stop_reason(&planned) == Some(reason),
        "a consumer stop is terminal for planning",
    )
}

fn selection_limit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::common::AMPLE_BUDGET, |bounds| {
            bounds.max_selections = 1;
            bounds.max_outstanding = 1;
        })?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let pending = session.plan(None)?;
    super::common::require(
        wait_reason(&pending) == Some(campaign_core::WaitReason::AwaitingResults),
        "the selection bound waits while work is outstanding",
    )?;
    session.settle(&selection, &super::common::outcome(&[]))?;
    super::common::require(
        session.state.selection_count == 1 && session.state.outstanding.is_empty(),
        "the only selection settled",
    )?;
    debug_assert_eq!(session.state.selection_count, 1);
    debug_assert!(session.state.outstanding.is_empty());
    let planned = session.plan(None)?;
    super::common::require(
        stop_reason(&planned) == Some(campaign_core::StopReason::SelectionLimitReached),
        "the selection bound stops once work settles",
    )
}

fn terminal(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let premature =
        campaign_core::admit_stop(&session.state, campaign_core::StopReason::FrontierExhausted);
    session.harness.deny(
        premature.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::InvalidStop,
        "an exhaustion stop was admitted with eligible candidates",
    )?;
    let reason = session.consumer_stop()?;
    session.stop(reason)?;
    debug_assert!(session.state.stopped.is_some());
    debug_assert!(session.state.outstanding.is_empty());
    let admitted = session.batch(&[super::common::operation(
        crate::vocabulary::OperationHandle(super::common::OP_C.0.saturating_add(1)),
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )]);
    session.harness.deny(
        admitted,
        campaign_core::CampaignError::CampaignStopped,
        "new candidates entered a stopped campaign",
    )?;
    let planned = session.plan(None)?;
    super::common::forbid(
        planned.selection().is_ok(),
        "a stopped campaign selected new work",
    )
}
