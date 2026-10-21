//! Execution-gate fixtures that present a real durable publication for another selection.

/// Presents a real durable publication of one selection together with a different selection: a
/// fresh plan that was never published, and the published selection re-derived for another
/// candidate. The gate must grant only the exact published decision.
pub(super) fn foreign_selection(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let published = session.select(None)?;
    let publication = session.publish_selection(&published)?;
    let evidence = crate::harness::decisions::durable_evidence(
        &publication,
        crate::harness::decisions::Durability::COMMITTED,
    );
    let unpublished = session.select(None)?;
    super::super::common::require(
        unpublished.selection_id != published.selection_id,
        "planning after the publication yields another selection",
    )?;
    debug_assert_ne!(unpublished.selection_id, published.selection_id);
    let authority = session.gate(
        &unpublished,
        Some(&publication),
        Some(&evidence),
        "an unpublished selection executed under another selection's durable publication",
    )?;
    super::super::common::forbid(
        authority == crate::port::ExecutionAuthority::Granted,
        "an unpublished selection executed under another selection's durable publication",
    )?;
    let mut relabelled = published.clone();
    relabelled.candidate_id = session.candidate_id(super::super::common::OP_B)?;
    let relabelled = relabelled.derive()?;
    super::super::common::require(
        relabelled.candidate_id != published.candidate_id
            && relabelled.selection_id != published.selection_id,
        "the relabelled selection names another candidate",
    )?;
    debug_assert_ne!(relabelled.candidate_id, published.candidate_id);
    let authority = session.gate(
        &relabelled,
        Some(&publication),
        Some(&evidence),
        "a relabelled selection executed under the original durable publication",
    )?;
    super::super::common::forbid(
        authority == crate::port::ExecutionAuthority::Granted,
        "a relabelled selection executed under the original durable publication",
    )
}

/// A planner that refreshed only its branch fence plans from the state before the winner's
/// publication: its selection reuses the winner's ordinal. Campaign refuses to publish it, and
/// the gate must not execute it under the winner's real durable publication.
pub(super) fn stale_state_selection(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let stale = session.state.clone();
    let winner = session.select(None)?;
    let publication = session.publish_selection(&winner)?;
    let evidence = crate::harness::decisions::durable_evidence(
        &publication,
        crate::harness::decisions::Durability::COMMITTED,
    );
    let refreshed = session.binding()?;
    let mut request = session.request(None, &refreshed)?;
    request.state_id = stale.state_id;
    let plan = campaign_core::plan_next(&stale, request, &refreshed)?;
    let campaign_core::NextDecision::Select(rival) = plan.decision else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "the stale-state planner selects",
        });
    };
    super::super::common::require(
        rival.ordinal == winner.ordinal && rival.selection_id != winner.selection_id,
        "the stale-state selection reuses the winner's ordinal under the refreshed fence",
    )?;
    debug_assert_eq!(rival.ordinal, winner.ordinal);
    debug_assert_ne!(rival.selection_id, winner.selection_id);
    let published = session.publish_selection(&rival);
    session.harness.deny(
        published,
        campaign_core::CampaignError::StaleState,
        "a stale-state selection was published under a refreshed fence",
    )?;
    let authority = session.gate(
        &rival,
        Some(&publication),
        Some(&evidence),
        "a stale-state selection executed under the winner's durable publication",
    )?;
    super::super::common::forbid(
        authority == crate::port::ExecutionAuthority::Granted,
        "a stale-state selection executed under the winner's durable publication",
    )
}
