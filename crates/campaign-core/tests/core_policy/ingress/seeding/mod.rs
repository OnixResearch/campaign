//! Root-based dynamic campaign admission.

const CHILD_DEPTH: u64 = 2;

fn root(
    input: &campaign_core::CampaignAdmission,
    label: &'static [u8],
) -> campaign_core::RootMoment {
    campaign_core::RootMoment {
        moment_id: crate::suite::admission::xid(campaign_core::IdKind::RootMoment, label),
        projection_id: input.projection.projection_id,
        adapter_id: input.projection.adapter_id,
    }
}

#[test]
// r[verify campaign.initial_admission.valid]
fn bounded_unique_roots_give_one_admitted_campaign_decision() -> Result<(), String> {
    let mut input = crate::suite::admission::default_input()?;
    let second = root(&input, b"second-root");
    input.roots.push(second);
    let event = crate::suite::ingress::support::text(campaign_core::admit_campaign(input))?;
    let campaign_core::CampaignControlEvent::CampaignAdmitted {
        event_id,
        admission,
    } = &event
    else {
        return Err(format!("expected a campaign admission, got {event:?}"));
    };
    assert_eq!(
        *event_id,
        crate::suite::ingress::support::text(campaign_core::derive_campaign_event_id(admission))?
    );
    let state = crate::suite::ingress::support::text(campaign_core::reconstruct(
        core::slice::from_ref(&event),
    ))?;
    assert_eq!(state.event_ids, vec![*event_id]);
    for admitted_root in &admission.roots {
        assert_eq!(
            state.known_moment_depth(admitted_root.moment_id),
            Some(campaign_core::frontier::ROOT_DEPTH)
        );
    }
    assert!(state.candidates.is_empty());
    Ok(())
}

#[test]
// r[verify campaign.initial_admission.invalid]
fn invalid_roots_fail_before_publication() -> Result<(), String> {
    let base = crate::suite::admission::default_input()?;
    let mut empty = base.clone();
    empty.roots.clear();
    let mut duplicate = base.clone();
    duplicate.roots.push(base.roots[0].clone());
    let mut crossed_projection = base.clone();
    crossed_projection.roots[0].projection_id =
        crate::suite::admission::xid(campaign_core::IdKind::Projection, b"other-history");
    let mut crossed_adapter = base.clone();
    crossed_adapter.roots[0].adapter_id =
        crate::suite::admission::xid(campaign_core::IdKind::Adapter, b"other-adapter");
    let mut crossed_domain = base.clone();
    crossed_domain.control_domain_id =
        crate::suite::admission::xid(campaign_core::IdKind::ControlDomain, b"other-domain");
    let mut over_limit = base.clone();
    over_limit.bounds.max_roots = crate::suite::constants::COST_ONE;
    over_limit.bounds = crate::suite::ingress::support::text(over_limit.bounds.derive())?;
    let second = root(&base, b"second-root");
    over_limit.roots.push(second);
    assert_eq!(over_limit.roots.len(), 2);
    assert!(empty.roots.is_empty());
    for (input, expected) in [
        (empty, campaign_core::CampaignError::EmptyRoots),
        (duplicate, campaign_core::CampaignError::DuplicateRoot),
        (
            crossed_projection,
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (
            crossed_adapter,
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (
            crossed_domain,
            campaign_core::CampaignError::CrossedIdentity,
        ),
        (over_limit, campaign_core::CampaignError::BoundExceeded),
    ] {
        assert_eq!(campaign_core::admit_campaign(input), Err(expected));
    }
    Ok(())
}

#[test]
// r[verify campaign.initial_admission.dynamic]
fn admission_needs_no_operation_inventory_and_observations_introduce_candidates()
-> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    assert!(state.candidates.is_empty());
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"first-operation",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (events, state) = crate::suite::selection::publish(events, selection.clone())?;
    let result = crate::suite::ingress::support::text(campaign_core::admit_result(
        &state,
        crate::suite::observation::result_for(&state, &selection, b"observed-child", b"child")?,
    ))?;
    let (events, state) = crate::suite::ingress::support::accept(events, result)?;
    let child_id =
        crate::suite::admission::xid(campaign_core::IdKind::ChildMoment, b"observed-child");
    let later = campaign_core::CandidateRecord {
        parent_moment_id: child_id,
        depth: CHILD_DEPTH,
        ..crate::suite::candidate::record(
            &state,
            b"operation-unknown-at-admission",
            crate::suite::constants::COST_ONE,
        )
    };
    let event = crate::suite::ingress::support::text(crate::suite::ingress::support::admit_batch(
        &state,
        vec![later],
        crate::suite::ingress::support::REJECT,
    ))?;
    let (_events, state) = crate::suite::ingress::support::accept(events, event)?;
    assert!(state.candidates.iter().any(|entry| {
        entry.candidate.parent_moment_id == child_id && entry.candidate.depth == CHILD_DEPTH
    }));
    Ok(())
}
