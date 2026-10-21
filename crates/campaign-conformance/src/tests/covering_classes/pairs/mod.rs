//! Claims keyed on one event kind at one stop reason, and a cold reload keyed on a 4-way
//! conjunction of declared feature values.

/// One named event kind.
type EventKind = (
    &'static str,
    fn(&campaign_core::CampaignControlEvent) -> bool,
);

/// One named stop reason.
type StopKind = (&'static str, fn(&campaign_core::FrontierState) -> bool);

/// Whether a batch holds a coalesced duplicate.
fn is_coalesced_batch(batch: &campaign_core::CandidateBatchEvent) -> bool {
    batch.outcomes.iter().any(|outcome| {
        matches!(
            outcome.outcome,
            campaign_core::AdmissionDecision::CoalescedDuplicate { .. }
        )
    })
}

/// Every event sub-kind a consumer publishes.
const EVENT_KINDS: [EventKind; 18] = [
    ("admission", |event| {
        matches!(
            event,
            campaign_core::CampaignControlEvent::CampaignAdmitted { .. }
        )
    }),
    (
        "batch",
        |event| matches!(event, campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) if !is_coalesced_batch(batch)),
    ),
    (
        "coalesced batch",
        |event| matches!(event, campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) if is_coalesced_batch(batch)),
    ),
    (
        "discovery",
        |event| matches!(event, campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::Discovery)),
    ),
    (
        "improvement",
        |event| matches!(event, campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::StrictImprovement { .. })),
    ),
    (
        "coalesced guidance",
        |event| matches!(event, campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::CoalescedDuplicate { .. })),
    ),
    (
        "revision",
        |event| matches!(event, campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) if matches!(guidance.dynamics, campaign_core::GuidanceDynamics::Revision { .. })),
    ),
    (
        "ranked selection",
        |event| matches!(event, campaign_core::CampaignControlEvent::SelectionPublished(selection) if selection.entropy.is_none()),
    ),
    (
        "exploratory selection",
        |event| matches!(event, campaign_core::CampaignControlEvent::SelectionPublished(selection) if selection.entropy.is_some()),
    ),
    (
        "plain result",
        |event| matches!(event, campaign_core::CampaignControlEvent::ResultObserved(result) if result.observation.children.is_empty() && result.observation.findings.is_empty()),
    ),
    (
        "child result",
        |event| matches!(event, campaign_core::CampaignControlEvent::ResultObserved(result) if !result.observation.children.is_empty() && result.observation.findings.is_empty()),
    ),
    (
        "finding result",
        |event| matches!(event, campaign_core::CampaignControlEvent::ResultObserved(result) if result.observation.children.is_empty() && !result.observation.findings.is_empty()),
    ),
    (
        "reported result",
        |event| matches!(event, campaign_core::CampaignControlEvent::ResultObserved(result) if !result.observation.children.is_empty() && !result.observation.findings.is_empty()),
    ),
    ("closure", |event| {
        matches!(
            event,
            campaign_core::CampaignControlEvent::UnobservedClosed(_)
        )
    }),
    (
        "consumer retirement",
        |event| matches!(event, campaign_core::CampaignControlEvent::CandidateRetired(retired) if retired.origin == campaign_core::RetirementOrigin::Consumer),
    ),
    (
        "policy retirement",
        |event| matches!(event, campaign_core::CampaignControlEvent::CandidateRetired(retired) if retired.origin == campaign_core::RetirementOrigin::Policy),
    ),
    ("prune", |event| {
        matches!(
            event,
            campaign_core::CampaignControlEvent::CandidatesPruned(_)
        )
    }),
    ("stop", |event| {
        matches!(event, campaign_core::CampaignControlEvent::StopAccepted(_))
    }),
];

/// The reason of a state's accepted stop.
fn stopped(state: &campaign_core::FrontierState) -> Option<&campaign_core::StopReason> {
    state.stopped.as_ref().map(|stop| &stop.reason)
}

/// Whether a consumer stop names the adapter's host-capacity reason.
fn is_capacity(reason: &campaign_core::StopReason) -> bool {
    matches!(reason, campaign_core::StopReason::ConsumerRequested { reason_id } if *reason_id == campaign_choregraph::control_capacity_reason_id())
}

