//! Receipt kinds the claims coverage presents: the event receipt of every control event sub-kind,
//! the plan receipts of the plans a lineage makes and of the state itself, and refusal receipts,
//! and which of them are compatible with a declared class.

/// The campaign admission event.
pub(in crate::catalog::evidence) const ADMISSION: u32 = 1;
/// A candidate batch whose every candidate was admitted.
pub(in crate::catalog::evidence) const BATCH: u32 = 1 << 1;
/// A candidate batch with a coalesced duplicate.
pub(in crate::catalog::evidence) const COALESCED_BATCH: u32 = 1 << 2;
/// A candidate's first guidance.
pub(in crate::catalog::evidence) const DISCOVERY: u32 = 1 << 3;
/// A strict guidance improvement.
pub(in crate::catalog::evidence) const IMPROVEMENT: u32 = 1 << 4;
/// An equal-rank guidance update coalesced into the latest record.
pub(in crate::catalog::evidence) const COALESCED_GUIDANCE: u32 = 1 << 5;
/// An explicit guidance revision.
pub(in crate::catalog::evidence) const REVISION: u32 = 1 << 6;
/// A published ranked selection.
pub(in crate::catalog::evidence) const RANKED_SELECTION: u32 = 1 << 7;
/// A published exploratory selection.
pub(in crate::catalog::evidence) const EXPLORATORY_SELECTION: u32 = 1 << 8;
/// A result with neither child moments nor findings.
pub(in crate::catalog::evidence) const PLAIN_RESULT: u32 = 1 << 9;
/// A result reporting a child moment.
pub(in crate::catalog::evidence) const CHILD_RESULT: u32 = 1 << 10;
/// A result reporting a finding.
pub(in crate::catalog::evidence) const FINDING_RESULT: u32 = 1 << 11;
/// A result reporting a child moment and a finding.
pub(in crate::catalog::evidence) const REPORTED_RESULT: u32 = 1 << 12;
/// An unobserved closure.
pub(in crate::catalog::evidence) const CLOSURE: u32 = 1 << 13;
/// A consumer-origin retirement.
pub(in crate::catalog::evidence) const CONSUMER_RETIREMENT: u32 = 1 << 14;
/// A policy-origin retirement.
pub(in crate::catalog::evidence) const POLICY_RETIREMENT: u32 = 1 << 15;
/// A declared prune.
pub(in crate::catalog::evidence) const PRUNE: u32 = 1 << 16;
/// The accepted stop.
pub(in crate::catalog::evidence) const STOP: u32 = 1 << 17;
/// The plan receipt of a ranked selection the lineage published.
pub(in crate::catalog::evidence) const RANKED_PLAN: u32 = 1 << 18;
/// The plan receipt of an exploratory selection the lineage published.
pub(in crate::catalog::evidence) const EXPLORATORY_PLAN: u32 = 1 << 19;
/// The ranked and exploratory plan receipts of a planner stop before it was recorded: every
/// lineage holds the frontier stop planned at its bare admission, and a planner-stopped one the
/// stop it records.
pub(in crate::catalog::evidence) const UNRECORDED_STOP_PLAN: u32 = 1 << 20;
/// The ranked plan receipt of the presented state.
pub(in crate::catalog::evidence) const STATE_RANKED_PLAN: u32 = 1 << 21;
/// The exploratory plan receipt of the presented state.
pub(in crate::catalog::evidence) const STATE_EXPLORATORY_PLAN: u32 = 1 << 22;
/// The refusal receipts of the reject rule at a running state.
pub(in crate::catalog::evidence) const REFUSAL: u32 = 1 << 23;

/// Number of receipt kinds.
pub(in crate::catalog::evidence) const KIND_COUNT: usize = 24;

/// `kinds` when `condition` holds, otherwise none.
const fn when(condition: bool, kinds: u32) -> u32 {
    if condition { kinds } else { 0 }
}

