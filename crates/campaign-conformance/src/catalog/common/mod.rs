//! Shared abstract scenarios.
//!
//! The standard frontier has three operations below the root whose levels are unique in every
//! rank dimension, so ranked order, any declared exploration order, and any declared retention
//! order are total without an identity tie-break.

/// Budget large enough that no standard scenario exhausts it.
pub(super) const AMPLE_BUDGET: u64 = 64;

/// Abstract cost of a standard operation.
pub(super) const UNIT_COST: u64 = 1;

/// First guidance generation of every operation.
pub(super) const FIRST_GENERATION: u64 = 1;

/// Second guidance generation.
pub(super) const SECOND_GENERATION: u64 = 2;

/// Standard operation handles, best ranked first.
pub(super) const OP_A: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(1);
pub(super) const OP_B: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(2);
pub(super) const OP_C: crate::vocabulary::OperationHandle = crate::vocabulary::OperationHandle(3);

/// Standard operations in suite operation order: every suite choice among standard candidates
/// follows this order, never the adapter's candidate identities.
pub(super) const STANDARD_OPERATIONS: [crate::vocabulary::OperationHandle; 3] = [OP_A, OP_B, OP_C];

/// Standard levels: ranked order A, B, C; dimension one orders B, C, A.
pub(super) const LEVELS_A: [u32; 2] = [30, 1];
pub(super) const LEVELS_B: [u32; 2] = [20, 3];
pub(super) const LEVELS_C: [u32; 2] = [10, 2];

/// Standard child moments.
pub(super) const CHILD_ONE: crate::vocabulary::MomentHandle = crate::vocabulary::MomentHandle(11);
pub(super) const CHILD_TWO: crate::vocabulary::MomentHandle = crate::vocabulary::MomentHandle(12);

/// One abstract operation below `parent` with a single-dimension cost.
pub(super) fn operation(
    handle: crate::vocabulary::OperationHandle,
    parent: crate::vocabulary::MomentHandle,
    units: u64,
) -> crate::vocabulary::ScenarioOperation {
    crate::vocabulary::ScenarioOperation {
        handle,
        parent,
        costs: alloc::vec![crate::vocabulary::AbstractCost {
            dimension: 0,
            units,
        }],
    }
}

/// One abstract preference.
pub(super) fn preference(
    operation: crate::vocabulary::OperationHandle,
    generation: u64,
    levels: [u32; 2],
) -> crate::vocabulary::ScenarioPreference {
    crate::vocabulary::ScenarioPreference {
        operation,
        generation,
        score: crate::vocabulary::AbstractScore {
            levels: levels.to_vec(),
        },
    }
}

/// Outcome with the given children and no findings.
pub(super) fn outcome(
    children: &[crate::vocabulary::MomentHandle],
) -> crate::vocabulary::ScenarioOutcome {
    crate::vocabulary::ScenarioOutcome {
        children: children.to_vec(),
        findings: 0,
    }
}

/// Child moment discovered below the root.
pub(super) const fn child(
    handle: crate::vocabulary::MomentHandle,
) -> crate::vocabulary::ScenarioMoment {
    crate::vocabulary::ScenarioMoment {
        handle,
        parent: Some(crate::harness::history::ROOT),
    }
}

/// Opens a campaign with an ample budget and the profile bounds.
///
/// # Errors
///
/// Returns the error of the session setup.
pub(super) fn ample<'s, 'a>(
    harness: &'s mut crate::harness::Driver<'a>,
) -> crate::verdicts::ConformanceResult<crate::harness::Session<'s, 'a>> {
    crate::harness::Session::open(harness, AMPLE_BUDGET, |_| {})
}

/// Admits the standard operations in one batch and guides each at its standard levels.
///
/// # Errors
///
/// Returns the error of the admission or guidance.
pub(super) fn guided_frontier(
    session: &mut crate::harness::Session<'_, '_>,
    units: u64,
) -> crate::verdicts::ConformanceResult<()> {
    guided_frontier_after(session, units).map(|_| ())
}