/// Every stop reason, the host-capacity consumer stop apart.
const STOP_KINDS: [StopKind; 7] = [
    ("frontier", |state| {
        matches!(
            stopped(state),
            Some(campaign_core::StopReason::FrontierExhausted)
        )
    }),
    ("budget", |state| {
        matches!(
            stopped(state),
            Some(campaign_core::StopReason::BudgetExhausted { .. })
        )
    }),
    ("selection limit", |state| {
        matches!(
            stopped(state),
            Some(campaign_core::StopReason::SelectionLimitReached)
        )
    }),
    ("control-event limit", |state| {
        matches!(
            stopped(state),
            Some(campaign_core::StopReason::ControlEventLimitReached)
        )
    }),
    ("policy", |state| {
        matches!(
            stopped(state),
            Some(campaign_core::StopReason::PolicyRequested { .. })
        )
    }),
    ("host capacity", |state| {
        stopped(state).is_some_and(is_capacity)
    }),
    ("consumer", |state| {
        stopped(state).is_some_and(|reason| {
            matches!(reason, campaign_core::StopReason::ConsumerRequested { .. })
                && !is_capacity(reason)
        })
    }),
];

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.receipts.non_claims]
// r[verify campaign.conformance.claims]
fn a_safe_deletion_claim_on_prune_receipts_after_the_frontier_stop_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let [(_, is_frontier), ..] = STOP_KINDS;
    let is_prune = |event: &campaign_core::CampaignControlEvent| {
        matches!(
            event,
            campaign_core::CampaignControlEvent::CandidatesPruned(_)
        )
    };
    super::only_state_coverage(&[super::wrapper::Hole::ClaimAfterStop(is_prune, is_frontier)])
}

/// Requires the state-coverage fixture, under `profile` for the state wrapper with `hole` open,
/// to fail as a negative fixture that let a forbidden effect through.
///
/// # Errors
///
/// Returns the fixture's verdict otherwise.
fn caught(
    hole: super::wrapper::Hole,
    profile: &crate::inputs::SuiteProfile,
) -> Result<(), Option<crate::verdicts::FixtureVerdict>> {
    let mut adapter = super::wrapper::StateAdapter::new(&[hole]);
    let verdict = crate::catalog::run_fixture(&mut adapter, profile, super::STATE_COVERAGE);
    if matches!(
        verdict,
        Some(crate::verdicts::FixtureVerdict::NegativeFixturePassed { .. })
    ) {
        Ok(())
    } else {
        Err(verdict)
    }
}

/// Every event kind and stop reason whose claims shell the state-coverage fixture lets pass under
/// one of `profiles`, with the fixture's verdict.
fn let_through(
    profiles: &[crate::inputs::SuiteProfile],
) -> alloc::vec::Vec<(
    &'static str,
    &'static str,
    Option<crate::verdicts::FixtureVerdict>,
)> {
    let mut passed = alloc::vec::Vec::with_capacity(
        EVENT_KINDS
            .len()
            .saturating_mul(STOP_KINDS.len())
            .saturating_mul(profiles.len()),
    );
    for (event_name, is_event) in EVENT_KINDS {
        for (stop_name, is_stop) in STOP_KINDS {
            for profile in profiles {
                let hole = super::wrapper::Hole::ClaimAfterStop(is_event, is_stop);
                if let Err(verdict) = caught(hole, profile) {
                    passed.push((event_name, stop_name, verdict));
                }
            }
        }
    }
    passed
}

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.receipts.non_claims]
fn every_event_kind_claimed_at_every_stop_reason_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    let passed = let_through(&super::profiles()?);
    assert!(passed.is_empty(), "claims let through: {passed:?}");
    Ok(())
}

/// A 4-way conjunction of declared feature values the covering table carries: a budget stop, a
/// closure, a pruned candidate, and a discovered child moment.
const CONJUNCTION: [fn(&campaign_core::FrontierState) -> bool; 4] = [
    STOP_KINDS[1].1,
    |state| {
        state
            .settled
            .iter()
            .any(|settled| !state.accepted_results.contains(&settled.settlement_id))
    },
    |state| {
        state.candidates.iter().any(|candidate| {
            matches!(
                candidate.status,
                campaign_core::CandidateStatus::Pruned { .. }
            )
        })
    },
    |state| !state.discoveries.is_empty(),
];

#[test]
// r[verify campaign.conformance.state_coverage]
// r[verify campaign.history_projection.stale]
fn a_cold_reload_keyed_on_a_four_way_conjunction_fails_state_coverage()
-> crate::verdicts::ConformanceResult<()> {
    super::only_state_coverage(&[super::wrapper::Hole::ColdWhen(super::wrapper::Held::All(
        CONJUNCTION,
    ))])
}
