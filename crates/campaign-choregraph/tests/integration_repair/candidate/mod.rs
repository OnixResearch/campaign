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
        crate::suite::identity::derive(
            campaign_core::IdKind::CoalescingPolicy,
            b"repair-loop-coalescing",
        ),
    ))?;
    history = history.append(event)?.0;
    let current = history.current()?.state().clone();
    let guidance = crate::suite::identity::observed(campaign_core::admit_guidance(
        &current,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: current.admission.campaign_id,
            candidate_id,
            generation: crate::suite::constants::BRANCH_EVENT_INCREMENT,
            ranks: vec![crate::suite::constants::CANDIDATE_RANK],
            source_facts_id: crate::suite::identity::derive(
                campaign_core::IdKind::SourceFacts,
                b"repair-loop-facts",
            ),
            ordering_profile_id: current.admission.guidance_profile_id,
            adapter_id: crate::suite::identity::adapter(),
        },
    ))?;
    history = history.append(guidance)?.0;
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
            parent_moment_id: initial.admission.roots[crate::suite::constants::FIRST_ROOT_INDEX]
                .moment_id,
            operation_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::OperationProfile,
                b"repair-loop-operation",
            ),
            operation_schema_id: crate::suite::identity::derive(
                campaign_core::IdKind::OperationSchema,
                b"repair-loop-operation-schema",
            ),
            source_adapter_id: crate::suite::identity::adapter(),
            depth: crate::suite::constants::BRANCH_EVENT_INCREMENT,
            cost_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::CostProfile,
                b"repair-loop-cost",
            ),
            declared_costs: vec![campaign_core::ResourceAmount {
                dimension_id: initial.admission.resource_budgets
                    [crate::suite::constants::FIRST_RESOURCE_INDEX]
                    .dimension_id,
                amount: crate::suite::constants::BRANCH_EVENT_INCREMENT,
            }],
            operation_generation_profile_id: crate::suite::identity::derive(
                campaign_core::IdKind::Policy,
                b"repair-loop-generation",
            ),
        }
        .derive(),
    )
}
