//! Selection, entropy, and reconstruction fixtures.

mod families;
mod levels;
pub(crate) mod tickets;

/// Selection, entropy, and reconstruction fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "selection.best_ranked",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: best_ranked,
    },
    super::Fixture {
        name: "selection.second_level",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: levels::second_level,
    },
    super::Fixture {
        name: "selection.level_counts",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: levels::level_counts,
    },
    super::Fixture {
        name: "selection.middle_level",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: families::middle_level,
    },
    super::Fixture {
        name: "selection.interior_level",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: families::interior_level,
    },
    super::Fixture {
        name: "selection.level_order",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: families::level_order,
    },
    super::Fixture {
        name: "selection.reuse",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: reuse,
    },
    super::Fixture {
        name: "selection.stale_request",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: stale_request,
    },
    super::Fixture {
        name: "entropy.exploit",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: tickets::exploit,
    },
    super::Fixture {
        name: "entropy.explore",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: explore,
    },
    super::Fixture {
        name: "entropy.wide_draws",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: tickets::wide_draws,
    },
    super::Fixture {
        name: "entropy.invalid_ticket",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: invalid_ticket,
    },
    super::Fixture {
        name: "reconstruction.incremental_equals_rebuild",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: incremental,
    },
    super::Fixture {
        name: "reconstruction.conflicting_history",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: conflicting_history,
    },
];

/// Second selection ordinal.
const SECOND_ORDINAL: u64 = 2;

/// Explore-index draw of the explore fixture.
const EXPLORE_INDEX: u64 = 5;

fn best_ranked(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let selection = session.select(None)?;
    super::common::require(
        super::common::selected(&session, &selection)? == super::common::OP_A,
        "the best ranked eligible candidate is selected",
    )?;
    super::common::require(
        selection.ordinal == 1,
        "the first selection has ordinal one",
    )
}

fn reuse(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let first = session.execute(None)?;
    session.settle(&first, &super::common::outcome(&[]))?;
    let second = session.select(None)?;
    super::common::require(
        second.candidate_id == first.candidate_id,
        "a settled candidate remains reusable",
    )?;
    super::common::require(
        second.ordinal == SECOND_ORDINAL,
        "the reuse takes the next ordinal",
    )?;
    super::common::require(
        second.operation_id != first.operation_id,
        "each selection is a distinct operation",
    )
}

fn stale_request(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    let earlier = session.state.state_id;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let current = session.load()?;
    let mut stale_state = session.request(None, current.binding())?;
    stale_state.state_id = earlier;
    debug_assert_ne!(earlier, current.state().state_id);
    let planned = campaign_choregraph::plan_next_from_published_projection(
        &current,
        campaign_choregraph::PublishedSelectionRequest {
            request: stale_state,
        },
    );
    session.harness.deny(
        planned.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::StaleState,
        "a request for another state was planned",
    )?;
    let mut stale_head = session.request(None, current.binding())?;
    stale_head.expected_generation = stale_head.expected_generation.saturating_sub(1);
    debug_assert_ne!(stale_head.expected_generation, current.binding().generation);
    let planned = campaign_choregraph::plan_next_from_published_projection(
        &current,
        campaign_choregraph::PublishedSelectionRequest {
            request: stale_head,
        },
    );
    session.harness.deny_with(
        planned.map_err(crate::verdicts::ConformanceError::from),
        "a request for a stale branch generation was planned",
        |error| {
            *error
                == crate::verdicts::ConformanceError::Adapter(
                    campaign_choregraph::AdapterError::ProjectionBindingMismatch,
                )
        },
    )
}

fn explore(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let expected =
        super::common::explored(&session.harness.profile.exploration_order, EXPLORE_INDEX)?;
    let selection = session.select(Some(crate::vocabulary::ScenarioDraw {
        explore_draw: 0,
        index_draw: EXPLORE_INDEX,
    }))?;
    super::common::require(
        super::common::selected(&session, &selection)? == expected,
        "the explore branch takes the declared exploration position",
    )
}

fn invalid_ticket(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let current = session.load()?;
    let draw = crate::vocabulary::ScenarioDraw {
        explore_draw: 0,
        index_draw: 0,
    };
    let exploratory = session.request(Some(draw), current.binding())?;
    let mut missing = exploratory.clone();
    missing.entropy = None;
    let mut ranked_with_ticket = session.request(None, current.binding())?;
    ranked_with_ticket.entropy.clone_from(&exploratory.entropy);
    super::common::require(
        exploratory.entropy.is_some() && missing.entropy.is_none(),
        "the exploratory request carries the adapter's ticket",
    )?;
    debug_assert!(exploratory.entropy.is_some());
    debug_assert!(missing.entropy.is_none());
    let mut crossed = exploratory;
    if let Some(ticket) = crossed.entropy.as_mut() {
        ticket.counter = ticket.counter.saturating_add(1);
    }
    for (request, forbidden) in [
        (missing, "an exploratory plan ran without a ticket"),
        (
            ranked_with_ticket,
            "a ranked plan accepted an unvalidated ticket",
        ),
        (crossed, "a forged ticket reached a selection"),
    ] {
        let planned = campaign_core::plan_next(current.state(), request, current.binding());
        session.harness.deny(
            planned.map_err(crate::verdicts::ConformanceError::from),
            campaign_core::CampaignError::InvalidEntropy,
            forbidden,
        )?;
    }
    Ok(())
}

fn incremental(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let session = super::outcomes::varied_session(harness)?;
    let events = session.control_events();
    let (admission, rest) = events
        .split_first()
        .ok_or(campaign_core::CampaignError::MissingAdmission)?;
    let mut folded = campaign_core::reconstruct(core::slice::from_ref(admission))?;
    for (index, event) in rest.iter().enumerate() {
        folded = campaign_core::apply_control_event(folded, event)?;
        let prefix = events
            .get(..index.saturating_add(2))
            .ok_or(campaign_core::CampaignError::MissingAdmission)?;
        super::common::require(
            folded == campaign_core::reconstruct(prefix)?,
            "incremental application equals reconstruction of every prefix",
        )?;
    }
    super::common::require(folded == session.state, "the fold equals the session state")
}

fn conflicting_history(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::outcomes::varied_session(harness)?;
    let events = session.control_events();
    let duplicate = events
        .get(1)
        .ok_or(campaign_core::CampaignError::MissingAdmission)?;
    let applied = campaign_core::apply_control_event(session.state.clone(), duplicate);
    session.harness.deny(
        applied.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::DuplicateControlEvent,
        "a duplicate control event was applied",
    )?;
    debug_assert!(events.len() > 1);
    debug_assert!(session.state.stopped.is_none());
    let consumer_stop = session.consumer_stop()?;
    let superseded = campaign_core::admit_stop(&session.state, consumer_stop)?;
    session.retire(super::common::OP_C)?;
    let state = session.state.clone();
    let reordered = session
        .plan_publication(superseded)
        .and_then(|publication| {
            Ok(campaign_core::apply_control_event(
                state.clone(),
                publication.control_event(),
            )?)
        });
    session.harness.deny(
        reordered,
        campaign_core::CampaignError::StaleState,
        "an event built against a superseded state was applied",
    )?;
    super::common::require(
        session.state == state,
        "denied history left the state unchanged",
    )
}
