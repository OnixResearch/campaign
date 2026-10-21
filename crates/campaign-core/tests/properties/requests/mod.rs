//! Selection profiles, explicit entropy tickets, and planning requests.

/// Penalty the property profiles subtract from the first rank per prior selection.
pub(crate) const DECAY_PENALTY: u64 = 1;

pub(crate) fn profile(
    mode: campaign_core::SelectionMode,
) -> Result<campaign_core::SelectionProfile, String> {
    crate::suite::fixture::observed(
        campaign_core::SelectionProfile {
            profile_id: campaign_core::Id::absent(),
            scheduling_policy_id: crate::suite::fixture::id(
                campaign_core::IdKind::SchedulingProfile,
                b"schedule",
                0,
            ),
            admission_policy_id: crate::suite::fixture::id(
                campaign_core::IdKind::AdmissionPolicy,
                b"admit",
                0,
            ),
            coalescing_policy_id: crate::suite::fixture::coalescing_policy(),
            mode,
            decay: campaign_core::SelectionCountDecay::SubtractFromFirstRank {
                penalty_per_selection: DECAY_PENALTY,
            },
            stopping: campaign_core::StoppingRule {
                finding_limit: None,
            },
            implementation: campaign_core::PolicyImplementation::ProductionCompiled {
                policy_id: crate::suite::fixture::id(campaign_core::IdKind::Policy, b"policy", 0),
            },
        }
        .derive(),
    )
}

pub(crate) fn exploratory(
    state: &campaign_core::FrontierState,
    threshold_parts_per_million: u64,
    exploration_order: Vec<u32>,
) -> Result<campaign_core::SelectionProfile, String> {
    profile(campaign_core::SelectionMode::Exploratory {
        threshold_parts_per_million,
        entropy_source_id: state.admission.entropy_profile_id,
        exploration_order,
    })
}

/// Explicit two-draw entropy input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Draws {
    pub(crate) explore: u64,
    pub(crate) index: u64,
}

/// Exact entropy ticket for `state` and `profile` carrying `draws`.
pub(crate) fn ticket(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    draws: Draws,
    counter: u64,
) -> Result<campaign_core::EntropyTicket, String> {
    let mut bytes = [0; campaign_core::DIGEST_LENGTH];
    bytes[..campaign_core::ENTROPY_PREFIX_BYTES].copy_from_slice(&draws.explore.to_le_bytes());
    bytes[campaign_core::ENTROPY_PREFIX_BYTES..campaign_core::ENTROPY_INDEX_DRAW_END]
        .copy_from_slice(&draws.index.to_le_bytes());
    crate::suite::fixture::observed(
        campaign_core::EntropyTicket {
            entropy_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            profile_id: profile.profile_id,
            source_id: state.admission.entropy_profile_id,
            counter,
            bytes,
        }
        .derive(),
    )
}

/// Request planned against the campaign's admitted projection binding.
pub(crate) fn request(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> campaign_core::SelectionRequest {
    campaign_core::SelectionRequest {
        request_id: crate::suite::fixture::id(campaign_core::IdKind::Request, b"request", 0),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile,
        entropy,
        expected_generation: state.admission.projection.generation,
        expected_head_id: state.admission.projection.head_id,
    }
}

pub(crate) fn plan(
    state: &campaign_core::FrontierState,
    request: campaign_core::SelectionRequest,
) -> campaign_core::CampaignResult<campaign_core::DecisionPlan> {
    campaign_core::plan_next(state, request, &state.admission.projection)
}
