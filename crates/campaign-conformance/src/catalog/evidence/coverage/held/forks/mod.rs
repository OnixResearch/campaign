//! Forks of the history before a selected state's head presented through the state: the
//! catalog's six head kinds at depths zero and one, then, greedily, forks of every (depth, head
//! sub-kind) shape whose depth value (zero to three, or the deepest the bounds admit for its head)
//! or head sub-kind no presented state carrying one of this state's 3-way combinations presented
//! yet. The greedy takes the shape of largest gain first (ties in declared sub-kind order, depth
//! one, two, three, the deepest, then zero), so the choice depends only on the declared classes
//! and Campaign's states, never on adapter identities.

/// Forbidden effects of loading, and of using a cache of, a fork of the history before a
/// selected state's head.
const FORKED: super::super::super::histories::shapes::Names = (
    "a fork of the history before a held state's head was loaded as a fast-forward",
    "a cache of a fork of the history before a held state's head was used as current state",
);

/// Depths the catalog forks present at (as far as the bounds admit).
const CATALOG_DEPTHS: [usize; 2] = [0, 1];

/// Named depths the greedy tries a sub-kind at, in order, before the deepest.
const GREEDY_DEPTHS: [usize; 3] = [1, 2, 3];

/// The fork sub-kinds.
type SubKind = super::super::super::lineages::forks::subkinds::SubKind;

/// One presented fork with Campaign's answer through the state and its fork values.
pub(super) type Forked = (super::shapes::Shape, u32);

/// The presented shape of one fork.
///
/// # Errors
///
/// Returns the error of the entry's construction.
fn shape<H>(
    session: &crate::harness::Session<'_, '_>,
    fork: super::super::super::lineages::forks::Fork<H>,
) -> crate::verdicts::ConformanceResult<Forked> {
    let values = super::super::forked::of_fork(fork.depth, fork.is_deepest, &fork.history);
    let answer = super::super::super::histories::shapes::NOT_FORWARD;
    let mut entry = super::super::super::histories::shapes::own(
        session,
        fork.history,
        FORKED,
        super::super::super::histories::Answers::alike(&answer),
    )?;
    entry.replayed = Some(fork.state);
    Ok(((entry, answer), values))
}

