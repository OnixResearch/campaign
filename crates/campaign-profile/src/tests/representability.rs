//! `campaign.configuration.invalid` decay domain: the loader admits only a rank magnitude and a
//! `subtract_from_first_rank` decay that planning can apply under the declared bounds without leaving `i64`.

/// Mirrors `config/fixtures/positive/decay-limit-profile.ncl`: `(2^31 - 1) * 2^32 + 2^32 == 2^63`.
const EDGE_SELECTIONS: u64 = 4_294_967_297;
const EDGE_RANK_MAGNITUDE: u64 = 4_294_967_296;
const EDGE_PENALTY: u64 = 2_147_483_647;
const OVER_EDGE_RANK_MAGNITUDE: u64 = 4_294_967_297;
const OVER_EDGE_PENALTY: u64 = 2_147_483_648;
/// Planning edge: one prior selection lowers the lowest rank `-i64::MAX` by one to exactly `i64::MIN`.
const PLANNING_EDGE_SELECTIONS: u64 = 2;
const PLANNING_EDGE_PENALTY: u64 = 1;
const PLANNING_OVER_EDGE_PENALTY: u64 = 2;
/// Mirrors `config/fixtures/positive/decay-small-bound-edge-profile.ncl`:
/// `2^61 * (3 - 1) + 2^62 == 2^63` with three selections and outstanding selections.
const SMALL_EDGE_SELECTIONS: u64 = 3;
const SMALL_EDGE_RANK_MAGNITUDE: u64 = 1 << 62;
const SMALL_EDGE_PENALTY: u64 = 1 << 61;

type Events = Vec<campaign_core::CampaignControlEvent>;

/// The default document with `bounds` overridden and ranked selection decaying by `penalty`.
fn ranked(bounds: &[(&str, u64)], penalty: u64) -> Result<serde_json::Value, String> {
    let mut document = super::default_document()?;
    for (bound, value) in bounds {
        *document
            .pointer_mut(&format!("/bounds/{bound}"))
            .ok_or_else(|| format!("fixture bound {bound} is absent"))? = serde_json::json!(value);
    }
    *document
        .pointer_mut("/selection")
        .ok_or("fixture selection is absent")? = serde_json::json!({
        "mode": "ranked",
        "decay": { "rule": "subtract_from_first_rank", "penalty_per_selection": penalty },
        "tie_break": "candidate_identity",
    });
    Ok(document)
}

#[test]
fn the_rank_magnitude_must_fit_i64() -> super::TestResult {
    const POINTER: &str = "/bounds/maximum_rank_magnitude";
    // `|i64::MIN|` is one past the limit: an admitted `i64::MIN` rank would have no negation.
    for over in [i64::MIN.unsigned_abs(), u64::MAX] {
        assert_eq!(
            super::rejected(&super::with(POINTER, serde_json::json!(over))?)?,
            crate::ProfileError::RankMagnitudeOutOfRange,
            "maximum_rank_magnitude {over}"
        );
    }
    let at_limit = super::rejection::loaded(&super::with(
        POINTER,
        serde_json::json!(campaign_core::MAX_RANK_MAGNITUDE),
    )?)?;
    assert_eq!(at_limit.bounds.max_rank_magnitude, i64::MAX.unsigned_abs());
    Ok(())
}

#[test]
// r[verify campaign.configuration.invalid]
fn the_decay_must_stay_within_the_i64_rank_domain() -> super::TestResult {
    // The former boundary penalty over the default 65536 selections.
    assert_eq!(
        super::rejected(&ranked(&[], u64::MAX)?)?,
        crate::ProfileError::DecayExceedsRankDomain
    );
    let edge = [
        ("maximum_selections", EDGE_SELECTIONS),
        ("maximum_rank_magnitude", EDGE_RANK_MAGNITUDE),
    ];
    let at_edge = super::rejection::loaded(&ranked(&edge, EDGE_PENALTY)?)?;
    assert_eq!(
        at_edge.decay,
        campaign_core::SelectionCountDecay::SubtractFromFirstRank {
            penalty_per_selection: EDGE_PENALTY
        }
    );
    assert_eq!(at_edge.bounds.max_selections, EDGE_SELECTIONS);
    // Mirrors `config/fixtures/negative/decay-one-over-rank-domain.ncl`, then one more unit of penalty.
    let one_over_magnitude = [
        ("maximum_selections", EDGE_SELECTIONS),
        ("maximum_rank_magnitude", OVER_EDGE_RANK_MAGNITUDE),
    ];
    for document in [
        ranked(&one_over_magnitude, EDGE_PENALTY)?,
        ranked(&edge, OVER_EDGE_PENALTY)?,
    ] {
        assert_eq!(
            super::rejected(&document)?,
            crate::ProfileError::DecayExceedsRankDomain
        );
    }
    // A single selection is never decayed, so every penalty is representable.
    let single = super::rejection::loaded(&ranked(
        &[
            ("maximum_selections", 1),
            ("maximum_outstanding_selections", 1),
            ("maximum_rank_magnitude", campaign_core::MAX_RANK_MAGNITUDE),
        ],
        u64::MAX,
    )?)?;
    assert_eq!(single.bounds.max_selections, 1);
    Ok(())
}

