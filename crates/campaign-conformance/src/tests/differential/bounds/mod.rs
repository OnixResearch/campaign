//! Reference planner parity at the bounds (D59): the differential corpus also plans a campaign
//! whose reservations reach a `u64::MAX` budget and a request whose exploration order names a
//! dimension beyond the campaign's rank shape; production and the reference must give the same
//! answer, typed errors included. Both also refuse a guidance projection that lacks a declared
//! dimension.

/// Budget of the `u64::MAX` corpus campaign.
const WIDE_BUDGET: u64 = u64::MAX;

/// Cost of the operation that reserves all but one unit of the wide budget.
const WIDE_COST: u64 = u64::MAX.saturating_sub(1);

/// Cost of the operations that no longer fit once the wide cost is reserved.
const NARROW_COST: u64 = 2;

/// Budget of the rank-shape corpus campaigns.
const AMPLE_BUDGET: u64 = 64;

/// Explore draw inside and outside the default threshold (`100_000` parts per million).
const EXPLORE_DRAW: u64 = 0;
const EXPLOIT_DRAW: u64 = 999_999;

fn operation(handle: u32, units: u64) -> crate::vocabulary::ScenarioOperation {
    crate::vocabulary::ScenarioOperation {
        handle: crate::vocabulary::OperationHandle(handle),
        parent: crate::harness::history::ROOT,
        costs: alloc::vec![crate::vocabulary::AbstractCost {
            dimension: 0,
            units,
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

/// Production's answer for one state and request after requiring the reference to give the same
/// answer: equal typed errors, or plans whose decision fields agree.
pub(super) fn agreed(
    state: &campaign_core::FrontierState,
    request: &campaign_core::SelectionRequest,
    binding: &campaign_core::ProjectionBinding,
) -> crate::verdicts::ConformanceResult<campaign_core::CampaignResult<campaign_core::DecisionPlan>>
{
    let production = campaign_core::plan_next(state, request.clone(), binding);
    let reference = crate::reference::plan_next(state, request, binding);
    match (&production, &reference) {
        (Ok(production), Ok(reference)) => {
            let report = campaign_core::compare_decisions(production, reference);
            assert!(
                report.field_identity_equal,
                "{production:?} != {reference:?}"
            );
            campaign_core::validate_differential_evidence(&report, production, reference)?;
        }
        (Err(production), Err(reference)) => assert_eq!(production, reference),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "production and the reference give the same answer",
            });
        }
    }
    Ok(production)
}

pub(super) fn request(
    session: &crate::harness::Session<'_, '_>,
    state: &campaign_core::FrontierState,
    binding: &campaign_core::ProjectionBinding,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> crate::verdicts::ConformanceResult<campaign_core::SelectionRequest> {
    Ok(campaign_core::SelectionRequest {
        request_id: crate::harness::suite_id(campaign_core::IdKind::Request, "bounds")?,
        campaign_id: session.state.admission.campaign_id,
        state_id: state.state_id,
        profile,
        entropy,
        expected_generation: binding.generation,
        expected_head_id: binding.head_id,
    })
}

/// An exploratory request for the session's current state under `exploration_order`, with a
/// ticket carrying `explore_draw`.
fn exploratory(
    session: &mut crate::harness::Session<'_, '_>,
    binding: &campaign_core::ProjectionBinding,
    exploration_order: &[u32],
    explore_draw: u64,
) -> crate::verdicts::ConformanceResult<campaign_core::SelectionRequest> {
    let mut profile = session.selection_profile(true)?;
    if let campaign_core::SelectionMode::Exploratory {
        exploration_order: order,
        ..
    } = &mut profile.mode
    {
        *order = exploration_order.to_vec();
    }
    profile.profile_id = campaign_core::Id::absent();
    let profile = profile.derive()?;
    let draw = crate::vocabulary::ScenarioDraw {
        explore_draw,
        index_draw: 1,
    };
    let ticket = session.ticket(&draw, &profile)?;
    let state = session.state.clone();
    request(session, &state, binding, profile, Some(ticket))
}

fn with_session<T>(
    budget: u64,
    body: impl FnOnce(&mut crate::harness::Session<'_, '_>) -> crate::verdicts::ConformanceResult<T>,
) -> crate::verdicts::ConformanceResult<T> {
    let profile = super::super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, budget, |_| {})?;
    body(&mut session)
}

/// A campaign whose first selection reserves `u64::MAX - 1` of a `u64::MAX` budget: every prefix
/// of its history plans alike in production and the reference, and the last state, where no
/// retained candidate fits (one of them unguided), stops with the budget dimension. The
/// reference computes the fit as `cost <= budget - reserved` instead of overflowing
/// `reserved + cost`.
#[test]
// r[verify campaign.budgets.fit]
fn production_and_reference_agree_when_reservations_reach_a_u64_budget()
-> crate::verdicts::ConformanceResult<()> {
    with_session(WIDE_BUDGET, |session| {
        session.admit(&[
            operation(1, WIDE_COST),
            operation(2, NARROW_COST),
            operation(3, NARROW_COST),
        ])?;
        session.guide(&preference(1, [30, 1]))?;
        session.guide(&preference(2, [20, 3]))?;
        let executed = session.execute(None)?;
        session.settle(
            &executed,
            &crate::vocabulary::ScenarioOutcome {
                children: alloc::vec::Vec::new(),
                findings: 0,
            },
        )?;
        let events = session.control_events();
        let binding = session.binding()?;
        let ranked = session.selection_profile(false)?;
        let mut decisions = alloc::vec::Vec::with_capacity(events.len());
        for end in 1..=events.len() {
            let state = campaign_core::reconstruct(&events[..end])?;
            let request = request(session, &state, &binding, ranked.clone(), None)?;
            decisions.push(agreed(&state, &request, &binding)??.decision);
        }
        let dimensions: alloc::vec::Vec<campaign_core::Id> = session
            .state
            .admission
            .resource_budgets
            .iter()
            .map(|budget| budget.dimension_id)
            .collect();
        assert_eq!(dimensions.len(), 1);
        assert!(matches!(
            decisions.last(),
            Some(campaign_core::NextDecision::Stop(stop))
                if stop.reason == campaign_core::StopReason::BudgetExhausted { dimensions }
        ));
        assert!(decisions.iter().any(|decision| matches!(
            decision,
            campaign_core::NextDecision::Wait(campaign_core::WaitReason::AwaitingGuidance)
        )));
        assert!(
            decisions
                .iter()
                .any(|decision| matches!(decision, campaign_core::NextDecision::Select(_)))
        );
        Ok(())
    })
}

/// An exploration order naming a dimension beyond the campaign's two-level rank shape is refused
/// by both planners with `InvalidConfig` whatever the draw; an order within the shape selects
/// alike.
#[test]
// r[verify campaign.guidance.declared_order]
fn production_and_reference_refuse_an_exploration_order_beyond_the_rank_shape()
-> crate::verdicts::ConformanceResult<()> {
    with_session(AMPLE_BUDGET, |session| {
        session.admit(&[operation(1, 1), operation(2, 1)])?;
        session.guide(&preference(1, [30, 1]))?;
        session.guide(&preference(2, [20, 3]))?;
        let binding = session.binding()?;
        for draw in [EXPLORE_DRAW, EXPLOIT_DRAW] {
            let beyond = exploratory(session, &binding, &[2], draw)?;
            assert_eq!(
                agreed(&session.state, &beyond, &binding)?.map(|_| ()),
                Err(campaign_core::CampaignError::InvalidConfig)
            );
            let within = exploratory(session, &binding, &[1], draw)?;
            assert!(matches!(
                agreed(&session.state, &within, &binding)??.decision,
                campaign_core::NextDecision::Select(_)
            ));
        }
        Ok(())
    })
}

/// A guidance projection without a dimension the exploration order names is refused with
/// `InvalidGuidanceShape` on the explore branch by production's exploration ordering and by the
/// reference alike; the exploit branch never projects and selects. No admitted state reaches this
/// check (the order check above refuses first), so the state is cut by hand and its identity
/// re-derived: the campaign's first guided candidate keeps two levels, a later one is cut to one.
/// Without the re-derived identity both refuse the cut state earlier (`StaleState`).
#[test]
fn the_reference_refuses_a_missing_projected_dimension() -> crate::verdicts::ConformanceResult<()> {
    with_session(AMPLE_BUDGET, |session| {
        session.admit(&[operation(1, 1), operation(2, 1)])?;
        session.guide(&preference(1, [30, 1]))?;
        session.guide(&preference(2, [20, 3]))?;
        let binding = session.binding()?;
        let mut cut = session.state.clone();
        let last = cut
            .candidates
            .last_mut()
            .and_then(|candidate| candidate.latest_guidance.as_mut())
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "the last candidate is guided",
            })?;
        last.ranks.truncate(1);
        let stale = cut.clone();
        cut.recompute_identity()?;
        let admitted = core::mem::replace(&mut session.state, cut.clone());
        let explore = exploratory(session, &binding, &[1], EXPLORE_DRAW)?;
        let exploit = exploratory(session, &binding, &[1], EXPLOIT_DRAW)?;
        session.state = admitted;
        assert_eq!(
            agreed(&cut, &explore, &binding)?.map(|_| ()),
            Err(campaign_core::CampaignError::InvalidGuidanceShape)
        );
        assert!(matches!(
            agreed(&cut, &exploit, &binding)??.decision,
            campaign_core::NextDecision::Select(_)
        ));
        assert_eq!(
            agreed(&stale, &explore, &binding)?.map(|_| ()),
            Err(campaign_core::CampaignError::StaleState)
        );
        Ok(())
    })
}
