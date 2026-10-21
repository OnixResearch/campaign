//! Reference planner parity with production's check order and bounds (D68, D69): the
//! differential corpus also plans campaigns at a tight control-event bound (the D29 selection
//! headroom and the D69 guidance-wait room), a stopped campaign under requests whose entropy is
//! invalid, and a profile whose decay leaves the admitted bounds' domain. Production and the
//! reference must give the same answer on every state, typed errors included.

/// Budget of the parity campaigns.
const AMPLE_BUDGET: u64 = 64;

/// Budget of the guidance-wait campaign: A's cost fits once and then leaves only B's.
const GUIDANCE_WAIT_BUDGET: u64 = 3;
const COST_A: u64 = 2;
const COST_B: u64 = 1;

/// Control-event bound of the headroom campaign: after the admission, one batch, two guidance
/// records, and one selection, the outstanding settlement leaves no room for another selection.
const HEADROOM_EVENTS: u64 = 7;

/// Control-event bound of the guidance-wait campaign: after the admission, one batch, one
/// guidance record, and one selection, B's guidance and one more selection no longer fit.
const GUIDANCE_WAIT_EVENTS: u64 = 8;

/// A penalty per selection outside the D44 decay domain of the default bounds.
const OUT_OF_DOMAIN_PENALTY: u64 = 1 << 62;

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

const fn quiet() -> crate::vocabulary::ScenarioOutcome {
    crate::vocabulary::ScenarioOutcome {
        children: alloc::vec::Vec::new(),
        findings: 0,
    }
}

fn with_session<T>(
    budget: u64,
    configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
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
    let mut session = crate::harness::Session::open(&mut harness, budget, configure)?;
    body(&mut session)
}

/// The decision each prefix of the session's history plans under a ranked request, after
/// requiring production and the reference to agree on it.
fn prefix_decisions(
    session: &crate::harness::Session<'_, '_>,
) -> crate::verdicts::ConformanceResult<alloc::vec::Vec<campaign_core::NextDecision>> {
    let events = session.control_events();
    let binding = session.binding()?;
    let ranked = session.selection_profile(false)?;
    let mut decisions = alloc::vec::Vec::with_capacity(events.len());
    for end in 1..=events.len() {
        let state = campaign_core::reconstruct(&events[..end])?;
        let request = super::bounds::request(session, &state, &binding, ranked.clone(), None)?;
        decisions.push(super::bounds::agreed(&state, &request, &binding)??.decision);
    }
    Ok(decisions)
}

/// The shape of a decision: selection, wait reason, or stop reason.
#[derive(Debug, Eq, PartialEq)]
enum Shape {
    Select,
    Wait(campaign_core::WaitReason),
    Stop(campaign_core::StopReason),
}

fn shapes(decisions: &[campaign_core::NextDecision]) -> alloc::vec::Vec<Shape> {
    decisions
        .iter()
        .map(|decision| match decision {
            campaign_core::NextDecision::Select(_) => Shape::Select,
            campaign_core::NextDecision::Wait(reason) => Shape::Wait(reason.clone()),
            campaign_core::NextDecision::Stop(stop) => Shape::Stop(stop.reason.clone()),
        })
        .collect()
}

/// D29: once the control events no longer admit one selection, its settlement, and the stop,
/// both planners wait for the outstanding result and then stop with `ControlEventLimitReached`.
#[test]
// r[verify campaign.stop.control_event_limit]
fn production_and_reference_agree_at_the_control_event_headroom()
-> crate::verdicts::ConformanceResult<()> {
    with_session(
        AMPLE_BUDGET,
        |bounds| bounds.max_control_events = HEADROOM_EVENTS,
        |session| {
            session.admit(&[operation(1, 1), operation(2, 1)])?;
            session.guide(&preference(1, [30, 1]))?;
            session.guide(&preference(2, [20, 3]))?;
            let executed = session.execute(None)?;
            session.settle(&executed, &quiet())?;
            assert_eq!(
                shapes(&prefix_decisions(session)?),
                alloc::vec![
                    Shape::Stop(campaign_core::StopReason::FrontierExhausted),
                    Shape::Wait(campaign_core::WaitReason::AwaitingGuidance),
                    Shape::Select,
                    Shape::Select,
                    Shape::Wait(campaign_core::WaitReason::AwaitingResults),
                    Shape::Stop(campaign_core::StopReason::ControlEventLimitReached),
                ]
            );
            Ok(())
        },
    )
}

