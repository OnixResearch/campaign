//! Guidance families whose deciding level is neither the first nor the last (the second level,
//! and interior levels between the second and the last), and whose levels rise from first to
//! last: a guidance mapping that drops or zeroes middle or interior levels, or reorders levels by
//! value, changes the ranked choice.

/// Level both candidates share first in the middle-level family.
const TIED_FIRST: u32 = 5;

/// Middle levels: A above B, then B's strict improvement above A.
const MIDDLE_HIGH: u32 = 9;
const MIDDLE_LOW: u32 = 1;
const MIDDLE_IMPROVED: u32 = 10;

/// Levels after the deciding middle level: A below B, so the last level alone decides otherwise.
const TRAILING_LOW: u32 = 1;
const TRAILING_HIGH: u32 = 9;

/// Smallest score with a level strictly between the first and the last.
const MIDDLE_COUNT: usize = 3;

/// Smallest score with a level strictly between the second and the last.
const INTERIOR_COUNT: usize = 4;

/// Position of the first interior level (after the two tied leading levels).
const FIRST_INTERIOR: usize = 2;

/// Most levels the suite presents in one score; a profile with fewer `max_rank_dimensions` caps
/// it.
const LEVELS_ROOM: usize = 64;

/// Rising scores: A's first level below B's while its second level exceeds its own first and
/// B's; then A's strict improvement above B's first level.
const RISING_A: [u32; 2] = [1, 30];
const FALLING_B: [u32; 2] = [20, 3];
const RISING_A_IMPROVED: [u32; 2] = [25, 31];

fn preference(
    operation: crate::vocabulary::OperationHandle,
    generation: u64,
    levels: alloc::vec::Vec<u32>,
) -> crate::vocabulary::ScenarioPreference {
    crate::vocabulary::ScenarioPreference {
        operation,
        generation,
        score: crate::vocabulary::AbstractScore { levels },
    }
}

/// In its own campaign, A and B guided at `a` and `b`: the ranked planner must select `first`,
/// and after `improved` (a strict improvement of the other candidate) the other one.
///
/// # Errors
///
/// Returns the error of the scenario and `Diverged` naming the choice that did not hold.
fn decides(
    harness: &mut crate::harness::Driver<'_>,
    (a, b): (alloc::vec::Vec<u32>, alloc::vec::Vec<u32>),
    first: crate::vocabulary::OperationHandle,
    improved: &crate::vocabulary::ScenarioPreference,
    expected: [&'static str; 2],
) -> crate::verdicts::ConformanceResult<()> {
    let [before_expected, after_expected] = expected;
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
    session.guide(&preference(
        super::super::common::OP_A,
        super::super::common::FIRST_GENERATION,
        a,
    ))?;
    session.guide(&preference(
        super::super::common::OP_B,
        super::super::common::FIRST_GENERATION,
        b,
    ))?;
    let before = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &before)? == first,
        before_expected,
    )?;
    debug_assert_ne!(improved.operation, first);
    session.guide(improved)?;
    let after = session.select(None)?;
    super::super::common::require(
        super::super::common::selected(&session, &after)? == improved.operation,
        after_expected,
    )
}

/// Most levels the profile admits in one score, capped at `LEVELS_ROOM`.
fn most_levels(harness: &crate::harness::Driver<'_>) -> usize {
    usize::try_from(harness.profile.bounds.max_rank_dimensions)
        .map_or(LEVELS_ROOM, |bound| bound.min(LEVELS_ROOM))
}

/// Scores of `count` levels that tie before `deciding` and are decided there: A's deciding level
/// above B's and every later level below B's; B improved at the deciding level above A's.
fn decided_at(
    (count, deciding): (usize, usize),
) -> (
    alloc::vec::Vec<u32>,
    alloc::vec::Vec<u32>,
    alloc::vec::Vec<u32>,
) {
    let trailing = count.saturating_sub(deciding.saturating_add(1));
    let mut a = alloc::vec![TIED_FIRST; deciding];
    let mut b = a.clone();
    a.push(MIDDLE_HIGH);
    a.extend(core::iter::repeat_n(TRAILING_LOW, trailing));
    b.push(MIDDLE_LOW);
    b.extend(core::iter::repeat_n(TRAILING_HIGH, trailing));
    let mut improved = b.clone();
    if let Some(level) = improved.get_mut(deciding) {
        *level = MIDDLE_IMPROVED;
    }
    debug_assert_eq!(a.len(), count);
    debug_assert_eq!(improved.len(), count);
    (a, b, improved)
}

