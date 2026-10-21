//! Role-identity fixtures: stored candidate, work item, and result stay distinct records for
//! ranked and exploratory executions, for a result settled quietly and for one that reports a
//! child moment and a finding.

pub(super) fn collapsed_identity(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::super::common::ample(harness)?;
    super::super::common::guided_frontier(&mut session, super::super::common::UNIT_COST)?;
    let selection = session.execute(None)?;
    let result = session.settle(&selection, &super::super::common::outcome(&[]))?;
    let candidate = session
        .state
        .candidate(selection.candidate_id)?
        .candidate
        .clone();
    super::super::common::require(
        result.selection_id == selection.selection_id
            && candidate.candidate_id == selection.candidate_id,
        "the accepted result settles the executed selection of the stored candidate",
    )?;
    debug_assert_eq!(result.selection_id, selection.selection_id);
    debug_assert_eq!(candidate.candidate_id, selection.candidate_id);
    let collapsed = campaign_core::RoleCollapseFixture {
        candidate_record_id: candidate.candidate_id,
        queued_work_id: candidate.candidate_id,
        computed_result_id: candidate.candidate_id,
    };
    session.harness.deny(
        campaign_core::check_no_role_collapse(&collapsed)
            .map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::RoleCollapse,
        "the role check accepted one record in three roles",
    )?;
    sample_identities(&mut session, &candidate, &selection, &result)?;
    let explored = session.execute(Some(crate::vocabulary::ScenarioDraw {
        explore_draw: 0,
        index_draw: 1,
    }))?;
    super::super::common::require(
        explored.entropy_id.is_some(),
        "the second sampled execution is exploratory",
    )?;
    session.add_moment(super::super::common::child(super::super::common::CHILD_ONE))?;
    let explored_result = session.settle(
        &explored,
        &crate::vocabulary::ScenarioOutcome {
            children: alloc::vec![super::super::common::CHILD_ONE],
            findings: 1,
        },
    )?;
    super::super::common::require(
        explored_result.children.len() == 1 && explored_result.findings.len() == 1,
        "the second sampled execution reports a child moment and a finding",
    )?;
    debug_assert_eq!(explored_result.findings.len(), 1);
    let explored_candidate = session
        .state
        .candidate(explored.candidate_id)?
        .candidate
        .clone();
    sample_identities(
        &mut session,
        &explored_candidate,
        &explored,
        &explored_result,
    )
}

/// Asks the adapter for the role identities of one executed operation: they must be exactly the
/// stored candidate, work item, and result identities, and never collapse.
fn sample_identities(
    session: &mut crate::harness::Session<'_, '_>,
    candidate: &campaign_core::CandidateRecord,
    selection: &campaign_core::SelectionDecision,
    result: &campaign_core::ResultObservation,
) -> crate::verdicts::ConformanceResult<()> {
    let mapped = session
        .harness
        .adapter
        .role_identities(candidate, selection, result);
    let checked = campaign_core::check_no_role_collapse(&mapped);
    super::super::common::forbid(
        checked == Err(campaign_core::CampaignError::RoleCollapse),
        "the adapter stored candidate, work item, and result as one record",
    )?;
    checked?;
    debug_assert_ne!(mapped.queued_work_id, mapped.candidate_record_id);
    let stored = campaign_core::RoleCollapseFixture {
        candidate_record_id: candidate.candidate_id,
        queued_work_id: selection.selection_id,
        computed_result_id: result.result_id,
    };
    debug_assert_eq!(stored.candidate_record_id, candidate.candidate_id);
    super::super::common::forbid(
        mapped != stored,
        "the adapter reported role identities other than its stored candidate, work item, and result",
    )
}
