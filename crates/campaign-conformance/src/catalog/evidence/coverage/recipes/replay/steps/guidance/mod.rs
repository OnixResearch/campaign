//! Guidance steps of a recipe on its first two operations.

/// Levels of the first operation's strict improvement (and of its equal-rank update), and of its
/// later revision: unique in every rank dimension among the guided operations.
const IMPROVED_LEVELS: [u32; 2] = [35, 5];
const REVISED_LEVELS: [u32; 2] = [25, 6];

/// Generations of the equal-rank update of the improved record and of the revision after it (the
/// second operation keeps its first generation, which a fork's fill improves next).
const COALESCED_GENERATION: u64 = 3;
const REVISED_GENERATION: u64 = 4;

/// The operation position, generation, and levels a guidance step publishes; none for a step
/// that publishes no guidance.
const fn target(step: super::super::super::Step) -> Option<(usize, u64, [u32; 2])> {
    if matches!(
        step,
        super::super::super::Step::Discover { is_second: false }
    ) {
        Some((
            0,
            crate::catalog::common::FIRST_GENERATION,
            crate::catalog::common::LEVELS_A,
        ))
    } else if matches!(
        step,
        super::super::super::Step::Discover { is_second: true }
    ) {
        Some((
            1,
            crate::catalog::common::FIRST_GENERATION,
            crate::catalog::common::LEVELS_B,
        ))
    } else if matches!(step, super::super::super::Step::Improve) {
        Some((
            0,
            crate::catalog::common::SECOND_GENERATION,
            IMPROVED_LEVELS,
        ))
    } else if matches!(step, super::super::super::Step::CoalescedGuidance) {
        Some((0, COALESCED_GENERATION, IMPROVED_LEVELS))
    } else if matches!(step, super::super::super::Step::Revise) {
        Some((0, REVISED_GENERATION, REVISED_LEVELS))
    } else {
        None
    }
}

/// Publishes one guidance step on the first or the second recipe operation.
///
/// # Errors
///
/// Returns the adapter or Campaign error of the guidance, and `Diverged` for a step that
/// publishes no guidance.
pub(super) fn publish(
    session: &mut crate::harness::Session<'_, '_>,
    step: super::super::super::Step,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(
        session.state.stopped.is_none(),
        "a recipe publishes guidance before its stop"
    );
    let (position, generation, levels) =
        target(step).ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the recipe publishes guidance only in its guidance steps",
        })?;
    let preference =
        crate::catalog::common::preference(super::operation(position)?, generation, levels);
    if matches!(step, super::super::super::Step::Revise) {
        session.revise(&preference)?;
    } else {
        session.guide(&preference)?;
    }
    debug_assert!(
        session.state.guidance_record_count > 0,
        "every guidance step follows a counted record"
    );
    Ok(())
}
