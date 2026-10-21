pub(crate) fn request_for(
    current: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::SelectionRequest, String> {
    Ok(campaign_core::SelectionRequest {
        request_id: crate::suite::identity::derive(campaign_core::IdKind::Request, b"loop-request"),
        campaign_id: current.state().admission.campaign_id,
        state_id: current.state().state_id,
        profile: campaign_core::SelectionProfile {
            profile_id: campaign_core::Id::absent(),
            scheduling_policy_id: crate::suite::identity::derive(
                campaign_core::IdKind::SchedulingProfile,
                b"loop-schedule",
            ),
            admission_policy_id: crate::suite::identity::derive(
                campaign_core::IdKind::AdmissionPolicy,
                b"loop-admission",
            ),
            coalescing_policy_id: crate::suite::identity::derive(
                campaign_core::IdKind::CoalescingPolicy,
                b"loop-coalescing",
            ),
            mode: campaign_core::SelectionMode::Ranked,
            decay: campaign_core::SelectionCountDecay::SubtractFromFirstRank {
                penalty_per_selection: 1,
            },
            stopping: campaign_core::StoppingRule {
                finding_limit: None,
            },
            implementation: campaign_core::PolicyImplementation::ProductionCompiled {
                policy_id: crate::suite::identity::derive(
                    campaign_core::IdKind::Policy,
                    b"loop-policy",
                ),
            },
        }
        .derive()
        .map_err(|error| format!("{error:?}"))?,
        entropy: None,
        expected_generation: current.binding().generation,
        expected_head_id: current.binding().head_id,
    })
}
