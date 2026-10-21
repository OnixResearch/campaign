//! Guidance whose first levels tie, so a later level decides the ranked choice: two-level scores
//! of the standard shape, and scores of one level, an intermediate count, and the most rank
//! dimensions the profile admits, each count in its own campaign (a campaign has one rank shape).

/// First level both candidates share.
const TIED_FIRST: u32 = 20;

/// Second levels: A below B, then A's strict improvement above B.
const SECOND_LOW: u32 = 1;
const SECOND_HIGH: u32 = 3;
const SECOND_IMPROVED: u32 = 5;

/// Two candidates whose first levels tie: the ranked planner must choose by the second level,
/// first B (higher second level), then A after A's second level strictly improves above B's. A
/// mapping that loses the second level ties both candidates both times, so an identity tie-break
/// chooses the same candidate twice and misses one of the two expected choices.
pub(super) fn second_level(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    session.admit(&[
        super::super::common::operation(
            super::super::common::OP_A,
            crate::harness::history::ROOT,
            super::super::common::UNIT_COST,
        ),
        super::super::common::operation(
            super::super::common::OP_B,
            crate::harness::history::ROOT,
            super::super::common::UNIT_COST,
        ),
    ])?;
    for (handle, second) in [
        (super::super::common::OP_A, SECOND_LOW),
        (super::super::common::OP_B, SECOND_HIGH),
    ] {
        session.guide(&super::super::common::preference(
            handle,
            super::super::common::FIRST_GENERATION,
            [TIED_FIRST, second],
        ))?;
    }
    let before = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &before)? == super::super::common::OP_B,
        "the second level decides between candidates whose first levels tie",
    )?;
    session.guide(&super::super::common::preference(
        super::super::common::OP_A,
        super::super::common::SECOND_GENERATION,
        [TIED_FIRST, SECOND_IMPROVED],
    ))?;
    let after = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &after)? == super::super::common::OP_A,
        "an improved second level decides between candidates whose first levels tie",
    )?;
    debug_assert_ne!(before.candidate_id, after.candidate_id);
    Ok(())
}

/// Intermediate level count: more than the two levels of every standard score.
const INTERMEDIATE_LEVELS: usize = 3;

/// Most levels the suite presents in one score; a profile with fewer `max_rank_dimensions` caps
/// it.
const LEVELS_ROOM: usize = 64;

/// Level count families the fixture presents, in order: one level, the intermediate count, and
/// the most the profile admits (`None`), each at most the most.
const PRESENTED: [Option<usize>; 3] = [Some(1), Some(INTERMEDIATE_LEVELS), None];

/// Level every leading level of a counted score carries: the candidates tie on all but the last.
const TIED_LEVEL: u32 = 5;

/// Last levels: A below B, then A's strict improvement above B.
const LAST_LOW: u32 = 1;
const LAST_HIGH: u32 = 9;
const LAST_IMPROVED: u32 = 12;

/// A preference of `count` levels that ties every level but the last, which is `last`.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` for a count of zero.
fn tied_preference(
    operation: crate::vocabulary::OperationHandle,
    generation: u64,
    count: usize,
    last: u32,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::ScenarioPreference> {
    let leading = count
        .checked_sub(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut levels = alloc::vec![TIED_LEVEL; leading];
    levels.push(last);
    debug_assert_eq!(levels.len(), leading.saturating_add(1));
    Ok(crate::vocabulary::ScenarioPreference {
        operation,
        generation,
        score: crate::vocabulary::AbstractScore { levels },
    })
}

/// In its own campaign, two candidates whose `count`-level scores tie on every level but the
/// last: the ranked planner chooses B (higher last level), then A after A's last level strictly
/// improves above B's.
///
/// # Errors
///
/// Returns the error of the scenario and `Diverged` when the last level does not decide.
fn last_level_decides(
    harness: &mut crate::harness::Driver<'_>,
    count: usize,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    session.admit(&[
        super::super::common::operation(
            super::super::common::OP_A,
            crate::harness::history::ROOT,
            super::super::common::UNIT_COST,
        ),
        super::super::common::operation(
            super::super::common::OP_B,
            crate::harness::history::ROOT,
            super::super::common::UNIT_COST,
        ),
    ])?;
    for (handle, last) in [
        (super::super::common::OP_A, LAST_LOW),
        (super::super::common::OP_B, LAST_HIGH),
    ] {
        session.guide(&tied_preference(
            handle,
            super::super::common::FIRST_GENERATION,
            count,
            last,
        )?)?;
    }
    let before = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &before)? == super::super::common::OP_B,
        "the last level decides between candidates whose leading levels tie",
    )?;
    session.guide(&tied_preference(
        super::super::common::OP_A,
        super::super::common::SECOND_GENERATION,
        count,
        LAST_IMPROVED,
    )?)?;
    let after = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &after)? == super::super::common::OP_A,
        "an improved last level decides between candidates whose leading levels tie",
    )?;
    debug_assert_ne!(before.candidate_id, after.candidate_id);
    Ok(())
}

/// Scores of one level, of more levels than the standard shape, and of the most rank dimensions
/// the profile admits: every level of a score reaches the planner, so the last level decides.
// r[impl campaign.conformance.bounds]
pub(super) fn level_counts(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let most = usize::try_from(harness.profile.bounds.max_rank_dimensions)
        .map_or(LEVELS_ROOM, |bound| bound.min(LEVELS_ROOM));
    for requested in PRESENTED {
        last_level_decides(harness, requested.map_or(most, |count| count.min(most)))?;
    }
    Ok(())
}
