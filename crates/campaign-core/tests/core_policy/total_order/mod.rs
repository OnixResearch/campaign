//! `campaign.selection.valid` / `campaign.guidance.valid`: one guidance rank-vector length per
//! campaign, fixed by its first admitted record, and a total ranked choice over it.

mod patterns;

pub(crate) type History = Vec<campaign_core::CampaignControlEvent>;

const SHAPE: &[u8] = b"rank-shape";
const WIDE: u64 = 1024;

fn indexed(kind: campaign_core::IdKind, index: u64) -> Result<campaign_core::Id, String> {
    campaign_core::identity::derive_id_with(kind, SHAPE, |writer| Ok(writer.u64(index)))
        .map_err(crate::suite::s3_fixture::err)
}

fn wide(bounds: &mut campaign_core::ExplorationBounds) {
    bounds.max_candidates = WIDE;
    bounds.max_candidate_batch = WIDE;
    bounds.max_retained_candidates = WIDE;
    bounds.max_guidance_records = WIDE;
    bounds.max_selections = WIDE;
    bounds.max_prune_batch = WIDE;
    bounds.max_control_events = WIDE.saturating_mul(4);
}

/// Admits `count` candidates in one batch and returns their identities in ascending order.
pub(crate) fn admitted_frontier(
    count: u64,
) -> Result<
    (
        History,
        campaign_core::FrontierState,
        Vec<campaign_core::Id>,
    ),
    String,
> {
    let (events, state) = crate::suite::s3_fixture::admitted_with(wide)?;
    let candidate_count = usize::try_from(count).map_err(crate::suite::s3_fixture::err)?;
    let mut records = Vec::with_capacity(candidate_count);
    for index in 0..count {
        let mut record =
            crate::suite::candidate::record(&state, SHAPE, crate::suite::constants::COST_ONE);
        record.operation_profile_id = indexed(campaign_core::IdKind::OperationProfile, index)?;
        record.cost_profile_id = indexed(campaign_core::IdKind::CostProfile, index)?;
        records.push(record.derive().map_err(crate::suite::s3_fixture::err)?);
    }
    let mut ids: Vec<campaign_core::Id> =
        records.iter().map(|record| record.candidate_id).collect();
    ids.sort();
    let event = campaign_core::admit_candidates(
        &state,
        records,
        crate::suite::ingress::support::REJECT,
        crate::suite::ingress::support::coalescing_policy(),
    )
    .map_err(crate::suite::s3_fixture::err)?;
    let (events, state) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(state.candidates.len(), candidate_count);
    assert_eq!(ids.len(), candidate_count);
    Ok((events, state, ids))
}

pub(crate) fn record(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    generation: u64,
    ranks: Vec<i64>,
) -> campaign_core::GuidanceRecord {
    campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation,
        ranks,
        source_facts_id: crate::suite::admission::xid(campaign_core::IdKind::SourceFacts, SHAPE),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    }
}

/// Admits one Discovery guidance record with `ranks` for each candidate.
pub(crate) fn guided(
    mut events: History,
    mut state: campaign_core::FrontierState,
    guidance: &[(campaign_core::Id, Vec<i64>)],
) -> Result<(History, campaign_core::FrontierState), String> {
    for (candidate_id, ranks) in guidance {
        let generation = crate::suite::constants::GENERATION_NEXT;
        let event = campaign_core::admit_guidance(
            &state,
            record(&state, *candidate_id, generation, ranks.clone()),
        )
        .map_err(crate::suite::s3_fixture::err)?;
        events.push(event.clone());
        state = campaign_core::apply_control_event(state, &event)
            .map_err(crate::suite::s3_fixture::err)?;
    }
    Ok((events, state))
}

/// A Discovery guidance event for `candidate_id` sealed through its canonical re-derivation, as a
/// forged history would carry it (no admission check ran).
pub(crate) fn sealed_discovery(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    ranks: Vec<i64>,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let generation = crate::suite::constants::GENERATION_NEXT;
    campaign_core::CampaignControlEvent::GuidanceAdmitted(campaign_core::GuidanceEvent {
        event_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        guidance: record(state, candidate_id, generation, ranks),
        dynamics: campaign_core::GuidanceDynamics::Discovery,
        admission_outcomes_id: campaign_core::Id::absent(),
    })
    .canonical()
    .map_err(crate::suite::s3_fixture::err)
}

