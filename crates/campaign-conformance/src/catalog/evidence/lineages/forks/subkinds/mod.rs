//! Event sub-kinds a fork can end in: every control event sub-kind a consumer publishes after the
//! admission (candidate batches new and with a coalesced duplicate; first guidance, strict
//! improvement, equal-rank update, and revision; ranked and exploratory selections; results plain,
//! reporting a child, a finding, or both; unobserved closures; consumer and policy retirements;
//! explicit prunes; and consumer, host-capacity, policy, and planner stops). Each head acts on
//! the first fitting candidate in suite operation order (ascending operation handle), never by
//! candidate identity.

mod guidance;
mod removals;
mod settled;

/// One event sub-kind a fork ends in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(in crate::catalog::evidence) enum SubKind {
    /// A candidate batch whose every candidate is new.
    Batch = 0,
    /// A candidate batch with a coalesced duplicate.
    CoalescedBatch = 1,
    /// A candidate's first guidance.
    Discovery = 2,
    /// A strict guidance improvement.
    Improvement = 3,
    /// An equal-rank guidance update coalesced into the latest record.
    CoalescedGuidance = 4,
    /// An explicit guidance revision.
    Revision = 5,
    /// A ranked selection.
    RankedSelection = 6,
    /// An exploratory selection.
    ExploratorySelection = 7,
    /// A result with neither child moments nor findings.
    PlainResult = 8,
    /// A result reporting a child moment.
    ChildResult = 9,
    /// A result reporting a finding.
    FindingResult = 10,
    /// A result reporting a child moment and a finding.
    ReportedResult = 11,
    /// An unobserved closure.
    Closure = 12,
    /// A consumer-origin retirement.
    ConsumerRetirement = 13,
    /// A policy-origin retirement.
    PolicyRetirement = 14,
    /// An explicit prune.
    Prune = 15,
    /// A consumer stop under the adapter's consumer reason.
    ConsumerStop = 16,
    /// A consumer stop under the adapter's host-capacity reason.
    HostCapacityStop = 17,
    /// A policy stop.
    PolicyStop = 18,
    /// A stop the planner plans (frontier, budget, selection bound, control-event bound).
    PlannerStop = 19,
}

/// Every sub-kind, in declared order (the position of a sub-kind is its discriminant).
pub(in crate::catalog::evidence) const SUB_KINDS: [SubKind; 20] = [
    SubKind::Batch,
    SubKind::CoalescedBatch,
    SubKind::Discovery,
    SubKind::Improvement,
    SubKind::CoalescedGuidance,
    SubKind::Revision,
    SubKind::RankedSelection,
    SubKind::ExploratorySelection,
    SubKind::PlainResult,
    SubKind::ChildResult,
    SubKind::FindingResult,
    SubKind::ReportedResult,
    SubKind::Closure,
    SubKind::ConsumerRetirement,
    SubKind::PolicyRetirement,
    SubKind::Prune,
    SubKind::ConsumerStop,
    SubKind::HostCapacityStop,
    SubKind::PolicyStop,
    SubKind::PlannerStop,
];

impl SubKind {
    /// Position of the sub-kind in `SUB_KINDS`.
    pub(in crate::catalog::evidence) const fn index(self) -> usize {
        self as usize
    }

    /// Whether the sub-kind is a stop.
    pub(in crate::catalog::evidence) const fn is_stop(self) -> bool {
        matches!(
            self,
            Self::ConsumerStop | Self::HostCapacityStop | Self::PolicyStop | Self::PlannerStop
        )
    }
}

impl super::shaped::Head for SubKind {
    fn settles(self) -> bool {
        matches!(
            self,
            Self::PlainResult
                | Self::ChildResult
                | Self::FindingResult
                | Self::ReportedResult
                | Self::Closure
        )
    }

    fn reserve(self, is_stopped: bool) -> usize {
        if is_stopped || self.is_stop() {
            0
        } else if matches!(self, Self::RankedSelection | Self::ExploratorySelection) {
            2
        } else {
            1
        }
    }

    fn event(
        self,
        session: &mut crate::harness::Session<'_, '_>,
        _base: &campaign_core::FrontierState,
        (history, view): (
            &crate::harness::oracle::HistoryView,
            &campaign_choregraph::ValidatedControlProjection,
        ),
        _step: usize,
    ) -> crate::verdicts::ConformanceResult<Option<campaign_core::CampaignControlEvent>> {
        let state = view.state();
        debug_assert_eq!(history.payloads.len(), view.control_events().len());
        if state.stopped.is_some() && !self.settles() {
            return Ok(None);
        }
        let event = match self {
            Self::Batch => super::fill::batch(session, state, super::heads::HEAD_OPERATION)?,
            Self::CoalescedBatch => removals::coalesced_batch(session, state)?,
            Self::Discovery | Self::Improvement | Self::CoalescedGuidance | Self::Revision => {
                guidance::event(session, state, self)?
            }
            Self::RankedSelection => super::heads::selection(session, view)?,
            Self::ExploratorySelection => settled::exploratory(session, view)?,
            Self::PlainResult | Self::ChildResult | Self::FindingResult | Self::ReportedResult => {
                settled::result(session, (history, state), self)?
            }
            Self::Closure => super::heads::settlement(
                session,
                (history, state),
                super::heads::HeadKind::Closure,
            )?,
            Self::ConsumerRetirement | Self::PolicyRetirement | Self::Prune => {
                removals::event(session, state, self)?
            }
            _ => settled::stop(session, view, self)?,
        };
        debug_assert!(
            event.as_ref().is_none_or(|event| !matches!(
                event,
                campaign_core::CampaignControlEvent::CampaignAdmitted { .. }
            )),
            "a fork head is never an admission"
        );
        Ok(event)
    }
}

/// The retained candidate of `state` first in suite operation order among those `is` accepts,
/// with its operation handle.
///
/// # Errors
///
/// Returns `Diverged` for a candidate the session never mapped.
fn first_in_suite_order<'s>(
    session: &crate::harness::Session<'_, '_>,
    state: &'s campaign_core::FrontierState,
    is: impl Fn(&campaign_core::CandidateState) -> bool,
) -> crate::verdicts::ConformanceResult<
    Option<(
        crate::vocabulary::OperationHandle,
        &'s campaign_core::CandidateState,
    )>,
> {
    let mut chosen: Option<(
        crate::vocabulary::OperationHandle,
        &campaign_core::CandidateState,
    )> = None;
    for candidate in state.candidates.iter().filter(|candidate| is(candidate)) {
        let operation = session.operation_of(candidate.candidate.candidate_id)?;
        if chosen.is_none_or(|(first, _)| operation < first) {
            chosen = Some((operation, candidate));
        }
    }
    debug_assert!(chosen.is_none_or(|(_, candidate)| is(candidate)));
    Ok(chosen)
}
