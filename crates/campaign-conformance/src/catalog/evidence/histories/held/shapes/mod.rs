//! The shapes the held-lineage matrix presents through each held prefix: the honest head, the
//! held lineage on the graph captured before the later drafts the held projection recorded (a
//! graph older than the held one), every rewind to a shorter prefix, forks of the history before the held head at every presented depth
//! and head kind, every forged and suffix catalog class built over the held head, through a
//! stopped prefix the post-stop settlements published over it, and, through the held root,
//! another campaign's root.

mod forks;

/// Forbidden effects of loading, and of using a cache of, the honest head through a prefix.
const FAST_FORWARD: super::super::shapes::Names = (
    "the honest history was loaded through a held prefix as another projection",
    "an honest cache through a held prefix stood for another state than Campaign's",
);

/// Forbidden effects of loading, and of using a cache of, a rewind behind a held head.
const REWOUND: super::super::shapes::Names = (
    "a branch rewound behind the held head was loaded as current history",
    "a cache of a branch rewound behind the held head was used as current state",
);

/// Forbidden effects of loading, and of using a cache of, another campaign's root through the
/// held root.
const FOREIGN_ROOT: super::super::shapes::Names = (
    "another campaign's control root was loaded through the held root",
    "a cache of another campaign's control root was used through the held root",
);

/// Forbidden effects of loading, and of using a cache of, the held lineage on a graph captured
/// before the later drafts the held projection recorded.
const STALE_GRAPH: super::super::shapes::Names = (
    "the held lineage on a graph older than the held projection's was loaded as current",
    "a cache of the held lineage on a graph older than the held projection's was used as current \
     state",
);

/// Every presentation of the matrix, holder by holder.
///
/// # Errors
///
/// Returns the error of a construction.
// r[impl campaign.conformance.bounds]
// r[impl campaign.conformance.negative]
// r[impl campaign.history_projection.stale]
// r[impl campaign.cache.invalid]
pub(super) fn presented(
    session: &mut crate::harness::Session<'_, '_>,
    holders: &[super::Holder],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::Presented>> {
    let honest = session.history();
    let mut presented = alloc::vec::Vec::with_capacity(holders.len().saturating_mul(PER_HOLDER));
    let mut unstopped = session.state.clone();
    let mut met_every_head = alloc::vec::Vec::with_capacity(super::HOLDER_KINDS.len());
    for (position, holder) in holders.iter().enumerate() {
        if holder.view.state().stopped.is_none() {
            unstopped = holder.view.state().clone();
        }
        presented.push(presentation(
            session,
            (position, super::ShapeKind::FastForward),
            honest.clone(),
            FAST_FORWARD,
            super::super::Answer::Loads,
        )?);
        let later = holders
            .get(position.saturating_add(1))
            .and_then(|next| next.base.as_ref());
        if let Some(earlier_graph) = later {
            presented.push(presentation(
                session,
                (position, super::ShapeKind::StaleGraph),
                earlier_graph.history.clone(),
                STALE_GRAPH,
                super::super::Answer::Refused(campaign_choregraph::AdapterError::StaleGraph),
            )?);
        }
        let Some(base) = holder.base.as_ref() else {
            let root = foreign_root(session, holder)?;
            presented.push(presentation(
                session,
                (position, super::ShapeKind::ForeignRoot),
                root,
                FOREIGN_ROOT,
                super::super::shapes::NOT_FORWARD,
            )?);
            continue;
        };
        presented.extend(rewinds(session, position, holders)?);
        let is_every_head = !met_every_head.contains(&holder.kind);
        let forked = forks::dropping_head(session, (position, holder), (base, is_every_head))?;
        if forks::meets_every_head(&forked) {
            met_every_head.push(holder.kind);
        }
        presented.extend(forked);
        let planned = later.map_or(&holder.view, |next| &next.view);
        let tip = super::super::classes::Tip {
            view: planned,
            history: &holder.history,
            base,
            unstopped: &unstopped,
        };
        presented.extend(classes(session, position, &tip)?);
    }
    debug_assert!(presented.len() >= holders.len());
    Ok(presented)
}

/// Every rewind of the holder at `position` to a shorter held prefix.
///
/// # Errors
///
/// Returns the adapter error of a binding.
fn rewinds(
    session: &crate::harness::Session<'_, '_>,
    position: usize,
    holders: &[super::Holder],
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::Presented>> {
    let mut rewound = alloc::vec::Vec::with_capacity(position);
    for depth in 1..=position {
        let Some(earlier) = position.checked_sub(depth).and_then(|at| holders.get(at)) else {
            continue;
        };
        rewound.push(presentation(
            session,
            (position, super::ShapeKind::Rewind(depth)),
            earlier.history.clone(),
            REWOUND,
            super::super::shapes::NOT_FORWARD,
        )?);
    }
    debug_assert_eq!(rewound.len(), position);
    debug_assert!(holders.len() > position);
    Ok(rewound)
}

/// Every forged and suffix catalog class built over the holder at `position` (`tip`), each
/// answered through the holder as Campaign answers it cold.
///
/// # Errors
///
/// Returns the error of a construction.
fn classes(
    session: &mut crate::harness::Session<'_, '_>,
    position: usize,
    tip: &super::super::classes::Tip<'_>,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::Presented>> {
    let built = super::super::classes::entries(session, tip)?;
    let mut presented = alloc::vec::Vec::with_capacity(built.len());
    for (class, entry) in built {
        presented.push(super::Presented {
            holder: position,
            shape: super::ShapeKind::Class(class),
            answer: entry.answers.current.clone(),
            entry,
        });
    }
    debug_assert!(!presented.is_empty());
    debug_assert!(presented.iter().all(|class| class.holder == position));
    Ok(presented)
}

/// Presentations the matrix builds per holder beyond its rewinds, as a capacity hint: the honest
/// head, forks of every presented depth and head kind, and every catalog class.
const PER_HOLDER: usize = 64;

/// One presentation of `history` under its own binding through the holder at `position`.
///
/// # Errors
///
/// Returns the adapter error of the binding.
pub(super) fn presentation(
    session: &crate::harness::Session<'_, '_>,
    (holder, shape): (usize, super::ShapeKind),
    history: crate::harness::oracle::HistoryView,
    names: super::super::shapes::Names,
    answer: super::super::Answer,
) -> crate::verdicts::ConformanceResult<super::Presented> {
    let entry = super::super::shapes::own(
        session,
        history,
        names,
        super::super::Answers::alike(&answer),
    )?;
    Ok(super::Presented {
        holder,
        shape,
        entry,
        answer,
    })
}

/// Another campaign's control root written next to the held root under the same envelope, with
/// the branch moved onto it at the root's generation.
///
/// # Errors
///
/// Returns the error of the construction.
fn foreign_root(
    session: &crate::harness::Session<'_, '_>,
    root: &super::Holder,
) -> crate::verdicts::ConformanceResult<crate::harness::oracle::HistoryView> {
    let (graph, second) =
        super::super::super::lineages::envelopes::second_root(session, &root.history)?;
    let branch = campaign_choregraph::BranchRef {
        head: second.history_event_id,
        ..root.history.branch.clone()
    };
    debug_assert_ne!(branch.head, root.history.branch.head);
    Ok(crate::harness::oracle::HistoryView {
        graph,
        branch,
        parent_choices: campaign_choregraph::ParentChoices::new(),
        payloads: alloc::vec![second],
    })
}