/// D69: a candidate awaits guidance only while the control events admit its guidance, one
/// selection, that selection's settlement, and the stop; otherwise both planners wait for the
/// outstanding result and then stop with `ControlEventLimitReached`.
#[test]
// r[verify campaign.wait.guidance_room]
fn production_and_reference_agree_on_the_guidance_wait_room()
-> crate::verdicts::ConformanceResult<()> {
    with_session(
        GUIDANCE_WAIT_BUDGET,
        |bounds| bounds.max_control_events = GUIDANCE_WAIT_EVENTS,
        |session| {
            session.admit(&[operation(1, COST_A), operation(2, COST_B)])?;
            session.guide(&preference(1, [30, 1]))?;
            let executed = session.execute(None)?;
            session.settle(&executed, &quiet())?;
            assert_eq!(
                shapes(&prefix_decisions(session)?),
                alloc::vec![
                    Shape::Stop(campaign_core::StopReason::FrontierExhausted),
                    Shape::Wait(campaign_core::WaitReason::AwaitingGuidance),
                    Shape::Select,
                    Shape::Wait(campaign_core::WaitReason::AwaitingResults),
                    Shape::Stop(campaign_core::StopReason::ControlEventLimitReached),
                ]
            );
            Ok(())
        },
    )
}

/// D33 order: entropy is validated before a recorded stop is returned, so a ranked request with
/// a ticket and an exploratory request without one are refused on a stopped campaign, while a
/// valid request receives the recorded stop.
#[test]
fn production_and_reference_check_entropy_before_a_recorded_stop()
-> crate::verdicts::ConformanceResult<()> {
    with_session(
        AMPLE_BUDGET,
        |_| {},
        |session| {
            session.admit(&[operation(1, 1)])?;
            session.guide(&preference(1, [30, 1]))?;
            let reason = session.consumer_stop()?;
            session.stop(reason)?;
            let binding = session.binding()?;
            let state = session.state.clone();
            let ranked = session.selection_profile(false)?;
            let exploratory = session.selection_profile(true)?;
            let draw = crate::vocabulary::ScenarioDraw {
                explore_draw: 0,
                index_draw: 1,
            };
            let ticket = session.ticket(&draw, &exploratory)?;
            let ticketed =
                super::bounds::request(session, &state, &binding, ranked.clone(), Some(ticket))?;
            let unticketed = super::bounds::request(session, &state, &binding, exploratory, None)?;
            for invalid in [ticketed, unticketed] {
                assert_eq!(
                    super::bounds::agreed(&state, &invalid, &binding)?.map(|_| ()),
                    Err(campaign_core::CampaignError::InvalidEntropy)
                );
            }
            let valid = super::bounds::request(session, &state, &binding, ranked, None)?;
            let recorded = super::bounds::agreed(&state, &valid, &binding)??;
            assert!(matches!(
                &recorded.decision,
                campaign_core::NextDecision::Stop(stop) if Some(stop.as_ref()) == state.stopped.as_ref()
            ));
            Ok(())
        },
    )
}

/// D44 domain: a decay penalty outside the admitted bounds' domain is refused by both planners
/// with `InvalidConfig`; a penalty within it selects alike.
#[test]
fn production_and_reference_refuse_a_decay_outside_the_bounds_domain()
-> crate::verdicts::ConformanceResult<()> {
    with_session(
        AMPLE_BUDGET,
        |_| {},
        |session| {
            session.admit(&[operation(1, 1)])?;
            session.guide(&preference(1, [30, 1]))?;
            let binding = session.binding()?;
            let state = session.state.clone();
            for (penalty, is_within) in [(OUT_OF_DOMAIN_PENALTY, false), (1, true)] {
                let mut decaying = session.selection_profile(false)?;
                decaying.decay = campaign_core::SelectionCountDecay::SubtractFromFirstRank {
                    penalty_per_selection: penalty,
                };
                decaying.profile_id = campaign_core::Id::absent();
                let decaying = decaying.derive()?;
                let request = super::bounds::request(session, &state, &binding, decaying, None)?;
                let planned = super::bounds::agreed(&state, &request, &binding)?;
                if is_within {
                    assert!(matches!(
                        planned?.decision,
                        campaign_core::NextDecision::Select(_)
                    ));
                } else {
                    assert_eq!(
                        planned.map(|_| ()),
                        Err(campaign_core::CampaignError::InvalidConfig)
                    );
                }
            }
            Ok(())
        },
    )
}
