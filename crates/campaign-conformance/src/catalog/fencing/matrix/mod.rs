//! Execution-gate oracle matrices.
//!
//! One ranked and one exploratory selection each face every gate question class: never planned,
//! planned without evidence, planned before commit, durably published (the only grant), without
//! a publication or evidence, published without evidence, and evidence without its publication
//! once the shell loaded the history carrying the publication, evidence of another branch
//! transition, a foreign selection, a relabelled selection, a stale-state rival, every
//! single-field tampering of the published decision under its selection identity, every
//! single-field mutation of the publication's own durable evidence and every self-consistent
//! move of it onto a mirror branch or shifted generations, every real transition the session
//! committed presented for a publication it does not belong to (among them a compare-and-set
//! loser under the winner's self-consistent committed transition), and non-selection
//! publications (a guidance improvement, a candidate batch, and a stop, each with its own honest
//! committed evidence) presented for the published selection, the loser, and a selection that
//! was never published. Every answer must equal Campaign's own gate answer.

mod non_selection;
mod observations;
mod tampering;

/// Draw that takes the explore branch at a non-zero exploration position.
const EXPLORE: crate::vocabulary::ScenarioDraw = crate::vocabulary::ScenarioDraw {
    explore_draw: 0,
    index_draw: 1,
};

/// Gate matrix of a ranked selection.
pub(super) fn ranked(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    ask_every_question(harness, None)
}

/// Gate matrix of an exploratory selection.
pub(super) fn exploratory(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    ask_every_question(harness, Some(EXPLORE))
}

