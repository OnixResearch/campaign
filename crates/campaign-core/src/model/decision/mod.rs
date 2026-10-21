#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
// r[impl campaign.wait]
pub enum WaitReason {
    /// Public Campaign core API variant.
    AwaitingGuidance,
    /// Public Campaign core API variant.
    AwaitingResults,
    /// Public Campaign core API variant.
    AwaitingCapacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub enum NextDecision {
    /// Public Campaign core API variant.
    Select(alloc::boxed::Box<super::publication::SelectionDecision>),
    /// Public Campaign core API variant.
    Wait(WaitReason),
    /// Public Campaign core API variant.
    Stop(alloc::boxed::Box<super::stopping::StopDecision>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Public Campaign core API value.
pub struct DecisionPlan {
    /// Public Campaign core API value.
    pub campaign_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub state_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub decision: NextDecision,
    /// Public Campaign core API value.
    pub scheduling_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub admission_policy_id: crate::identity::Id,
    /// Public Campaign core API value.
    pub coalescing_policy_id: crate::identity::Id,
    /// Semantic work the production planner performed for this decision.
    pub work: PlanningWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Semantic planning work reported separately from decision fields.
pub struct PlanningWork {
    /// Candidate records inspected for eligibility.
    pub candidates_examined: u64,
    /// Candidates that were eligible for selection.
    pub eligible_candidates: u64,
}

impl PlanningWork {
    /// Work for a decision that examined no candidate.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            candidates_examined: 0,
            eligible_candidates: 0,
        }
    }
}
