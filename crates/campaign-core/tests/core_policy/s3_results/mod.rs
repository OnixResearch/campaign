//! Exact results: valid, crossed, duplicate, child bound, derived records, and retry boundary.

mod identity_checks;
mod retry_precedence;

const ONE: u64 = 1;

fn derived(label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::DerivedRecord, label)
}

#[test]
// r[verify campaign.fd.reservation.ordering]
// r[verify campaign.observations]
// r[verify campaign.observations.valid]
fn an_exact_result_reserves_every_child_binds_derived_records_and_settles() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"valid")?;
    let children = vec![
        crate::suite::s3_fixture::child(&state, &selection, b"valid-left", b"valid-left-event")?,
        crate::suite::s3_fixture::child(&state, &selection, b"valid-right", b"valid-right-event")?,
    ];
    let mut observation =
        crate::suite::s3_fixture::observation(&state, &selection, children.clone());
    observation.derived_records = vec![derived(b"valid-record-b"), derived(b"valid-record-a")];
    let event =
        campaign_core::admit_result(&state, observation).map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::ResultObserved(result) = event.clone() else {
        return Err(format!("expected a result event, got {event:?}"));
    };
    let mut rebound = result.observation.clone();
    rebound.derived_records = vec![derived(b"valid-record-a")];
    assert_ne!(
        rebound
            .derive()
            .map_err(crate::suite::s3_fixture::err)?
            .result_id,
        result.observation.result_id
    );
    let (_events, settled) = crate::suite::s3_fixture::append(events, event)?;
    assert!(settled.outstanding.is_empty());
    assert_eq!(
        settled.settled,
        vec![campaign_core::Settlement {
            selection_id: selection.selection_id,
            settlement_id: result.observation.result_id,
        }]
    );
    assert_eq!(settled.accepted_results, vec![result.observation.result_id]);
    for child in &children {
        assert!(settled.reservations.contains(&child.reservation_id));
        assert!(settled.has_known_moment(child.child_moment_id));
        assert!(settled.discovery(child.child_moment_id).is_some());
    }
    assert_eq!(settled.candidates.len(), state.candidates.len());
    Ok(())
}

#[test]
fn derived_records_are_bounded_typed_and_unique_at_entry_and_replay() -> Result<(), String> {
    let (events, state) =
        crate::suite::s3_fixture::admitted_with(|bounds| bounds.max_derived_records = ONE)?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"derived")?;
    let mut over = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    over.derived_records = vec![derived(b"first"), derived(b"second")];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, over.clone()))?,
        campaign_core::CampaignError::BoundExceeded
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, over)?
        )?,
        campaign_core::CampaignError::BoundExceeded
    );
    let mut wrong_kind = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    wrong_kind.derived_records = vec![crate::suite::admission::xid(
        campaign_core::IdKind::Finding,
        b"not-derived",
    )];
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, wrong_kind))?,
        campaign_core::CampaignError::CrossedIdentity
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    let mut one = crate::suite::s3_fixture::observation(&state, &selection, Vec::new());
    one.derived_records = vec![derived(b"first")];
    campaign_core::admit_result(&state, one).map_err(crate::suite::s3_fixture::err)?;
    Ok(())
}

#[test]
// r[verify campaign.observations.duplicate]
// r[verify campaign.retry_boundary]
// r[verify campaign.retry_boundary.invalid]
fn a_settled_selection_accepts_no_second_settlement() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"retry")?;
    let accepted = crate::suite::s3_fixture::observation(
        &state,
        &selection,
        vec![crate::suite::s3_fixture::child(
            &state,
            &selection,
            b"retry-child",
            b"retry-event",
        )?],
    );
    let event = campaign_core::admit_result(&state, accepted.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let (events, settled) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&settled, accepted.clone()))?,
        campaign_core::CampaignError::DuplicateResult
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&settled, accepted)?
        )?,
        campaign_core::CampaignError::DuplicateResult
    );
    let new_fork = crate::suite::s3_fixture::observation(
        &settled,
        &selection,
        vec![crate::suite::s3_fixture::child(
            &settled,
            &selection,
            b"retry-fork",
            b"retry-fork-event",
        )?],
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&settled, new_fork.clone()))?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&settled, new_fork)?
        )?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::close_unobserved(
            &settled,
            crate::suite::s3_fixture::closure(&settled, &selection, b"retry-closure", true)?,
        ))?,
        campaign_core::CampaignError::InvalidRetryBoundary
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &settled)
}

