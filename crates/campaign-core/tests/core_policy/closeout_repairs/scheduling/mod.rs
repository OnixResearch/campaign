//! Scheduling explanations re-plan both requests and validate their tickets.

/// An admission label or an unreachable finding limit cannot explain a forged selection: the
/// explanation re-plans both requests and the forged plan is not the declared policy's decision.
#[test]
fn a_non_causal_rule_difference_explains_no_forged_selection() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let (plan, request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    for relabel in [
        crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.admission_policy_id = crate::suite::admission::xid(
                campaign_core::IdKind::AdmissionPolicy,
                b"audit-admission-label",
            );
        })?,
        crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.stopping = campaign_core::StoppingRule {
                finding_limit: Some(1_000),
            };
        })?,
    ] {
        let (other_plan, other_request) =
            crate::suite::schedules::planned(&frontier.state, relabel, None)?;
        assert_eq!(
            crate::suite::schedules::selected(&other_plan)?,
            frontier.first
        );
        let genuine = crate::suite::world::observed(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request,
            },
            campaign_core::PlannedRequest {
                plan: &other_plan,
                request: &other_request,
            },
        ))?;
        assert!(!genuine.outcome_differs);
        let forged = crate::suite::schedules::forge_selected(other_plan, frontier.second);
        assert_eq!(
            crate::suite::s3_fixture::denial(campaign_core::explain_selection_difference(
                &frontier.state,
                &frontier.state.admission.projection,
                campaign_core::PlannedRequest {
                    plan: &plan,
                    request: &request
                },
                campaign_core::PlannedRequest {
                    plan: &forged,
                    request: &other_request
                }
            ))?,
            campaign_core::CampaignError::UndeclaredSchedulingDifference
        );
    }
    Ok(())
}

/// Ticket bytes explain nothing unless the planner validates the ticket: a ticket on a ranked
/// request is refused.
#[test]
fn unvalidated_ticket_bytes_explain_no_difference() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let (plan, request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    let ticket = crate::suite::schedules::ticket(&frontier.state, &request.profile, 7)?;
    let mut ticketed = request.clone();
    ticketed.entropy = Some(ticket);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::plan_next(
            &frontier.state,
            ticketed.clone(),
            &frontier.state.admission.projection,
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let mut forged = plan.clone();
    forged.decision =
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request
            },
            campaign_core::PlannedRequest {
                plan: &forged,
                request: &ticketed
            }
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    Ok(())
}
