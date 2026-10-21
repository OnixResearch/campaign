pub(crate) fn selectable() -> Result<crate::suite::history::Fixture, String> {
    let mut history = crate::suite::admission::initialized()?;
    let initial = history.current()?.state().clone();
    let candidate = record(&initial)?;
    let candidate_id = candidate.candidate_id;
    let event = crate::suite::identity::observed(campaign_core::admit_candidates(
        &initial,
        vec![candidate],
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::identity::derive(campaign_core::IdKind::CoalescingPolicy, b"loop-coalescing"),
    ))?;
    history = history.append(event)?;
    let current = history.current()?.state().clone();
    let guidance = crate::suite::identity::observed(campaign_core::admit_guidance(
        &current,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: current.admission.campaign_id,
            candidate_id,
            generation: 1,
            ranks: vec![crate::suite::constants::CANDIDATE_RANK],
            source_facts_id: crate::suite::identity::derive(
                campaign_core::IdKind::SourceFacts,
                b"loop-facts",
            ),
            ordering_profile_id: current.admission.guidance_profile_id,
            adapter_id: crate::suite::identity::adapter(),
        },
    ))?;
    history = history.append(guidance)?;
    let projected = history.current()?.state().clone();
    assert_eq!(projected.candidates.len(), 1);
    assert!(
        projected
            .candidates
            .iter()
            .filter_map(|entry| entry.latest_guidance.as_ref())
            .any(|record| record.candidate_id == candidate_id)
    );
    Ok(history)
}
fn record(
    initial: &campaign_core::FrontierState,
) -> Result<campaign_core::CandidateRecord, String> {
    crate::suite::identity::observed(
        campaign_core::CandidateRecord {
            candidate_id: campaign_core::Id::absent(),
            campaign_id: initial.admission.campaign_id,
            parent_moment_id: initial.admission.roots[0].moment_id,
            operation_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::OperationProfile,
                b"loop-operation",
            ),
            operation_schema_id: crate::suite::identity::derive(
                campaign_core::IdKind::OperationSchema,
                b"loop-operation-schema",
            ),
            source_adapter_id: crate::suite::identity::adapter(),
            depth: 1,
            cost_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::CostProfile,
                b"loop-cost",
            ),
            declared_costs: vec![campaign_core::ResourceAmount {
                dimension_id: initial.admission.resource_budgets[0].dimension_id,
                amount: 1,
            }],
            operation_generation_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::Policy,
                b"loop-generation",
            ),
        }
        .derive(),
    )
}
