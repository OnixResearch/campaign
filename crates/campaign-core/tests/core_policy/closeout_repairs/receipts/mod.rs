//! Receipts only for genuine decisions of a canonical, current state.

/// A receipt is projected only for the decision `plan_next` makes: a selection carrying another
/// campaign's identities is crossed.
#[test]
// r[verify campaign.receipts.crossed]
fn a_receipt_rejects_a_selection_the_planner_did_not_make() -> Result<(), String> {
    let fixture = crate::suite::world::selecting()?;
    let binding = fixture.binding.clone();
    let mut other_input = crate::suite::admission::default_input()?;
    other_input.budget_profile_id =
        crate::suite::admission::xid(campaign_core::IdKind::BudgetProfile, b"audit-other-budget");
    let (other_events, other_state) = crate::suite::admission::admitted(other_input)?;
    let (other_events, other_state, other_candidate) = crate::suite::candidate::admit(
        other_events,
        &other_state,
        b"audit-foreign",
        crate::suite::constants::COST_ONE,
    )?;
    let (_other_events, other_state, _guidance) = crate::suite::candidate::guide(
        other_events,
        &other_state,
        other_candidate,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let other = crate::suite::world::planned(other_state)?;
    let campaign_core::NextDecision::Select(foreign) = &other.plan.decision else {
        return Err(String::from("other campaign must select"));
    };
    assert_ne!(foreign.campaign_id, fixture.state.admission.campaign_id);
    let mut forged_plan = fixture.plan.clone();
    if let campaign_core::NextDecision::Select(selection) = &mut forged_plan.decision {
        selection.selection_id = foreign.selection_id;
        selection.guidance_id = foreign.guidance_id;
    }
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_receipt(
            &fixture.state,
            &forged_plan,
            &fixture.request,
            &binding,
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    crate::suite::world::observed(campaign_core::project_receipt(
        &fixture.state,
        &fixture.plan,
        &fixture.request,
        &binding,
    ))?;
    Ok(())
}

/// A frontier stop the planner never produced (a selection is outstanding) projects no receipt.
#[test]
fn a_receipt_rejects_an_unplanned_stop_while_work_is_outstanding() -> Result<(), String> {
    let world = crate::suite::world::populated()?;
    assert_eq!(world.state.outstanding.len(), 1);
    let planned = crate::suite::world::planned(world.state.clone())?;
    assert!(!matches!(
        planned.plan.decision,
        campaign_core::NextDecision::Stop(_)
    ));
    let forged_stop = campaign_core::StopDecision {
        stop_id: crate::suite::admission::xid(campaign_core::IdKind::Stop, b"audit-forged-stop"),
        campaign_id: world.state.admission.campaign_id,
        state_before_id: world.state.state_id,
        reason: campaign_core::StopReason::FrontierExhausted,
        incomplete_search_acknowledged: false,
    };
    let mut forged_plan = planned.plan.clone();
    forged_plan.decision = campaign_core::NextDecision::Stop(Box::new(forged_stop));
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_receipt(
            &world.state,
            &forged_plan,
            &planned.request,
            &planned.binding,
        ))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_stop(
            &world.state,
            campaign_core::StopReason::FrontierExhausted
        ))?,
        campaign_core::CampaignError::InvalidStop
    );
    Ok(())
}

/// A permuted copy of a reconstructed state is not current: planning and receipts refuse it.
#[test]
fn a_permuted_state_is_not_current() -> Result<(), String> {
    let (_events, state, _best, _lower) = super::guided_pair()?;
    let request =
        crate::suite::selection::request(&state, crate::suite::selection::ranked_profile()?, None);
    let plan = campaign_core::plan_next(&state, request.clone(), &state.admission.projection)
        .map_err(crate::suite::s3_fixture::err)?;
    let mut permuted = state.clone();
    permuted.candidates.reverse();
    permuted.event_ids.reverse();
    let mut rederived = permuted.clone();
    rederived
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(rederived, state);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::project_receipt(
            &permuted,
            &plan,
            &request,
            &state.admission.projection,
        ))?,
        campaign_core::CampaignError::StaleState
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::plan_next(
            &permuted,
            request,
            &state.admission.projection
        ))?,
        campaign_core::CampaignError::StaleState
    );
    Ok(())
}

/// A stopped campaign still validates entropy before returning its recorded stop.
#[test]
// r[verify campaign.entropy.invalid]
fn a_stopped_plan_validates_entropy_before_returning_the_stop() -> Result<(), String> {
    let (events, state, _best, _lower) = super::guided_pair()?;
    let foreign_ticket = campaign_core::EntropyTicket {
        entropy_id: campaign_core::Id::absent(),
        campaign_id: crate::suite::admission::xid(
            campaign_core::IdKind::Campaign,
            b"audit-foreign-campaign",
        ),
        state_id: crate::suite::admission::xid(
            campaign_core::IdKind::State,
            b"audit-foreign-state",
        ),
        profile_id: crate::suite::admission::xid(
            campaign_core::IdKind::SelectionProfile,
            b"audit-foreign-profile",
        ),
        source_id: crate::suite::admission::xid(
            campaign_core::IdKind::EntropyProfile,
            b"audit-foreign-source",
        ),
        counter: 1,
        bytes: [7; campaign_core::DIGEST_LENGTH],
    }
    .derive()
    .map_err(crate::suite::s3_fixture::err)?;
    let stop = campaign_core::admit_stop(
        &state,
        campaign_core::StopReason::ConsumerRequested {
            reason_id: crate::suite::admission::xid(
                campaign_core::IdKind::Reason,
                b"audit-entropy-stop",
            ),
        },
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, stopped) = crate::suite::s3_fixture::append(events, stop)?;
    let request = crate::suite::selection::request(
        &stopped,
        crate::suite::selection::ranked_profile()?,
        Some(foreign_ticket),
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::plan_next(
            &stopped,
            request,
            &stopped.admission.projection
        ))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let honest = crate::suite::world::planned(stopped)?;
    assert!(matches!(
        honest.plan.decision,
        campaign_core::NextDecision::Stop(_)
    ));
    assert_eq!(
        crate::suite::world::project(&honest)
            .map_err(crate::suite::s3_fixture::err)?
            .entropy_ticket_id,
        None
    );
    Ok(())
}
