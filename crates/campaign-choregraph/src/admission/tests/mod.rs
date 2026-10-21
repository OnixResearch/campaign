//! Execution-gate controls for publications that do not encode their control event.
//!
//! These publications cannot be assembled outside the adapter; the gate still re-checks every
//! layer so a publication value is only trusted for the event it actually encodes.

const CONTROL_BRANCH: &str = "gate-control";
const BEFORE_GENERATION: u64 = 4;
const STALE_GENERATION: u64 = 0;
const GENERATION_SKIP: u64 = 2;
const FIRST_ORDINAL: u64 = 1;
const HISTORY_MAX_EVENTS: u32 = 16;
const HISTORY_MAX_PARENTS: u32 = 2;
const HISTORY_MAX_METADATA: u32 = 3;
const HISTORY_MAX_LABEL_BYTES: u32 = 64;

fn id(kind: campaign_core::IdKind, label: &[u8]) -> campaign_core::Id {
    campaign_core::derive_id(kind, label, &[])
}

fn schema() -> campaign_core::Id {
    id(campaign_core::IdKind::Projection, b"gate-schema")
}

fn adapter() -> campaign_core::Id {
    id(campaign_core::IdKind::Adapter, b"gate-adapter")
}

fn domain() -> campaign_core::Id {
    id(campaign_core::IdKind::ControlDomain, b"gate-domain")
}

fn decision(
    binding: &campaign_core::ProjectionBinding,
    candidate: &[u8],
) -> crate::AdapterResult<campaign_core::SelectionDecision> {
    let campaign_id = id(campaign_core::IdKind::Campaign, b"gate-campaign");
    let state_before_id = id(campaign_core::IdKind::State, b"gate-state");
    let decision = campaign_core::SelectionDecision {
        selection_id: campaign_core::Id::absent(),
        ordinal: FIRST_ORDINAL,
        campaign_id,
        state_before_id,
        candidate_id: id(campaign_core::IdKind::Candidate, candidate),
        parent_moment_id: id(campaign_core::IdKind::RootMoment, b"gate-root"),
        operation_id: campaign_core::Id::absent(),
        guidance_id: id(campaign_core::IdKind::GuidanceRecord, b"gate-guidance"),
        profile_id: gate_profile()?.profile_id,
        entropy_id: None,
        reserved_costs: alloc::vec::Vec::new(),
        publication_plan: campaign_core::PublicationPlan {
            event_id: campaign_core::Id::absent(),
            campaign_id,
            control_branch_id: binding.control_branch_id,
            expected_head_id: binding.head_id,
            expected_generation: binding.generation,
            state_before_id,
            selection_id: campaign_core::Id::absent(),
        },
    }
    .derive()?;
    assert_eq!(
        decision.publication_plan.selection_id,
        decision.selection_id
    );
    assert_eq!(
        decision.publication_plan.expected_generation,
        binding.generation
    );
    Ok(decision)
}

/// Ranked production profile named by every gate fixture decision.
fn gate_profile() -> crate::AdapterResult<campaign_core::SelectionProfile> {
    Ok(campaign_core::SelectionProfile {
        profile_id: campaign_core::Id::absent(),
        scheduling_policy_id: id(campaign_core::IdKind::SchedulingProfile, b"gate-schedule"),
        admission_policy_id: id(campaign_core::IdKind::AdmissionPolicy, b"gate-admission"),
        coalescing_policy_id: id(campaign_core::IdKind::CoalescingPolicy, b"gate-coalescing"),
        mode: campaign_core::SelectionMode::Ranked,
        decay: campaign_core::SelectionCountDecay::None,
        stopping: campaign_core::StoppingRule {
            finding_limit: None,
        },
        implementation: campaign_core::PolicyImplementation::ProductionCompiled {
            policy_id: id(campaign_core::IdKind::Policy, b"gate-policy"),
        },
    }
    .derive()?)
}

fn published(
    selection: &campaign_core::SelectionDecision,
) -> crate::AdapterResult<campaign_core::CampaignControlEvent> {
    Ok(campaign_core::CampaignControlEvent::SelectionPublished(
        campaign_core::SelectionEvent::new(selection.clone(), gate_profile()?, None)?,
    ))
}