#[test]
// r[verify campaign.observations.child_bound]
fn too_many_or_duplicate_children_are_denied_at_entry_and_replay() -> Result<(), String> {
    let (events, state) =
        crate::suite::s3_fixture::admitted_with(|bounds| bounds.max_child_moments = ONE)?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"bound")?;
    let left =
        crate::suite::s3_fixture::child(&state, &selection, b"bound-left", b"bound-left-event")?;
    let right =
        crate::suite::s3_fixture::child(&state, &selection, b"bound-right", b"bound-right-event")?;
    let too_many =
        crate::suite::s3_fixture::observation(&state, &selection, vec![left.clone(), right]);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, too_many.clone()))?,
        campaign_core::CampaignError::BoundExceeded
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, too_many)?
        )?,
        campaign_core::CampaignError::BoundExceeded
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) =
        crate::suite::s3_fixture::selected(events, &state, b"duplicate")?;
    let child = crate::suite::s3_fixture::child(
        &state,
        &selection,
        b"duplicate-child",
        b"duplicate-event",
    )?;
    let repeated =
        crate::suite::s3_fixture::observation(&state, &selection, vec![child.clone(), child]);
    assert_eq!(
        crate::suite::s3_fixture::denial(campaign_core::admit_result(&state, repeated.clone()))?,
        campaign_core::CampaignError::DuplicateChildMoment
    );
    assert_eq!(
        crate::suite::s3_fixture::replay_denial(
            &events,
            crate::suite::s3_fixture::forged_result(&state, repeated)?
        )?,
        campaign_core::CampaignError::DuplicateChildMoment
    );
    crate::suite::s3_fixture::assert_unchanged(&events, &state)
}

#[test]
// r[verify campaign.retry_boundary.valid]
fn an_infrastructure_retry_reuses_the_published_selection_and_operation() -> Result<(), String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_outstanding = crate::suite::constants::COST_ONE;
    })?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"infra")?;
    let evidence = campaign_core::PublicationEvidence {
        event_id: selection.publication_plan.event_id,
        selection_id: selection.selection_id,
        branch_before: campaign_core::BranchFact {
            control_branch_id: selection.publication_plan.control_branch_id,
            head_id: selection.publication_plan.expected_head_id,
            generation: selection.publication_plan.expected_generation,
        },
        branch_after: campaign_core::BranchFact {
            control_branch_id: selection.publication_plan.control_branch_id,
            head_id: crate::suite::admission::xid(campaign_core::IdKind::Head, b"infra-after"),
            generation: selection
                .publication_plan
                .expected_generation
                .checked_add(ONE)
                .ok_or_else(|| String::from("generation overflow"))?,
        },
        durable_event: true,
        durable_branch_update: true,
    };
    let is_first_attempt_allowed =
        campaign_core::can_execute_published_selection(&selection, &evidence)
            .map_err(crate::suite::s3_fixture::err)?;
    let is_retry_allowed = campaign_core::can_execute_published_selection(&selection, &evidence)
        .map_err(crate::suite::s3_fixture::err)?;
    assert!(is_first_attempt_allowed);
    assert_eq!(is_first_attempt_allowed, is_retry_allowed);
    assert!(matches!(
        crate::suite::s3_fixture::plan_decision(&state)?,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingCapacity)
    ));
    assert_eq!(state.outstanding.len(), 1);
    assert_eq!(state.outstanding[0].operation_id, selection.operation_id);
    let result = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &selection, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, settled) = crate::suite::s3_fixture::append(events, result)?;
    assert_eq!(settled.selection_count, state.selection_count);
    assert_eq!(settled.candidates.len(), state.candidates.len());
    Ok(())
}

#[test]
// r[verify campaign.budgets.no_refund]
fn a_failed_result_does_not_refund_reserved_costs() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, selection) = crate::suite::s3_fixture::selected(events, &state, b"refund")?;
    let reserved = state.reserved_resources.clone();
    assert_eq!(reserved[0].amount, crate::suite::constants::COST_ONE);
    let failed = campaign_core::admit_result(
        &state,
        crate::suite::s3_fixture::observation(&state, &selection, Vec::new()),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (_events, settled) = crate::suite::s3_fixture::append(events, failed)?;
    assert!(settled.outstanding.is_empty());
    assert_eq!(settled.reserved_resources, reserved);
    Ok(())
}
