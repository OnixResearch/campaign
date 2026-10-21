//! `campaign.fd.evidence` controls: field identity and semantic work are separate questions.

const RANK_SECOND: i64 = 15;

/// Readable two-pass reference planner over one state: a filtering pass, then a ranking pass.
/// It reports its own work and must reach the production decision.
fn reference_plan(
    state: &campaign_core::FrontierState,
    production: &campaign_core::DecisionPlan,
) -> Result<campaign_core::DecisionPlan, String> {
    let mut examined: u64 = 0;
    let mut eligible = Vec::with_capacity(state.candidates.len());
    for candidate in &state.candidates {
        examined = examined.checked_add(1).ok_or("work overflow")?;
        if candidate.status == campaign_core::CandidateStatus::Retained
            && let Some(guidance) = &candidate.latest_guidance
        {
            eligible.push((guidance.ranks.clone(), candidate.candidate.candidate_id));
        }
    }
    let mut best: Option<&(Vec<i64>, campaign_core::Id)> = None;
    for entry in &eligible {
        examined = examined.checked_add(1).ok_or("work overflow")?;
        best = match best {
            Some(current)
                if (current.0 > entry.0) || (current.0 == entry.0 && current.1 < entry.1) =>
            {
                Some(current)
            }
            Some(_) | None => Some(entry),
        };
    }
    let reference_choice = best.map(|entry| entry.1);
    let production_choice = match &production.decision {
        campaign_core::NextDecision::Select(selection) => Some(selection.candidate_id),
        campaign_core::NextDecision::Wait(_) | campaign_core::NextDecision::Stop(_) => None,
    };
    assert_eq!(reference_choice, production_choice);
    assert!(eligible.len() <= state.candidates.len());
    let mut reference = production.clone();
    reference.work = campaign_core::PlanningWork {
        candidates_examined: examined,
        eligible_candidates: u64::try_from(eligible.len()).map_err(|error| format!("{error}"))?,
    };
    Ok(reference)
}

fn production_plan(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::DecisionPlan, String> {
    let request =
        crate::suite::selection::request(state, crate::suite::selection::ranked_profile()?, None);
    crate::suite::world::observed(campaign_core::plan_next(
        state,
        request,
        &state.admission.projection,
    ))
}

fn corpus() -> Result<Vec<campaign_core::FrontierState>, String> {
    let cost = crate::suite::constants::COST_ONE;
    let (events, empty) = crate::suite::admission::initial_state()?;
    let (events, unguided, first) = crate::suite::candidate::admit(events, &empty, b"first", cost)?;
    let (events, one_guided, _g) = crate::suite::candidate::guide(
        events,
        &unguided,
        first,
        crate::suite::constants::RANK_HIGH,
        cost,
    )?;
    let (events, with_second, second) =
        crate::suite::candidate::admit(events, &one_guided, b"second", cost)?;
    let (_events, two_guided, _g) =
        crate::suite::candidate::guide(events, &with_second, second, RANK_SECOND, cost)?;
    Ok(vec![empty, unguided, two_guided])
}

#[test]
// r[verify campaign.fd.evidence.questions]
fn equal_fields_and_unequal_work_are_reported_as_separate_facts() -> Result<(), String> {
    let mut is_work_different = false;
    let mut is_work_matching = false;
    for state in corpus()? {
        let production = production_plan(&state)?;
        let reference = reference_plan(&state, &production)?;
        let report = campaign_core::compare_decisions(&production, &reference);
        assert!(report.field_identity_equal);
        assert_eq!(report.time, campaign_core::EvidenceQuestion::NotMeasured);
        assert_eq!(report.memory, campaign_core::EvidenceQuestion::NotMeasured);
        let is_work_equal = production.work == reference.work;
        assert_eq!(report.semantic_work_equal, Some(is_work_equal));
        is_work_different |= !is_work_equal;
        is_work_matching |= is_work_equal;
        crate::suite::world::observed(campaign_core::validate_differential_evidence(
            &report,
            &production,
            &reference,
        ))?;
    }
    assert!(is_work_different);
    assert!(is_work_matching);
    Ok(())
}

#[test]
// r[verify campaign.fd.evidence.questions]
fn a_report_that_infers_work_from_fields_is_rejected() -> Result<(), String> {
    let states = corpus()?;
    let selecting = states.last().ok_or("corpus must not be empty")?;
    let production = production_plan(selecting)?;
    let reference = reference_plan(selecting, &production)?;
    assert_ne!(production.work, reference.work);
    let measured = campaign_core::compare_decisions(&production, &reference);
    let inferred = campaign_core::DifferentialReport {
        semantic_work_equal: Some(measured.field_identity_equal),
        ..measured
    };
    let unanswered = campaign_core::DifferentialReport {
        semantic_work_equal: None,
        ..measured
    };
    let misreported_fields = campaign_core::DifferentialReport {
        field_identity_equal: false,
        ..measured
    };
    for report in [inferred, unanswered, misreported_fields] {
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_differential_evidence(
                &report,
                &production,
                &reference,
            ))?,
            campaign_core::CampaignError::ConflatedEvidenceQuestion
        );
    }
    Ok(())
}