/// One exact publication of the winner, and a rival planned at the same fence but never published.
fn fixture() -> crate::AdapterResult<(
    campaign_core::SelectionDecision,
    campaign_core::SelectionDecision,
    crate::contracts::ControlPublication,
)> {
    let limits = choregraph_history::HistoryLimits::new(
        HISTORY_MAX_EVENTS,
        HISTORY_MAX_PARENTS,
        HISTORY_MAX_METADATA,
        HISTORY_MAX_LABEL_BYTES,
        HISTORY_MAX_EVENTS,
    )?;
    let root = choregraph_history::HistoryGraph::new(limits)?.admit(
        choregraph_history::EventDraft::v1(
            choregraph_history::EventClass::Application,
            crate::identity::digest_from_id(id(campaign_core::IdKind::EventPayload, b"gate-root")),
            alloc::vec::Vec::new(),
            crate::identity::digest_from_id(schema()),
        ),
    )?;
    let branch = choregraph_history::BranchRef {
        name: alloc::string::String::from(CONTROL_BRANCH),
        head: root.event,
        generation: BEFORE_GENERATION,
    };
    let binding = crate::identity::derive_projection_binding(
        &root.graph,
        &branch,
        schema(),
        adapter(),
        domain(),
    )?;
    let winner = decision(&binding, b"gate-winner")?;
    let rival = decision(&binding, b"gate-rival")?;
    let control_event = published(&winner)?;
    let draft = crate::envelope::control_event_draft(
        &control_event,
        schema(),
        adapter(),
        domain(),
        Some(branch.head),
    )?;
    let admission = root.graph.admit(draft.clone())?;
    let branch_plan = choregraph_history::plan_branch_advance(
        &admission.graph,
        &branch,
        BEFORE_GENERATION,
        branch.head,
        admission.event,
    )?;
    assert_eq!(branch_plan.after.head, admission.event);
    assert!(admission.graph.contains(admission.event));
    let publication = crate::contracts::ControlPublication {
        payload_id: crate::identity::derive_control_payload_id(&control_event)?,
        draft,
        history_event_id: admission.event,
        graph_after_admission: admission.graph,
        branch_plan,
        control_event,
        binding,
    };
    Ok((winner, rival, publication))
}

fn exact_evidence(
    publication: &crate::contracts::ControlPublication,
) -> crate::contracts::DurablePublicationEvidence {
    crate::contracts::DurablePublicationEvidence {
        history_event_id: publication.history_event_id,
        branch_before: publication.branch_plan.before.clone(),
        branch_after: publication.branch_plan.after.clone(),
        durable_event: true,
        durable_branch_update: true,
    }
}

fn gate(
    selection: &campaign_core::SelectionDecision,
    publication: &crate::contracts::ControlPublication,
) -> crate::AdapterResult<bool> {
    super::can_execute_selection_after_publication(
        selection,
        publication,
        &exact_evidence(publication),
    )
}

#[test]
// r[verify campaign.publication_fence.valid]
fn an_exact_publication_authorizes_only_its_own_selection() -> crate::AdapterResult<()> {
    let (winner, rival, publication) = fixture()?;
    assert_eq!(gate(&winner, &publication), Ok(true));
    assert_eq!(
        gate(&rival, &publication),
        Err(crate::AdapterError::MissingControlEvent)
    );
    Ok(())
}

#[test]
fn every_layer_of_a_spliced_publication_is_rejected() -> crate::AdapterResult<()> {
    let (_winner, rival, publication) = fixture()?;
    let rival_event = published(&rival)?;
    let rival_payload_id = crate::identity::derive_control_payload_id(&rival_event)?;
    let rival_draft = crate::envelope::control_event_draft(
        &rival_event,
        schema(),
        adapter(),
        domain(),
        Some(publication.branch_plan.before.head),
    )?;
    let mut spliced = publication.clone();
    spliced.control_event = rival_event;
    assert_eq!(
        gate(&rival, &spliced),
        Err(crate::AdapterError::PayloadIdentityMismatch)
    );
    spliced.payload_id = rival_payload_id;
    assert_eq!(
        gate(&rival, &spliced),
        Err(crate::AdapterError::PayloadIdentityMismatch)
    );
    spliced.draft.payload_identity = rival_draft.payload_identity;
    assert_eq!(
        gate(&rival, &spliced),
        Err(crate::AdapterError::MetadataMismatch)
    );
    spliced.draft = rival_draft;
    assert_eq!(
        gate(&rival, &spliced),
        Err(crate::AdapterError::PublicationEvidenceMismatch)
    );
    spliced.history_event_id = choregraph_history::HistoryGraph::expected_identity(&spliced.draft);
    spliced.branch_plan.after.head = spliced.history_event_id;
    assert_eq!(
        gate(&rival, &spliced),
        Err(crate::AdapterError::PublicationEvidenceMismatch)
    );
    Ok(())
}

#[test]
fn a_forged_branch_plan_cannot_enable_execution() -> crate::AdapterResult<()> {
    let (winner, _rival, publication) = fixture()?;
    let mut skipped = publication.clone();
    skipped.branch_plan.after.generation = BEFORE_GENERATION
        .checked_add(GENERATION_SKIP)
        .ok_or(campaign_core::CampaignError::ArithmeticOverflow)?;
    assert_eq!(
        gate(&winner, &skipped),
        Err(crate::AdapterError::Campaign(
            campaign_core::CampaignError::InvalidPublication
        ))
    );
    let mut unmoved = publication.clone();
    unmoved.history_event_id = unmoved.branch_plan.before.head;
    unmoved.branch_plan.after.head = unmoved.branch_plan.before.head;
    assert_eq!(
        gate(&winner, &unmoved),
        Err(crate::AdapterError::PublicationEvidenceMismatch)
    );
    let mut stale_before = publication;
    stale_before.branch_plan.before.generation = STALE_GENERATION;
    assert_eq!(
        gate(&winner, &stale_before),
        Err(crate::AdapterError::PublicationFenceMismatch)
    );
    Ok(())
}
