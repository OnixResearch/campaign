pub(super) const STOP_REASON_FRONTIER: u32 = 1;
pub(super) const STOP_REASON_BUDGET: u32 = 2;
pub(super) const STOP_REASON_CONSUMER: u32 = 3;
pub(super) const STOP_REASON_POLICY: u32 = 4;
pub(super) const STOP_REASON_SELECTION_LIMIT: u32 = 5;
pub(super) const STOP_REASON_CONTROL_EVENT_LIMIT: u32 = 6;
pub(super) fn write_outstanding(
    writer: crate::identity::FrameWriter,
    outstanding: &[crate::model::OutstandingSelection],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(outstanding.len())?);
    for selection in outstanding {
        writer = writer
            .id(selection.selection_id)
            .id(selection.candidate_id)
            .id(selection.parent_moment_id)
            .id(selection.operation_id)
            .u64(selection.ordinal);
        writer = crate::model::write_resources(writer, &selection.reserved_costs)?;
    }
    Ok(writer)
}
pub(super) fn write_discoveries(
    writer: crate::identity::FrameWriter,
    discoveries: &[crate::frontier::StableDiscovery],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(discoveries.len())?);
    for discovery in discoveries {
        writer = writer
            .id(discovery.child_moment_id)
            .id(discovery.discovery_id)
            .id(discovery.event_payload_id)
            .id(discovery.first_result_id);
    }
    Ok(writer)
}
pub(super) fn write_payload_bindings(
    writer: crate::identity::FrameWriter,
    bindings: &[crate::frontier::PayloadBinding],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(bindings.len())?);
    for binding in bindings {
        writer = writer.id(binding.payload_id).id(binding.owner_id);
    }
    Ok(writer)
}
pub(super) fn write_known_moments(
    writer: crate::identity::FrameWriter,
    moments: &[crate::frontier::KnownMoment],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(moments.len())?);
    for moment in moments {
        writer = writer.id(moment.moment_id).u64(moment.depth);
    }
    Ok(writer)
}
pub(super) fn write_settled(
    writer: crate::identity::FrameWriter,
    settled: &[crate::frontier::Settlement],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(settled.len())?);
    for settlement in settled {
        writer = writer
            .id(settlement.selection_id)
            .id(settlement.settlement_id);
    }
    Ok(writer)
}
pub(super) fn write_findings(
    writer: crate::identity::FrameWriter,
    findings: &[crate::model::FindingRef],
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let mut writer = writer.seq_len(crate::identity::usize_to_u64(findings.len())?);
    for finding in findings {
        writer = writer.id(finding.finding_id).id(finding.priority_facts_id);
    }
    Ok(writer)
}
pub(super) fn write_stopped(
    writer: crate::identity::FrameWriter,
    stopped: Option<&crate::model::StopDecision>,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    let Some(stop) = stopped else {
        return Ok(writer.option_id(None));
    };
    let writer = writer
        .option_id(Some(stop.stop_id))
        .id(stop.campaign_id)
        .id(stop.state_before_id);
    let writer = crate::frontier::encoding::observations::write_stop_reason(writer, &stop.reason)?;
    Ok(writer.bool(stop.incomplete_search_acknowledged))
}
pub(super) fn write_stop_reason(
    writer: crate::identity::FrameWriter,
    reason: &crate::model::StopReason,
) -> crate::error::CampaignResult<crate::identity::FrameWriter> {
    match reason {
        crate::model::StopReason::FrontierExhausted => {
            Ok(writer.u32(crate::frontier::encoding::observations::STOP_REASON_FRONTIER))
        }
        crate::model::StopReason::BudgetExhausted { dimensions } => crate::model::write_ids(
            writer.u32(crate::frontier::encoding::observations::STOP_REASON_BUDGET),
            dimensions,
        ),
        crate::model::StopReason::ConsumerRequested { reason_id } => Ok(writer
            .u32(crate::frontier::encoding::observations::STOP_REASON_CONSUMER)
            .id(*reason_id)),
        crate::model::StopReason::PolicyRequested { reason_id } => Ok(writer
            .u32(crate::frontier::encoding::observations::STOP_REASON_POLICY)
            .id(*reason_id)),
        crate::model::StopReason::SelectionLimitReached => {
            Ok(writer.u32(crate::frontier::encoding::observations::STOP_REASON_SELECTION_LIMIT))
        }
        crate::model::StopReason::ControlEventLimitReached => Ok(
            writer.u32(crate::frontier::encoding::observations::STOP_REASON_CONTROL_EVENT_LIMIT)
        ),
    }
}
