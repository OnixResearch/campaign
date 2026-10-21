//! Stop events and a recorded stop carry no selection profile, so their receipts, including those
//! of a `PolicyRequested` stop, bind whichever valid production profile the request names and do
//! not attest that this profile's stopping rule produced the stop.

const ONE_FINDING: u64 = 1;
const TWO_FINDINGS: u64 = 2;
const RANK: i64 = 20;

fn limited(finding_limit: Option<u64>) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::planning_support::profile_with(|profile| {
        profile.stopping = campaign_core::StoppingRule { finding_limit };
    })
}

/// A `PolicyRequested` stop admitted after one finding: the stop, its event, and the states before
/// and after it.
struct PolicyStopped {
    stop: campaign_core::StopDecision,
    event: campaign_core::CampaignControlEvent,
    before: campaign_core::FrontierState,
    state: campaign_core::FrontierState,
}

fn policy_stopped(stopping: &campaign_core::SelectionProfile) -> Result<PolicyStopped, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, _candidate) =
        crate::suite::planning_support::guided(events, &state, b"policy-stop", RANK)?;
    let (events, state, selection) =
        crate::suite::planning_support::select_and_publish(events, &state)?;
    let labels: (&'static [u8], &'static [u8]) = (b"policy-child", b"policy-payload");
    let findings = vec![crate::suite::planning_support::finding(b"policy-finding")];
    let (mut events, before) =
        crate::suite::planning_support::settle(events, &state, &selection, labels, findings)?;
    let plan = crate::suite::planning_support::observed(
        crate::suite::planning_support::plan_with(&before, stopping.clone(), None),
    )?;
    let stop = crate::suite::planning_support::stopped(&plan)?;
    assert_eq!(stop.state_before_id, before.state_id);
    let event = crate::suite::planning_support::observed(campaign_core::admit_stop(
        &before,
        stop.reason.clone(),
    ))?;
    events.push(event.clone());
    let state = crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?;
    assert_eq!(state.stopped.as_ref(), Some(&stop));
    Ok(PolicyStopped {
        stop,
        event,
        before,
        state,
    })
}

/// The stop event receipt and the recorded-stop plan receipt project under the stopping profile,
/// the ranked profile without a stopping rule, and a profile whose finding limit would not have
/// stopped, each binding the profile it is requested with; a reference-evidence profile is
/// refused.
#[test]
// r[verify campaign.receipts.crossed]
fn a_policy_requested_stop_is_receipted_under_any_named_production_profile() -> Result<(), String> {
    let stopping = limited(Some(ONE_FINDING))?;
    let stopped = policy_stopped(&stopping)?;
    let reason_id = campaign_core::StoppingRule {
        finding_limit: Some(ONE_FINDING),
    }
    .policy_reason_id()
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or("a finding limit names its stop reason")?;
    assert_eq!(
        stopped.stop.reason,
        campaign_core::StopReason::PolicyRequested { reason_id }
    );
    let other_limit = limited(Some(TWO_FINDINGS))?;
    let unstopped =
        crate::suite::planning_support::plan_with(&stopped.before, other_limit.clone(), None);
    let unstopped = crate::suite::planning_support::observed(unstopped)?;
    assert!(!matches!(
        unstopped.decision,
        campaign_core::NextDecision::Stop(_)
    ));
    let state = &stopped.state;
    let binding = crate::suite::admission::current_projection(state)?;
    let ranked = crate::suite::selection::ranked_profile()?;
    let profiles = [&stopping, &ranked, &other_limit];
    let mut receipts = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let recorded =
            crate::suite::certified::event_receipt(state, &stopped.event, profile, &binding);
        let recorded = crate::suite::world::observed(recorded)?;
        assert_eq!(recorded.decision_id, stopped.event.event_id());
        assert_eq!(recorded.selection_profile_id, profile.profile_id);
        let request = crate::suite::selection::request_on(state, profile.clone(), None, &binding);
        let plan = campaign_core::plan_next(state, request.clone(), &binding);
        let plan = crate::suite::world::observed(plan)?;
        assert_eq!(
            crate::suite::planning_support::stopped(&plan)?,
            stopped.stop
        );
        let decision = campaign_core::project_receipt(state, &plan, &request, &binding);
        let decision = crate::suite::world::observed(decision)?;
        assert_eq!(decision.decision_id, stopped.stop.stop_id);
        assert_eq!(decision.selection_profile_id, profile.profile_id);
        receipts.push((recorded.receipt_id, decision.receipt_id));
    }
    receipts.sort();
    receipts.dedup();
    assert_eq!(receipts.len(), profiles.len());
    let reference = crate::suite::certified::reference_profile()?;
    let refused =
        crate::suite::certified::event_receipt(state, &stopped.event, &reference, &binding);
    assert_eq!(
        crate::suite::s3_fixture::denial(refused)?,
        campaign_core::CampaignError::ReferencePolicySelected
    );
    Ok(())
}
