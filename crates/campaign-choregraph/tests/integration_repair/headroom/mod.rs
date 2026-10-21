//! Host control capacity: publication and planning through a published projection keep the
//! Choregraph projection bound, the control branch generation range, and the Choregraph graph's
//! own event limit for every outstanding settlement and the stop, so no granted selection is
//! stranded.

mod host;
mod liveness;

fn adapter_plan(
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::DecisionPlan, String> {
    crate::suite::identity::observed(campaign_choregraph::plan_next_from_published_projection(
        held,
        campaign_choregraph::PublishedSelectionRequest {
            request: crate::suite::selection::request_for(held)?,
        },
    ))
}

fn core_plan(
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::DecisionPlan, String> {
    crate::suite::identity::observed(campaign_core::plan_next(
        held.state(),
        crate::suite::selection::request_for(held)?,
        held.binding(),
    ))
}

fn capacity_reason() -> campaign_core::StopReason {
    campaign_core::StopReason::ConsumerRequested {
        reason_id: campaign_choregraph::control_capacity_reason_id(),
    }
}

/// Requires `plan` to be the host-capacity stop on `held`'s state and returns its stop event.
fn capacity_stop(
    plan: &campaign_core::DecisionPlan,
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let campaign_core::NextDecision::Stop(stop) = &plan.decision else {
        return Err(format!("expected the capacity stop, planned {plan:?}"));
    };
    assert_eq!(stop.reason, capacity_reason());
    assert!(stop.incomplete_search_acknowledged);
    assert_eq!(plan.work, campaign_core::PlanningWork::none());
    assert_eq!(plan.state_id, held.state().state_id);
    let event = crate::suite::identity::observed(campaign_core::admit_stop(
        held.state(),
        capacity_reason(),
    ))?;
    let campaign_core::CampaignControlEvent::StopAccepted(accepted) = &event else {
        return Err(String::from("admit_stop returns a stop event"));
    };
    assert_eq!(accepted.stop, **stop);
    Ok(event)
}

fn selection_of(
    plan: campaign_core::DecisionPlan,
) -> Result<campaign_core::SelectionDecision, String> {
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Err(format!("expected a selection, planned {plan:?}"));
    };
    Ok(*selection)
}

/// Publishes the result of `selection` over `hosted`.
fn settled(
    hosted: crate::suite::configured::Hosted,
    selection: &campaign_core::SelectionDecision,
) -> Result<crate::suite::configured::Hosted, String> {
    let held = crate::suite::identity::observed(hosted.validate())?;
    let result = crate::suite::identity::observed(campaign_core::admit_result(
        held.state(),
        crate::suite::selection::result_for(&held, selection),
    ))?;
    Ok(hosted.append(result)?.0)
}

/// Requires the committed stop to load cold and as an advance of `held`, and to stay the plan.
fn stopped_history(
    hosted: &crate::suite::configured::Hosted,
    held: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_choregraph::ValidatedControlProjection, String> {
    let stopped = crate::suite::identity::observed(hosted.validate())?;
    assert_eq!(hosted.advance(held), Ok(stopped.clone()));
    let stop = stopped
        .state()
        .stopped
        .clone()
        .ok_or_else(|| String::from("the stop is recorded"))?;
    assert_eq!(
        adapter_plan(&stopped)?.decision,
        campaign_core::NextDecision::Stop(Box::new(stop))
    );
    Ok(stopped)
}

/// A selection whose settlement and the stop would not fit is refused with the typed capacity
/// error, the held projection is unchanged, and the planner produces the capacity stop instead,
/// which publishes, reloads, and projects a receipt that makes no claims.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
// r[verify campaign.receipts.host_capacity]
// r[verify campaign.stop.consumer]
fn a_selection_beyond_the_bound_is_refused_and_the_planner_stops_instead() -> Result<(), String> {
    for bound in host::bounds(4)? {
        let hosted = crate::suite::configured::campaign::guided(bound.admitted()?, 1)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        assert_eq!(held.control_events().len(), 3);
        let selection = selection_of(core_plan(&held)?)?;
        assert_eq!(
            hosted.publish(&held, crate::suite::selection::event(&selection)),
            Err(bound.exhausted.clone()),
            "{bound:?}"
        );
        assert_eq!(hosted.validate().as_ref(), Ok(&held));
        let stop = capacity_stop(&adapter_plan(&held)?, &held)?;
        let (stopped_hosted, _) = hosted.append(stop.clone())?;
        let stopped = stopped_history(&stopped_hosted, &held)?;
        let receipt = crate::suite::identity::observed(campaign_core::project_event_receipt(
            stopped.state(),
            &campaign_core::EventReceiptRequest {
                request_id: crate::suite::identity::derive(
                    campaign_core::IdKind::Request,
                    b"capacity-stop-receipt",
                ),
                event: stop,
                profile: crate::suite::selection::request_for(&stopped)?.profile,
            },
            stopped.binding(),
        ))?;
        assert_eq!(
            campaign_core::validate_receipt(&receipt, &campaign_core::ReceiptClaims::no_claims()),
            Ok(())
        );
        let exhaustive = campaign_core::ReceiptClaims {
            exhaustive_search: true,
            ..campaign_core::ReceiptClaims::no_claims()
        };
        assert_eq!(
            campaign_core::validate_receipt(&receipt, &exhaustive),
            Err(campaign_core::CampaignError::InvalidReceiptClaim)
        );
    }
    Ok(())
}

/// With room for exactly one selection, its settlement, and the stop, the planner agrees with
/// Campaign core and the selection is granted; its result and the stop then publish, the stop as
/// the last event the host capacity holds.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn a_selection_its_settlement_and_the_stop_publish_at_the_bound() -> Result<(), String> {
    for bound in host::bounds(6)? {
        let hosted = crate::suite::configured::campaign::guided(bound.admitted()?, 1)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let plan = adapter_plan(&held)?;
        assert_eq!(plan, core_plan(&held)?);
        let selection = selection_of(plan)?;
        let publication = crate::suite::identity::observed(
            hosted.publish(&held, crate::suite::selection::event(&selection)),
        )?;
        assert_eq!(
            campaign_choregraph::can_execute_selection_after_publication(
                &selection,
                &publication,
                &crate::suite::selection::exact_durable_evidence(&publication),
            ),
            Ok(true)
        );
        let hosted = settled(hosted.committed(&publication), &selection)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        assert!(held.state().outstanding.is_empty());
        assert!(matches!(
            core_plan(&held)?.decision,
            campaign_core::NextDecision::Select(_)
        ));
        let stop = capacity_stop(&adapter_plan(&held)?, &held)?;
        let (stopped_hosted, _) = hosted.append(stop)?;
        let stopped = stopped_history(&stopped_hosted, &held)?;
        assert_eq!(stopped.control_events().len(), 6);
        assert_eq!(stopped.binding().generation, bound.generation_after(6)?);
    }
    Ok(())
}

/// While a selection is outstanding and another would leave its settlement no room, the planner
/// waits for results where Campaign core would select, and that selection is refused; once
/// settled, the planner stops and the stop publishes.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn an_outstanding_selection_makes_the_planner_wait_within_the_bound() -> Result<(), String> {
    for bound in host::bounds(7)? {
        let hosted = crate::suite::configured::campaign::guided(bound.admitted()?, 2)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let first = selection_of(adapter_plan(&held)?)?;
        let (hosted, _) = hosted.append(crate::suite::selection::event(&first))?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let second = selection_of(core_plan(&held)?)?;
        let plan = adapter_plan(&held)?;
        assert_eq!(
            plan.decision,
            campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
        );
        assert_eq!(plan.work, campaign_core::PlanningWork::none());
        assert_eq!(
            hosted.publish(&held, crate::suite::selection::event(&second)),
            Err(bound.exhausted.clone()),
            "{bound:?}"
        );
        let hosted = settled(hosted, &first)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let stop = capacity_stop(&adapter_plan(&held)?, &held)?;
        let (stopped_hosted, _) = hosted.append(stop)?;
        assert_eq!(
            stopped_history(&stopped_hosted, &held)?
                .control_events()
                .len(),
            7
        );
    }
    Ok(())
}

/// Under a bound of three control events the candidate batch publishes but its guidance would
/// leave no room for the stop, so it is refused; where Campaign core waits for that guidance the
/// planner stops, and the stop fills the capacity.
#[test]
// r[verify campaign.publication_fence.capacity]
// r[verify campaign.stop.host_capacity]
fn three_events_refuse_guidance_and_publish_the_stop() -> Result<(), String> {
    for bound in host::bounds(3)? {
        let (hosted, candidate_ids) =
            crate::suite::configured::campaign::with_candidates(bound.admitted()?, 1)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let guidance = crate::suite::configured::campaign::guidance_for(&held, candidate_ids[0])?;
        assert_eq!(
            hosted.publish(&held, guidance),
            Err(bound.exhausted.clone()),
            "{bound:?}"
        );
        assert_eq!(
            core_plan(&held)?.decision,
            campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
        );
        let stop = capacity_stop(&adapter_plan(&held)?, &held)?;
        let (stopped_hosted, _) = hosted.append(stop)?;
        assert_eq!(
            stopped_history(&stopped_hosted, &held)?
                .control_events()
                .len(),
            3
        );
    }
    Ok(())
}
