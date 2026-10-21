//! `campaign.receipts` / `campaign.receipts.crossed` / `campaign.fd.oracle.isolated`: receipts
//! bind the event they certify (its profile, ticket, guidance and cost reservation), only a
//! production profile, only the canonical event presentation, and only the state's own current
//! history binding.

mod attribution;
mod history;
mod presentation;
mod stops;

pub(crate) fn event_receipt(
    state: &campaign_core::FrontierState,
    event: &campaign_core::CampaignControlEvent,
    profile: &campaign_core::SelectionProfile,
    binding: &campaign_core::ProjectionBinding,
) -> campaign_core::CampaignResult<campaign_core::CampaignReceipt> {
    campaign_core::project_event_receipt(
        state,
        &campaign_core::EventReceiptRequest {
            request_id: crate::suite::admission::xid(
                campaign_core::IdKind::Request,
                b"bound-event",
            ),
            event: event.clone(),
            profile: profile.clone(),
        },
        binding,
    )
}

/// The event receipt of `event` on `state` under its current binding and the ranked profile.
pub(crate) fn ranked_event_receipt(
    state: &campaign_core::FrontierState,
    event: &campaign_core::CampaignControlEvent,
) -> Result<campaign_core::CampaignResult<campaign_core::CampaignReceipt>, String> {
    let binding = crate::suite::admission::current_projection(state)?;
    let profile = crate::suite::selection::ranked_profile()?;
    Ok(crate::suite::certified::event_receipt(
        state, event, &profile, &binding,
    ))
}

pub(crate) fn reference_profile() -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::schedules::variant(|profile| {
        profile.implementation = campaign_core::PolicyImplementation::ReferenceEvidence {
            reference_id: crate::suite::admission::xid(campaign_core::IdKind::Policy, b"oracle"),
        };
    })
}
