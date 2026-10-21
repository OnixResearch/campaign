//! The declared state features of a Campaign frontier state: its status (running, or the reason
//! of its accepted stop, the adapter's host-capacity stop apart from other consumer stops) and ten
//! booleans. The suite's state coverage is stated over these features only.

/// Status of a frontier state: running, or stopped for one declared reason. The discriminant is
/// the status value of a feature vector (its position in `STATUSES`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub(in crate::catalog::evidence) enum Status {
    /// No stop accepted.
    Running = 0,
    /// Stopped because the frontier is exhausted.
    Frontier = 1,
    /// Stopped because a budget is exhausted.
    Budget = 2,
    /// Stopped at the selection bound.
    SelectionLimit = 3,
    /// Stopped at the control-event bound.
    ControlEventLimit = 4,
    /// Stopped by a policy request.
    Policy = 5,
    /// Stopped by the consumer under the adapter's host-capacity reason.
    HostCapacity = 6,
    /// Stopped by the consumer under any other reason.
    Consumer = 7,
}

/// Every status, in declared order (the status value of a feature vector is its position).
pub(in crate::catalog::evidence) const STATUSES: [Status; 8] = [
    Status::Running,
    Status::Frontier,
    Status::Budget,
    Status::SelectionLimit,
    Status::ControlEventLimit,
    Status::Policy,
    Status::HostCapacity,
    Status::Consumer,
];

impl Status {
    /// Whether the status is a stop the planner computes (it is admitted only as planned).
    pub(in crate::catalog::evidence) const fn is_planned(self) -> bool {
        matches!(
            self,
            Self::Frontier | Self::Budget | Self::SelectionLimit | Self::ControlEventLimit
        )
    }

    /// Position of the status in `STATUSES`.
    pub(in crate::catalog::evidence) const fn index(self) -> u16 {
        self as u16
    }

    /// The status of an accepted stop for `reason`; `is_host_capacity` tells whether a consumer
    /// stop names the adapter's host-capacity reason.
    const fn of_stop(reason: &campaign_core::StopReason, is_host_capacity: bool) -> Self {
        match reason {
            campaign_core::StopReason::FrontierExhausted => Self::Frontier,
            campaign_core::StopReason::BudgetExhausted { .. } => Self::Budget,
            campaign_core::StopReason::SelectionLimitReached => Self::SelectionLimit,
            campaign_core::StopReason::ControlEventLimitReached => Self::ControlEventLimit,
            campaign_core::StopReason::PolicyRequested { .. } => Self::Policy,
            campaign_core::StopReason::ConsumerRequested { .. } => {
                if is_host_capacity {
                    Self::HostCapacity
                } else {
                    Self::Consumer
                }
            }
        }
    }
}

/// Some guidance record was counted (`guidance_record_count > 0`).
pub(in crate::catalog::evidence) const GUIDED: u16 = 1;
/// Some selection was published (`selection_count > 0`).
pub(in crate::catalog::evidence) const SELECTED: u16 = 1 << 1;
/// Some selection is outstanding.
pub(in crate::catalog::evidence) const OUTSTANDING: u16 = 1 << 2;
/// Some result was accepted.
pub(in crate::catalog::evidence) const RESULTS: u16 = 1 << 3;
/// Some finding was accepted.
pub(in crate::catalog::evidence) const FINDINGS: u16 = 1 << 4;
/// Some child moment was discovered.
pub(in crate::catalog::evidence) const DISCOVERIES: u16 = 1 << 5;
/// Some selection was settled by an unobserved closure.
pub(in crate::catalog::evidence) const CLOSURES: u16 = 1 << 6;
/// Some candidate is retired.
pub(in crate::catalog::evidence) const RETIRED: u16 = 1 << 7;
/// Some candidate is pruned.
pub(in crate::catalog::evidence) const PRUNED: u16 = 1 << 8;
/// Some candidate is retained.
pub(in crate::catalog::evidence) const RETAINED: u16 = 1 << 9;

