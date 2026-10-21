//! Deterministic retention plans over the declared rank order.

mod refusal_precedence;
mod refusals;

const RETAINED: u64 = 2;
pub(crate) const WIDE_BATCH: u64 = 16;
pub(crate) const NARROW_BATCH: u64 = 2;
const RANK_A: i64 = 40;
const RANK_B: i64 = 30;
const RANK_C: i64 = 20;
const RANK_D: i64 = 10;
pub(crate) const FIRST_DIMENSION: u32 = 0;
/// The frontier's guidance has one rank dimension, so dimension 1 lies beyond its fixed shape.
pub(crate) const BEYOND_RANK_SHAPE_DIMENSION: u32 = 1;
pub(crate) const OUT_OF_BOUND_DIMENSION: u32 = 16;

pub(crate) struct Frontier {
    pub(crate) events: crate::suite::s3_fixture::History,
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) top: campaign_core::Id,
    pub(crate) kept: campaign_core::Id,
    pub(crate) middle: campaign_core::Id,
    pub(crate) lowest: campaign_core::Id,
    pub(crate) unguided: campaign_core::Id,
}

/// Five candidates over a retained bound of two: `top` is outstanding, `unguided` has no guidance.
pub(crate) fn frontier(prune_batch: u64) -> Result<Frontier, String> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(|bounds| {
        bounds.max_retained_candidates = RETAINED;
        bounds.max_prune_batch = prune_batch;
    })?;
    let (events, state, top) =
        crate::suite::s3_fixture::guided(events, &state, b"prune-a", RANK_A)?;
    let (events, state, kept) =
        crate::suite::s3_fixture::guided(events, &state, b"prune-b", RANK_B)?;
    let (events, state, middle) =
        crate::suite::s3_fixture::guided(events, &state, b"prune-c", RANK_C)?;
    let (events, state, lowest) =
        crate::suite::s3_fixture::guided(events, &state, b"prune-d", RANK_D)?;
    let (events, state, unguided) = crate::suite::candidate::admit(
        events,
        &state,
        b"prune-e",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, selection) = crate::suite::s3_fixture::published(events, &state)?;
    assert_eq!(selection.candidate_id, top);
    assert!(state.has_outstanding_for_candidate(top));
    Ok(Frontier {
        events,
        state,
        top,
        kept,
        middle,
        lowest,
        unguided,
    })
}

pub(crate) fn profile(label: &'static [u8]) -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::RetentionProfile, label)
}

pub(crate) fn plan_request(
    state: &campaign_core::FrontierState,
    rank_order: Vec<u32>,
) -> campaign_core::RetentionPlanRequest {
    campaign_core::RetentionPlanRequest {
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        retention_profile_id: profile(b"retention"),
        rank_order,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"over-limit"),
    }
}

pub(crate) fn explicit(
    state: &campaign_core::FrontierState,
    candidates: Vec<campaign_core::Id>,
) -> campaign_core::PruneRequest {
    campaign_core::PruneRequest {
        prune_plan_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        candidates,
        reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, b"explicit"),
        retention_profile_id: None,
        rank_order: Vec::new(),
        claims_artifact_deletion: false,
        claims_history_rewrite: false,
        claims_safe_deletion: false,
    }
}

fn sorted(mut ids: Vec<campaign_core::Id>) -> Vec<campaign_core::Id> {
    ids.sort();
    ids
}

pub(crate) fn forged_prune(
    state: &campaign_core::FrontierState,
    mut request: campaign_core::PruneRequest,
) -> Result<campaign_core::CampaignControlEvent, String> {
    request.candidates.sort();
    request.prune_plan_id =
        campaign_core::derive_prune_plan_id(&request).map_err(crate::suite::s3_fixture::err)?;
    let invalidation_id =
        campaign_core::derive_prune_invalidation_id(request.prune_plan_id, request.reason_id)
            .map_err(crate::suite::s3_fixture::err)?;
    let event_id = campaign_core::derive_event_id(
        &campaign_core::ControlEventKind::CandidatesPruned,
        state.admission.campaign_id,
        state.state_id,
        &[request.prune_plan_id, invalidation_id],
    )
    .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(invalidation_id.kind, campaign_core::IdKind::Invalidation);
    assert_eq!(request.prune_plan_id.kind, campaign_core::IdKind::PrunePlan);
    Ok(campaign_core::CampaignControlEvent::CandidatesPruned(
        campaign_core::PruneEvent {
            event_id,
            campaign_id: state.admission.campaign_id,
            state_before_id: state.state_id,
            pruned_candidates: request.candidates,
            prune_plan_id: request.prune_plan_id,
            reason_id: request.reason_id,
            retention_profile_id: request.retention_profile_id,
            rank_order: request.rank_order,
            invalidation_id,
        },
    ))
}

