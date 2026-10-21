pub(super) const CANDIDATE_STATUS_RETAINED: u32 = 1;
pub(super) const CANDIDATE_STATUS_RETIRED: u32 = 2;
pub(super) const CANDIDATE_STATUS_PRUNED: u32 = 3;
pub(super) fn write(
    writer: crate::identity::FrameWriter,
    candidates: &[crate::model::CandidateState],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(candidates.len())?);
    for candidate in candidates {
        writer = crate::frontier::encoding::candidates::write_candidate_record(
            writer,
            &candidate.candidate,
        )?;
        writer = crate::frontier::encoding::candidates::write_candidate_status(
            writer,
            &candidate.status,
        );
        writer = writer.u64(candidate.selection_count);
        writer = crate::frontier::encoding::candidates::write_guidance(
            writer,
            candidate.latest_guidance.as_ref(),
        )?;
    }
    Ok(writer)
}
pub(super) fn write_candidate_record(
    writer: crate::identity::FrameWriter,
    candidate: &crate::model::CandidateRecord,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let writer = writer
        .id(candidate.candidate_id)
        .id(candidate.campaign_id)
        .id(candidate.parent_moment_id)
        .id(candidate.operation_profile_id)
        .id(candidate.operation_schema_id)
        .id(candidate.source_adapter_id)
        .u64(candidate.depth)
        .id(candidate.cost_profile_id);
    let writer = crate::model::write_resources(writer, &candidate.declared_costs)?;
    Ok(writer.id(candidate.operation_generation_profile_id))
}
pub(super) fn write_candidate_status(
    writer: crate::identity::FrameWriter,
    status: &crate::model::CandidateStatus,
) -> crate::identity::FrameWriter {
    match status {
        crate::model::CandidateStatus::Retained => {
            writer.u32(crate::frontier::encoding::candidates::CANDIDATE_STATUS_RETAINED)
        }
        crate::model::CandidateStatus::Retired { reason_id } => writer
            .u32(crate::frontier::encoding::candidates::CANDIDATE_STATUS_RETIRED)
            .id(*reason_id),
        crate::model::CandidateStatus::Pruned { prune_plan_id } => writer
            .u32(crate::frontier::encoding::candidates::CANDIDATE_STATUS_PRUNED)
            .id(*prune_plan_id),
    }
}
pub(super) fn write_guidance(
    writer: crate::identity::FrameWriter,
    guidance: Option<&crate::model::GuidanceRecord>,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let Some(guidance) = guidance else {
        return Ok(writer.option_id(None));
    };
    let mut writer = writer
        .option_id(Some(guidance.guidance_id))
        .id(guidance.campaign_id)
        .id(guidance.candidate_id)
        .u64(guidance.generation)
        .seq_len(crate::identity::usize_to_u64(guidance.ranks.len())?);
    for rank in &guidance.ranks {
        writer = writer.i64(*rank);
    }
    Ok(writer
        .id(guidance.source_facts_id)
        .id(guidance.ordering_profile_id)
        .id(guidance.adapter_id))
}
