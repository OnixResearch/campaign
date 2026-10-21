pub(crate) fn record(
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    cost: u64,
) -> campaign_core::CandidateRecord {
    campaign_core::CandidateRecord {
        candidate_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        parent_moment_id: state.admission.roots[0].moment_id,
        operation_profile_id: crate::suite::admission::tid(
            campaign_core::IdKind::OperationProfile,
            label,
        ),
        operation_schema_id: crate::suite::admission::tid(
            campaign_core::IdKind::OperationSchema,
            b"adversarial-operation-schema",
        ),
        source_adapter_id: state.admission.adapter_id,
        depth: crate::suite::constants::CANDIDATE_DEPTH,
        cost_profile_id: crate::suite::admission::tid(campaign_core::IdKind::CostProfile, label),
        declared_costs: vec![campaign_core::ResourceAmount {
            dimension_id: state.admission.resource_budgets[0].dimension_id,
            amount: cost,
        }],
        operation_generation_profile_id: crate::suite::admission::tid(
            campaign_core::IdKind::Policy,
            b"adversarial-operation-generation",
        ),
    }
}
pub(crate) fn guidance(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    ordering_profile_id: campaign_core::Id,
) -> campaign_core::GuidanceRecord {
    campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation: crate::suite::constants::GUIDANCE_GENERATION,
        ranks: vec![crate::suite::constants::RANK_VALID],
        source_facts_id: crate::suite::admission::tid(
            campaign_core::IdKind::SourceFacts,
            b"adversarial-source-facts",
        ),
        ordering_profile_id,
        adapter_id: state.admission.adapter_id,
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
        crate::suite::admission::tid(
            campaign_core::IdKind::CoalescingPolicy,
            b"adversarial-coalesce",
        ),
    ))
    .map_err(|error| format!("{error:?}"))?;
    let campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) = &event else {
        return Err(String::from(
            "candidate admission returns a candidate event",
        ));
    };
    let candidate_id = batch.candidates[0].candidate_id;
    events.push(event);
    let next_state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        next_state.admission.campaign_id,
        state.admission.campaign_id
    );
    assert!(
        next_state
            .candidates
            .iter()
            .any(|entry| entry.candidate.candidate_id == candidate_id)
    );
    Ok((events, next_state, candidate_id))
}
pub(crate) fn guide(
    mut events: Vec<campaign_core::CampaignControlEvent>,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
) -> Result<
    (
        Vec<campaign_core::CampaignControlEvent>,
        campaign_core::FrontierState,
    ),
    String,
> {
    let event = (campaign_core::admit_guidance(
        state,
        guidance(state, candidate_id, state.admission.guidance_profile_id),
    ))
    .map_err(|error| format!("{error:?}"))?;
    events.push(event);
    let next_state = (campaign_core::reconstruct(&events)).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        next_state.admission.campaign_id,
        state.admission.campaign_id
    );
    assert!(
        next_state
            .candidates
            .iter()
            .filter_map(|entry| entry.latest_guidance.as_ref())
            .any(|record| record.candidate_id == candidate_id)
    );
    Ok((events, next_state))
}
