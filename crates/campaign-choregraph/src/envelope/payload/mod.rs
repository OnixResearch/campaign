//! Canonical Campaign control-payload framing.

const CONTROL_PAYLOAD_LABEL: &[u8] = b"campaign-choregraph-control-payload";
const KIND_CAMPAIGN_ADMITTED: u32 = 1;
const KIND_CANDIDATES_ADMITTED: u32 = 2;
const KIND_GUIDANCE_ADMITTED: u32 = 3;
const KIND_SELECTION_PUBLISHED: u32 = 4;
const KIND_RESULT_OBSERVED: u32 = 5;
const KIND_UNOBSERVED_CLOSED: u32 = 6;
const KIND_CANDIDATE_RETIRED: u32 = 7;
const KIND_CANDIDATES_PRUNED: u32 = 8;
const KIND_STOP_ACCEPTED: u32 = 9;

pub(super) fn derive_id(
    event: &campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::Id> {
    campaign_core::identity::derive_id_with(
        campaign_core::IdKind::EventPayload,
        CONTROL_PAYLOAD_LABEL,
        |writer| {
            let header = writer
                .u32(kind_code(event))
                .id(event.event_id())
                .option_id(event.state_before_id());
            encode_body(header, event)
        },
    )
}

#[cfg_attr(
    dylint_lib = "octet",
    allow(
        tigerstyle::missing_const_fn,
        reason = "ADAPTER owns this framing adapter: FrameWriter::id updates a runtime BLAKE3 transcript and is not const"
    )
)]
fn encode_body(
    writer: campaign_core::identity::FrameWriter,
    event: &campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::identity::FrameWriter> {
    match event {
        campaign_core::CampaignControlEvent::CampaignAdmitted { admission, .. } => Ok(writer
            .id(admission.campaign_id)
            .id(admission.projection.projection_id)
            .id(admission.bounds.bounds_id)),
        campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) => {
            encode_candidates(writer, batch)
        }
        campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) => Ok(writer
            .id(guidance.campaign_id)
            .id(guidance.guidance.guidance_id)
            .id(guidance.admission_outcomes_id)),
        campaign_core::CampaignControlEvent::SelectionPublished(selection) => Ok(writer
            .id(selection.decision.campaign_id)
            .id(selection.decision.selection_id)
            .id(selection.decision.operation_id)
            .id(selection.profile.profile_id)
            .option_id(selection.entropy.as_ref().map(|ticket| ticket.entropy_id))),
        campaign_core::CampaignControlEvent::ResultObserved(result) => Ok(writer
            .id(result.campaign_id)
            .id(result.observation.result_id)),
        campaign_core::CampaignControlEvent::UnobservedClosed(closure) => Ok(writer
            .id(closure.campaign_id)
            .id(closure.closure.closure_id)
            .id(closure.closure.selection_id)),
        campaign_core::CampaignControlEvent::CandidateRetired(retirement) => Ok(writer
            .id(retirement.campaign_id)
            .id(retirement.candidate_id)
            .id(retirement.reason_id)
            .id(retirement.policy_id)
            .id(retirement.adapter_id)
            .u32(retirement.origin.code())
            .id(retirement.invalidation_id)),
        campaign_core::CampaignControlEvent::CandidatesPruned(prune) => Ok(writer
            .id(prune.campaign_id)
            .id(prune.prune_plan_id)
            .option_id(prune.retention_profile_id)
            .id(prune.invalidation_id)),
        campaign_core::CampaignControlEvent::StopAccepted(stop) => {
            Ok(writer.id(stop.campaign_id).id(stop.stop.stop_id))
        }
    }
}

fn encode_candidates(
    writer: campaign_core::identity::FrameWriter,
    batch: &campaign_core::CandidateBatchEvent,
) -> campaign_core::CampaignResult<campaign_core::identity::FrameWriter> {
    let count = u64::try_from(batch.candidates.len())
        .map_err(|_| campaign_core::CampaignError::ArithmeticOverflow)?;
    let mut transcript = writer
        .id(batch.campaign_id)
        .id(batch.admission_policy_id)
        .id(batch.coalescing_policy_id)
        .id(batch.admission_outcomes_id)
        .seq_len(count);
    for candidate in &batch.candidates {
        transcript = transcript.id(candidate.candidate_id);
    }
    Ok(transcript)
}

const fn kind_code(event: &campaign_core::CampaignControlEvent) -> u32 {
    match event {
        campaign_core::CampaignControlEvent::CampaignAdmitted { .. } => KIND_CAMPAIGN_ADMITTED,
        campaign_core::CampaignControlEvent::CandidatesAdmitted(_) => KIND_CANDIDATES_ADMITTED,
        campaign_core::CampaignControlEvent::GuidanceAdmitted(_) => KIND_GUIDANCE_ADMITTED,
        campaign_core::CampaignControlEvent::SelectionPublished(_) => KIND_SELECTION_PUBLISHED,
        campaign_core::CampaignControlEvent::ResultObserved(_) => KIND_RESULT_OBSERVED,
        campaign_core::CampaignControlEvent::UnobservedClosed(_) => KIND_UNOBSERVED_CLOSED,
        campaign_core::CampaignControlEvent::CandidateRetired(_) => KIND_CANDIDATE_RETIRED,
        campaign_core::CampaignControlEvent::CandidatesPruned(_) => KIND_CANDIDATES_PRUNED,
        campaign_core::CampaignControlEvent::StopAccepted(_) => KIND_STOP_ACCEPTED,
    }
}
