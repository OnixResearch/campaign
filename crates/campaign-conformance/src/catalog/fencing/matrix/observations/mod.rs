//! Gate questions about the observations presented with a publication: missing, partial, or
//! crossed evidence; every single-field mutation of the publication's own durable evidence and
//! every self-consistent move of it onto a mirror branch or shifted generations; every real
//! branch transition the session committed, presented for a publication it does not belong to;
//! and non-selection publications presented with their own committed evidence.

mod mutations;

/// One publication the gate is asked about: its decision, its planned publication, and the
/// forbidden effect of executing it under another publication's transition.
struct Presented<'p> {
    selection: &'p campaign_core::SelectionDecision,
    publication: &'p campaign_choregraph::ControlPublication,
    forbidden: &'static str,
}

/// Presents every committed session transition that is not `presented`'s own as its evidence.
fn foreign_transitions(
    session: &mut crate::harness::Session<'_, '_>,
    presented: &Presented<'_>,
) -> crate::verdicts::ConformanceResult<usize> {
    let own = crate::harness::decisions::durable_evidence(
        presented.publication,
        crate::harness::decisions::Durability::COMMITTED,
    );
    let foreign: alloc::vec::Vec<campaign_choregraph::DurablePublicationEvidence> = session
        .transitions
        .iter()
        .filter(|transition| **transition != own)
        .cloned()
        .collect();
    for transition in &foreign {
        super::refuse(
            session,
            presented.selection,
            Some(presented.publication),
            Some(transition),
            presented.forbidden,
        )?;
    }
    debug_assert!(foreign.iter().all(|transition| *transition != own));
    Ok(foreign.len())
}

/// A compare-and-set loser: after other control work (a guidance improvement, a selection that is
/// never published, and a candidate batch), a selection planned and publication planned from the
/// current branch value, after which a concurrent writer commits a stop from the same value
/// first. The published selection faces every single-field mutation of its own durable evidence
/// (other events: the head before it, the loser's event, and its own event where it is not the
/// value) and every self-consistent move of it (onto a mirror branch, to shifted generations),
/// which Campaign refuses as mismatched evidence; the published selection and the loser each face
/// every real transition the session committed that is not their own, among them the winner's
/// self-consistent transition from the loser's branch value; and the published selection, the
/// loser, and the unpublished selection each face the guidance, batch, and stop publications with
/// their own committed evidence.
///
/// # Errors
///
/// Returns the oracle mismatch of a gate answer and the typed error of the scenario.
pub(super) fn evidence_questions(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<()> {
    let other_work = super::non_selection::publish(session, draw)?;
    let loser_selection = session.select(draw)?;
    let loser = session.plan_publication(session.selection_event(&loser_selection)?)?;
    let reason = session.consumer_stop()?;
    let stop = session.stop(reason)?;
    let winner = crate::harness::decisions::durable_evidence(
        &stop,
        crate::harness::decisions::Durability::COMMITTED,
    );
    super::super::super::common::require(
        session.transitions.last() == Some(&winner)
            && winner.branch_before == loser.branch_plan().before
            && winner.history_event_id == winner.branch_after.head
            && winner.history_event_id != loser.history_event_id()
            && session.state.outstanding.len() == 1,
        "a concurrent writer committed another transition from the loser's branch value",
    )?;
    debug_assert_ne!(winner.branch_after, loser.branch_plan().after);
    let events = [
        evidence.branch_before.head,
        evidence.history_event_id,
        loser.history_event_id(),
    ];
    for (forbidden, mutated) in mutations::mutated_evidence(evidence, &events)? {
        super::refuse(
            session,
            selection,
            Some(publication),
            Some(&mutated),
            forbidden,
        )?;
    }
    for (forbidden, moved) in mutations::moved_evidence(evidence)? {
        super::super::super::common::require(
            campaign_choregraph::can_execute_selection_after_publication(
                selection,
                publication,
                &moved,
            ) == Err(campaign_choregraph::AdapterError::PublicationEvidenceMismatch),
            "Campaign refuses a self-consistent move of the published transition as mismatched \
             evidence",
        )?;
        super::refuse(
            session,
            selection,
            Some(publication),
            Some(&moved),
            forbidden,
        )?;
    }
    for presented in [
        Presented {
            selection,
            publication,
            forbidden: "a published selection executed under another publication's transition",
        },
        Presented {
            selection: &loser_selection,
            publication: &loser,
            forbidden: "a compare-and-set loser executed under a committed transition",
        },
    ] {
        foreign_transitions(session, &presented)?;
    }
    other_work
        .with_stop(stop)
        .ask(session, &[selection, &loser_selection])?;
    Ok(())
}

/// Once the shell loaded the history carrying the publication, the gate questions without a
/// publication, without evidence, or without either are asked again: a loaded publication is no
/// substitute for the presented publication and its durable evidence.
pub(super) fn loaded_questions(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
) -> crate::verdicts::ConformanceResult<()> {
    let loaded = session.load()?;
    super::super::super::common::require(
        loaded
            .control_events()
            .last()
            .is_some_and(|event| event == publication.control_event()),
        "the loaded history carries the publication at its head",
    )?;
    debug_assert!(!loaded.control_events().is_empty());
    for (presented_publication, presented_evidence, forbidden) in [
        (
            None,
            None,
            "a loaded published selection executed without a publication or evidence",
        ),
        (
            Some(publication),
            None,
            "a loaded published selection executed without durability evidence",
        ),
        (
            None,
            Some(evidence),
            "durability evidence executed a loaded selection without its publication",
        ),
    ] {
        super::refuse(
            session,
            selection,
            presented_publication,
            presented_evidence,
            forbidden,
        )?;
    }
    Ok(())
}

/// Gate questions about the published selection itself with missing, partial, or mismatched
/// observations.
pub(super) fn published_questions(
    session: &mut crate::harness::Session<'_, '_>,
    selection: &campaign_core::SelectionDecision,
    publication: &campaign_choregraph::ControlPublication,
    evidence: &campaign_choregraph::DurablePublicationEvidence,
) -> crate::verdicts::ConformanceResult<()> {
    super::refuse(
        session,
        selection,
        Some(publication),
        None,
        "a published selection executed without durability evidence",
    )?;
    super::refuse(
        session,
        selection,
        None,
        Some(evidence),
        "durability evidence executed a selection without its publication",
    )?;
    let pending = crate::harness::decisions::durable_evidence(
        publication,
        crate::harness::decisions::Durability::PENDING,
    );
    super::refuse(
        session,
        selection,
        Some(publication),
        Some(&pending),
        "a selection executed without durable publication",
    )?;
    let mut crossed = evidence.clone();
    crossed.branch_before = session.branch.clone();
    super::super::super::common::require(
        crossed.branch_before != publication.branch_plan().before,
        "the crossed evidence names another branch transition",
    )?;
    debug_assert_ne!(crossed.branch_before, publication.branch_plan().before);
    debug_assert!(evidence.durable_event && evidence.durable_branch_update);
    super::refuse(
        session,
        selection,
        Some(publication),
        Some(&crossed),
        "evidence of another branch transition executed the selection",
    )
}
