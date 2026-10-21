//! Catalog entries around the session's published selection: the honest history, the earlier
//! branch value, and forks of the held heads.

pub(in crate::catalog::evidence) mod forked;

/// Non-fast-forward refusal of a history that does not extend the held head.
pub(in crate::catalog::evidence) const NOT_FORWARD: super::Answer =
    super::Answer::Refused(campaign_choregraph::AdapterError::NonFastForwardBranch);

/// Forbidden effects of loading a history and of using its cache.
pub(in crate::catalog::evidence) type Names = (&'static str, &'static str);

/// One catalog entry under the history's own binding.
pub(in crate::catalog::evidence) fn own(
    session: &crate::harness::Session<'_, '_>,
    history: crate::harness::oracle::HistoryView,
    names: Names,
    answers: super::Answers,
) -> crate::verdicts::ConformanceResult<super::Entry> {
    let binding = campaign_choregraph::derive_projection_binding(
        &history.graph,
        &history.branch,
        session.identities.projection_schema_id,
        session.identities.adapter_id,
        session.identities.control_domain_id,
    )?;
    Ok(bound(history, binding, names, answers))
}

/// One catalog entry under a caller-chosen binding.
pub(super) const fn bound(
    history: crate::harness::oracle::HistoryView,
    binding: campaign_core::ProjectionBinding,
    names: Names,
    answers: super::Answers,
) -> super::Entry {
    let (loaded, cached) = names;
    super::Entry {
        history,
        binding,
        maximum_events: crate::harness::HISTORY_MAX_EVENTS,
        loaded,
        cached,
        answers,
        holders: &super::HOLDERS,
        replayed: None,
    }
}

/// Answers of a history Campaign loads cold and through H0 but refuses through H1 as not a
/// fast-forward of the current head.
pub(super) const fn behind_current() -> super::Answers {
    super::Answers {
        cold: super::Answer::Loads,
        earlier: super::Answer::Loads,
        current: NOT_FORWARD,
    }
}

/// The honest history and its earlier branch value (H0's head at H0's generation, a rewind of
/// the current head).
///
/// # Errors
///
/// Returns the error of a construction.
pub(super) fn branch_values(
    session: &crate::harness::Session<'_, '_>,
    earlier: &super::super::lineages::forks::Earlier,
    honest: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::Entry>> {
    let rewound = crate::harness::oracle::HistoryView {
        graph: honest.graph.clone(),
        ..earlier.head.history.clone()
    };
    debug_assert_ne!(rewound.branch.head, honest.branch.head);
    Ok(alloc::vec![
        own(
            session,
            honest.clone(),
            (
                "an honest history was loaded as another projection",
                "an honest cache stood for another state than Campaign's",
            ),
            super::Answers::alike(&super::Answer::Loads),
        )?,
        own(
            session,
            rewound,
            (
                "a rewound control branch was loaded as current history",
                "a cache of a rewound control branch was used as current state",
            ),
            behind_current(),
        )?,
    ])
}
