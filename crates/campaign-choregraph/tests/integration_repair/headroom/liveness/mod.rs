//! Guidance waits need room to select: the planner waits for guidance only while the host capacity
//! still holds the guidance event, a selection, its settlement, and the stop.

/// Under a bound of five control events Campaign core waits for the unguided candidate's guidance,
/// but after the guidance the selection, its settlement, and the stop would not fit: the planner
/// stops instead (although the guidance itself would still publish), and the stop publishes.
#[test]
// r[verify campaign.stop.host_capacity]
fn a_guidance_wait_without_room_to_select_becomes_the_capacity_stop() -> Result<(), String> {
    for bound in super::host::bounds(5)? {
        let (hosted, candidate_ids) =
            crate::suite::configured::campaign::with_candidates(bound.admitted()?, 1)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        assert_eq!(
            super::core_plan(&held)?.decision,
            campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
        );
        let stop = super::capacity_stop(&super::adapter_plan(&held)?, &held)?;
        let guidance = crate::suite::configured::campaign::guidance_for(&held, candidate_ids[0])?;
        assert_eq!(
            hosted.publish(&held, guidance).map(|_| ()),
            Ok(()),
            "{bound:?}"
        );
        let (stopped_hosted, _) = hosted.append(stop)?;
        assert_eq!(
            super::stopped_history(&stopped_hosted, &held)?
                .control_events()
                .len(),
            3
        );
    }
    Ok(())
}

/// Under a bound of six control events the planner keeps Campaign core's wait for guidance; the
/// guidance, the selection it makes eligible, its settlement, and the stop then all publish, the
/// stop as the last event the bound holds.
#[test]
// r[verify campaign.stop.host_capacity]
fn a_guidance_wait_with_room_to_select_leads_to_a_granted_selection() -> Result<(), String> {
    for bound in super::host::bounds(6)? {
        let (hosted, candidate_ids) =
            crate::suite::configured::campaign::with_candidates(bound.admitted()?, 1)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let plan = super::adapter_plan(&held)?;
        assert_eq!(plan, super::core_plan(&held)?);
        assert_eq!(
            plan.decision,
            campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
        );
        let guidance = crate::suite::configured::campaign::guidance_for(&held, candidate_ids[0])?;
        let (hosted, _) = hosted.append(guidance)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let selection = super::selection_of(super::adapter_plan(&held)?)?;
        let (hosted, publication) = hosted.append(crate::suite::selection::event(&selection))?;
        assert_eq!(
            campaign_choregraph::can_execute_selection_after_publication(
                &selection,
                &publication,
                &crate::suite::selection::exact_durable_evidence(&publication),
            ),
            Ok(true),
            "{bound:?}"
        );
        let hosted = super::settled(hosted, &selection)?;
        let held = crate::suite::identity::observed(hosted.validate())?;
        let stop = super::capacity_stop(&super::adapter_plan(&held)?, &held)?;
        let (stopped_hosted, _) = hosted.append(stop)?;
        let stopped = super::stopped_history(&stopped_hosted, &held)?;
        assert_eq!(stopped.control_events().len(), 6);
        assert_eq!(stopped.binding().generation, bound.generation_after(6)?);
    }
    Ok(())
}
