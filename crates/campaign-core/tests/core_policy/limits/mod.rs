const ONE: u64 = 1;
const TWO: u64 = 2;
const SECOND_ORDINAL: u64 = 2;
const UNSUPPORTED_SCHEMA_VERSION: u32 = u32::MAX;

fn admitted_with(
    configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    let mut input = crate::suite::admission::default_input()?;
    configure(&mut input.bounds);
    input.bounds = input
        .bounds
        .derive()
        .map_err(|error| format!("{error:?}"))?;
    crate::suite::admission::admitted(input)
}

fn plan_decision(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
) -> Result<campaign_core::DecisionPlan, String> {
    campaign_core::plan_next(
        state,
        crate::suite::selection::request(state, profile, None),
        &state.admission.projection,
    )
    .map_err(|error| format!("{error:?}"))
}

fn schema(label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::OperationSchema, label)
}

#[test]
fn candidate_batches_use_the_batch_bound_and_totals_use_the_candidate_bound() -> Result<(), String>
{
    let (_events, state) = admitted_with(|bounds| bounds.max_candidate_batch = ONE)?;
    let pair = vec![
        crate::suite::candidate::record(&state, b"first", ONE),
        crate::suite::candidate::record(&state, b"second", ONE),
    ];
    assert_eq!(
        campaign_core::admit_candidates(
            &state,
            pair,
            campaign_core::AdmissionRule {
                duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
            },
            crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
        ),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    let (events, state) = admitted_with(|bounds| {
        bounds.max_candidates = TWO;
        bounds.max_candidate_batch = ONE;
    })?;
    let (events, state, _first) = crate::suite::candidate::admit(events, &state, b"first", ONE)?;
    let (_events, state, _second) = crate::suite::candidate::admit(events, &state, b"second", ONE)?;
    assert_eq!(
        campaign_core::admit_candidates(
            &state,
            vec![crate::suite::candidate::record(&state, b"third", ONE)],
            campaign_core::AdmissionRule {
                duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
            },
            crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
        ),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    Ok(())
}

#[test]
// r[verify campaign.stop.selection_limit]
// r[verify campaign.wait.results]
// r[verify campaign.stop.precedence]
fn selection_limit_waits_for_outstanding_work_then_stops() -> Result<(), String> {
    let (events, state) = admitted_with(|bounds| bounds.max_selections = ONE)?;
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, &state, b"limited", ONE)?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    let selection = crate::suite::selection::plan(&state)?;
    let (mut events, state) = crate::suite::selection::publish(events, selection.clone())?;
    let waiting = plan_decision(&state, crate::suite::selection::ranked_profile()?)?;
    assert_eq!(
        waiting.decision,
        campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingResults)
    );
    let result = campaign_core::admit_result(
        &state,
        crate::suite::observation::result_for(&state, &selection, b"limit-child", b"payload")?,
    )
    .map_err(|error| format!("{error:?}"))?;
    events.push(result);
    let state = campaign_core::reconstruct(&events).map_err(|error| format!("{error:?}"))?;
    let stopping = plan_decision(&state, crate::suite::selection::ranked_profile()?)?;
    let campaign_core::NextDecision::Stop(stop) = &stopping.decision else {
        return Err(format!(
            "expected a selection-limit stop, got {:?}",
            stopping.decision
        ));
    };
    assert_eq!(
        stop.reason,
        campaign_core::StopReason::SelectionLimitReached
    );
    let stop_event = campaign_core::admit_stop(&state, stop.reason.clone())
        .map_err(|error| format!("{error:?}"))?;
    let mut stopped_events = events.clone();
    stopped_events.push(stop_event);
    let stopped =
        campaign_core::reconstruct(&stopped_events).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        stopped.stopped.map(|accepted| accepted.reason),
        Some(campaign_core::StopReason::SelectionLimitReached)
    );
    let over_limit = campaign_core::SelectionDecision {
        ordinal: SECOND_ORDINAL,
        state_before_id: state.state_id,
        publication_plan: campaign_core::PublicationPlan {
            state_before_id: state.state_id,
            ..selection.publication_plan.clone()
        },
        ..selection
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    events.push(crate::suite::selection::event(over_limit)?);
    assert_eq!(
        campaign_core::reconstruct(&events),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.oracle.isolated]
fn reference_evidence_implementation_is_never_planned() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, &state, b"reference", ONE)?;
    let (_events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    assert_eq!(
        crate::suite::selection::plan(&state)?.candidate_id,
        candidate_id
    );
    let reference = campaign_core::SelectionProfile {
        implementation: campaign_core::PolicyImplementation::ReferenceEvidence {
            reference_id: crate::suite::admission::xid(campaign_core::IdKind::Policy, b"oracle"),
        },
        ..crate::suite::selection::ranked_profile()?
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        campaign_core::plan_next(
            &state,
            crate::suite::selection::request(&state, reference, None),
            &state.admission.projection,
        ),
        Err(campaign_core::CampaignError::ReferencePolicySelected)
    );
    Ok(())
}

