pub(crate) fn record(
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    cost: u64,
) -> campaign_core::CandidateRecord {
    campaign_core::CandidateRecord {
        candidate_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        parent_moment_id: state.admission.roots[0].moment_id,
        operation_profile_id: crate::suite::admission::xid(
            campaign_core::IdKind::OperationProfile,
            label,
        ),
        operation_schema_id: crate::suite::admission::xid(
            campaign_core::IdKind::OperationSchema,
            b"operation-schema",
        ),
        source_adapter_id: state.admission.adapter_id,
        depth: crate::suite::constants::DEPTH_ROOT_CHILD,
        cost_profile_id: crate::suite::admission::xid(campaign_core::IdKind::CostProfile, label),
        declared_costs: vec![campaign_core::ResourceAmount {
            dimension_id: state.admission.resource_budgets[0].dimension_id,
            amount: cost,
        }],
        operation_generation_profile_id: crate::suite::admission::xid(
            campaign_core::IdKind::Policy,
            b"operation-generation",
        ),
    }
}
pub(crate) fn admit(
    mut events: Vec<campaign_core::CampaignControlEvent>,
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    cost: u64,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
        campaign_core::Id,
    ),
    String,
> {
    let event = (campaign_core::admit_candidates(
        state,
        vec![record(state, label, cost)],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    ))
    .map_err(|error| format!("{error:?}"))?;
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = &event else {
        return Err(String::from("fixture produced an unexpected event"));
    };
    let candidate_id = batch.candidates[0].candidate_id;
    events.push(event);
    let next = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(next.admission.campaign_id, state.admission.campaign_id);
    assert!(
        next.candidates
            .iter()
            .any(|entry| entry.candidate.candidate_id == candidate_id)
    );
    Ok((events, next, candidate_id))
}
pub(crate) fn guide(
    mut events: Vec<campaign_core::CampaignControlEvent>,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    rank: i64,
    generation: u64,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
        campaign_core::Id,
    ),
    String,
> {
    let guidance = campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation,
        ranks: vec![rank],
        source_facts_id: crate::suite::admission::xid(
            campaign_core::IdKind::SourceFacts,
            b"source-facts",
        ),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    };
    let event =
        (campaign_core::admit_guidance(state, guidance)).map_err(|error| format!("{error:?}"))?;
    let campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance_event) = &event else {
        return Err(String::from("fixture produced an unexpected event"));
    };
    let guidance_id = guidance_event.guidance.guidance_id;
    events.push(event);
    let next = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(next.admission.campaign_id, state.admission.campaign_id);
    assert!(
        next.candidates
            .iter()
            .filter_map(|entry| entry.latest_guidance.as_ref())
            .any(|record| record.guidance_id == guidance_id)
    );
    Ok((events, next, guidance_id))
}
