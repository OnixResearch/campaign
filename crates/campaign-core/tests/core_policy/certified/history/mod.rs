//! Receipts accept only the state's own current history value: the admitted control-branch name at
//! the admission generation plus the state's control-event count.

/// `current` re-derived after `edit`.
fn rederived(
    current: &campaign_core::ProjectionBinding,
    edit: impl FnOnce(&mut campaign_core::ProjectionBinding),
) -> Result<campaign_core::ProjectionBinding, String> {
    let mut binding = current.clone();
    binding.projection_id = campaign_core::Id::absent();
    edit(&mut binding);
    binding.derive().map_err(crate::suite::s3_fixture::err)
}

/// Decision and event receipts of `planned` against `binding` are both crossed.
fn crossed_under(
    planned: &crate::suite::world::Planned,
    admission: &campaign_core::CampaignControlEvent,
    binding: &campaign_core::ProjectionBinding,
) -> Result<(), String> {
    let mut request = planned.request.clone();
    request.expected_generation = binding.generation;
    request.expected_head_id = binding.head_id;
    let plan = campaign_core::plan_next(&planned.state, request.clone(), binding)
        .map_err(crate::suite::s3_fixture::err)?;
    let crossed = campaign_core::CampaignError::CrossedIdentity;
    let decision = campaign_core::project_receipt(&planned.state, &plan, &request, binding);
    assert_eq!(crate::suite::s3_fixture::denial(decision)?, crossed);
    let profile = crate::suite::selection::ranked_profile()?;
    let recorded =
        crate::suite::certified::event_receipt(&planned.state, admission, &profile, binding);
    assert_eq!(crate::suite::s3_fixture::denial(recorded)?, crossed);
    Ok(())
}

/// A mirror branch name, a generation ahead of the history, and the admission binding itself
/// (stale by the control-event count) are crossed for decision and event receipts, while the
/// current binding projects both.
#[test]
// r[verify campaign.history_projection.caller_binding]
// r[verify campaign.receipts.crossed]
fn receipts_refuse_another_branch_name_or_generation() -> Result<(), String> {
    let planned = crate::suite::world::selecting()?;
    let current = planned.binding.clone();
    let admitted = planned.state.admission.projection.clone();
    let expected = admitted
        .generation
        .checked_add(planned.state.control_event_count)
        .ok_or("generation overflow")?;
    assert_eq!(current.generation, expected);
    crate::suite::world::observed(crate::suite::world::project(&planned))?;
    let (admission_events, _admitted_state) = crate::suite::admission::initial_state()?;
    let admission = &admission_events[0];
    assert!(planned.state.event_ids.contains(&admission.event_id()));
    let profile = crate::suite::selection::ranked_profile()?;
    let honest =
        crate::suite::certified::event_receipt(&planned.state, admission, &profile, &current);
    crate::suite::world::observed(honest)?;
    let mirror = rederived(&current, |binding| {
        binding.control_branch_name_id =
            crate::suite::admission::xid(campaign_core::IdKind::ControlBranch, b"mirror");
    })?;
    let ahead = rederived(&current, |binding| {
        binding.generation = binding.generation.saturating_add(1000);
    })?;
    for binding in [&mirror, &ahead, &admitted] {
        crossed_under(&planned, admission, binding)?;
    }
    Ok(())
}

/// An admitted binding whose branch name is edited under an unchanged state identity does not
/// re-derive, so it cannot make a mirror branch current: state currency, which re-derives the
/// admission and its admitted binding, refuses the state before its history binding is checked.
#[test]
// r[verify campaign.history_projection.caller_binding]
// r[verify campaign.identity.admission_current]
fn an_edited_admitted_branch_name_does_not_rederive() -> Result<(), String> {
    let planned = crate::suite::world::selecting()?;
    let mirror = rederived(&planned.binding, |binding| {
        binding.control_branch_name_id =
            crate::suite::admission::xid(campaign_core::IdKind::ControlBranch, b"mirror");
    })?;
    let mut edited = planned.state.clone();
    edited.admission.projection.control_branch_name_id = mirror.control_branch_name_id;
    edited
        .recompute_identity()
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(edited.state_id, planned.state.state_id);
    let receipt = campaign_core::project_receipt(&edited, &planned.plan, &planned.request, &mirror);
    assert_eq!(
        crate::suite::s3_fixture::denial(receipt)?,
        campaign_core::CampaignError::StaleState
    );
    Ok(())
}

/// Campaign core cannot observe Choregraph: a binding of another head and graph at the state's exact
/// current generation is receipted as supplied, each plan and event receipt binding the supplied
/// projection, while a request fenced on the state's own head and the sibling at another
/// generation are crossed.
#[test]
// r[verify campaign.receipts.crossed]
fn a_sibling_head_at_the_current_generation_is_receipted_as_supplied() -> Result<(), String> {
    let planned = crate::suite::world::selecting()?;
    let current = planned.binding.clone();
    let sibling = rederived(&current, |binding| {
        binding.head_id = crate::suite::admission::xid(campaign_core::IdKind::Head, b"sibling");
        binding.graph_id = crate::suite::admission::xid(campaign_core::IdKind::Graph, b"sibling");
    })?;
    assert_eq!(sibling.generation, current.generation);
    assert_ne!(sibling.projection_id, current.projection_id);
    let (admission_events, _admitted_state) = crate::suite::admission::initial_state()?;
    let admission = &admission_events[0];
    let profile = crate::suite::selection::ranked_profile()?;
    for binding in [&current, &sibling] {
        let mut request = planned.request.clone();
        request.expected_generation = binding.generation;
        request.expected_head_id = binding.head_id;
        let plan = campaign_core::plan_next(&planned.state, request.clone(), binding);
        let plan = crate::suite::world::observed(plan)?;
        let decision = campaign_core::project_receipt(&planned.state, &plan, &request, binding);
        let decision = crate::suite::world::observed(decision)?;
        assert_eq!(decision.projection_id, binding.projection_id);
        let recorded =
            crate::suite::certified::event_receipt(&planned.state, admission, &profile, binding);
        let recorded = crate::suite::world::observed(recorded)?;
        assert_eq!(recorded.projection_id, binding.projection_id);
    }
    let fenced_on_own_head =
        campaign_core::project_receipt(&planned.state, &planned.plan, &planned.request, &sibling);
    assert_eq!(
        crate::suite::s3_fixture::denial(fenced_on_own_head)?,
        campaign_core::CampaignError::CrossedIdentity
    );
    let sibling_ahead = rederived(&sibling, |binding| {
        binding.generation = binding.generation.saturating_add(1);
    })?;
    crossed_under(&planned, admission, &sibling_ahead)
}