#[test]
fn admission_requires_the_supported_schema_version() -> Result<(), String> {
    assert_ne!(
        UNSUPPORTED_SCHEMA_VERSION,
        campaign_core::CAMPAIGN_SCHEMA_VERSION
    );
    let mut input = crate::suite::admission::default_input()?;
    input.schema_version = UNSUPPORTED_SCHEMA_VERSION;
    assert_eq!(
        campaign_core::admit_campaign(input),
        Err(campaign_core::CampaignError::UnsupportedVersion)
    );
    Ok(())
}

#[test]
fn operation_schema_set_is_canonical_and_bound_into_the_campaign_id() -> Result<(), String> {
    let mut forward = crate::suite::admission::default_input()?;
    forward.operation_schema_ids = vec![schema(b"schema-a"), schema(b"schema-b")];
    let mut reversed = forward.clone();
    reversed.operation_schema_ids.reverse();
    let forward = forward.derive().map_err(|error| format!("{error:?}"))?;
    let reversed = reversed.derive().map_err(|error| format!("{error:?}"))?;
    assert_eq!(forward.campaign_id, reversed.campaign_id);
    assert!(forward.operation_schema_ids.is_sorted());
    let mut narrowed = crate::suite::admission::default_input()?;
    narrowed.operation_schema_ids = vec![schema(b"schema-a")];
    let narrowed = narrowed.derive().map_err(|error| format!("{error:?}"))?;
    assert_ne!(forward.campaign_id, narrowed.campaign_id);
    Ok(())
}

#[test]
fn operation_schema_set_rejects_empty_over_limit_duplicate_and_crossed_ids() -> Result<(), String> {
    let base = crate::suite::admission::default_input()?;
    let mut empty = base.clone();
    empty.operation_schema_ids = Vec::new();
    assert_eq!(
        campaign_core::admit_campaign(empty),
        Err(campaign_core::CampaignError::UnsupportedSchema)
    );
    let mut over_limit = base.clone();
    over_limit.bounds.max_operation_schemas = ONE;
    over_limit.bounds = over_limit
        .bounds
        .derive()
        .map_err(|error| format!("{error:?}"))?;
    over_limit.operation_schema_ids = vec![schema(b"schema-a"), schema(b"schema-b")];
    assert_eq!(
        campaign_core::admit_campaign(over_limit),
        Err(campaign_core::CampaignError::BoundExceeded)
    );
    let mut duplicate = base.clone();
    duplicate.operation_schema_ids = vec![schema(b"schema-a"), schema(b"schema-a")];
    assert_eq!(
        campaign_core::admit_campaign(duplicate),
        Err(campaign_core::CampaignError::DuplicateIdentity)
    );
    let mut crossed = base;
    crossed.operation_schema_ids = vec![crate::suite::admission::xid(
        campaign_core::IdKind::OperationProfile,
        b"schema-a",
    )];
    assert_eq!(
        campaign_core::admit_campaign(crossed),
        Err(campaign_core::CampaignError::CrossedIdentity)
    );
    Ok(())
}
