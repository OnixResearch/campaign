//! `campaign.fd.scheduling.declared` controls: declared rules explain differing selections.

#[test]
// r[verify campaign.fd.scheduling.declared]
fn declared_priority_explains_two_different_selection_orders() -> Result<(), String> {
    let frontier = crate::suite::schedules::frontier()?;
    let heavy = crate::suite::schedules::variant(|profile| {
        profile.decay = campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: crate::suite::schedules::HEAVY_PENALTY,
        };
    })?;
    let (plain_plan, plain_request) = crate::suite::schedules::planned(
        &frontier.state,
        crate::suite::schedules::undecayed()?,
        None,
    )?;
    let (heavy_plan, heavy_request) =
        crate::suite::schedules::planned(&frontier.state, heavy, None)?;
    assert_eq!(
        crate::suite::schedules::selected(&plain_plan)?,
        frontier.first
    );
    assert_eq!(
        crate::suite::schedules::selected(&heavy_plan)?,
        frontier.second
    );
    let difference = crate::suite::world::observed(campaign_core::explain_selection_difference(
        &frontier.state,
        &frontier.state.admission.projection,
        campaign_core::PlannedRequest {
            plan: &plain_plan,
            request: &plain_request,
        },
        campaign_core::PlannedRequest {
            plan: &heavy_plan,
            request: &heavy_request,
        },
    ))?;
    assert_eq!(
        difference,
        campaign_core::SelectionDifference {
            scheduling: campaign_core::SchedulingComparison {
                admission_differs: false,
                priority_differs: true,
                stopping_differs: false,
            },
            entropy_differs: false,
            outcome_differs: true,
        }
    );
    Ok(())
}

#[test]
// r[verify campaign.fd.scheduling]
fn comparison_reports_each_declared_rule_separately() -> Result<(), String> {
    let base = crate::suite::schedules::declared_of(&crate::suite::schedules::undecayed()?)?;
    let admission =
        crate::suite::schedules::declared_of(&crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.admission_policy_id = crate::suite::admission::xid(
                campaign_core::IdKind::AdmissionPolicy,
                b"other-admission",
            );
        })?)?;
    let stopping =
        crate::suite::schedules::declared_of(&crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.stopping.finding_limit = Some(crate::suite::schedules::FINDING_LIMIT);
        })?)?;
    let relabelled =
        crate::suite::schedules::declared_of(&crate::suite::schedules::variant(|profile| {
            profile.decay = campaign_core::SelectionCountDecay::None;
            profile.scheduling_policy_id = crate::suite::admission::xid(
                campaign_core::IdKind::SchedulingProfile,
                b"relabelled",
            );
        })?)?;
    let only = |admission_differs, priority_differs, stopping_differs| {
        campaign_core::SchedulingComparison {
            admission_differs,
            priority_differs,
            stopping_differs,
        }
    };
    assert_eq!(
        campaign_core::compare_scheduling(&base, &admission),
        only(true, false, false)
    );
    assert_eq!(
        campaign_core::compare_scheduling(&base, &stopping),
        only(false, false, true)
    );
    let decayed =
        crate::suite::schedules::declared_of(&crate::suite::selection::ranked_profile()?)?;
    assert_eq!(
        campaign_core::compare_scheduling(&base, &decayed),
        only(false, true, false)
    );
    let relabelled_comparison = campaign_core::compare_scheduling(&base, &relabelled);
    assert_eq!(relabelled_comparison, only(false, false, false));
    assert!(!relabelled_comparison.declares_difference());
    assert_eq!(
        base.priority.tie_break,
        campaign_core::TieBreak::CandidateIdAscending
    );
    Ok(())
}