/// The ranked plan's selection on `state`.
pub(crate) fn selected(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::SelectionDecision, String> {
    let profile = crate::suite::selection::ranked_profile()?;
    let request = crate::suite::selection::request(state, profile, None);
    let plan = campaign_core::plan_next(state, request, &state.admission.projection)
        .map_err(crate::suite::s3_fixture::err)?;
    match plan.decision {
        campaign_core::NextDecision::Select(selection) => Ok(*selection),
        other => Err(format!("expected a selection, got {other:?}")),
    }
}

/// After the first record fixes two dimensions, a shorter or longer vector is refused with
/// `InvalidGuidanceShape` as an admission, a revision, and a replayed event, without mutation;
/// vectors of the fixed length are admitted.
#[test]
// r[verify campaign.guidance.shape]
fn another_rank_length_is_refused_at_admission_revision_and_replay() -> Result<(), String> {
    let (events, state, ids) = admitted_frontier(3)?;
    let (low, mid, high) = (ids[0], ids[1], ids[2]);
    let (events, state) = guided(events, state, &[(low, vec![5, 1])])?;
    let latest = state
        .candidate(low)
        .map_err(crate::suite::s3_fixture::err)?
        .latest_guidance
        .clone()
        .ok_or("low is guided")?;
    let shape = campaign_core::CampaignError::InvalidGuidanceShape;
    let next = crate::suite::constants::GENERATION_NEXT.saturating_add(1);
    for ranks in [vec![5], vec![5, 3, 0]] {
        let update = record(&state, mid, next, ranks.clone());
        let admitted = campaign_core::admit_guidance(&state, update);
        assert_eq!(crate::suite::s3_fixture::denial(admitted)?, shape);
        let revision = campaign_core::GuidanceRevisionRequest {
            candidate_id: low,
            successor: record(&state, low, next, ranks.clone()),
            invalidated_guidance_id: latest.guidance_id,
            reason_id: crate::suite::admission::xid(campaign_core::IdKind::Reason, SHAPE),
        };
        let revised = campaign_core::revise_guidance(&state, revision);
        assert_eq!(crate::suite::s3_fixture::denial(revised)?, shape);
        let forged = sealed_discovery(&state, mid, ranks)?;
        let applied = campaign_core::apply_control_event(state.clone(), &forged);
        assert_eq!(crate::suite::s3_fixture::denial(applied)?, shape);
        assert_eq!(
            crate::suite::s3_fixture::replay_denial(&events, forged)?,
            shape
        );
        crate::suite::s3_fixture::assert_unchanged(&events, &state)?;
    }
    let (events, state) = guided(events, state, &[(mid, vec![5, 2]), (high, vec![5, 3])])?;
    assert_eq!(state.guidance_record_count, 3);
    let rebuilt = campaign_core::reconstruct(&events).map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(rebuilt, state);
    Ok(())
}

/// A first record of one dimension fixes a one-dimension campaign: a two-dimension vector is
/// refused, another one-dimension vector admitted.
#[test]
// r[verify campaign.guidance.shape]
fn the_first_record_fixes_a_one_dimension_campaign() -> Result<(), String> {
    let (events, state, ids) = admitted_frontier(2)?;
    let (events, state) = guided(events, state, &[(ids[0], vec![5])])?;
    let generation = crate::suite::constants::GENERATION_NEXT;
    let wider =
        campaign_core::admit_guidance(&state, record(&state, ids[1], generation, vec![5, 1]));
    assert_eq!(
        crate::suite::s3_fixture::denial(wider)?,
        campaign_core::CampaignError::InvalidGuidanceShape
    );
    let (_events, state) = guided(events, state, &[(ids[1], vec![7])])?;
    assert_eq!(selected(&state)?.candidate_id, ids[1]);
    Ok(())
}

/// The ranked planner selects the lexicographic maximum, as the reference order does; the
/// selection replays, and retiring a never-selected candidate does not change the choice.
#[test]
// r[verify campaign.guidance.valid]
fn the_ranked_planner_selects_the_lexicographic_maximum() -> Result<(), String> {
    let (events, state, ids) = admitted_frontier(3)?;
    let (low, mid, high) = (ids[0], ids[1], ids[2]);
    let guidance = [(low, vec![5, 1]), (mid, vec![5, 2]), (high, vec![5, 3])];
    let (events, state) = guided(events, state, &guidance)?;
    let selection = selected(&state)?;
    assert_eq!(selection.candidate_id, high);
    let profile = crate::suite::selection::ranked_profile()?;
    let event = crate::suite::selection::event_with(selection, profile, None)?;
    let (_published_events, published) = crate::suite::s3_fixture::append(events, event)?;
    assert_eq!(published.selection_count, 1);
    let retirement = crate::suite::s3_fixture::retirement(
        &state,
        mid,
        campaign_core::RetirementOrigin::Consumer,
    );
    let retire = campaign_core::retire_candidate(&state, retirement)
        .map_err(crate::suite::s3_fixture::err)?;
    let without_mid = campaign_core::apply_control_event(state, &retire)
        .map_err(crate::suite::s3_fixture::err)?;
    assert_eq!(selected(&without_mid)?.candidate_id, high);
    Ok(())
}
