//! The shapes presented through one selected state, each with Campaign's declared answer through
//! the state's projection.

/// One presented shape with Campaign's answer through the selected state.
pub(super) type Shape = (
    super::super::super::histories::Entry,
    super::super::super::histories::Answer,
);

/// Forbidden effects of loading, and of using a cache of, the fast-forward of a selected state.
const FAST_FORWARD: super::super::super::histories::shapes::Names = (
    "the next event of a held state was answered as another projection",
    "a cache of the next event of a held state stood for another state than Campaign's",
);

/// Forbidden effects of loading, and of using a cache of, a rewind behind a selected state.
const REWOUND: super::super::super::histories::shapes::Names = (
    "a branch rewound behind a held state was loaded as current history",
    "a cache of a branch rewound behind a held state was used as current state",
);

/// Forbidden effects of loading, and of using a cache of, a selected state's lineage on a graph
/// older than its projection's.
const STALE_GRAPH: super::super::super::histories::shapes::Names = (
    "a held state's lineage on an older graph was loaded as current",
    "a cache of a held state's lineage on an older graph was used as current state",
);

/// Forbidden effects of loading, and of using a cache of, another campaign's root through a
/// held admission.
const FOREIGN_ROOT: super::super::super::histories::shapes::Names = (
    "another campaign's control root was loaded through a held admission",
    "a cache of another campaign's control root was used through a held admission",
);

