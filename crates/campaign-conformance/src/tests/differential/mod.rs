//! `campaign.fd.oracle.isolated` and `campaign.fd.evidence.questions`: production planning versus
//! the explicit reference path over a corpus of states.

mod bounds;
mod parity;
mod requests;

/// Plans every corpus state with production and reference planners and returns the reports.
fn corpus_reports()
-> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::DifferentialReport>> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 2, |bounds| {
        bounds.max_outstanding = 1;
    })?;
    varied(&mut session)?;
    let events = session.control_events();
    let binding = session.binding()?;
    let mut reports = alloc::vec::Vec::with_capacity(events.len());
    for end in 1..=events.len() {
        let state = campaign_core::reconstruct(&events[..end])?;
        let request = campaign_core::SelectionRequest {
            state_id: state.state_id,
            ..planned_request(&session, &binding)?
        };
        let production = campaign_core::plan_next(&state, request.clone(), &binding)?;
        let reference = crate::reference::plan_next(&state, &request, &binding)?;
        let report = campaign_core::compare_decisions(&production, &reference);
        campaign_core::validate_differential_evidence(&report, &production, &reference)?;
        reports.push(report);
    }
    assert!(!events.is_empty());
    assert_eq!(reports.len(), events.len());
    Ok(reports)
}

fn operation(handle: u32) -> crate::vocabulary::ScenarioOperation {
    crate::vocabulary::ScenarioOperation {
        handle: crate::vocabulary::OperationHandle(handle),
        parent: crate::harness::history::ROOT,
        costs: alloc::vec![crate::vocabulary::AbstractCost {
            dimension: 0,
            units: 1,
        }],
    }
}

fn preference(handle: u32, levels: [u32; 2]) -> crate::vocabulary::ScenarioPreference {
    crate::vocabulary::ScenarioPreference {
        operation: crate::vocabulary::OperationHandle(handle),
        generation: 1,
        score: crate::vocabulary::AbstractScore {
            levels: levels.to_vec(),
        },
    }
}

/// Admission, a partly guided frontier, an execution and result, late guidance, a second
/// execution and closure: states that select, wait for guidance, results, or capacity, and stop.
fn varied(session: &mut crate::harness::Session<'_, '_>) -> crate::verdicts::ConformanceResult<()> {
    session.admit(&[operation(1), operation(2), operation(3)])?;
    session.guide(&preference(1, [30, 1]))?;
    session.guide(&preference(2, [20, 3]))?;
    let first = session.execute(None)?;
    session.settle(
        &first,
        &crate::vocabulary::ScenarioOutcome {
            children: alloc::vec::Vec::new(),
            findings: 0,
        },
    )?;
    session.guide(&preference(3, [10, 2]))?;
    let second = session.execute(None)?;
    session.close(&second)
}

fn planned_request(
    session: &crate::harness::Session<'_, '_>,
    binding: &campaign_core::ProjectionBinding,
) -> crate::verdicts::ConformanceResult<campaign_core::SelectionRequest> {
    Ok(campaign_core::SelectionRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "differential")?,
        campaign_id: session.state.admission.campaign_id,
        state_id: session.state.state_id,
        profile: session.selection_profile(false)?,
        entropy: None,
        expected_generation: binding.generation,
        expected_head_id: binding.head_id,
    })
}

#[test]
// r[verify campaign.fd.evidence.questions]
fn production_and_reference_agree_on_fields_and_report_work_separately()
-> crate::verdicts::ConformanceResult<()> {
    let reports = corpus_reports()?;
    assert!(reports.iter().all(|report| report.field_identity_equal));
    assert!(
        reports
            .iter()
            .any(|report| report.semantic_work_equal == Some(false))
    );
    assert!(
        reports
            .iter()
            .any(|report| report.semantic_work_equal == Some(true))
    );
    assert!(reports.iter().all(|report| {
        report.time == campaign_core::EvidenceQuestion::NotMeasured
            && report.memory == campaign_core::EvidenceQuestion::NotMeasured
    }));
    Ok(())
}

#[test]
// r[verify campaign.fd.oracle.isolated]
fn only_the_explicit_reference_path_plans_with_the_reference()
-> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 2, |_| {})?;
    session.admit(&[operation(1)])?;
    session.guide(&preference(1, [30, 1]))?;
    let binding = session.binding()?;
    let mut request = planned_request(&session, &binding)?;
    let reference_id = crate::harness::suite_id(campaign_core::IdKind::Policy, "reference")?;
    request.profile.implementation =
        campaign_core::PolicyImplementation::ReferenceEvidence { reference_id };
    request.profile = request.profile.derive()?;
    assert_eq!(
        campaign_core::plan_next(&session.state, request.clone(), &binding),
        Err(campaign_core::CampaignError::ReferencePolicySelected)
    );
    assert_eq!(
        campaign_core::select_production_policy(&request.profile.implementation),
        Err(campaign_core::CampaignError::ReferencePolicySelected)
    );
    let reference = crate::reference::plan_next(&session.state, &request, &binding)?;
    assert!(matches!(
        reference.decision,
        campaign_core::NextDecision::Select(_)
    ));
    let production_request = planned_request(&session, &binding)?;
    let production =
        campaign_core::plan_next(&session.state, production_request.clone(), &binding)?;
    let evidence = crate::reference::plan_next(&session.state, &production_request, &binding)?;
    let report = campaign_core::compare_decisions(&production, &evidence);
    assert!(report.field_identity_equal);
    assert_eq!(report.semantic_work_equal, Some(false));
    assert!(!campaign_core::compare_decisions(&production, &reference).field_identity_equal);
    let conflated = campaign_core::DifferentialReport {
        semantic_work_equal: Some(report.field_identity_equal),
        ..report
    };
    assert_eq!(
        campaign_core::validate_differential_evidence(&conflated, &production, &evidence),
        Err(campaign_core::CampaignError::ConflatedEvidenceQuestion)
    );
    Ok(())
}

/// A selection the readable reference planned under a `ReferenceEvidence` profile never replays:
/// replay re-validates the published profile and refuses the reference implementation.
#[test]
// r[verify campaign.fd.oracle.isolated]
fn a_reference_planned_selection_is_refused_by_replay() -> crate::verdicts::ConformanceResult<()> {
    let profile = super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, 2, |_| {})?;
    session.admit(&[operation(1)])?;
    session.guide(&preference(1, [30, 1]))?;
    let binding = session.binding()?;
    let mut request = planned_request(&session, &binding)?;
    let reference_id = crate::harness::suite_id(campaign_core::IdKind::Policy, "reference")?;
    request.profile.implementation =
        campaign_core::PolicyImplementation::ReferenceEvidence { reference_id };
    request.profile = request.profile.derive()?;
    let reference = crate::reference::plan_next(&session.state, &request, &binding)?;
    let campaign_core::NextDecision::Select(selection) = reference.decision else {
        return Err(crate::verdicts::ConformanceError::Diverged {
            expected: "the reference selects",
        });
    };
    assert_eq!(selection.profile_id, request.profile.profile_id);
    let event = campaign_core::CampaignControlEvent::SelectionPublished(
        campaign_core::SelectionEvent::new(*selection, request.profile, None)?,
    );
    assert_eq!(
        campaign_core::apply_control_event(session.state.clone(), &event),
        Err(campaign_core::CampaignError::ReferencePolicySelected)
    );
    Ok(())
}