/// The receipt kinds compatible with `class`: every kind a lineage reaching the class can hold
/// and every plan receipt its state admits.
// r[impl campaign.conformance.state_coverage]
pub(in crate::catalog::evidence) const fn compatible(class: super::features::StateClass) -> u32 {
    let is_running = matches!(class.status, super::features::Status::Running);
    let kinds = ADMISSION
        | STATE_RANKED_PLAN
        | STATE_EXPLORATORY_PLAN
        | when(
            class.has(
                super::features::RETAINED | super::features::RETIRED | super::features::PRUNED,
            ),
            BATCH | COALESCED_BATCH,
        )
        | when(
            class.has(super::features::GUIDED),
            DISCOVERY | IMPROVEMENT | COALESCED_GUIDANCE | REVISION,
        )
        | when(
            class.has(super::features::SELECTED),
            RANKED_SELECTION | EXPLORATORY_SELECTION | RANKED_PLAN | EXPLORATORY_PLAN,
        )
        | when(class.has(super::features::RESULTS), PLAIN_RESULT)
        | when(class.has(super::features::DISCOVERIES), CHILD_RESULT)
        | when(class.has(super::features::FINDINGS), FINDING_RESULT)
        | when(
            class.has(super::features::FINDINGS) && class.has(super::features::DISCOVERIES),
            REPORTED_RESULT,
        )
        | when(class.has(super::features::CLOSURES), CLOSURE)
        | when(
            class.has(super::features::RETIRED),
            CONSUMER_RETIREMENT | POLICY_RETIREMENT,
        )
        | when(class.has(super::features::PRUNED), PRUNE)
        | when(!is_running, STOP)
        | UNRECORDED_STOP_PLAN
        | when(is_running, REFUSAL);
    debug_assert!(
        kinds >> KIND_COUNT == 0,
        "every compatible kind is declared"
    );
    debug_assert!(
        kinds & (STOP | REFUSAL) != STOP | REFUSAL,
        "refusals are presented only at running states, which hold no stop"
    );
    kinds
}

/// The sub-kind of one control event's receipt.
pub(super) fn of_event(event: &campaign_core::CampaignControlEvent) -> u32 {
    match event {
        campaign_core::CampaignControlEvent::CampaignAdmitted { .. } => ADMISSION,
        campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) => {
            let is_coalesced = batch.outcomes.iter().any(|outcome| {
                matches!(
                    outcome.outcome,
                    campaign_core::AdmissionDecision::CoalescedDuplicate { .. }
                )
            });
            if is_coalesced { COALESCED_BATCH } else { BATCH }
        }
        campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) => {
            match guidance.dynamics {
                campaign_core::GuidanceDynamics::Discovery => DISCOVERY,
                campaign_core::GuidanceDynamics::StrictImprovement { .. } => IMPROVEMENT,
                campaign_core::GuidanceDynamics::CoalescedDuplicate { .. } => COALESCED_GUIDANCE,
                campaign_core::GuidanceDynamics::Revision { .. } => REVISION,
            }
        }
        campaign_core::CampaignControlEvent::SelectionPublished(selection) => {
            if selection.entropy.is_some() {
                EXPLORATORY_SELECTION
            } else {
                RANKED_SELECTION
            }
        }
        campaign_core::CampaignControlEvent::ResultObserved(result) => {
            let observation = &result.observation;
            match (
                observation.children.is_empty(),
                observation.findings.is_empty(),
            ) {
                (true, true) => PLAIN_RESULT,
                (false, true) => CHILD_RESULT,
                (true, false) => FINDING_RESULT,
                (false, false) => REPORTED_RESULT,
            }
        }
        campaign_core::CampaignControlEvent::UnobservedClosed(_) => CLOSURE,
        campaign_core::CampaignControlEvent::CandidateRetired(retired) => {
            if retired.origin == campaign_core::RetirementOrigin::Policy {
                POLICY_RETIREMENT
            } else {
                CONSUMER_RETIREMENT
            }
        }
        campaign_core::CampaignControlEvent::CandidatesPruned(_) => PRUNE,
        campaign_core::CampaignControlEvent::StopAccepted(_) => STOP,
    }
}