#[test]
// r[verify campaign.pruning.valid]
fn a_wide_batch_prunes_every_candidate_above_the_retained_bound() -> Result<(), String> {
    let wide = frontier(WIDE_BATCH)?;
    let wide_plan = campaign_core::plan_pruning(
        &wide.state,
        plan_request(&wide.state, vec![FIRST_DIMENSION]),
    )
    .map_err(crate::suite::s3_fixture::err)?
    .ok_or_else(|| String::from("expected a prune plan"))?;
    assert_eq!(
        wide_plan.candidates,
        sorted(vec![wide.middle, wide.lowest, wide.unguided])
    );
    assert!(wide.state.has_outstanding_for_candidate(wide.top));
    Ok(())
}

#[test]
// r[verify campaign.fd.dynamics.invalidate]
// r[verify campaign.pruning]
// r[verify campaign.pruning.valid]
fn an_over_limit_frontier_keeps_the_declared_best_and_prunes_the_lowest() -> Result<(), String> {
    let Frontier {
        events,
        state,
        top,
        kept,
        middle,
        lowest,
        unguided,
    } = frontier(NARROW_BATCH)?;
    let plan = campaign_core::plan_pruning(&state, plan_request(&state, vec![FIRST_DIMENSION]))
        .map_err(crate::suite::s3_fixture::err)?
        .ok_or_else(|| String::from("expected a prune plan"))?;
    assert_eq!(plan.candidates, sorted(vec![lowest, unguided]));
    assert_eq!(plan.retention_profile_id, Some(profile(b"retention")));
    assert_eq!(plan.rank_order, vec![FIRST_DIMENSION]);
    let mut other_profile = plan_request(&state, vec![FIRST_DIMENSION]);
    other_profile.retention_profile_id = profile(b"other-retention");
    let other_plan = campaign_core::plan_pruning(&state, other_profile)
        .map_err(crate::suite::s3_fixture::err)?
        .ok_or_else(|| String::from("expected a prune plan"))?;
    assert_eq!(other_plan.candidates, plan.candidates);
    assert_ne!(other_plan.prune_plan_id, plan.prune_plan_id);
    let event = campaign_core::prune_frontier(&state, plan.clone())
        .map_err(crate::suite::s3_fixture::err)?;
    let campaign_core::CampaignControlEvent::CandidatesPruned(prune) = event.clone() else {
        return Err(String::from("expected a prune event"));
    };
    assert_eq!(prune.prune_plan_id, plan.prune_plan_id);
    assert_eq!(
        prune.dynamics(),
        campaign_core::FrontierDynamics::Invalidation {
            invalidation_id: prune.invalidation_id,
            cleared_dependency_id: plan.prune_plan_id,
        }
    );
    campaign_core::validate_dynamics(&prune.dynamics()).map_err(crate::suite::s3_fixture::err)?;
    let (_events, pruned) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(pruned.candidates.len(), state.candidates.len());
    for retained in [top, kept, middle] {
        assert_eq!(
            pruned
                .candidate(retained)
                .map_err(crate::suite::s3_fixture::err)?
                .status,
            campaign_core::CandidateStatus::Retained
        );
    }
    for gone in [lowest, unguided] {
        assert_eq!(
            pruned
                .candidate(gone)
                .map_err(crate::suite::s3_fixture::err)?
                .status,
            campaign_core::CandidateStatus::Pruned {
                prune_plan_id: plan.prune_plan_id
            }
        );
    }
    let next = campaign_core::plan_pruning(&pruned, plan_request(&pruned, vec![FIRST_DIMENSION]))
        .map_err(crate::suite::s3_fixture::err)?
        .ok_or_else(|| String::from("expected the next prune plan"))?;
    assert_eq!(next.candidates, vec![middle]);
    Ok(())
}

#[test]
fn a_frontier_within_the_retained_bound_needs_no_plan() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (_events, state, _candidate) = crate::suite::s3_fixture::guided(
        events,
        &state,
        b"within",
        crate::suite::constants::RANK_HIGH,
    )?;
    assert_eq!(
        campaign_core::plan_pruning(&state, plan_request(&state, vec![FIRST_DIMENSION]))
            .map_err(crate::suite::s3_fixture::err)?,
        None
    );
    Ok(())
}
