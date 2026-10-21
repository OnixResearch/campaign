pub(crate) fn request_for(
    current: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::SelectionRequest, String> {
    Ok(campaign_core::SelectionRequest {
        request_id: crate::suite::identity::derive(
            campaign_core::IdKind::Request,
            b"repair-loop-request",
        ),
        campaign_id: current.state().admission.campaign_id,
        state_id: current.state().state_id,
        profile: profile_template()
            .derive()
            .map_err(|error| format!("{error:?}"))?,
        entropy: None,
        expected_generation: current.binding().generation,
        expected_head_id: current.binding().head_id,
    })
}
pub(crate) fn select_from_current(
    current: &campaign_choregraph::ValidatedControlProjection,
) -> Result<campaign_core::SelectionDecision, String> {
    let plan = crate::suite::identity::observed(
        campaign_choregraph::plan_next_from_published_projection(
            current,
            campaign_choregraph::PublishedSelectionRequest {
                request: request_for(current)?,
            },
        ),
    )?;
    let campaign_core::NextDecision::Select(selection) = plan.decision else {
        return Err(String::from("the current candidate must remain selectable"));
    };
    Ok(*selection)
}
pub(crate) fn exact_durable_evidence(
    publication: &campaign_choregraph::ControlPublication,
) -> campaign_choregraph::DurablePublicationEvidence {
    campaign_choregraph::DurablePublicationEvidence {
        history_event_id: publication.history_event_id(),
        branch_before: publication.branch_plan().before.clone(),
        branch_after: publication.branch_plan().after.clone(),
        durable_event: true,
        durable_branch_update: true,
    }
}
pub(crate) fn result_for(
    current: &campaign_choregraph::ValidatedControlProjection,
    selection: &campaign_core::SelectionDecision,
) -> campaign_core::ResultObservation {
    campaign_core::ResultObservation {
        result_id: campaign_core::Id::absent(),
        campaign_id: current.state().admission.campaign_id,
        selection_id: selection.selection_id,
        parent_moment_id: selection.parent_moment_id,
        operation_id: selection.operation_id,
        adapter_id: crate::suite::identity::adapter(),
        payload_id: crate::suite::identity::derive(
            campaign_core::IdKind::EventPayload,
            b"repair-result-payload",
        ),
        children: Vec::new(),
        findings: Vec::new(),
        derived_records: Vec::new(),
    }
}
/// The repair planner's declared profile before identity derivation.
fn profile_template() -> campaign_core::SelectionProfile {
    campaign_core::SelectionProfile {
        profile_id: campaign_core::Id::absent(),
        scheduling_policy_id: crate::suite::identity::derive(
            campaign_core::IdKind::SchedulingProfile,
            b"repair-loop-schedule",
        ),
        admission_policy_id: crate::suite::identity::derive(
            campaign_core::IdKind::AdmissionPolicy,
            b"repair-loop-admission",
        ),
        coalescing_policy_id: crate::suite::identity::derive(
            campaign_core::IdKind::CoalescingPolicy,
            b"repair-loop-coalescing",
        ),
        mode: campaign_core::SelectionMode::Ranked,
        decay: campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: crate::suite::constants::BRANCH_EVENT_INCREMENT,
        },
        stopping: campaign_core::StoppingRule {
            finding_limit: None,
        },
        implementation: campaign_core::PolicyImplementation::ProductionCompiled {
            policy_id: crate::suite::identity::derive(
                campaign_core::IdKind::Policy,
                b"repair-loop-policy",
            ),
        },
    }
}
/// Publication event of a selection planned by a repair planner: the ranked repair profile or the
/// rival that declares no decay, whichever the decision names.
pub(crate) fn event(
    selection: &campaign_core::SelectionDecision,
) -> campaign_core::CampaignControlEvent {
    let rival = campaign_core::SelectionProfile {
        decay: campaign_core::SelectionCountDecay::None,
        ..profile_template()
    };
    let profile = [profile_template(), rival]
        .into_iter()
        .filter_map(|profile| profile.derive().ok())
        .find(|profile| profile.profile_id == selection.profile_id)
        .unwrap_or_else(profile_template);
    campaign_core::CampaignControlEvent::SelectionPublished(campaign_core::SelectionEvent {
        event_id: selection.publication_plan.event_id,
        decision: selection.clone(),
        profile: Box::new(profile),
        entropy: None,
    })
}
