//! The deduplication key of the bounded reachability search: a frontier state with its
//! history-derived identities erased.
//!
//! Two states of one campaign with equal keys admit the same control events up to those
//! identities: every Campaign decision (intake, eligibility, ranking and its identity tie-break,
//! budgets, bounds, pruning, and stops) reads only the fields the key keeps, and candidate and
//! guidance identities are content-derived, so they are equal on every path. Event, selection,
//! result, closure, discovery, and invalidation identities depend on the path (they bind the
//! state before their event) and are dropped; only their counts are kept.

/// One candidate: identity, status (0 retained, 1 retired, 2 pruned), selections, and latest
/// guidance (generation and ranks).
type CandidateKey = (
    campaign_core::Id,
    u8,
    u64,
    Option<(u64, alloc::vec::Vec<i64>)>,
);

/// The key of one state.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Key {
    candidates: alloc::vec::Vec<CandidateKey>,
    outstanding: alloc::vec::Vec<campaign_core::Id>,
    counts: [usize; 7],
    counters: [u64; 3],
    reserved: alloc::vec::Vec<u64>,
    stopped: Option<(u8, alloc::vec::Vec<campaign_core::Id>)>,
}

/// A stop reason as its kind and the identities it names.
fn reason(reason: &campaign_core::StopReason) -> (u8, alloc::vec::Vec<campaign_core::Id>) {
    match reason {
        campaign_core::StopReason::FrontierExhausted => (0, alloc::vec::Vec::new()),
        campaign_core::StopReason::BudgetExhausted { dimensions } => (1, dimensions.clone()),
        campaign_core::StopReason::SelectionLimitReached => (2, alloc::vec::Vec::new()),
        campaign_core::StopReason::ControlEventLimitReached => (3, alloc::vec::Vec::new()),
        campaign_core::StopReason::PolicyRequested { reason_id } => (4, alloc::vec![*reason_id]),
        campaign_core::StopReason::ConsumerRequested { reason_id } => (5, alloc::vec![*reason_id]),
    }
}

/// A candidate status as its code: 0 retained, 1 retired, 2 pruned.
const fn status_code(status: &campaign_core::CandidateStatus) -> u8 {
    match status {
        campaign_core::CandidateStatus::Retained => 0,
        campaign_core::CandidateStatus::Retired { .. } => 1,
        campaign_core::CandidateStatus::Pruned { .. } => 2,
    }
}

/// The key of `state`.
pub(super) fn of(state: &campaign_core::FrontierState) -> Key {
    debug_assert!(
        state
            .candidates
            .iter()
            .all(|candidate| candidate.candidate.campaign_id == state.admission.campaign_id),
        "every candidate belongs to the state's campaign"
    );
    let candidates: alloc::vec::Vec<CandidateKey> = state
        .candidates
        .iter()
        .map(|candidate| {
            let guidance = candidate
                .latest_guidance
                .as_ref()
                .map(|record| (record.generation, record.ranks.clone()));
            (
                candidate.candidate.candidate_id,
                status_code(&candidate.status),
                candidate.selection_count,
                guidance,
            )
        })
        .collect();
    let mut outstanding: alloc::vec::Vec<campaign_core::Id> = state
        .outstanding
        .iter()
        .map(|selection| selection.candidate_id)
        .collect();
    outstanding.sort();
    debug_assert!(
        outstanding.iter().all(|held| candidates
            .iter()
            .any(|(candidate_id, ..)| candidate_id == held)),
        "every outstanding selection names a candidate of the state"
    );
    let closures = state
        .settled
        .iter()
        .filter(|settled| !state.accepted_results.contains(&settled.settlement_id))
        .count();
    Key {
        candidates,
        outstanding,
        counts: [
            state.accepted_results.len(),
            state.findings.len(),
            state.discoveries.len(),
            closures,
            state.settled.len(),
            state.known_moments.len(),
            state.reservations.len(),
        ],
        counters: [
            state.selection_count,
            state.guidance_record_count,
            state.control_event_count,
        ],
        reserved: state
            .reserved_resources
            .iter()
            .map(|reserved| reserved.amount)
            .collect(),
        stopped: state.stopped.as_ref().map(|stop| reason(&stop.reason)),
    }
}
