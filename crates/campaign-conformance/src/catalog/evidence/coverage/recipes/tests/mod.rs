//! Every declared class is reached by its canonical recipe, under the default and the reordered
//! profile, and the recipe's lineage and plans hold exactly the receipt kinds the class is
//! compatible with.

/// Receipt kinds of a replayed lineage and of the plans it made.
fn replayed_kinds(
    session: &crate::harness::Session<'_, '_>,
    replayed: &super::replay::Replayed,
) -> u32 {
    let events = session.payloads.iter().fold(0, |kinds, payload| {
        kinds | super::super::kinds::of_event(&payload.control_event)
    });
    replayed
        .plans
        .iter()
        .fold(events, |kinds, (kind, _)| kinds | kind)
}

/// Replays every declared class under `profile`; returns the longest lineage (control events).
fn replay_every_class(profile: &crate::inputs::SuiteProfile) -> usize {
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let classes = super::super::classes::all();
    let bound = usize::try_from(crate::harness::HISTORY_MAX_EVENTS).unwrap_or(0);
    let mut longest = 0;
    let mut failures = alloc::vec::Vec::with_capacity(classes.len());
    for class in classes {
        let mut harness = crate::harness::Driver {
            adapter: &mut adapter,
            profile,
            transcript: &mut transcript,
        };
        let (session, replayed) = match super::replay::build(&mut harness, class) {
            Ok(replayed) => replayed,
            Err(error) => {
                failures.push((class, alloc::format!("{error:?}")));
                continue;
            }
        };
        let expected = super::super::kinds::compatible(class)
            & !(super::super::kinds::STATE_RANKED_PLAN
                | super::super::kinds::STATE_EXPLORATORY_PLAN
                | super::super::kinds::REFUSAL);
        let reached = replayed_kinds(&session, &replayed);
        if reached != expected {
            failures.push((class, alloc::format!("kinds {reached:#x} != {expected:#x}")));
        }
        assert!(
            session.payloads.len() <= bound,
            "the recipe of {class:?} fits the history bound"
        );
        longest = longest.max(session.payloads.len());
    }
    assert!(
        failures.is_empty(),
        "{} failures: {failures:#?}",
        failures.len()
    );
    longest
}

#[test]
// r[verify campaign.conformance.state_coverage]
fn every_declared_class_is_reached_by_its_recipe() -> crate::verdicts::ConformanceResult<()> {
    let default = crate::tests::default_profile()?;
    let mut reordered = default.clone();
    reordered.exploration_order = alloc::vec![1];
    reordered.retention_rank_order = alloc::vec![1];
    reordered.validate()?;
    let longest = replay_every_class(&default).max(replay_every_class(&reordered));
    assert!(longest <= usize::try_from(crate::harness::HISTORY_MAX_EVENTS).unwrap_or(0));
    Ok(())
}