/// Admits the standard operations in one batch and guides each at its standard levels; returns
/// the history before each of those four events (each graph holds nothing after its head).
///
/// # Errors
///
/// Returns the error of the admission or guidance.
pub(super) fn guided_frontier_after(
    session: &mut crate::harness::Session<'_, '_>,
    units: u64,
) -> crate::verdicts::ConformanceResult<[crate::harness::oracle::HistoryView; 4]> {
    let before_batch = session.history();
    session.admit(&[
        operation(OP_A, crate::harness::history::ROOT, units),
        operation(OP_B, crate::harness::history::ROOT, units),
        operation(OP_C, crate::harness::history::ROOT, units),
    ])?;
    let before_first = session.history();
    session.guide(&preference(OP_A, FIRST_GENERATION, LEVELS_A))?;
    let before_second = session.history();
    session.guide(&preference(OP_B, FIRST_GENERATION, LEVELS_B))?;
    let before_last = session.history();
    session.guide(&preference(OP_C, FIRST_GENERATION, LEVELS_C))?;
    require(
        session.state.candidates.len() == 3 && session.state.guidance_record_count == 3,
        "the standard frontier holds three guided candidates",
    )?;
    debug_assert_eq!(session.state.candidates.len(), 3);
    debug_assert_eq!(session.state.guidance_record_count, 3);
    Ok([before_batch, before_first, before_second, before_last])
}

/// Requires a declared expectation.
///
/// # Errors
///
/// Returns `Diverged` naming the expectation when it does not hold.
pub(super) const fn require(
    is_held: bool,
    expected: &'static str,
) -> crate::verdicts::ConformanceResult<()> {
    if is_held {
        Ok(())
    } else {
        Err(crate::verdicts::ConformanceError::Diverged { expected })
    }
}

/// Requires that a forbidden effect did not happen.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` naming the effect when it happened.
pub(super) const fn forbid(
    has_happened: bool,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<()> {
    if has_happened {
        Err(crate::verdicts::ConformanceError::ForbiddenAccepted { forbidden })
    } else {
        Ok(())
    }
}

/// Operation handle of a selection.
///
/// # Errors
///
/// Returns `Diverged` for a candidate outside the scenario.
pub(super) fn selected(
    session: &crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::OperationHandle> {
    session.operation_of(selection.candidate_id)
}

/// The standard operation at position `index % 3` of a declared rank-dimension order.
///
/// # Errors
///
/// Returns a typed error when the position cannot be computed.
pub(super) fn explored(
    order: &[u32],
    index: u64,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::OperationHandle> {
    explored_among(order, &STANDARD_OPERATIONS, index)
}

/// The eligible standard operation at position `index % eligible` of a declared rank-dimension
/// order.
///
/// # Errors
///
/// Returns a typed error when the position cannot be computed.
pub(super) fn explored_among(
    order: &[u32],
    eligible: &[crate::vocabulary::OperationHandle],
    index: u64,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::OperationHandle> {
    let ordered: alloc::vec::Vec<crate::vocabulary::OperationHandle> = ordered_by(order)
        .into_iter()
        .filter(|handle| eligible.contains(handle))
        .collect();
    let count = u64::try_from(ordered.len())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let remainder = index
        .checked_rem(count)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let position =
        usize::try_from(remainder).map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    ordered
        .get(position)
        .copied()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the position names a standard operation",
        })
}

/// Standard operations ordered by their levels projected onto `order`, highest first.
pub(super) fn ordered_by(order: &[u32]) -> alloc::vec::Vec<crate::vocabulary::OperationHandle> {
    ordered_levels(
        order,
        &[(LEVELS_A, OP_A), (LEVELS_B, OP_B), (LEVELS_C, OP_C)],
    )
}

/// Operations ordered by their levels projected onto `order`, highest first.
pub(super) fn ordered_levels(
    order: &[u32],
    scored: &[([u32; 2], crate::vocabulary::OperationHandle)],
) -> alloc::vec::Vec<crate::vocabulary::OperationHandle> {
    let project = |levels: [u32; 2]| -> alloc::vec::Vec<u32> {
        order
            .iter()
            .filter_map(|dimension| usize::try_from(*dimension).ok())
            .filter_map(|dimension| levels.get(dimension).copied())
            .collect()
    };
    let mut ordered: alloc::vec::Vec<(alloc::vec::Vec<u32>, crate::vocabulary::OperationHandle)> =
        scored
            .iter()
            .map(|(levels, handle)| (project(*levels), *handle))
            .collect();
    ordered.sort_by(|left, right| right.0.cmp(&left.0));
    ordered.into_iter().map(|(_, handle)| handle).collect()
}