/// The catalog's forks over `base` (the history before the head of `history`) that drop its head:
/// the six head kinds at depths zero and one, as far as the bounds admit.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction, and `Diverged` when no
/// fork fits the bounds.
// r[impl campaign.conformance.bounds]
pub(super) fn of_head_kinds(
    session: &mut crate::harness::Session<'_, '_>,
    base: &super::super::super::lineages::forks::Base,
    history: &crate::harness::oracle::HistoryView,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Forked>> {
    debug_assert!(
        base.history.payloads.len() < history.payloads.len(),
        "the base precedes the held head"
    );
    let dropped = super::super::super::histories::shapes::forked::head_event(history)?;
    let maximal = super::super::super::lineages::forks::extent::maximal_depth(session, base)?;
    let depths: alloc::vec::Vec<usize> = CATALOG_DEPTHS
        .into_iter()
        .filter(|depth| *depth <= maximal)
        .collect();
    let forks = super::super::super::lineages::forks::dropping(
        session,
        base,
        (&history.branch, &dropped),
        (
            &depths,
            &super::super::super::lineages::forks::heads::HEAD_KINDS,
        ),
    )?;
    let mut presented = alloc::vec::Vec::with_capacity(forks.len());
    for fork in forks {
        presented.push(shape(session, fork)?);
    }
    super::super::super::super::common::require(
        !presented.is_empty(),
        "the history before a held head admits a fork",
    )?;
    Ok(presented)
}

/// The greedy forks over `base` (the history before the head of `history`, the selected state of
/// `class`) that drop its head, beyond the fork values `chosen` for the state already: the forks
/// of the (depth, sub-kind) shapes `pairs` still lacks.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction.
// r[impl campaign.conformance.state_coverage]
// r[impl campaign.conformance.bounds]
pub(super) fn greedy(
    session: &mut crate::harness::Session<'_, '_>,
    base: &super::super::super::lineages::forks::Base,
    (history, class): (
        &crate::harness::oracle::HistoryView,
        super::super::features::StateClass,
    ),
    (pairs, chosen): (&super::super::forked::Pairs, u32),
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<Forked>> {
    let bits = super::super::combos::triple_bits(class);
    let gains = pairs.gains(&bits, chosen);
    if super::super::forked::Pairs::gain_of(&gains, u32::MAX) == 0 {
        return Ok(alloc::vec::Vec::new());
    }
    let is_deepest_wanted =
        super::super::forked::Pairs::gain_of(&gains, super::super::forked::Pairs::deepest()) > 0;
    let dropped = super::super::super::histories::shapes::forked::head_event(history)?;
    let built = shaped(
        session,
        (base, (&history.branch, &dropped)),
        (&bits, pairs, chosen),
        is_deepest_wanted,
    )?;
    let mut presented = alloc::vec::Vec::with_capacity(built.len());
    for fork in built {
        presented.push(shape(session, fork)?);
    }
    debug_assert!(presented.iter().all(|(_, values)| *values != 0));
    Ok(presented)
}

/// The forks of the greedy shapes through one state: repeatedly the untried (depth, sub-kind)
/// shape whose values add the most (value, 3-way combination) pairs, over the state's 3-way
/// combinations `bits`, to `pairs` and to the values `chosen` for the state so far, until no
/// shape adds one; the deepest depths only when `is_deepest_wanted`.
///
/// # Errors
///
/// Returns the adapter, Campaign, or Choregraph error of a construction.
fn shaped(
    session: &mut crate::harness::Session<'_, '_>,
    (base, dropped): (
        &super::super::super::lineages::forks::Base,
        (
            &campaign_choregraph::BranchRef,
            &campaign_core::CampaignControlEvent,
        ),
    ),
    (bits, pairs, mut chosen): (&[u32], &super::super::forked::Pairs, u32),
    is_deepest_wanted: bool,
) -> crate::verdicts::ConformanceResult<
    alloc::vec::Vec<super::super::super::lineages::forks::Fork<SubKind>>,
> {
    let maximal = super::super::super::lineages::forks::extent::maximal_depth(session, base)?;
    let is_stopped = base.state.stopped.is_some();
    let mut shapes: alloc::vec::Vec<(usize, SubKind, u32)> = alloc::vec::Vec::with_capacity(
        super::super::super::lineages::forks::subkinds::SUB_KINDS
            .len()
            .saturating_mul(GREEDY_DEPTHS.len().saturating_add(2)),
    );
    for kind in super::super::super::lineages::forks::subkinds::SUB_KINDS {
        let ceiling = maximal.saturating_sub(
            super::super::super::lineages::forks::shaped::Head::reserve(kind, is_stopped),
        );
        let deepest = is_deepest_wanted.then_some(ceiling);
        for depth in GREEDY_DEPTHS.into_iter().chain(deepest).chain([0]) {
            let is_new = !shapes
                .iter()
                .any(|(known, known_kind, _)| *known == depth && *known_kind == kind);
            if depth <= ceiling && is_new {
                let values = super::super::forked::of_depth(depth, depth == ceiling)
                    | super::super::forked::of_sub_kind(kind);
                shapes.push((depth, kind, values));
            }
        }
    }
    let mut depths: alloc::vec::Vec<usize> = shapes.iter().map(|(depth, _, _)| *depth).collect();
    depths.sort_unstable();
    depths.dedup();
    let forking = super::super::super::lineages::forks::shaped::Forking::over(
        session, base, dropped, &depths,
    )?;
    let mut forks = alloc::vec::Vec::with_capacity(shapes.len());
    loop {
        let gains = pairs.gains(bits, chosen);
        let best = shapes
            .iter()
            .enumerate()
            .map(|(index, (_, _, values))| {
                (
                    super::super::forked::Pairs::gain_of(&gains, *values),
                    core::cmp::Reverse(index),
                )
            })
            .max()
            .filter(|(gain, _)| *gain > 0);
        let Some((_, core::cmp::Reverse(index))) = best else {
            break;
        };
        let (depth, kind, _) = shapes.remove(index);
        if let Some(fork) = forking.fork(session, (depth, kind))? {
            chosen |= super::super::forked::of_fork(fork.depth, fork.is_deepest, &fork.history);
            forks.push(fork);
        }
    }
    debug_assert!(forks.iter().all(|fork| fork.depth <= maximal));
    Ok(forks)
}
