//! Forks of the history before a held head presented through the holder, and the post-stop
//! settlements a stopped holder advances over.

/// Forbidden effects of loading, and of using a cache of, a fork of the history before a held
/// head.
const FORKED: super::super::super::shapes::Names = (
    "a branch moved onto a fork of the history before the held head was loaded as a \
     fast-forward",
    "a cache of a fork of the history before the held head was used as current state",
);

/// Forbidden effects of loading, and of using a cache of, a settlement published over a stopped
/// held head after its stop.
const ADVANCED: super::super::super::shapes::Names = (
    "a settlement published after the held stop was answered as another projection",
    "a cache of a settlement published after the held stop stood for another state than \
     Campaign's",
);

/// Whether `forks` meet every head kind at its deepest depth.
pub(super) fn meets_every_head(forks: &[super::super::Presented]) -> bool {
    super::super::super::super::lineages::forks::heads::HEAD_KINDS
        .iter()
        .all(|head| {
            forks.iter().any(|fork| {
                matches!(
                    fork.shape,
                    super::super::ShapeKind::Fork {
                        head: forked,
                        is_deepest: true,
                        ..
                    } if forked == *head
                )
            })
        })
}

/// Head kinds of the deepest fork through a holder whose kind an earlier holder already met with
/// every head kind at the deepest depth: the stop, the one head that reaches the maximal depth.
const DEEPEST_STOP: [super::super::super::super::lineages::forks::heads::HeadKind; 1] =
    [super::super::super::super::lineages::forks::heads::HeadKind::Stop];

/// Forks of the history before the held head (`base`) that drop the held head, one per presented
/// depth (zero to three and the deepest the base admits) and head kind the fork state admits; at
/// the deepest depth every head kind until a holder of the same kind met them all (and over a
/// stopped base, whose forks are shallow), otherwise the stop-headed fork.
/// When `base` is stopped, each fork is a settlement published after the stop and is also
/// presented through the stopped holder before this one, which it fast-forwards.
///
/// # Errors
///
/// Returns the error of a construction.
pub(super) fn dropping_head(
    session: &mut crate::harness::Session<'_, '_>,
    (position, holder): (usize, &super::super::Holder),
    (base, is_every_head): (&super::super::super::super::lineages::forks::Base, bool),
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<super::super::Presented>> {
    let dropped_event = super::super::super::shapes::forked::head_event(&holder.history)?;
    let depths = super::super::super::super::lineages::forks::extent::depths(session, base)?;
    let deepest_heads: &[super::super::super::super::lineages::forks::heads::HeadKind] =
        if is_every_head || base.state.stopped.is_some() {
            &super::super::super::super::lineages::forks::heads::HEAD_KINDS
        } else {
            &DEEPEST_STOP
        };
    let forks = super::super::super::super::lineages::forks::dropping(
        session,
        base,
        (&holder.history.branch, &dropped_event),
        (&depths, deepest_heads),
    )?;
    debug_assert_eq!(
        base.history.payloads.len().saturating_add(1),
        holder.history.payloads.len()
    );
    let stopped_before = base
        .state
        .stopped
        .is_some()
        .then(|| position.checked_sub(1))
        .flatten();
    let mut presented = alloc::vec::Vec::with_capacity(forks.len().saturating_mul(2));
    for fork in forks {
        if let Some(stopped) = stopped_before {
            let mut advance = super::presentation(
                session,
                (stopped, super::super::ShapeKind::Advance),
                fork.history.clone(),
                ADVANCED,
                super::super::super::Answer::Loads,
            )?;
            advance.entry.replayed = Some(fork.state.clone());
            presented.push(advance);
        }
        let shape = super::super::ShapeKind::Fork {
            depth: fork.depth,
            is_deepest: fork.is_deepest,
            head: fork.head,
        };
        let mut forked = super::presentation(
            session,
            (position, shape),
            fork.history,
            FORKED,
            super::super::super::shapes::NOT_FORWARD,
        )?;
        forked.entry.replayed = Some(fork.state);
        presented.push(forked);
    }
    Ok(presented)
}