/// Asks one gate question Campaign refuses; the adapter must refuse it too.
fn refuse(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: Option<&campaign_choregraph::ControlPublication>,
    evidence: Option<&campaign_choregraph::DurablePublicationEvidence>,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<()> {
    let authority = session.gate(selection, publication, evidence, forbidden)?;
    super::super::common::forbid(
        authority == crate::port::ExecutionAuthority::Granted,
        forbidden,
    )
}

fn ask_every_question(
    harness: &mut crate::harness::Driver<'_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let stale = session.state.clone();
    let selection = session.select(draw)?;
    super::super::common::require(
        selection.entropy_id.is_some() == draw.is_some(),
        "the selection binds a ticket exactly when it is exploratory",
    )?;
    debug_assert_eq!(selection.entropy_id.is_some(), draw.is_some());
    refuse(
        &mut session,
        &selection,
        None,
        None,
        "a selection executed without a publication or evidence",
    )?;
    let planned = session.plan_publication(session.selection_event(&selection)?)?;
    refuse(
        &mut session,
        &selection,
        Some(&planned),
        None,
        "a planned selection executed without durability evidence",
    )?;
    let pending = crate::harness::decisions::durable_evidence(
        &planned,
        crate::harness::decisions::Durability::PENDING,
    );
    refuse(
        &mut session,
        &selection,
        Some(&planned),
        Some(&pending),
        "a planned selection executed before its publication committed",
    )?;
    let publication = session.publish_selection(&selection)?;
    let evidence = crate::harness::decisions::durable_evidence(
        &publication,
        crate::harness::decisions::Durability::COMMITTED,
    );
    let granted = session.gate(
        &selection,
        Some(&publication),
        Some(&evidence),
        "a selection executed without Campaign's publication gate",
    )?;
    super::super::common::require(
        granted == crate::port::ExecutionAuthority::Granted,
        "the gate grants the durably published selection",
    )?;
    debug_assert_eq!(granted, crate::port::ExecutionAuthority::Granted);
    observations::published_questions(&mut session, &selection, &publication, &evidence)?;
    observations::loaded_questions(&mut session, &selection, &publication, &evidence)?;
    foreign_questions(&mut session, &selection, &publication, &evidence, draw)?;
    tampered_questions(&mut session, &selection, &publication, &evidence)?;
    let rival = stale_rival(&mut session, &stale, &selection, draw)?;
    refuse(
        &mut session,
        &rival,
        Some(&publication),
        Some(&evidence),
        "a stale-state selection executed under the winner's durable publication",
    )?;
    observations::evidence_questions(&mut session, &selection, &publication, &evidence, draw)
}

/// Gate questions that present the real durable publication with another selection.
fn foreign_questions(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<()> {
    let foreign = session.select(draw)?;
    super::super::common::require(
        foreign.selection_id != selection.selection_id,
        "planning after the publication yields another selection",
    )?;
    debug_assert_ne!(foreign.selection_id, selection.selection_id);
    refuse(
        session,
        &foreign,
        Some(publication),
        Some(evidence),
        "an unpublished selection executed under another selection's durable publication",
    )?;
    let other = other_candidate(session, selection.candidate_id)?;
    let mut relabelled = selection.clone();
    relabelled.candidate_id = other;
    let relabelled = relabelled.derive()?;
    super::super::common::require(
        relabelled.selection_id != selection.selection_id,
        "the relabelled selection has its own identity",
    )?;
    refuse(
        session,
        &relabelled,
        Some(publication),
        Some(evidence),
        "a relabelled selection executed under the original durable publication",
    )
}

/// Gate questions that present the real durable publication with every single-field tampering
/// of its decision under the published selection identity.
fn tampered_questions(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
) -> crate::verdicts::ConformanceResult<()> {
    for (forbidden, tampered) in tampering::tampered_decisions(selection)? {
        refuse(
            session,
            &tampered,
            Some(publication),
            Some(evidence),
            forbidden,
        )?;
    }
    Ok(())
}

/// A standard candidate other than `candidate_id`.
fn other_candidate(
    session: &crate::harness::Session<'_, '_>,
    candidate_id: campaign_core::Id,
) -> crate::verdicts::ConformanceResult<campaign_core::Id> {
    for handle in super::super::common::STANDARD_OPERATIONS {
        let other = session.candidate_id(handle)?;
        if other != candidate_id {
            return Ok(other);
        }
    }
    Err(crate::verdicts::ConformanceError::Diverged {
        expected: "the standard frontier holds another candidate",
    })
}

/// A selection planned from the state before the winner's publication under the refreshed fence;
/// exploratory rivals carry a ticket of that stale state.
fn stale_rival(
    session: &mut crate::harness::Session<'_, '_>,
    stale: &campaign_core::FrontierState,
    winner: &campaign_core::SelectionDecision,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<campaign_core::SelectionDecision> {
    let refreshed = session.binding()?;
    let profile = session.selection_profile(draw.is_some())?;
    let entropy = match draw {
        Some(draw) => {
            let context = crate::port::EntropyContext {
                campaign_id: stale.admission.campaign_id,
                state_id: stale.state_id,
                selection_profile_id: profile.profile_id,
                counter: stale
                    .selection_count
                    .checked_add(1)
                    .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?,
            };
            Some(crate::harness::decisions::entropy::decoded(
                session.harness.adapter.entropy_ticket(&draw, &context)?,
                &draw,
                &context,
            )?)
        }
        None => None,
    };
    let request = campaign_core::SelectionRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "stale-rival")?,
        campaign_id: stale.admission.campaign_id,
        state_id: stale.state_id,
        profile,
        entropy,
        expected_generation: refreshed.generation,
        expected_head_id: refreshed.head_id,
    };
    let plan = campaign_core::plan_next(stale, request, &refreshed)?;
    let campaign_core::NextDecision::Select(rival) = plan.decision else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "the stale-state planner selects",
        });
    };
    super::super::common::require(
        rival.ordinal == winner.ordinal && rival.selection_id != winner.selection_id,
        "the stale-state rival reuses the winner's ordinal under the refreshed fence",
    )?;
    debug_assert_eq!(rival.ordinal, winner.ordinal);
    debug_assert_ne!(rival.selection_id, winner.selection_id);
    Ok(*rival)
}
