//! Guidance relaxation, revision, and coalescing fixtures.

/// Guidance fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "guidance.strict_improvement",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: strict_improvement,
    },
    super::Fixture {
        name: "guidance.revision",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: revision,
    },
    super::Fixture {
        name: "guidance.non_improving",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: non_improving,
    },
    super::Fixture {
        name: "guidance.coalesced_duplicate",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: coalesced_duplicate,
    },
    super::Fixture {
        name: "guidance.monotone_levels",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: monotone_levels,
    },
];

/// Level pairs probed for strict monotonicity: the lowest levels, both sides of the byte and
/// half-word boundaries, the highest levels, and the extreme span.
const BOUNDARY_PAIRS: [(u32, u32); 6] = [
    (0, 1),
    (254, 255),
    (255, 256),
    (65_535, 65_536),
    (4_294_967_294, u32::MAX),
    (0, u32::MAX),
];

/// Guidance generation of a revision published after a second-generation update.
const THIRD_GENERATION: u64 = 3;

fn strict_improvement(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    session.guide(&super::common::preference(
        super::common::OP_C,
        super::common::SECOND_GENERATION,
        [40, 4],
    ))?;
    let selection = session.select(None)?;
    super::common::require(
        super::common::selected(&session, &selection)? == super::common::OP_C,
        "later selection uses the strictly improved record",
    )
}

fn revision(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let candidate = session.candidate_id(super::common::OP_A)?;
    session.revise(&super::common::preference(
        super::common::OP_A,
        super::common::SECOND_GENERATION,
        [5, 0],
    ))?;
    let revised = session.state.candidate(candidate)?.latest_guidance.clone();
    super::common::require(
        revised.is_some() && session.state.selection_count == 0,
        "the revision left a latest record before any selection",
    )?;
    debug_assert!(revised.is_some());
    debug_assert_eq!(session.state.selection_count, 0);
    let selection = session.select(None)?;
    super::common::require(
        revised.is_some_and(|guidance| guidance.generation == super::common::SECOND_GENERATION),
        "the revision replaced the latest record",
    )?;
    super::common::require(
        super::common::selected(&session, &selection)? == super::common::OP_B,
        "later selection uses the revised record",
    )
}

fn non_improving(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let before = session.state.state_id;
    let lowered = session.guidance_record(&super::common::preference(
        super::common::OP_A,
        super::common::SECOND_GENERATION,
        super::common::LEVELS_B,
    ))?;
    super::common::require(
        lowered.generation == super::common::SECOND_GENERATION && !lowered.ranks.is_empty(),
        "the lowered preference maps to a second-generation record",
    )?;
    debug_assert_eq!(lowered.generation, super::common::SECOND_GENERATION);
    debug_assert!(!lowered.ranks.is_empty());
    let admitted = campaign_core::admit_guidance(&session.state, lowered);
    session.harness.deny(
        admitted.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::NonImprovingRelaxation,
        "a non-improving update was admitted as relaxation",
    )?;
    super::common::require(
        session.state.state_id == before,
        "the denied update left the state unchanged",
    )?;
    let selection = session.select(None)?;
    super::common::require(
        super::common::selected(&session, &selection)? == super::common::OP_A,
        "the denied update scheduled nothing",
    )
}

/// An equal-rank second-generation update coalesces into the first record, which stays latest; a
/// later revision must name that kept record (the revision context's invalidated record), not the
/// coalesced successor, and replaces it.
fn coalesced_duplicate(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    session.guide(&super::common::preference(
        super::common::OP_A,
        super::common::FIRST_GENERATION,
        super::common::LEVELS_A,
    ))?;
    let records = session.state.guidance_record_count;
    let event = session.guide(&super::common::preference(
        super::common::OP_A,
        super::common::SECOND_GENERATION,
        super::common::LEVELS_A,
    ))?;
    let receipt = super::intake::event_receipt(&session, &event)?;
    super::common::require(
        receipt.decision_id == event.event_id() && session.state.candidates.len() == 1,
        "the guidance receipt names the guidance event of the one candidate",
    )?;
    debug_assert_eq!(receipt.decision_id, event.event_id());
    debug_assert_eq!(session.state.candidates.len(), 1);
    super::common::require(
        session.state.guidance_record_count == records,
        "an equal-rank update adds no guidance record",
    )?;
    super::common::require(
        receipt.admission_outcomes_id.is_some(),
        "the guidance receipt binds the recorded dynamic",
    )?;
    let candidate = session.candidate_id(super::common::OP_A)?;
    let kept = session.state.candidate(candidate)?.latest_guidance.clone();
    session.revise(&super::common::preference(
        super::common::OP_A,
        THIRD_GENERATION,
        [5, 0],
    ))?;
    let revised = session.state.candidate(candidate)?.latest_guidance.clone();
    debug_assert!(revised.is_some());
    super::common::require(
        kept.is_some_and(|guidance| guidance.generation == super::common::FIRST_GENERATION)
            && revised.is_some_and(|guidance| guidance.generation == THIRD_GENERATION),
        "a revision after a coalesced update replaces the kept first-generation record",
    )
}

/// Probes the adapter's score conversion at level boundaries: equal levels map to equal ranks and
/// a higher level maps to a strictly higher rank in every dimension.
fn monotone_levels(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    for (lower, higher) in BOUNDARY_PAIRS {
        let low_preference = super::common::preference(
            super::common::OP_A,
            super::common::FIRST_GENERATION,
            [lower, lower],
        );
        let low = session.guidance_record(&low_preference)?;
        let repeated = session.guidance_record(&low_preference)?;
        let high = session.guidance_record(&super::common::preference(
            super::common::OP_A,
            super::common::FIRST_GENERATION,
            [higher, higher],
        ))?;
        super::common::require(
            low.ranks.len() == low_preference.score.levels.len()
                && high.ranks.len() == low.ranks.len(),
            "every level maps to one rank per scenario dimension",
        )?;
        super::common::require(
            repeated.ranks == low.ranks,
            "equal levels map to equal ranks",
        )?;
        super::common::require(
            low.ranks
                .iter()
                .zip(&high.ranks)
                .all(|(low, high)| low < high),
            "a higher level maps to a strictly higher rank at every level boundary",
        )?;
        debug_assert_eq!(repeated.ranks, low.ranks);
        debug_assert!(lower < higher);
    }
    Ok(())
}