/// Requires the family decided at `deciding` among `count` levels: A first, then B once B's
/// deciding level strictly improves above A's.
fn family(
    harness: &mut crate::harness::Driver<'_>,
    (count, deciding): (usize, usize),
    expected: [&'static str; 2],
) -> crate::verdicts::ConformanceResult<()> {
    let (a, b, improved) = decided_at((count, deciding));
    decides(
        harness,
        (a, b),
        super::super::common::OP_A,
        &preference(
            super::super::common::OP_B,
            super::super::common::SECOND_GENERATION,
            improved,
        ),
        expected,
    )
}

/// Scores of three levels and of the most rank dimensions the profile admits (when it admits a
/// middle level) whose first levels tie and whose second level decides against every later
/// level: A first, then B once B's second level strictly improves above A's.
// r[impl campaign.conformance.bounds]
pub(super) fn middle_level(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let most = most_levels(harness);
    let mut counts = alloc::vec![MIDDLE_COUNT, most];
    counts.retain(|count| *count >= MIDDLE_COUNT && *count <= most);
    counts.dedup();
    for count in counts {
        family(
            harness,
            (count, 1),
            [
                "a middle level decides between candidates whose first levels tie",
                "an improved middle level decides between candidates whose first levels tie",
            ],
        )?;
    }
    Ok(())
}

/// Scores of four levels and of the most rank dimensions the profile admits (when it admits an
/// interior level) whose leading levels tie and whose deciding level lies between the second and
/// the last: the first interior level, the middle level, and the second-to-last level. A first,
/// then B once B's deciding level strictly improves above A's.
// r[impl campaign.guidance.valid]
// r[impl campaign.conformance.bounds]
pub(super) fn interior_level(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let most = most_levels(harness);
    let counts = [INTERIOR_COUNT, most];
    let deciding_levels = 3;
    let mut families = alloc::vec::Vec::with_capacity(counts.len().saturating_mul(deciding_levels));
    for count in counts {
        if count < INTERIOR_COUNT || count > most {
            continue;
        }
        let last_interior = count.saturating_sub(2);
        for deciding in [
            FIRST_INTERIOR,
            count.checked_div(2).unwrap_or(0),
            last_interior,
        ] {
            let family = (count, deciding);
            if (FIRST_INTERIOR..=last_interior).contains(&deciding) && !families.contains(&family) {
                families.push(family);
            }
        }
    }
    debug_assert!(families.len() <= counts.len().saturating_mul(deciding_levels));
    debug_assert!(
        families
            .iter()
            .all(|(count, deciding)| *deciding >= FIRST_INTERIOR && deciding < count)
    );
    for decided in families {
        family(
            harness,
            decided,
            [
                "an interior level decides between candidates whose leading levels tie",
                "an improved interior level decides between candidates whose leading levels tie",
            ],
        )?;
    }
    Ok(())
}

/// Two-level scores whose second level exceeds the first: B first (first level 20 above 1,
/// although A's second level 30 is the largest level), then A once its first level strictly
/// improves above B's.
// r[impl campaign.conformance.bounds]
pub(super) fn level_order(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    decides(
        harness,
        (RISING_A.to_vec(), FALLING_B.to_vec()),
        super::super::common::OP_B,
        &preference(
            super::super::common::OP_A,
            super::super::common::SECOND_GENERATION,
            RISING_A_IMPROVED.to_vec(),
        ),
        [
            "the first level decides although a later level is larger",
            "an improved first level decides although a later level is larger",
        ],
    )
}
