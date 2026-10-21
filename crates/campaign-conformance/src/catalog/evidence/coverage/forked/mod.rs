//! The presented-history dimension of the state coverage: the values of a fork of the history
//! before a selected state's head (its depth beyond the dropped head, zero to three or the
//! deepest the bounds admit for its head, and its head's event sub-kind), each paired with every
//! 3-way combination of the selected state's feature values. A state admits a value when a fork
//! through it carrying the value was built; a value is presented through it once such a fork was
//! presented to both the load path and cache validation.

/// Named fork depths (value bits zero to three).
const DEPTHS: [usize; 4] = [0, 1, 2, 3];

/// Value bit of the deepest depth the bounds admit for the fork's head.
const DEEPEST: u32 = 1 << 4;

/// Position of the first sub-kind value bit.
const SUB_KIND_SHIFT: usize = 5;

/// Number of fork values: the named depths, the deepest depth, and every head sub-kind.
const VALUE_COUNT: usize = 25;

/// The fork sub-kinds.
type SubKind = super::super::lineages::forks::subkinds::SubKind;

/// The depth values of a fork `depth` events beyond the dropped head (`is_deepest` when that is
/// the deepest depth its head admits).
pub(super) fn of_depth(depth: usize, is_deepest: bool) -> u32 {
    let named = DEPTHS
        .iter()
        .position(|named| *named == depth)
        .map_or(0, |position| 1_u32 << position);
    let values = if is_deepest { named | DEEPEST } else { named };
    debug_assert!(depth > 3 || values != 0);
    values
}

/// The value of a fork head's sub-kind.
pub(super) fn of_sub_kind(kind: SubKind) -> u32 {
    let bit = 1_u32 << SUB_KIND_SHIFT.saturating_add(kind.index());
    debug_assert!(bit.trailing_zeros() < 32);
    bit
}

/// The head sub-kind of a stop for `reason`: a consumer stop under the adapter's host-capacity
/// reason or another, a policy stop, or a planner stop.
pub(super) fn stop_sub_kind(reason: &campaign_core::StopReason) -> SubKind {
    let kind = match reason {
        campaign_core::StopReason::ConsumerRequested { reason_id } => {
            if *reason_id == campaign_choregraph::control_capacity_reason_id() {
                SubKind::HostCapacityStop
            } else {
                SubKind::ConsumerStop
            }
        }
        campaign_core::StopReason::PolicyRequested { .. } => SubKind::PolicyStop,
        campaign_core::StopReason::FrontierExhausted
        | campaign_core::StopReason::BudgetExhausted { .. }
        | campaign_core::StopReason::SelectionLimitReached
        | campaign_core::StopReason::ControlEventLimitReached => SubKind::PlannerStop,
    };
    debug_assert!(kind.is_stop());
    kind
}

/// The head sub-kind of one control event; `None` for the admission, which no fork ends in.
pub(super) fn sub_kind(event: &campaign_core::CampaignControlEvent) -> Option<SubKind> {
    let kind = match event {
        campaign_core::CampaignControlEvent::CampaignAdmitted { .. } => return None,
        campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) => {
            let is_coalesced = batch.outcomes.iter().any(|outcome| {
                matches!(
                    outcome.outcome,
                    campaign_core::AdmissionDecision::CoalescedDuplicate { .. }
                )
            });
            if is_coalesced {
                SubKind::CoalescedBatch
            } else {
                SubKind::Batch
            }
        }
        campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) => {
            match guidance.dynamics {
                campaign_core::GuidanceDynamics::Discovery => SubKind::Discovery,
                campaign_core::GuidanceDynamics::StrictImprovement { .. } => SubKind::Improvement,
                campaign_core::GuidanceDynamics::CoalescedDuplicate { .. } => {
                    SubKind::CoalescedGuidance
                }
                campaign_core::GuidanceDynamics::Revision { .. } => SubKind::Revision,
            }
        }
        campaign_core::CampaignControlEvent::SelectionPublished(selection) => {
            if selection.entropy.is_some() {
                SubKind::ExploratorySelection
            } else {
                SubKind::RankedSelection
            }
        }
        campaign_core::CampaignControlEvent::ResultObserved(result) => match (
            result.observation.children.is_empty(),
            result.observation.findings.is_empty(),
        ) {
            (true, true) => SubKind::PlainResult,
            (false, true) => SubKind::ChildResult,
            (true, false) => SubKind::FindingResult,
            (false, false) => SubKind::ReportedResult,
        },
        campaign_core::CampaignControlEvent::UnobservedClosed(_) => SubKind::Closure,
        campaign_core::CampaignControlEvent::CandidateRetired(retired) => {
            if retired.origin == campaign_core::RetirementOrigin::Policy {
                SubKind::PolicyRetirement
            } else {
                SubKind::ConsumerRetirement
            }
        }
        campaign_core::CampaignControlEvent::CandidatesPruned(_) => SubKind::Prune,
        campaign_core::CampaignControlEvent::StopAccepted(stopped) => {
            stop_sub_kind(&stopped.stop.reason)
        }
    };
    debug_assert!(
        kind.is_stop() == matches!(event, campaign_core::CampaignControlEvent::StopAccepted(_))
    );
    Some(kind)
}

