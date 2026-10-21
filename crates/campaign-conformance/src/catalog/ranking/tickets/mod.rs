//! Entropy draws that tell ticket mappings apart.

pub(crate) mod draws;

/// Presents one explore draw on the explore side with each generated index draw; each must take
/// the declared exploration position.
fn explore_each(
    session: &mut crate::harness::Session<'_, '_>,
    explore_draw: u64,
    eligible: &[([u32; 2], crate::vocabulary::OperationHandle)],
    expected: &'static str,
) -> crate::verdicts::ConformanceResult<()> {
    let order = session.harness.profile.exploration_order.clone();
    let ordered = super::super::common::ordered_levels(&order, eligible);
    let count = u64::try_from(ordered.len())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    for index_draw in draws::wide_indexes(ordered.len())? {
        let position = index_draw
            .checked_rem(count)
            .and_then(|position| usize::try_from(position).ok())
            .and_then(|position| ordered.get(position))
            .copied()
            .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
        let selection = session.select(Some(crate::vocabulary::ScenarioDraw {
            explore_draw,
            index_draw,
        }))?;
        super::super::common::require(
            super::super::common::selected(session, &selection)? == position,
            expected,
        )?;
    }
    debug_assert_eq!(ordered.len(), eligible.len());
    Ok(())
}

/// Operations a campaign adds beside A and B for the seven-candidate exploration, with levels
/// unique in every rank dimension against A's and B's.
const ADDED: [([u32; 2], crate::vocabulary::OperationHandle); 5] = [
    ([40, 5], crate::vocabulary::OperationHandle(4)),
    ([50, 6], crate::vocabulary::OperationHandle(5)),
    ([60, 7], crate::vocabulary::OperationHandle(6)),
    ([70, 8], crate::vocabulary::OperationHandle(7)),
    ([80, 9], crate::vocabulary::OperationHandle(8)),
];

/// Candidates per added batch: the smallest `max_candidate_batch` the suite admits.
const ADDED_BATCH: usize = 3;

/// Admits and guides the added operations in batches the smallest batch bound admits.
fn add_candidates(
    session: &mut crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<()> {
    debug_assert!(
        u64::try_from(ADDED_BATCH)
            .is_ok_and(|batch| batch <= session.state.admission.bounds.max_candidate_batch)
    );
    for chunk in ADDED.chunks(ADDED_BATCH) {
        let operations: alloc::vec::Vec<crate::vocabulary::ScenarioOperation> = chunk
            .iter()
            .map(|(_, handle)| {
                super::super::common::operation(
                    *handle,
                    crate::harness::history::ROOT,
                    super::super::common::UNIT_COST,
                )
            })
            .collect();
        session.admit(&operations)?;
    }
    for (levels, handle) in ADDED {
        session.guide(&super::super::common::preference(
            handle,
            super::super::common::FIRST_GENERATION,
            levels,
        ))?;
    }
    Ok(())
}

/// Presents explore-side draws (one strictly inside the explore range, one above `2^32` whose
/// low word exploits, and, where one exists, one of at least `2^53` whose binary64 value exploits)
/// with index draws above `u32::MAX` in every residue of three, two,
/// and then seven eligible candidates, so a ticket that encodes the draws in another byte order,
/// truncates or narrows them (including a reduction modulo a power of ten, which keeps residues
/// modulo two and three but not seven), or ignores the index draw selects another candidate.
// r[impl campaign.conformance.bounds]
// r[impl campaign.entropy.valid]
pub(super) fn wide_draws(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let threshold = session
        .harness
        .profile
        .exploratory_threshold_parts_per_million;
    let inside = threshold
        .checked_div(2)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    super::super::common::require(
        inside > 0 && inside < threshold,
        "the explore draw lies strictly between zero and the threshold",
    )?;
    debug_assert!(inside > 0);
    debug_assert!(inside < threshold);
    let wide = draws::beyond_low_word(threshold, true)?;
    let precise = draws::beyond_binary64(threshold, true);
    let a = (super::super::common::LEVELS_A, super::super::common::OP_A);
    let b = (super::super::common::LEVELS_B, super::super::common::OP_B);
    let c = (super::super::common::LEVELS_C, super::super::common::OP_C);
    for explore_draw in [Some(inside), Some(wide), precise].into_iter().flatten() {
        explore_each(
            &mut session,
            explore_draw,
            &[a, b, c],
            "an inside draw with a wide index takes the declared exploration position",
        )?;
    }
    session.retire(super::super::common::OP_C)?;
    for explore_draw in [1, wide] {
        explore_each(
            &mut session,
            explore_draw,
            &[a, b],
            "a wide index over two eligible candidates takes the declared exploration position",
        )?;
    }
    add_candidates(&mut session)?;
    let mut seven = alloc::vec![a, b];
    seven.extend(ADDED);
    explore_each(
        &mut session,
        inside,
        &seven,
        "a wide index over seven eligible candidates takes the declared exploration position",
    )
}

/// Presents exploit-side draws (the threshold itself, the last draw below the entropy scale, one
/// above `2^32` whose low word explores, and, where one exists, one of at least `2^53` whose
/// binary64 value explores), each with index draws above `u32::MAX` in every
/// residue of the three eligible candidates, so at least one index draw's exploration position
/// under the declared exploration order differs from the ranked choice; every one must exploit
/// the ranked order.
// r[impl campaign.entropy.valid]
pub(super) fn exploit(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let threshold = session
        .harness
        .profile
        .exploratory_threshold_parts_per_million;
    let last = campaign_core::ENTROPY_SCALE
        .checked_sub(1)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    let order = session.harness.profile.exploration_order.clone();
    let indexes = draws::wide_indexes(super::super::common::STANDARD_OPERATIONS.len())?;
    let mut positions = alloc::vec::Vec::with_capacity(indexes.len());
    for index_draw in &indexes {
        positions.push(super::super::common::explored(&order, *index_draw)?);
    }
    super::super::common::require(
        threshold <= last
            && positions
                .iter()
                .any(|position| *position != super::super::common::OP_A),
        "an exploit-side index draw has an exploration position other than the ranked choice",
    )?;
    let precise = draws::beyond_binary64(threshold, false);
    for explore_draw in [
        Some(threshold),
        Some(last),
        Some(draws::beyond_low_word(threshold, false)?),
        precise,
    ]
    .into_iter()
    .flatten()
    {
        for index_draw in &indexes {
            let selection = session.select(Some(crate::vocabulary::ScenarioDraw {
                explore_draw,
                index_draw: *index_draw,
            }))?;
            super::super::common::require(
                selection.entropy_id.is_some(),
                "the exploratory selection binds its ticket",
            )?;
            super::super::common::require(
                super::super::common::selected(&session, &selection)? == super::super::common::OP_A,
                "an exploit-side draw exploits the ranked order",
            )?;
        }
    }
    debug_assert_eq!(positions.len(), indexes.len());
    Ok(())
}
