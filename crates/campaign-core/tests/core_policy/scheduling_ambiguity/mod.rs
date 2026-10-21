//! `campaign.fd.scheduling.declared` negatives: undeclared differences and crossed comparisons.

#[test]
// r[verify campaign.fd.scheduling.declared]
fn different_explicit_entropy_explains_different_exploratory_choices() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let profile = crate::suite::schedules::always_explore(&frontier.state)?;
    let first_ticket = crate::suite::schedules::ticket(
        &frontier.state,
        &profile,
        crate::suite::schedules::EXPLORE_FIRST,
    )?;
    let second_ticket = crate::suite::schedules::ticket(
        &frontier.state,
        &profile,
        crate::suite::schedules::EXPLORE_SECOND,
    )?;
    let (left_plan, left_request) =
        crate::suite::schedules::planned(&frontier.state, profile.clone(), Some(first_ticket))?;
    let (right_plan, right_request) =
        crate::suite::schedules::planned(&frontier.state, profile, Some(second_ticket.clone()))?;
    assert_ne!(
        crate::suite::schedules::selected(&left_plan)?,
        crate::suite::schedules::selected(&right_plan)?
    );
    let difference = crate::suite::world::observed(campaign_core::explain_selection_difference(
        &frontier.state,
        &frontier.state.admission.projection,
        campaign_core::PlannedRequest {
            plan: &left_plan,
            request: &left_request,
        },
        campaign_core::PlannedRequest {
            plan: &right_plan,
            request: &right_request,
        },
    ))?;
    assert!(difference.entropy_differs);
    assert!(!difference.scheduling.declares_difference());
    let forged = crate::suite::schedules::forge_selected(
        left_plan.clone(),
        crate::suite::schedules::selected(&right_plan)?,
    );
    assert_eq!(
        crate::suite::world::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &left_plan,
                request: &left_request
            },
            campaign_core::PlannedRequest {
                plan: &forged,
                request: &left_request
            }
        ))?,
        campaign_core::CampaignError::UndeclaredSchedulingDifference
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.scheduling.declared]
fn an_ambiguous_schedule_without_a_declared_difference_fails() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let (plan, request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    let (relabelled_plan, relabelled_request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.scheduling_policy_id = crate::suite::admission::xid(
                campaign_core::IdKind::SchedulingProfile,
                b"ambiguous-label",
            );
        })?,
        None,
    )?;
    assert_eq!(
        crate::suite::schedules::selected(&relabelled_plan)?,
        frontier.first
    );
    let equal = crate::suite::world::observed(campaign_core::explain_selection_difference(
        &frontier.state,
        &frontier.state.admission.projection,
        campaign_core::PlannedRequest {
            plan: &plan,
            request: &request,
        },
        campaign_core::PlannedRequest {
            plan: &relabelled_plan,
            request: &relabelled_request,
        },
    ))?;
    assert!(!equal.outcome_differs);
    let ambiguous = crate::suite::schedules::forge_selected(relabelled_plan, frontier.second);
    assert_eq!(
        crate::suite::world::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request
            },
            campaign_core::PlannedRequest {
                plan: &ambiguous,
                request: &relabelled_request
            }
        ))?,
        campaign_core::CampaignError::UndeclaredSchedulingDifference
    );
    let identical = crate::suite::schedules::forge_selected(plan.clone(), frontier.second);
    assert_eq!(
        crate::suite::world::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request
            },
            campaign_core::PlannedRequest {
                plan: &identical,
                request: &request
            }
        ))?,
        campaign_core::CampaignError::UndeclaredSchedulingDifference
    );
    Ok(())
}

#[test]
fn plans_from_other_states_or_requests_cannot_be_compared() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let (plan, request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    let (_events, other_state) = crate::suite::admission::initial_state()?;
    let (other_plan, other_request) = crate::suite::schedules::planned(
        &other_state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    assert_eq!(
        crate::suite::world::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request
            },
            campaign_core::PlannedRequest {
                plan: &other_plan,
                request: &other_request
            }
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let mut foreign_policy = plan.clone();
    foreign_policy.coalescing_policy_id =
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"foreign");
    assert_eq!(
        crate::suite::world::denial(campaign_core::explain_selection_difference(
            &frontier.state,
            &frontier.state.admission.projection,
            campaign_core::PlannedRequest {
                plan: &plan,
                request: &request
            },
            campaign_core::PlannedRequest {
                plan: &foreign_policy,
                request: &request
            }
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    Ok(())
}
