//! Campaigns admitted, supplied with candidates, and guided under one host configuration.

pub(crate) mod root;

/// The suite's admission bound at `generation` on the exploration root, admitted through
/// `admit_control_root` as the control root of a graph of the suite's standard event limit whose
/// projection bound and host `maximum_events` are both `maximum_events`.
pub(crate) fn admitted_at(
    generation: u64,
    maximum_events: u32,
) -> Result<crate::suite::configured::Hosted, String> {
    admitted_under(root::Limits::standard(generation, maximum_events))
}

/// The suite's admission admitted as the control root of a graph under `limits`.
pub(crate) fn admitted_under(
    limits: root::Limits,
) -> Result<crate::suite::configured::Hosted, String> {
    root::Unadmitted::bound(limits)?.hosted()
}

/// One candidate batch of `count` candidates published over `hosted`.
pub(crate) fn with_candidates(
    hosted: crate::suite::configured::Hosted,
    count: usize,
) -> Result<(crate::suite::configured::Hosted, Vec<campaign_core::Id>), String> {
    let held = crate::suite::identity::observed(hosted.validate())?;
    let state = held.state();
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let label = [b"hosted-operation-".as_slice(), &index.to_le_bytes()].concat();
        records.push(crate::suite::identity::observed(
            campaign_core::CandidateRecord {
                candidate_id: campaign_core::Id::absent(),
                campaign_id: state.admission.campaign_id,
                parent_moment_id: state.admission.roots[crate::suite::constants::FIRST_ROOT_INDEX]
                    .moment_id,
                operation_profile_id: crate::suite::identity::derive(
                    campaign_core::IdKind::OperationProfile,
                    &label,
                ),
                operation_schema_id: state.admission.operation_schema_ids[0],
                source_adapter_id: crate::suite::identity::adapter(),
                depth: crate::suite::constants::BRANCH_EVENT_INCREMENT,
                cost_profile_id: crate::suite::identity::derive(
                    campaign_core::IdKind::CostProfile,
                    b"hosted-cost",
                ),
                declared_costs: vec![campaign_core::ResourceAmount {
                    dimension_id: state.admission.resource_budgets
                        [crate::suite::constants::FIRST_RESOURCE_INDEX]
                        .dimension_id,
                    amount: crate::suite::constants::BRANCH_EVENT_INCREMENT,
                }],
                operation_generation_profile_id: crate::suite::identity::derive(
                    campaign_core::IdKind::Policy,
                    b"hosted-generation",
                ),
            }
            .derive(),
        )?);
    }
    let candidate_ids: Vec<campaign_core::Id> =
        records.iter().map(|record| record.candidate_id).collect();
    assert_eq!(candidate_ids.len(), count);
    let batch = crate::suite::identity::observed(campaign_core::admit_candidates(
        state,
        records,
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::identity::derive(
            campaign_core::IdKind::CoalescingPolicy,
            b"hosted-coalescing",
        ),
    ))?;
    let (hosted, publication) = hosted.append(batch)?;
    assert_eq!(
        publication.branch_plan().after.head,
        publication.history_event_id()
    );
    Ok((hosted, candidate_ids))
}

/// The guidance event for `candidate_id` on the loaded state of `hosted`.
pub(crate) fn guidance_for(
    held: &campaign_choregraph::ValidatedControlProjection,
    candidate_id: campaign_core::Id,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let state = held.state();
    crate::suite::identity::observed(campaign_core::admit_guidance(
        state,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            candidate_id,
            generation: crate::suite::constants::BRANCH_EVENT_INCREMENT,
            ranks: vec![crate::suite::constants::CANDIDATE_RANK],
            source_facts_id: crate::suite::identity::derive(
                campaign_core::IdKind::SourceFacts,
                b"hosted-facts",
            ),
            ordering_profile_id: state.admission.guidance_profile_id,
            adapter_id: crate::suite::identity::adapter(),
        },
    ))
}

/// A candidate batch of `count` candidates and guidance for each, published over `hosted`.
pub(crate) fn guided(
    hosted: crate::suite::configured::Hosted,
    count: usize,
) -> Result<crate::suite::configured::Hosted, String> {
    let (mut hosted, candidate_ids) = with_candidates(hosted, count)?;
    for candidate_id in candidate_ids {
        let held = crate::suite::identity::observed(hosted.validate())?;
        hosted = hosted.append(guidance_for(&held, candidate_id)?)?.0;
    }
    Ok(hosted)
}