/// The boolean features in declared order (feature `i + 1` of a feature vector).
pub(in crate::catalog::evidence) const FLAGS: [u16; 10] = [
    GUIDED,
    SELECTED,
    OUTSTANDING,
    RESULTS,
    FINDINGS,
    DISCOVERIES,
    CLOSURES,
    RETIRED,
    PRUNED,
    RETAINED,
];

/// Number of declared features: the status and the ten booleans.
pub(in crate::catalog::evidence) const FEATURE_COUNT: usize = 11;

/// Bits of the boolean features in a class code.
const FLAG_BITS: u16 = 10;

/// Every boolean feature bit of a class code.
const FLAG_MASK: u16 = GUIDED
    | SELECTED
    | OUTSTANDING
    | RESULTS
    | FINDINGS
    | DISCOVERIES
    | CLOSURES
    | RETIRED
    | PRUNED
    | RETAINED;

/// The declared features of one state: its class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::catalog::evidence) struct StateClass {
    pub(in crate::catalog::evidence) status: Status,
    pub(in crate::catalog::evidence) flags: u16,
}

impl StateClass {
    /// Whether the boolean feature `flag` holds.
    pub(in crate::catalog::evidence) const fn has(self, flag: u16) -> bool {
        self.flags & flag != 0
    }

    /// The value of feature `feature`: the status position for feature zero, otherwise zero or
    /// one.
    pub(super) fn value(self, feature: usize) -> u16 {
        match feature.checked_sub(1) {
            None => self.status.index(),
            Some(flag) => FLAGS.get(flag).map_or(0, |bit| u16::from(self.has(*bit))),
        }
    }

    /// Compact code of the class: the status position above the ten flag bits.
    pub(in crate::catalog::evidence) const fn code(self) -> u16 {
        (self.status.index() << FLAG_BITS) | self.flags
    }

    /// The class of a code, when the code names one.
    pub(in crate::catalog::evidence) fn of_code(code: u16) -> Option<Self> {
        let status = STATUSES.get(usize::from(code >> FLAG_BITS)).copied()?;
        debug_assert_eq!(status.index(), code >> FLAG_BITS);
        Some(Self {
            status,
            flags: code & FLAG_MASK,
        })
    }
}

/// The declared features of `state` (alpha).
// r[impl campaign.conformance.state_coverage]
pub(super) fn of(state: &campaign_core::FrontierState) -> StateClass {
    let has_status = |is: fn(&campaign_core::CandidateStatus) -> bool| {
        state
            .candidates
            .iter()
            .any(|candidate| is(&candidate.status))
    };
    let has_closure = state
        .settled
        .iter()
        .any(|settled| !state.accepted_results.contains(&settled.settlement_id));
    debug_assert!(
        state.outstanding.is_empty() || state.selection_count > 0,
        "outstanding selections were published"
    );
    debug_assert!(
        !has_closure || state.selection_count > 0,
        "closures settle published selections"
    );
    let facts = [
        state.guidance_record_count > 0,
        state.selection_count > 0,
        !state.outstanding.is_empty(),
        !state.accepted_results.is_empty(),
        !state.findings.is_empty(),
        !state.discoveries.is_empty(),
        has_closure,
        has_status(|status| matches!(status, campaign_core::CandidateStatus::Retired { .. })),
        has_status(|status| matches!(status, campaign_core::CandidateStatus::Pruned { .. })),
        has_status(|status| matches!(status, campaign_core::CandidateStatus::Retained)),
    ];
    let flags = FLAGS
        .iter()
        .zip(facts)
        .filter(|(_, fact)| *fact)
        .fold(0, |flags, (flag, _)| flags | flag);
    let status = state.stopped.as_ref().map_or(Status::Running, |stop| {
        let is_host_capacity = matches!(
            &stop.reason,
            campaign_core::StopReason::ConsumerRequested { reason_id }
                if *reason_id == campaign_choregraph::control_capacity_reason_id()
        );
        Status::of_stop(&stop.reason, is_host_capacity)
    });
    StateClass { status, flags }
}
