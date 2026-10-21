//! Exact current-projection binding for planning (`campaign.history_projection`).

const ADVANCED_GENERATION: u64 = 11;
const OLDER_GENERATION: u64 = 10;

fn advanced(
    admitted: &campaign_core::ProjectionBinding,
    edit: impl FnOnce(&mut campaign_core::ProjectionBinding),
) -> Result<campaign_core::ProjectionBinding, String> {
    let mut binding = admitted.clone();
    binding.graph_id = crate::suite::admission::xid(campaign_core::IdKind::Graph, b"graph-now");
    binding.control_branch_id =
        crate::suite::admission::xid(campaign_core::IdKind::ControlBranch, b"branch-now");
    binding.generation = ADVANCED_GENERATION;
    binding.head_id = crate::suite::admission::xid(campaign_core::IdKind::Head, b"head-now");
    edit(&mut binding);
    crate::suite::planning_support::observed(binding.derive())
}

fn request_for(
    state: &campaign_core::FrontierState,
    current: &campaign_core::ProjectionBinding,
) -> Result<campaign_core::SelectionRequest, String> {
    let mut request =
        crate::suite::selection::request(state, crate::suite::selection::ranked_profile()?, None);
    request.expected_generation = current.generation;
    request.expected_head_id = current.head_id;
    Ok(request)
}

fn selectable() -> Result<crate::suite::planning_support::History, String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, _candidate_id) = crate::suite::planning_support::guided(
        events,
        &state,
        b"projection-candidate",
        crate::suite::constants::RANK_HIGH,
    )?;
    Ok((events, state))
}

#[test]
// r[verify campaign.history_projection.valid]
fn a_current_projection_binds_the_selection_fence() -> Result<(), String> {
    let (_events, state) = selectable()?;
    let current = advanced(&state.admission.projection, |_| {})?;
    let plan = crate::suite::planning_support::observed(campaign_core::plan_next(
        &state,
        request_for(&state, &current)?,
        &current,
    ))?;
    let selection = crate::suite::planning_support::selected(&plan)?;
    assert_eq!(selection.state_before_id, state.state_id);
    assert_eq!(
        selection.publication_plan.control_branch_id,
        current.control_branch_id
    );
    assert_eq!(selection.publication_plan.expected_head_id, current.head_id);
    assert_eq!(
        selection.publication_plan.expected_generation,
        current.generation
    );
    Ok(())
}

#[test]
// r[verify campaign.history_projection.stale]
fn a_stale_or_mismatched_projection_is_denied_without_a_decision() -> Result<(), String> {
    let (events, state) = selectable()?;
    let current = advanced(&state.admission.projection, |_| {})?;
    let mut stale_generation = request_for(&state, &current)?;
    stale_generation.expected_generation = OLDER_GENERATION;
    let mut wrong_head = request_for(&state, &current)?;
    wrong_head.expected_head_id =
        crate::suite::admission::xid(campaign_core::IdKind::Head, b"head-forged");
    for request in [stale_generation, wrong_head] {
        assert_eq!(
            campaign_core::plan_next(&state, request, &current),
            Err(campaign_core::CampaignError::StaleProjection)
        );
    }
    let mut forged_graph = current.clone();
    forged_graph.graph_id =
        crate::suite::admission::xid(campaign_core::IdKind::Graph, b"graph-forged");
    assert_eq!(
        campaign_core::plan_next(&state, request_for(&state, &current)?, &forged_graph),
        Err(campaign_core::CampaignError::StaleProjection)
    );
    let unsupported_schema = advanced(&state.admission.projection, |binding| {
        binding.projection_schema_id =
            crate::suite::admission::xid(campaign_core::IdKind::Projection, b"other-schema");
    })?;
    let other_adapter = advanced(&state.admission.projection, |binding| {
        binding.adapter_id =
            crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"other-adapter");
    })?;
    let other_domain = advanced(&state.admission.projection, |binding| {
        binding.control_domain_id =
            crate::suite::admission::xid(campaign_core::IdKind::ControlDomain, b"other-domain");
    })?;
    for crossed in [unsupported_schema, other_adapter, other_domain] {
        assert_eq!(
            campaign_core::plan_next(&state, request_for(&state, &crossed)?, &crossed),
            Err(campaign_core::CampaignError::CrossedIdentity)
        );
    }
    assert_eq!(
        crate::suite::planning_support::observed(campaign_core::reconstruct(&events))?.state_id,
        state.state_id
    );
    Ok(())
}