#[test]
fn the_loader_matches_nickel_at_a_small_selection_bound_decay_edge() -> super::TestResult {
    let bounds = [
        ("maximum_selections", SMALL_EDGE_SELECTIONS),
        ("maximum_outstanding_selections", SMALL_EDGE_SELECTIONS),
        ("maximum_rank_magnitude", SMALL_EDGE_RANK_MAGNITUDE),
    ];
    let at_edge = super::rejection::loaded(&ranked(&bounds, SMALL_EDGE_PENALTY)?)?;
    assert_eq!(at_edge.bounds.max_selections, SMALL_EDGE_SELECTIONS);
    assert_eq!(at_edge.bounds.max_rank_magnitude, SMALL_EDGE_RANK_MAGNITUDE);
    // Mirrors `config/fixtures/negative/decay-small-bound-one-over.ncl`.
    let over = SMALL_EDGE_PENALTY
        .checked_add(1)
        .ok_or("penalty overflow")?;
    assert_eq!(
        super::rejected(&ranked(&bounds, over)?)?,
        crate::ProfileError::DecayExceedsRankDomain
    );
    Ok(())
}

/// Admits `profile.bounds` with a two-unit budget and one one-unit candidate.
fn with_candidate(
    profile: &crate::CampaignProfile,
) -> Result<(Events, campaign_core::FrontierState, campaign_core::Id), String> {
    let debug = |error: campaign_core::CampaignError| format!("{error:?}");
    let mut input = super::consumption::admitted(profile.bounds.clone())?;
    input.resource_budgets[0].amount = 2;
    let mut events = vec![campaign_core::admit_campaign(input).map_err(debug)?];
    let state = campaign_core::reconstruct(&events).map_err(debug)?;
    let candidate = campaign_core::CandidateRecord {
        candidate_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        parent_moment_id: state.admission.roots[0].moment_id,
        operation_profile_id: super::id(campaign_core::IdKind::OperationProfile, b"operation"),
        operation_schema_id: state.admission.operation_schema_ids[0],
        source_adapter_id: state.admission.adapter_id,
        depth: 1,
        cost_profile_id: super::id(campaign_core::IdKind::CostProfile, b"cost"),
        declared_costs: vec![campaign_core::ResourceAmount {
            dimension_id: state.admission.resource_budgets[0].dimension_id,
            amount: 1,
        }],
        operation_generation_profile_id: super::id(campaign_core::IdKind::Policy, b"generation"),
    }
    .derive()
    .map_err(debug)?;
    let candidate_id = candidate.candidate_id;
    let coalescing = super::id(campaign_core::IdKind::CoalescingPolicy, b"coalesce");
    events.push(
        campaign_core::admit_candidates(&state, vec![candidate], profile.admission, coalescing)
            .map_err(debug)?,
    );
    let state = campaign_core::reconstruct(&events).map_err(debug)?;
    assert_eq!(state.admission.bounds, profile.bounds);
    assert_eq!(state.candidates.len(), 1);
    Ok((events, state, candidate_id))
}

fn guidance(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    rank: i64,
) -> campaign_core::GuidanceRecord {
    campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation: 1,
        ranks: vec![rank],
        source_facts_id: super::id(campaign_core::IdKind::SourceFacts, b"facts"),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    }
}

fn plan(
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionProfile,
) -> campaign_core::CampaignResult<campaign_core::DecisionPlan> {
    campaign_core::plan_next(
        state,
        campaign_core::SelectionRequest {
            request_id: super::id(campaign_core::IdKind::Request, b"request"),
            campaign_id: state.admission.campaign_id,
            state_id: state.state_id,
            profile: selection.clone(),
            entropy: None,
            expected_generation: state.admission.projection.generation,
            expected_head_id: state.admission.projection.head_id,
        },
        &state.admission.projection,
    )
}

#[test]
fn a_loaded_decay_at_the_rank_domain_edge_keeps_planning() -> super::TestResult {
    let debug = |error: campaign_core::CampaignError| format!("{error:?}");
    let bounds = [
        ("maximum_selections", PLANNING_EDGE_SELECTIONS),
        ("maximum_outstanding_selections", PLANNING_EDGE_SELECTIONS),
        ("maximum_rank_magnitude", campaign_core::MAX_RANK_MAGNITUDE),
    ];
    assert_eq!(
        super::rejected(&ranked(&bounds, PLANNING_OVER_EDGE_PENALTY)?)?,
        crate::ProfileError::DecayExceedsRankDomain
    );
    let profile = super::rejection::loaded(&ranked(&bounds, PLANNING_EDGE_PENALTY)?)?;
    let selection = profile
        .selection_profile(super::consumption::runtime(None))
        .map_err(|error| error.to_string())?;
    let (mut events, state, candidate_id) = with_candidate(&profile)?;
    // The admitted magnitude excludes `i64::MIN`, the only rank whose negation is not an `i64`.
    assert_eq!(
        campaign_core::admit_guidance(&state, guidance(&state, candidate_id, i64::MIN)),
        Err(campaign_core::CampaignError::InvalidGuidanceShape)
    );
    events.push(
        campaign_core::admit_guidance(&state, guidance(&state, candidate_id, -i64::MAX))
            .map_err(debug)?,
    );
    let state = campaign_core::reconstruct(&events).map_err(debug)?;
    let campaign_core::NextDecision::Select(first) =
        plan(&state, &selection).map_err(debug)?.decision
    else {
        return Err(String::from(
            "the first plan must select the guided candidate",
        ));
    };
    events.push(campaign_core::CampaignControlEvent::SelectionPublished(
        campaign_core::SelectionEvent::new(*first, selection.clone(), None).map_err(debug)?,
    ));
    let state = campaign_core::reconstruct(&events).map_err(debug)?;
    // The decayed first rank is exactly `i64::MIN`; planning selects instead of overflowing.
    let campaign_core::NextDecision::Select(second) =
        plan(&state, &selection).map_err(debug)?.decision
    else {
        return Err(String::from(
            "the edge decay must still select the candidate",
        ));
    };
    assert_eq!(second.candidate_id, candidate_id);
    Ok(())
}