/// The values of a presented fork: its depth values and the sub-kind of its head event.
pub(super) fn of_fork(
    depth: usize,
    is_deepest: bool,
    history: &crate::harness::oracle::HistoryView,
) -> u32 {
    let head = history
        .payloads
        .last()
        .and_then(|payload| sub_kind(&payload.control_event));
    of_depth(depth, is_deepest) | head.map_or(0, of_sub_kind)
}

/// (value, 3-way combination) pairs: for every set of three features and every fork value, the
/// value combinations of the states admitting (`admitted`) or presenting (`presented`) it.
pub(in crate::catalog::evidence) struct Pairs {
    admitted: alloc::vec::Vec<[u32; VALUE_COUNT]>,
    presented: alloc::vec::Vec<[u32; VALUE_COUNT]>,
}

/// Marks `values` for `class` in `masks`.
fn mark(masks: &mut [[u32; VALUE_COUNT]], class: super::features::StateClass, values: u32) {
    let bits = super::combos::triple_bits(class);
    debug_assert_eq!(bits.len(), masks.len());
    for (triple, bit) in masks.iter_mut().zip(bits) {
        for (value, mask) in triple.iter_mut().enumerate() {
            if values & (1 << value) != 0 {
                *mask |= bit;
            }
        }
    }
}

impl Pairs {
    /// No pair admitted or presented.
    pub(in crate::catalog::evidence) fn empty() -> Self {
        let triples = super::combos::triple_bits(super::features::StateClass {
            status: super::features::Status::Running,
            flags: 0,
        })
        .len();
        Self {
            admitted: alloc::vec![[0; VALUE_COUNT]; triples],
            presented: alloc::vec![[0; VALUE_COUNT]; triples],
        }
    }

    /// Records that a state of `class` admits `values`.
    pub(super) fn admit(&mut self, class: super::features::StateClass, values: u32) {
        mark(&mut self.admitted, class, values);
    }

    /// Records that `values` were presented through a state of `class` to the load path and
    /// cache validation.
    pub(super) fn present(&mut self, class: super::features::StateClass, values: u32) {
        mark(&mut self.presented, class, values);
    }

    /// For every fork value, the 3-way combinations of a state (`bits`, from
    /// `combos::triple_bits`) no presented state pairs it with yet; none for the values in
    /// `chosen` (already chosen for the state).
    pub(super) fn gains(&self, bits: &[u32], chosen: u32) -> [usize; VALUE_COUNT] {
        debug_assert_eq!(bits.len(), self.presented.len());
        let mut gains = [0_usize; VALUE_COUNT];
        for (value, gain) in gains.iter_mut().enumerate() {
            if chosen & (1 << value) != 0 {
                continue;
            }
            *gain = self
                .presented
                .iter()
                .zip(bits)
                .filter(|(triple, bit)| triple.get(value).is_some_and(|mask| mask & **bit == 0))
                .count();
        }
        debug_assert!(gains.iter().all(|gain| *gain <= bits.len()));
        gains
    }

    /// The gain of a fork carrying `values`: the sum of `gains` over its values.
    pub(super) fn gain_of(gains: &[usize; VALUE_COUNT], values: u32) -> usize {
        gains
            .iter()
            .enumerate()
            .filter(|(value, _)| values & (1 << value) != 0)
            .map(|(_, gain)| *gain)
            .sum()
    }

    /// The value of the deepest depth.
    pub(super) const fn deepest() -> u32 {
        DEEPEST
    }

    /// Admitted pairs no presented state carries: (depth pairs, sub-kind pairs).
    pub(super) fn missing(&self) -> (u32, u32) {
        let mut missing = (0_u32, 0_u32);
        for (admitted, presented) in self.admitted.iter().zip(&self.presented) {
            for (value, (needed, carried)) in admitted.iter().zip(presented).enumerate() {
                let count = (needed & !carried).count_ones();
                if value < SUB_KIND_SHIFT {
                    missing.0 = missing.0.saturating_add(count);
                } else {
                    missing.1 = missing.1.saturating_add(count);
                }
            }
        }
        missing
    }
}