/// Every shape through the state `(history, view)` the session holds (of `class`), with
/// Campaign's answer through it and, for a fork, its fork values (`forks::dropping`, which takes
/// the forks `pairs` still lacks); `captured` is its lineage on the graph before the later moment.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction.
// r[impl campaign.conformance.bounds]
// r[impl campaign.conformance.state_coverage]
pub(super) fn every(
    session: &mut crate::harness::Session<'_, '_>,
    (replayed, class, pairs): (
        &super::super::recipes::replay::Replayed,
        super::super::features::StateClass,
        &super::super::forked::Pairs,
    ),
    (history, view): (
        &crate::harness::oracle::HistoryView,
        &campaign_choregraph::ValidatedControlProjection,
    ),
    captured: crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::forks::Forked>> {
    let refused = super::super::super::histories::Answer::Refused(
        campaign_choregraph::AdapterError::StaleGraph,
    );
    let mut shapes = alloc::vec![(
        super::super::super::histories::shapes::own(
            session,
            captured,
            STALE_GRAPH,
            super::super::super::histories::Answers::alike(&refused)
        )?,
        refused,
    )];
    if let Some(advanced) = fast_forward(session, (history, view))? {
        shapes.push(advanced);
    }
    debug_assert_eq!(
        replayed.prefixes.len().saturating_add(1),
        history.payloads.len(),
        "the replay captured the state before every publication of the held lineage"
    );
    shapes.extend(rewinds(session, history, &replayed.prefixes)?);
    debug_assert!(
        shapes.len() > replayed.prefixes.len(),
        "the stale graph and every rewind are presented"
    );
    let Some(base) = replayed.before.as_ref() else {
        shapes.push(foreign_root(session, history)?);
        return Ok(shapes.into_iter().map(|shape| (shape, 0)).collect());
    };
    let mut every: alloc::vec::Vec<super::forks::Forked> =
        shapes.into_iter().map(|shape| (shape, 0)).collect();
    every.extend(super::forks::of_head_kinds(session, base, history)?);
    let tip = super::super::super::histories::classes::Tip {
        view,
        history,
        base,
        unstopped: &replayed.unstopped,
    };
    for (_, entry) in super::super::super::histories::classes::entries(session, &tip)? {
        let answer = entry.answers.current.clone();
        every.push(((entry, answer), 0));
    }
    let chosen = every.iter().fold(0, |chosen, (_, values)| chosen | values);
    every.extend(super::forks::greedy(
        session,
        base,
        (history, class),
        (pairs, chosen),
    )?);
    Ok(every)
}

/// The next event a writer publishes over the held state, when one exists: the consumer stop of
/// a running state, the closure of the latest outstanding selection of a stopped one.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of the construction.
fn fast_forward(
    session: &mut crate::harness::Session<'_, '_>,
    (history, view): (
        &crate::harness::oracle::HistoryView,
        &campaign_choregraph::ValidatedControlProjection,
    ),
) -> crate::verdicts::ConformanceResult<Option<Shape>> {
    let state = view.state();
    debug_assert!(
        u64::try_from(history.payloads.len()).ok() == Some(state.control_event_count),
        "the held projection counts every event of the held lineage"
    );
    let event = if state.stopped.is_none() {
        super::super::super::lineages::forks::heads::event(
            session,
            state,
            (history, view),
            0,
            super::super::super::lineages::forks::heads::HeadKind::Stop,
        )?
    } else {
        super::super::super::lineages::forks::heads::settlement(
            session,
            (history, state),
            super::super::super::lineages::forks::heads::HeadKind::Closure,
        )?
    };
    let Some(event) = event else {
        return Ok(None);
    };
    let published =
        super::super::super::lineages::forks::publish_over(session, history, view, event)?;
    debug_assert_eq!(
        published.history.payloads.len(),
        history.payloads.len().saturating_add(1),
        "a fast-forward extends the held lineage by one event"
    );
    let mut entry = super::super::super::histories::shapes::own(
        session,
        published.history,
        FAST_FORWARD,
        super::super::super::histories::Answers::alike(
            &super::super::super::histories::Answer::Loads,
        ),
    )?;
    entry.replayed = Some(published.view.state().clone());
    Ok(Some((entry, super::super::super::histories::Answer::Loads)))
}

/// Every rewind of the held state to a shorter prefix of its lineage (on its graph, with the
/// branch value each prefix was published at), each with the state it replays to (`prefixes`).
///
/// # Errors
///
/// Returns the adapter error of a binding.
fn rewinds(
    session: &crate::harness::Session<'_, '_>,
    history: &crate::harness::oracle::HistoryView,
    prefixes: &[campaign_core::FrontierState],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Shape>> {
    let shorter = history.payloads.len().saturating_sub(1);
    let mut rewound = alloc::vec::Vec::with_capacity(shorter);
    let no_choices = campaign_choregraph::ParentChoices::new();
    for ((length, transition), state) in (1..=shorter).zip(&session.transitions).zip(prefixes) {
        let prefix = crate::harness::oracle::HistoryView {
            graph: history.graph.clone(),
            branch: transition.branch_after.clone(),
            parent_choices: no_choices.clone(),
            payloads: history
                .payloads
                .get(..length)
                .map(<[_]>::to_vec)
                .unwrap_or_default(),
        };
        let answer = super::super::super::histories::shapes::NOT_FORWARD;
        let mut entry = super::super::super::histories::shapes::own(
            session,
            prefix,
            REWOUND,
            super::super::super::histories::Answers::alike(&answer),
        )?;
        entry.replayed = Some(state.clone());
        rewound.push((entry, answer));
    }
    debug_assert_eq!(rewound.len(), shorter);
    Ok(rewound)
}

/// Another campaign's control root written next to the held admission under the same envelope,
/// with the branch moved onto it at the admission's generation.
///
/// # Errors
///
/// Returns the error of the construction.
fn foreign_root(
    session: &crate::harness::Session<'_, '_>,
    history: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<Shape> {
    debug_assert_eq!(
        history.payloads.len(),
        1,
        "another campaign's root meets a bare admission"
    );
    let (graph, second) = super::super::super::lineages::envelopes::second_root(session, history)?;
    debug_assert!(
        second.history_event_id != history.branch.head,
        "the other root is not the held head"
    );
    let root = crate::harness::oracle::HistoryView {
        graph,
        branch: campaign_choregraph::BranchRef {
            head: second.history_event_id,
            ..history.branch.clone()
        },
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads: alloc::vec![second],
    };
    let answer = super::super::super::histories::shapes::NOT_FORWARD;
    Ok((
        super::super::super::histories::shapes::own(
            session,
            root,
            FOREIGN_ROOT,
            super::super::super::histories::Answers::alike(&answer),
        )?,
        answer,
    ))
}
