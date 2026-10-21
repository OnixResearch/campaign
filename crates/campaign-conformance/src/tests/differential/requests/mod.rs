//! Reference planner request checks (D80): the reference runs production's request checks in
//! production's order, so a state whose identity is not its re-derived identity, a request
//! without a request identity, a projection binding whose identity is not its re-derived
//! identity, and a binding of another adapter's envelope are refused alike, with the same typed
//! error, by production and the reference.

/// Budget of the request-check campaigns.
const AMPLE_BUDGET: u64 = 64;

fn with_guided<T>(
    body: impl FnOnce(
        &mut crate::harness::Session<'_, '_>,
        campaign_core::SelectionRequest,
        campaign_core::ProjectionBinding,
    ) -> crate::verdicts::ConformanceResult<T>,
) -> crate::verdicts::ConformanceResult<T> {
    let profile = super::super::default_profile()?;
    let mut adapter = crate::fixtures::animus_shaped::OpaqueCandidateAdapter::new();
    let mut transcript = crate::ledger::DecisionTranscript::default();
    let mut harness = crate::harness::Driver {
        adapter: &mut adapter,
        profile: &profile,
        transcript: &mut transcript,
    };
    let mut session = crate::harness::Session::open(&mut harness, AMPLE_BUDGET, |_| {})?;
    session.admit(&[crate::vocabulary::ScenarioOperation {
        handle: crate::vocabulary::OperationHandle(1),
        parent: crate::harness::history::ROOT,
        costs: alloc::vec![crate::vocabulary::AbstractCost {
            dimension: 0,
            units: 1,
        }],
    }])?;
    session.guide(&crate::vocabulary::ScenarioPreference {
        operation: crate::vocabulary::OperationHandle(1),
        generation: 1,
        score: crate::vocabulary::AbstractScore {
            levels: alloc::vec![30, 1],
        },
    })?;
    let binding = session.binding()?;
    let state = session.state.clone();
    let profile = session.selection_profile(false)?;
    let request = super::bounds::request(&session, &state, &binding, profile, None)?;
    assert_eq!(request.state_id, session.state.state_id);
    assert!(matches!(
        super::bounds::agreed(&state, &request, &binding)??.decision,
        campaign_core::NextDecision::Select(_)
    ));
    body(&mut session, request, binding)
}

#[test]
// r[verify campaign.selection.invalid]
fn production_and_reference_refuse_a_state_whose_identity_is_not_re_derived()
-> crate::verdicts::ConformanceResult<()> {
    with_guided(|session, request, binding| {
        let mut stale = session.state.clone();
        stale.selection_count = stale.selection_count.saturating_add(1);
        assert_eq!(
            super::bounds::agreed(&stale, &request, &binding)?.map(|_| ()),
            Err(campaign_core::CampaignError::StaleState)
        );
        Ok(())
    })
}

#[test]
// r[verify campaign.identity.missing]
fn production_and_reference_refuse_a_request_without_a_request_identity()
-> crate::verdicts::ConformanceResult<()> {
    with_guided(|session, request, binding| {
        let unnamed = campaign_core::SelectionRequest {
            request_id: campaign_core::Id::absent(),
            ..request
        };
        assert_eq!(
            super::bounds::agreed(&session.state, &unnamed, &binding)?.map(|_| ()),
            Err(campaign_core::CampaignError::MissingIdentity)
        );
        Ok(())
    })
}

#[test]
// r[verify campaign.history_projection.stale]
fn production_and_reference_refuse_a_binding_whose_identity_is_not_re_derived()
-> crate::verdicts::ConformanceResult<()> {
    with_guided(|session, request, binding| {
        let moved = campaign_core::ProjectionBinding {
            graph_id: crate::harness::suite_id(campaign_core::IdKind::Graph, "moved-graph")?,
            ..binding
        };
        assert_eq!(
            super::bounds::agreed(&session.state, &request, &moved)?.map(|_| ()),
            Err(campaign_core::CampaignError::StaleProjection)
        );
        Ok(())
    })
}

#[test]
// r[verify campaign.history_projection]
fn production_and_reference_refuse_a_binding_of_another_adapter()
-> crate::verdicts::ConformanceResult<()> {
    with_guided(|session, request, binding| {
        let foreign = campaign_core::ProjectionBinding {
            adapter_id: crate::harness::suite_id(campaign_core::IdKind::Adapter, "foreign")?,
            ..binding
        }
        .derive()?;
        assert_eq!(
            super::bounds::agreed(&session.state, &request, &foreign)?.map(|_| ()),
            Err(campaign_core::CampaignError::CrossedIdentity)
        );
        Ok(())
    })
}
