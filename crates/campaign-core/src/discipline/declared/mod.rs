#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Declared final tie-break between candidates with equal priority.
pub enum TieBreak {
    /// Candidates with equal ranks are ordered by candidate identity ascending.
    CandidateIdAscending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Declared frontier priority: selection mode (threshold, entropy source, exploration order),
/// selection-count decay, and tie-break.
pub struct PriorityRule {
    /// Ranked or exploratory mode with its threshold, entropy source, and exploration order.
    pub mode: crate::model::SelectionMode,
    /// Selection-count decay arithmetic.
    pub decay: crate::model::SelectionCountDecay,
    /// Final tie-break.
    pub tie_break: TieBreak,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Declared scheduling rules of one selection profile: admission, priority, and stopping.
// r[impl campaign.fd.scheduling]
pub struct SchedulingRules {
    /// Declared admission policy identity.
    pub admission_policy_id: crate::identity::Id,
    /// Declared coalescing policy identity.
    pub coalescing_policy_id: crate::identity::Id,
    /// Declared priority rule.
    pub priority: PriorityRule,
    /// Declared stopping rule.
    pub stopping: crate::model::StoppingRule,
}

impl SchedulingRules {
    /// Views the declared scheduling rules of a validated selection profile.
    ///
    /// # Errors
    ///
    /// Returns the profile's typed validation error when the profile is malformed.
    pub fn from_profile(
        profile: &crate::model::SelectionProfile,
    ) -> crate::error::CampaignResult<Self> {
        profile.validate()?;
        Ok(Self {
            admission_policy_id: profile.admission_policy_id,
            coalescing_policy_id: profile.coalescing_policy_id,
            priority: PriorityRule {
                mode: profile.mode.clone(),
                decay: profile.decay.clone(),
                tie_break: TieBreak::CandidateIdAscending,
            },
            stopping: profile.stopping.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Which declared scheduling rules differ between two policies.
pub struct SchedulingComparison {
    /// The declared admission or coalescing policy differs.
    pub admission_differs: bool,
    /// The declared priority rule differs.
    pub priority_differs: bool,
    /// The declared stopping rule differs.
    pub stopping_differs: bool,
}

impl SchedulingComparison {
    /// Returns true when at least one declared rule differs.
    #[must_use]
    pub const fn declares_difference(&self) -> bool {
        self.admission_differs || self.priority_differs || self.stopping_differs
    }
}

/// Compares the declared admission, priority, and stopping rules of two policies.
#[must_use]
// r[impl campaign.fd.scheduling]
pub fn compare_scheduling(left: &SchedulingRules, right: &SchedulingRules) -> SchedulingComparison {
    SchedulingComparison {
        admission_differs: left.admission_policy_id != right.admission_policy_id
            || left.coalescing_policy_id != right.coalescing_policy_id,
        priority_differs: left.priority != right.priority,
        stopping_differs: left.stopping != right.stopping,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Explanation of why two decisions from one state differ (or do not).
pub struct SelectionDifference {
    /// Declared rule differences.
    pub scheduling: SchedulingComparison,
    /// Whether the explicit entropy draws differ.
    pub entropy_differs: bool,
    /// Whether the decisions select, wait, or stop differently.
    pub outcome_differs: bool,
}

#[derive(Clone, Copy, Debug)]
/// One supplied plan and the request it claims to answer.
pub struct PlannedRequest<'a> {
    /// Supplied decision plan.
    pub plan: &'a crate::model::DecisionPlan,
    /// Request the plan claims to answer.
    pub request: &'a crate::model::SelectionRequest,
}

/// Explains why two decisions planned from one shared state differ, using only declared rules and
/// explicit entropy.
///
/// Both requests are re-planned on `state` against `current_projection` with the production
/// planner (which validates their entropy tickets), and each supplied plan must equal its
/// re-planned decision. Two selections of different candidates are explained only by a priority
/// difference or by different validated entropy tickets; other outcome differences are explained
/// by any declared rule difference or by entropy.
///
/// # Errors
///
/// Returns the profile's validation error for a malformed profile, `CrossedIdentity` when a plan
/// does not belong to its request or the plans come from another campaign or state, the planner's
/// typed error for a request it rejects (for example `InvalidEntropy` for a ticket on a ranked
/// request), and `UndeclaredSchedulingDifference` when a supplied plan is not the decision its
/// declared policy makes or when the declared rules and entropy do not explain the difference.
// r[impl campaign.fd.scheduling.declared]
pub fn explain_selection_difference(
    state: &crate::frontier::FrontierState,
    current_projection: &crate::model::ProjectionBinding,
    left: PlannedRequest<'_>,
    right: PlannedRequest<'_>,
) -> crate::error::CampaignResult<SelectionDifference> {
    let (left_plan, left_request) = (left.plan, left.request);
    let (right_plan, right_request) = (right.plan, right.request);
    let left_rules = rules_of_plan(state, left_plan, left_request)?;
    let right_rules = rules_of_plan(state, right_plan, right_request)?;
    let left_planned = crate::policy::plan_next(state, left_request.clone(), current_projection)?;
    let right_planned = crate::policy::plan_next(state, right_request.clone(), current_projection)?;
    if left_planned != *left_plan || right_planned != *right_plan {
        return Err(crate::error::CampaignError::UndeclaredSchedulingDifference);
    }
    let left_entropy = left_request
        .entropy
        .as_ref()
        .map(|ticket| ticket.entropy_id);
    let right_entropy = right_request
        .entropy
        .as_ref()
        .map(|ticket| ticket.entropy_id);
    let (is_outcome_different, is_selection_difference) =
        match (&left_plan.decision, &right_plan.decision) {
            (
                crate::model::NextDecision::Select(left),
                crate::model::NextDecision::Select(right),
            ) => {
                let is_different = left.candidate_id != right.candidate_id;
                (is_different, is_different)
            }
            (crate::model::NextDecision::Wait(left), crate::model::NextDecision::Wait(right)) => {
                (left != right, false)
            }
            (crate::model::NextDecision::Stop(left), crate::model::NextDecision::Stop(right)) => {
                (left.reason != right.reason, false)
            }
            _ => (true, false),
        };
    let difference = SelectionDifference {
        scheduling: compare_scheduling(&left_rules, &right_rules),
        entropy_differs: left_entropy != right_entropy,
        outcome_differs: is_outcome_different,
    };
    let is_explained = if is_selection_difference {
        difference.entropy_differs || difference.scheduling.priority_differs
    } else {
        difference.entropy_differs || difference.scheduling.declares_difference()
    };
    if difference.outcome_differs && !is_explained {
        return Err(crate::error::CampaignError::UndeclaredSchedulingDifference);
    }
    debug_assert_eq!(left_plan.state_id, right_plan.state_id);
    debug_assert!(!difference.outcome_differs || is_explained);
    Ok(difference)
}

/// Returns the declared rules of `request` after checking that `plan` was planned for it on
/// `state`.
fn rules_of_plan(
    state: &crate::frontier::FrontierState,
    plan: &crate::model::DecisionPlan,
    request: &crate::model::SelectionRequest,
) -> crate::error::CampaignResult<SchedulingRules> {
    let rules = SchedulingRules::from_profile(&request.profile)?;
    let is_request_state =
        plan.campaign_id == request.campaign_id && plan.state_id == request.state_id;
    let is_shared_state =
        plan.campaign_id == state.admission.campaign_id && plan.state_id == state.state_id;
    let is_request_profile = crate::discipline::declared::follows_profile(plan, &request.profile);
    if !(is_request_state && is_shared_state && is_request_profile) {
        return Err(crate::error::CampaignError::CrossedIdentity);
    }
    debug_assert_eq!(plan.state_id, state.state_id);
    debug_assert_eq!(rules.admission_policy_id, plan.admission_policy_id);
    Ok(rules)
}

/// Whether a plan carries exactly the scheduling, admission, and coalescing policies of a profile.
pub(crate) fn follows_profile(
    plan: &crate::model::DecisionPlan,
    profile: &crate::model::SelectionProfile,
) -> bool {
    let is_same_scheduling = plan.scheduling_policy_id == profile.scheduling_policy_id;
    let is_same_admission = plan.admission_policy_id == profile.admission_policy_id;
    let is_same_coalescing = plan.coalescing_policy_id == profile.coalescing_policy_id;
    is_same_scheduling && is_same_admission && is_same_coalescing
}
