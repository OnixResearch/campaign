const FIXTURE_LABEL: &[u8] = b"publication-fence-fixture";
const PLANNED_GENERATION: u64 = 4;
const ADVANCED_GENERATION: u64 = 5;
const SKIPPED_GENERATION: u64 = 6;
const TAMPERED_ORDINAL: u64 = 99;

fn id(kind: crate::identity::IdKind, label: &[u8]) -> crate::identity::Id {
    crate::identity::derive_id(
        kind,
        FIXTURE_LABEL,
        &[crate::identity::derive_id(kind, label, &[])],
    )
}

fn selection() -> crate::error::CampaignResult<crate::model::SelectionDecision> {
    let selection_id = id(crate::identity::IdKind::Selection, b"selection");
    let decision = crate::model::SelectionDecision {
        selection_id,
        ordinal: 1,
        campaign_id: id(crate::identity::IdKind::Campaign, b"campaign"),
        state_before_id: id(crate::identity::IdKind::State, b"state"),
        candidate_id: id(crate::identity::IdKind::Candidate, b"candidate"),
        parent_moment_id: id(crate::identity::IdKind::RootMoment, b"root"),
        operation_id: id(crate::identity::IdKind::Operation, b"operation"),
        guidance_id: id(crate::identity::IdKind::GuidanceRecord, b"guidance"),
        profile_id: id(crate::identity::IdKind::SelectionProfile, b"profile"),
        entropy_id: None,
        reserved_costs: alloc::vec::Vec::new(),
        publication_plan: crate::model::PublicationPlan {
            event_id: id(crate::identity::IdKind::Event, b"event"),
            campaign_id: id(crate::identity::IdKind::Campaign, b"campaign"),
            control_branch_id: id(crate::identity::IdKind::ControlBranch, b"branch-before"),
            expected_head_id: id(crate::identity::IdKind::Head, b"head-before"),
            expected_generation: PLANNED_GENERATION,
            state_before_id: id(crate::identity::IdKind::State, b"state"),
            selection_id,
        },
    }
    .derive()?;
    assert_eq!(
        decision.publication_plan.selection_id,
        decision.selection_id
    );
    assert_eq!(
        decision.publication_plan.expected_generation,
        PLANNED_GENERATION
    );
    Ok(decision)
}

fn evidence(selection: &crate::model::SelectionDecision) -> super::PublicationEvidence {
    super::PublicationEvidence {
        event_id: selection.publication_plan.event_id,
        selection_id: selection.selection_id,
        branch_before: super::BranchFact {
            control_branch_id: selection.publication_plan.control_branch_id,
            head_id: selection.publication_plan.expected_head_id,
            generation: selection.publication_plan.expected_generation,
        },
        branch_after: super::BranchFact {
            control_branch_id: id(crate::identity::IdKind::ControlBranch, b"branch-after"),
            head_id: id(crate::identity::IdKind::Head, b"head-after"),
            generation: ADVANCED_GENERATION,
        },
        durable_event: true,
        durable_branch_update: true,
    }
}

#[test]
// r[verify campaign.publication_fence.valid]
fn exact_durable_evidence_is_permitted() -> crate::error::CampaignResult<()> {
    let selection = selection()?;
    assert_eq!(
        super::can_execute_published_selection(&selection, &evidence(&selection)),
        Ok(true)
    );
    Ok(())
}

#[test]
// r[verify campaign.publication_fence.unpublished]
fn non_durable_evidence_is_withheld() -> crate::error::CampaignResult<()> {
    let selection = selection()?;
    let mut event_only = evidence(&selection);
    event_only.durable_branch_update = false;
    assert_eq!(
        super::can_execute_published_selection(&selection, &event_only),
        Ok(false)
    );
    let mut branch_only = evidence(&selection);
    branch_only.durable_event = false;
    assert_eq!(
        super::can_execute_published_selection(&selection, &branch_only),
        Ok(false)
    );
    Ok(())
}

#[test]
fn every_fence_mismatch_is_an_invalid_publication() -> crate::error::CampaignResult<()> {
    let selection = selection()?;
    let mutations: [fn(&mut super::PublicationEvidence); 7] = [
        |evidence| evidence.event_id = id(crate::identity::IdKind::Event, b"other-event"),
        |evidence| {
            evidence.selection_id = id(crate::identity::IdKind::Selection, b"other-selection");
        },
        |evidence| {
            evidence.branch_before.control_branch_id =
                id(crate::identity::IdKind::ControlBranch, b"other-branch");
        },
        |evidence| evidence.branch_before.head_id = id(crate::identity::IdKind::Head, b"stale"),
        |evidence| evidence.branch_before.generation = ADVANCED_GENERATION,
        |evidence| evidence.branch_after.generation = SKIPPED_GENERATION,
        |evidence| evidence.branch_after.head_id = evidence.branch_before.head_id,
    ];
    for mutate in mutations {
        let mut forged = evidence(&selection);
        mutate(&mut forged);
        assert_ne!(forged, evidence(&selection));
        assert_eq!(
            super::can_execute_published_selection(&selection, &forged),
            Err(crate::error::CampaignError::InvalidPublication)
        );
    }
    Ok(())
}

#[test]
fn crossed_and_overflowing_evidence_is_rejected() -> crate::error::CampaignResult<()> {
    let selection = selection()?;
    let mut crossed = evidence(&selection);
    crossed.branch_after.head_id = id(crate::identity::IdKind::Event, b"not-a-head");
    assert_eq!(
        super::can_execute_published_selection(&selection, &crossed),
        Err(crate::error::CampaignError::CrossedIdentity)
    );
    let mut saturated = selection.clone();
    saturated.publication_plan.expected_generation = u64::MAX;
    let saturated = saturated.derive()?;
    let mut overflow = evidence(&saturated);
    overflow.branch_before.generation = u64::MAX;
    assert_eq!(
        super::can_execute_published_selection(&saturated, &overflow),
        Err(crate::error::CampaignError::ArithmeticOverflow)
    );
    Ok(())
}

#[test]
fn a_selection_that_does_not_rederive_is_never_authorized() -> crate::error::CampaignResult<()> {
    let selection = selection()?;
    let exact = evidence(&selection);
    let mut tampered = selection.clone();
    tampered.operation_id = id(crate::identity::IdKind::Operation, b"unpublished-operation");
    tampered.candidate_id = id(crate::identity::IdKind::Candidate, b"other-candidate");
    tampered.ordinal = TAMPERED_ORDINAL;
    assert_eq!(tampered.selection_id, selection.selection_id);
    assert_eq!(
        super::can_execute_published_selection(&tampered, &exact),
        Err(crate::error::CampaignError::MalformedIdentity)
    );
    assert_eq!(
        super::can_execute_published_selection(&selection, &exact),
        Ok(true)
    );
    Ok(())
}
