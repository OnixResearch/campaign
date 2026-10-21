//! History, cache, receipt, and role fixtures.

mod caches;
mod claims;
mod coverage;
mod forged;
mod histories;
mod lineages;
mod loads;
mod roles;

/// History, cache, receipt, and role fixtures in suite order.
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "history.rewritten_payload",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: forged::rewritten_payload,
    },
    super::Fixture {
        name: "history.altered_graph",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: forged::altered_graph,
    },
    super::Fixture {
        name: "history.ambiguous_lineage",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: forged::ambiguous_lineage,
    },
    super::Fixture {
        name: "history.load_matrix",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: loads::load_matrix,
    },
    super::Fixture {
        name: "caches.fresh",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: fresh_cache,
    },
    super::Fixture {
        name: "caches.stale",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: stale_cache,
    },
    super::Fixture {
        name: "caches.moved_binding",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: caches::moved_binding,
    },
    super::Fixture {
        name: "caches.validation_matrix",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: caches::validation_matrix,
    },
    super::Fixture {
        name: "history.state_coverage",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: coverage::four_way_states,
    },
    super::Fixture {
        name: "receipts.non_claims",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: non_claims,
    },
    super::Fixture {
        name: "receipts.promotion_overclaim",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: promotion_overclaim,
    },
    super::Fixture {
        name: "receipts.non_selection_overclaim",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: claims::non_selection_overclaim,
    },
    super::Fixture {
        name: "receipts.planner_overclaim",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: claims::planned::planner_overclaim,
    },
    super::Fixture {
        name: "receipts.recorded_stop_overclaim",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: claims::stops::recorded_stop_overclaim,
    },
    super::Fixture {
        name: "receipts.crossed",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: crossed_receipt,
    },
    super::Fixture {
        name: "roles.collapsed_identity",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: roles::collapsed_identity,
    },
];

/// Presents one cache of the session's current history cold to the adapter's cache validation
/// and requires the adapter's answer to equal Campaign's.
fn validate_cache(
    session: &mut crate::harness::Session<'_, '_>,
    cache: &campaign_core::CacheProjection,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<campaign_core::FrontierState> {
    let history = session.history();
    let validated = session.present_cache(
        &history,
        cache,
        crate::harness::oracle::Presentation::Cold,
        forbidden,
    )?;
    Ok(validated?)
}

fn fresh_cache(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let cache = campaign_core::derive_cache(&session.state, &session.binding()?)?;
    let state = validate_cache(&mut session, &cache, caches::FRESH)?;
    super::common::require(
        state == session.state,
        "a fresh cache stands for the current state",
    )
}

fn stale_cache(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let cache = campaign_core::derive_cache(&session.state, &session.binding()?)?;
    session.execute(None)?;
    let forbidden = "a cache of a stale head was used as current state";
    let validated = validate_cache(&mut session, &cache, forbidden);
    session.harness.deny(
        validated,
        campaign_core::CampaignError::InvalidCache,
        forbidden,
    )
}

fn selection_receipt(
    session: &mut crate::harness::Session<'_, '_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignReceipt> {
    let planned = session.plan(draw)?;
    claims::plan_receipt(session, &planned)
}

fn non_claims(harness: &mut crate::harness::Driver<'_>) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let receipt = selection_receipt(&mut session, None)?;
    let repeated = selection_receipt(&mut session, None)?;
    let quiet = crate::port::AuthoritySignals {
        is_top_ranked: false,
        is_selected: false,
        has_zero_exit_status: false,
        has_check_reference: false,
    };
    super::common::require(
        receipt.state_id == session.state.state_id && receipt.guidance_record_id.is_some(),
        "the selection receipt binds the current state and the selected guidance",
    )?;
    debug_assert_eq!(receipt.state_id, session.state.state_id);
    debug_assert!(receipt.guidance_record_id.is_some());
    let claims = session.harness.adapter.promotion_claims(&receipt, &quiet);
    campaign_core::validate_receipt(&receipt, &claims)?;
    super::common::require(
        receipt.non_claims == campaign_core::NonClaim::ALL.to_vec(),
        "every receipt lists the canonical non-claims",
    )?;
    super::common::require(
        repeated == receipt,
        "equal receipt inputs give equal receipts",
    )
}

fn promotion_overclaim(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let receipt = selection_receipt(&mut session, None)?;
    super::common::require(
        receipt.non_claims.len() == campaign_core::NON_CLAIM_COUNT
            && !receipt.decision_id.is_absent(),
        "the selection receipt binds its decision and every non-claim",
    )?;
    debug_assert_eq!(receipt.non_claims.len(), campaign_core::NON_CLAIM_COUNT);
    debug_assert!(!receipt.decision_id.is_absent());
    let explored = selection_receipt(
        &mut session,
        Some(crate::vocabulary::ScenarioDraw {
            explore_draw: 0,
            index_draw: 1,
        }),
    )?;
    super::common::require(
        explored.entropy_ticket_id.is_some() && explored.decision_id != receipt.decision_id,
        "the exploratory selection receipt binds its ticket",
    )?;
    claims::require_unclaimed(
        &mut session,
        &[
            (
                "rank, selection, exit status, or a check reference became promotion authority",
                receipt.clone(),
            ),
            (
                "an exploratory selection receipt carried a promotion claim",
                explored,
            ),
        ],
    )?;
    let overclaims: [fn(&mut campaign_core::ReceiptClaims); campaign_core::NON_CLAIM_COUNT] = [
        |claims| claims.exhaustive_search = true,
        |claims| claims.event_truth = true,
        |claims| claims.operation_success = true,
        |claims| claims.replay_success = true,
        |claims| claims.finding_correctness = true,
        |claims| claims.authorization = true,
        |claims| claims.release_readiness = true,
        |claims| claims.safe_deletion = true,
    ];
    for overclaim in overclaims {
        let mut claimed = campaign_core::ReceiptClaims::no_claims();
        overclaim(&mut claimed);
        let validated = campaign_core::validate_receipt(&receipt, &claimed);
        session.harness.deny(
            validated.map_err(crate::verdicts::ConformanceError::from),
            campaign_core::CampaignError::InvalidReceiptClaim,
            "a receipt carried a forbidden claim",
        )?;
    }
    Ok(())
}

fn crossed_receipt(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let planned = session.plan(None)?;
    session.retire(super::common::OP_C)?;
    let crossed = campaign_core::project_receipt(
        &session.state,
        &planned.plan,
        &planned.request,
        &session.binding()?,
    );
    session.harness.deny(
        crossed.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::CrossedIdentity,
        "a receipt combined a decision with another state",
    )
}
