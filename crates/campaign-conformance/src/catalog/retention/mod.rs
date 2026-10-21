//! Retirement and declared-pruning fixtures.

/// Retirement and pruning fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "retirement.retired",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: retired,
    },
    super::Fixture {
        name: "retirement.invalid",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: invalid_retirement,
    },
    super::Fixture {
        name: "pruning.declared_order",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: declared_order,
    },
    super::Fixture {
        name: "pruning.authority_claims",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: authority_claims,
    },
];

/// Retention bound of the pruning fixtures.
const RETAINED: u64 = 1;

fn retired(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    session.retire(super::common::OP_A)?;
    let selection = session.select(None)?;
    super::common::require(
        super::common::selected(&session, &selection)? == super::common::OP_B,
        "a retired candidate is never selected",
    )
}

fn invalid_retirement(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let stale = session.retirement(super::common::OP_B)?;
    session.retire(super::common::OP_A)?;
    let again = session.retirement(super::common::OP_A)?;
    debug_assert_ne!(stale.state_id, session.state.state_id);
    debug_assert_eq!(again.state_id, session.state.state_id);
    let retired_twice = campaign_core::retire_candidate(&session.state, again.clone());
    session.harness.deny(
        retired_twice.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::InvalidRetirement,
        "a candidate that is not retained was retired",
    )?;
    let superseded = campaign_core::retire_candidate(&session.state, stale);
    session.harness.deny(
        superseded.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::StaleState,
        "a retirement for a superseded state was admitted",
    )?;
    let mut crossed = again;
    crossed.policy_id = crossed.adapter_id;
    let crossed_retirement = campaign_core::retire_candidate(&session.state, crossed);
    session.harness.deny(
        crossed_retirement.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::CrossedIdentity,
        "a retirement under another policy was admitted",
    )
}

fn declared_order(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::common::AMPLE_BUDGET, |bounds| {
            bounds.max_retained_candidates = RETAINED;
        })?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let expected = retained_excess(&session.harness.profile.retention_rank_order)?;
    let pruned = session.prune()?;
    super::common::require(
        pruned == expected,
        "pruning removes the lowest candidates of the declared rank order",
    )
}

/// Standard operations outside the first `RETAINED` of the declared rank order.
fn retained_excess(
    rank_order: &[u32],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<crate::vocabulary::OperationHandle>> {
    let retained =
        usize::try_from(RETAINED).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut excess: alloc::vec::Vec<crate::vocabulary::OperationHandle> =
        super::common::ordered_by(rank_order)
            .into_iter()
            .skip(retained)
            .collect();
    excess.sort();
    Ok(excess)
}

fn authority_claims(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session =
        crate::harness::Session::open(harness, super::common::AMPLE_BUDGET, |bounds| {
            bounds.max_retained_candidates = RETAINED;
        })?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let planned = session
        .retention_plan()?
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the frontier exceeds its retention bound",
        })?;
    let before = session.state.state_id;
    debug_assert!(!planned.candidates.is_empty());
    debug_assert!(planned.retention_profile_id.is_some());
    let overclaims: [fn(&mut campaign_core::PruneRequest); 3] = [
        |request| request.claims_history_rewrite = true,
        |request| request.claims_artifact_deletion = true,
        |request| request.claims_safe_deletion = true,
    ];
    for overclaim in overclaims {
        let mut request = planned.clone();
        overclaim(&mut request);
        request.prune_plan_id = campaign_core::derive_prune_plan_id(&request)?;
        let pruned = campaign_core::prune_frontier(&session.state, request);
        session.harness.deny(
            pruned.map_err(crate::verdicts::ConformanceError::from),
            campaign_core::CampaignError::InvalidPruning,
            "pruning claimed history, artifact, or deletion authority",
        )?;
    }
    super::common::require(
        session.state.state_id == before,
        "denied pruning left the state unchanged",
    )
}