#[test]
fn different_fields_with_equal_work_are_also_reported_separately() -> Result<(), String> {
    let states = corpus()?;
    let selecting = states.last().ok_or("corpus must not be empty")?;
    let production = production_plan(selecting)?;
    let mut other_choice = production.clone();
    if let campaign_core::NextDecision::Select(selection) = &mut other_choice.decision {
        let chosen = selection.candidate_id;
        selection.candidate_id = selecting
            .candidates
            .iter()
            .map(|candidate| candidate.candidate.candidate_id)
            .find(|candidate_id| *candidate_id != chosen)
            .ok_or("corpus needs two candidates")?;
    }
    let report = campaign_core::compare_decisions(&production, &other_choice);
    assert!(!report.field_identity_equal);
    assert_eq!(report.semantic_work_equal, Some(true));
    crate::suite::world::observed(campaign_core::validate_differential_evidence(
        &report,
        &production,
        &other_choice,
    ))?;
    let unmeasured_work = campaign_core::DifferentialReport {
        semantic_work_equal: None,
        ..report
    };
    crate::suite::world::observed(campaign_core::validate_differential_evidence(
        &unmeasured_work,
        &production,
        &other_choice,
    ))?;
    Ok(())
}

/// Time and memory answers the core never measures are rejected, including answers copied from
/// field equality.
#[test]
// r[verify campaign.fd.evidence]
fn unmeasured_time_and_memory_answers_are_rejected() -> Result<(), String> {
    let states = corpus()?;
    let selecting = states.last().ok_or("corpus must not be empty")?;
    let production = production_plan(selecting)?;
    let reference = reference_plan(selecting, &production)?;
    let measured = campaign_core::compare_decisions(&production, &reference);
    assert_eq!(measured.time, campaign_core::EvidenceQuestion::NotMeasured);
    assert_eq!(
        measured.memory,
        campaign_core::EvidenceQuestion::NotMeasured
    );
    crate::suite::world::observed(campaign_core::validate_differential_evidence(
        &measured,
        &production,
        &reference,
    ))?;
    let copied = campaign_core::EvidenceQuestion::Measured {
        equal: measured.field_identity_equal,
    };
    for report in [
        campaign_core::DifferentialReport {
            time: copied,
            ..measured
        },
        campaign_core::DifferentialReport {
            memory: copied,
            ..measured
        },
    ] {
        assert_eq!(
            crate::suite::world::denial(campaign_core::validate_differential_evidence(
                &report,
                &production,
                &reference,
            ))?,
            campaign_core::CampaignError::ConflatedEvidenceQuestion
        );
    }
    Ok(())
}
