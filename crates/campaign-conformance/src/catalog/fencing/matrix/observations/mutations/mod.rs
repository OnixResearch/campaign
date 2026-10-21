//! Mutations of a publication's own durable evidence: every single-field mutation, and the
//! self-consistent two-field mutations that keep the transition's shape (the same branch name
//! before and after, one generation step) while moving it onto a mirror branch or to shifted
//! generations.

/// Branch name no session publishes on.
const MIRROR_BRANCH: &str = "campaign-conformance-mirror";

/// One evidence question: the forbidden effect its grant names and the presented evidence.
pub(super) type Question = (
    &'static str,
    campaign_choregraph::DurablePublicationEvidence,
);

/// Requires every question's evidence to differ from `honest` in a shape `is_shaped` accepts,
/// given which of the nine evidence fields it changes: the event identity, the name, head, and
/// generation of the branch value before and after, and the two durability flags.
///
/// # Errors
///
/// Returns `Diverged` naming `requirement` when some evidence is not shaped.
fn require_shaped(
    questions: &[Question],
    honest: &campaign_choregraph::DurablePublicationEvidence,
    is_shaped: impl Fn([bool; 9], &campaign_choregraph::DurablePublicationEvidence) -> bool,
    requirement: &'static str,
) -> crate::verdicts::ConformanceResult<()> {
    let (honest_before, honest_after) = (&honest.branch_before, &honest.branch_after);
    let differing_fields = |evidence: &campaign_choregraph::DurablePublicationEvidence| {
        let (before, after) = (&evidence.branch_before, &evidence.branch_after);
        [
            evidence.history_event_id != honest.history_event_id,
            before.name != honest_before.name,
            before.head != honest_before.head,
            before.generation != honest_before.generation,
            after.name != honest_after.name,
            after.head != honest_after.head,
            after.generation != honest_after.generation,
            evidence.durable_event != honest.durable_event,
            evidence.durable_branch_update != honest.durable_branch_update,
        ]
    };
    super::super::super::super::common::require(
        questions
            .iter()
            .all(|(_, evidence)| is_shaped(differing_fields(evidence), evidence)),
        requirement,
    )?;
    debug_assert!(questions.iter().all(|(_, evidence)| evidence != honest));
    Ok(())
}

/// `honest` with one edit applied.
fn edited(
    honest: &campaign_choregraph::DurablePublicationEvidence,
    forbidden: &'static str,
    edit: impl FnOnce(&mut campaign_choregraph::DurablePublicationEvidence),
) -> Question {
    let mut evidence = honest.clone();
    edit(&mut evidence);
    (forbidden, evidence)
}

/// The representable generations next to `generation`.
fn adjacent(generation: u64) -> impl Iterator<Item = u64> {
    [generation.checked_sub(1), generation.checked_add(1)]
        .into_iter()
        .flatten()
}

/// Every single-field mutation of `honest`: its event identity replaced by each other real
/// event, the name, head (each other real event), and generation (each adjacent value) of the
/// branch value before and after replaced separately, and each durability flag cleared.
///
/// # Errors
///
/// Returns `Diverged` when a mutation differs from `honest` in other than exactly one field, with
/// the branch values split into name, head, and generation.
pub(super) fn mutated_evidence(
    honest: &campaign_choregraph::DurablePublicationEvidence,
    events: &[campaign_choregraph::EventId],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Question>> {
    let mut questions =
        alloc::vec::Vec::with_capacity(events.len().saturating_mul(3).saturating_add(8));
    for event in events {
        questions.push(edited(
            honest,
            "evidence naming another event executed the published selection",
            |evidence| evidence.history_event_id = *event,
        ));
        questions.push(edited(
            honest,
            "evidence from another head executed the published selection",
            |evidence| evidence.branch_before.head = *event,
        ));
        questions.push(edited(
            honest,
            "evidence onto another head executed the published selection",
            |evidence| evidence.branch_after.head = *event,
        ));
    }
    for generation in adjacent(honest.branch_before.generation) {
        questions.push(edited(
            honest,
            "evidence from another generation executed the published selection",
            |evidence| evidence.branch_before.generation = generation,
        ));
    }
    for generation in adjacent(honest.branch_after.generation) {
        questions.push(edited(
            honest,
            "evidence onto another generation executed the published selection",
            |evidence| evidence.branch_after.generation = generation,
        ));
    }
    questions.extend([
        edited(
            honest,
            "evidence from another branch name executed the published selection",
            |evidence| evidence.branch_before.name = alloc::string::String::from(MIRROR_BRANCH),
        ),
        edited(
            honest,
            "evidence onto another branch name executed the published selection",
            |evidence| evidence.branch_after.name = alloc::string::String::from(MIRROR_BRANCH),
        ),
        edited(
            honest,
            "evidence without a durable event executed the published selection",
            |evidence| evidence.durable_event = false,
        ),
        edited(
            honest,
            "evidence without a durable branch update executed the published selection",
            |evidence| evidence.durable_branch_update = false,
        ),
    ]);
    questions.retain(|(_, evidence)| evidence != honest);
    require_shaped(
        &questions,
        honest,
        |fields, _| fields.iter().filter(|is_different| **is_different).count() == 1,
        "every mutated evidence differs from the publication's own in exactly one field, with \
         the branch values split into name, head, and generation",
    )?;
    debug_assert!(!questions.is_empty());
    Ok(questions)
}

/// Every self-consistent two-field mutation of `honest`: the same transition (its heads and
/// generation step kept) moved with its branch name onto a mirror branch, and its generations
/// before and after shifted together by each representable step of one.
///
/// # Errors
///
/// Returns `Diverged` when a mutation differs from `honest` in other than exactly its two branch
/// names or its two generations, or breaks the transition's shape: one branch name before and
/// after, and the publication's own generation step.
pub(super) fn moved_evidence(
    honest: &campaign_choregraph::DurablePublicationEvidence,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Question>> {
    const NAMES: [bool; 9] = [false, true, false, false, true, false, false, false, false];
    const GENERATIONS: [bool; 9] = [false, false, false, true, false, false, true, false, false];
    let step = |evidence: &campaign_choregraph::DurablePublicationEvidence| {
        evidence
            .branch_after
            .generation
            .checked_sub(evidence.branch_before.generation)
    };
    let (before, after) = (
        honest.branch_before.generation,
        honest.branch_after.generation,
    );
    let shifted = [
        before.checked_sub(1).zip(after.checked_sub(1)),
        before.checked_add(1).zip(after.checked_add(1)),
    ];
    let mut questions = alloc::vec::Vec::with_capacity(3);
    questions.push(edited(
        honest,
        "evidence of the published transition on another branch executed the published selection",
        |evidence| {
            evidence.branch_before.name = alloc::string::String::from(MIRROR_BRANCH);
            evidence.branch_after.name = alloc::string::String::from(MIRROR_BRANCH);
        },
    ));
    for (before, after) in shifted.into_iter().flatten() {
        questions.push(edited(
            honest,
            "evidence of the published transition at shifted generations executed the published \
             selection",
            |evidence| {
                evidence.branch_before.generation = before;
                evidence.branch_after.generation = after;
            },
        ));
    }
    require_shaped(
        &questions,
        honest,
        |fields, evidence| {
            (fields == NAMES || fields == GENERATIONS)
                && evidence.branch_before.name == evidence.branch_after.name
                && step(evidence) == step(honest)
        },
        "every moved evidence differs from the publication's own in exactly its two branch names \
         or its two generations, with one name before and after and the publication's own step",
    )?;
    debug_assert!(questions.len() >= 2);
    debug_assert!(step(honest).is_some());
    Ok(questions)
}
