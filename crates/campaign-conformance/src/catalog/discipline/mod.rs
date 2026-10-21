//! Scheduling-declaration, reference-isolation, and differential-evidence fixtures.

/// Discipline fixtures in suite order.
// r[impl campaign.fd.verification]
pub(super) const FIXTURES: &[super::Fixture] = &[
    super::Fixture {
        name: "scheduling.declared_difference",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: declared_difference,
    },
    super::Fixture {
        name: "scheduling.ambiguous_schedule",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: ambiguous_schedule,
    },
    super::Fixture {
        name: "oracle.reference_override",
        polarity: crate::verdicts::FixturePolarity::Negative,
        body: reference_override,
    },
    super::Fixture {
        name: "oracle.differential_evidence",
        polarity: crate::verdicts::FixturePolarity::Positive,
        body: differential_evidence,
    },
];

/// Explore-index draws tried until the explore branch picks another operation than ranking.
const EXPLORE_INDICES: [u64; 3] = [0, 1, 2];

fn declared_difference(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let order = session.harness.profile.exploration_order.clone();
    let mut index_draw = 0;
    for index in EXPLORE_INDICES {
        if super::common::explored(&order, index)? != super::common::OP_A {
            index_draw = index;
            break;
        }
    }
    let ranked = session.plan(None)?;
    let explored = session.plan(Some(crate::vocabulary::ScenarioDraw {
        explore_draw: 0,
        index_draw,
    }))?;
    debug_assert_eq!(ranked.plan.state_id, explored.plan.state_id);
    debug_assert_ne!(
        ranked.request.profile.profile_id,
        explored.request.profile.profile_id
    );
    let difference = campaign_core::explain_selection_difference(
        &session.state,
        &ranked.binding,
        campaign_core::PlannedRequest {
            plan: &ranked.plan,
            request: &ranked.request,
        },
        campaign_core::PlannedRequest {
            plan: &explored.plan,
            request: &explored.request,
        },
    )?;
    super::common::require(
        difference.outcome_differs,
        "the two policies select differently",
    )?;
    super::common::require(
        difference.scheduling.priority_differs,
        "the declared priority rule explains the difference",
    )
}

fn ambiguous_schedule(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let planned = session.plan(None)?;
    let mut undeclared = planned.plan.clone();
    let other = session.candidate_id(super::common::OP_B)?;
    if let campaign_core::NextDecision::Select(selection) = &mut undeclared.decision {
        selection.candidate_id = other;
    }
    debug_assert_ne!(undeclared, planned.plan);
    debug_assert_eq!(undeclared.state_id, planned.plan.state_id);
    let explained = campaign_core::explain_selection_difference(
        &session.state,
        &planned.binding,
        campaign_core::PlannedRequest {
            plan: &planned.plan,
            request: &planned.request,
        },
        campaign_core::PlannedRequest {
            plan: &undeclared,
            request: &planned.request,
        },
    );
    session.harness.deny(
        explained.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::UndeclaredSchedulingDifference,
        "equal declared rules and entropy explained different selections",
    )
}

fn reference_override(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    super::common::guided_frontier(&mut session, super::common::UNIT_COST)?;
    let current = session.load()?;
    let mut request = session.request(None, current.binding())?;
    request.profile.implementation = campaign_core::PolicyImplementation::ReferenceEvidence {
        reference_id: crate::harness::suite_id(campaign_core::IdKind::Policy, "reference")?,
    };
    request.profile = request.profile.derive()?;
    let planned = campaign_choregraph::plan_next_from_published_projection(
        &current,
        campaign_choregraph::PublishedSelectionRequest { request },
    );
    session.harness.deny(
        planned.map_err(crate::verdicts::ConformanceError::from),
        campaign_core::CampaignError::ReferencePolicySelected,
        "production dispatch selected the reference policy",
    )
}

/// Compares production and reference plans for the session's current state and records the
/// separated answers.
fn compare_current(
    session: &mut crate::harness::Session<'_, '_>,
    draw: Option<crate::vocabulary::ScenarioDraw>,
) -> crate::verdicts::ConformanceResult<campaign_core::DifferentialReport> {
    let current = session.load()?;
    let request = session.request(draw, current.binding())?;
    let production = campaign_core::plan_next(current.state(), request.clone(), current.binding())?;
    let reference = crate::reference::plan_next(current.state(), &request, current.binding())?;
    let report = campaign_core::compare_decisions(&production, &reference);
    debug_assert_eq!(production.state_id, reference.state_id);
    debug_assert_eq!(report.time, campaign_core::EvidenceQuestion::NotMeasured);
    campaign_core::validate_differential_evidence(&report, &production, &reference)?;
    super::common::require(
        report.field_identity_equal,
        "reference fields equal production",
    )?;
    let label = alloc::format!(
        "fields-equal={} work-equal={:?} time={:?} memory={:?}",
        report.field_identity_equal,
        report.semantic_work_equal,
        report.time,
        report.memory
    );
    session.record(crate::ledger::TranscriptEntry::labelled(
        crate::ledger::kinds::COMPARED,
        &label,
    ));
    Ok(report)
}

fn differential_evidence(
    harness: &mut crate::harness::Driver<'_>,
) -> crate::verdicts::ConformanceResult<()> {
    let mut session = super::common::ample(harness)?;
    let mut reports = alloc::vec![compare_current(&mut session, None)?];
    session.admit(&[super::common::operation(
        super::common::OP_A,
        crate::harness::history::ROOT,
        super::common::UNIT_COST,
    )])?;
    reports.push(compare_current(&mut session, None)?);
    session.guide(&super::common::preference(
        super::common::OP_A,
        super::common::FIRST_GENERATION,
        super::common::LEVELS_A,
    ))?;
    session.admit(&[
        super::common::operation(
            super::common::OP_B,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
        super::common::operation(
            super::common::OP_C,
            crate::harness::history::ROOT,
            super::common::UNIT_COST,
        ),
    ])?;
    session.guide(&super::common::preference(
        super::common::OP_B,
        super::common::FIRST_GENERATION,
        super::common::LEVELS_B,
    ))?;
    reports.push(compare_current(&mut session, None)?);
    let draw = crate::vocabulary::ScenarioDraw {
        explore_draw: 0,
        index_draw: 1,
    };
    reports.push(compare_current(&mut session, Some(draw))?);
    let selection = session.execute(None)?;
    reports.push(compare_current(&mut session, None)?);
    session.settle(&selection, &super::common::outcome(&[]))?;
    let reason = session.consumer_stop()?;
    session.stop(reason)?;
    reports.push(compare_current(&mut session, None)?);
    debug_assert!(!reports.is_empty());
    debug_assert!(session.state.stopped.is_some());
    let has_equal_work = reports
        .iter()
        .any(|report| report.semantic_work_equal == Some(true));
    let has_unequal_work = reports
        .iter()
        .any(|report| report.semantic_work_equal == Some(false));
    super::common::require(
        has_equal_work,
        "some equal-field decisions report equal work",
    )?;
    super::common::require(has_unequal_work, "equal fields never imply equal work")
}
