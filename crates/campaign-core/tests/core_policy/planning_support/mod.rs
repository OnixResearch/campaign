//! Shared builders for the planning-path suites (projection, frontier, ranking, entropy,
//! selection, budgets, waits, and stopping).

pub(crate) type Events = Vec<campaign_core::CampaignControlEvent>;
pub(crate) type History = (Events, campaign_core::FrontierState);

pub(crate) const FIRST_GENERATION: u64 = 1;
pub(crate) const FIRST_ORDINAL: u64 = 1;
pub(crate) const SECOND_ORDINAL: u64 = 2;

pub(crate) fn observed<T>(value: campaign_core::CampaignResult<T>) -> Result<T, String> {
    value.map_err(|error| format!("{error:?}"))
}

pub(crate) fn denied<T: core::fmt::Debug>(
    value: campaign_core::CampaignResult<T>,
) -> Result<campaign_core::CampaignError, String> {
    match value {
        Ok(accepted) => Err(format!("expected a typed denial, got {accepted:?}")),
        Err(error) => Ok(error),
    }
}

pub(crate) fn admitted_with(
    configure: impl FnOnce(&mut campaign_core::CampaignAdmission),
) -> Result<History, String> {
    let mut input = crate::suite::admission::default_input()?;
    configure(&mut input);
    input.bounds = observed(input.bounds.derive())?;
    crate::suite::admission::admitted(input)
}

/// Admits one candidate with the default single-dimension cost and one first-generation rank.
pub(crate) fn guided(
    events: Events,
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    rank: i64,
) -> Result<(Events, campaign_core::FrontierState, campaign_core::Id), String> {
    let (events, state, candidate_id) =
        crate::suite::candidate::admit(events, state, label, crate::suite::constants::COST_ONE)?;
    let (events, state, _guidance_id) =
        crate::suite::candidate::guide(events, &state, candidate_id, rank, FIRST_GENERATION)?;
    Ok((events, state, candidate_id))
}

/// Admits one guidance record with explicit rank dimensions and returns its identity.
pub(crate) fn guide_ranks(
    mut events: Events,
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
    ranks: &[i64],
    generation: u64,
) -> Result<(Events, campaign_core::FrontierState, campaign_core::Id), String> {
    events.push(observed(campaign_core::admit_guidance(
        state,
        campaign_core::GuidanceRecord {
            guidance_id: campaign_core::Id::absent(),
            campaign_id: state.admission.campaign_id,
            candidate_id,
            generation,
            ranks: ranks.to_vec(),
            source_facts_id: crate::suite::admission::xid(
                campaign_core::IdKind::SourceFacts,
                b"planning-facts",
            ),
            ordering_profile_id: state.admission.guidance_profile_id,
            adapter_id: state.admission.adapter_id,
        },
    ))?);
    let next = observed(campaign_core::reconstruct(&events))?;
    let guidance = observed(next.candidate(candidate_id))?
        .latest_guidance
        .clone()
        .ok_or_else(|| String::from("admitted guidance must be the latest record"))?;
    assert_eq!(guidance.ranks, ranks.to_vec());
    assert_eq!(guidance.generation, generation);
    Ok((events, next, guidance.guidance_id))
}

pub(crate) fn profile_with(
    edit: impl FnOnce(&mut campaign_core::SelectionProfile),
) -> Result<campaign_core::SelectionProfile, String> {
    let mut profile = crate::suite::selection::ranked_profile()?;
    edit(&mut profile);
    observed(profile.derive())
}

pub(crate) fn plan_with(
    state: &campaign_core::FrontierState,
    profile: campaign_core::SelectionProfile,
    entropy: Option<campaign_core::EntropyTicket>,
) -> campaign_core::CampaignResult<campaign_core::DecisionPlan> {
    campaign_core::plan_next(
        state,
        crate::suite::selection::request(state, profile, entropy),
        &state.admission.projection,
    )
}

pub(crate) fn ranked(
    state: &campaign_core::FrontierState,
) -> Result<campaign_core::DecisionPlan, String> {
    observed(plan_with(
        state,
        crate::suite::selection::ranked_profile()?,
        None,
    ))
}

pub(crate) fn selected(
    plan: &campaign_core::DecisionPlan,
) -> Result<campaign_core::SelectionDecision, String> {
    let campaign_core::NextDecision::Select(selection) = &plan.decision else {
        return Err(format!("expected a selection, got {:?}", plan.decision));
    };
    Ok((**selection).clone())
}

pub(crate) fn stopped(
    plan: &campaign_core::DecisionPlan,
) -> Result<campaign_core::StopDecision, String> {
    let campaign_core::NextDecision::Stop(stop) = &plan.decision else {
        return Err(format!("expected a stop, got {:?}", plan.decision));
    };
    Ok((**stop).clone())
}

/// Re-derives an edited selection decision so replay validation, not identity framing, decides.
pub(crate) fn forged_selection(
    template: &campaign_core::SelectionDecision,
    edit: impl FnOnce(&mut campaign_core::SelectionDecision),
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut decision = template.clone();
    edit(&mut decision);
    let decision = observed(decision.derive())?;
    crate::suite::selection::event(decision)
}

pub(crate) fn replay_with(
    events: &[campaign_core::CampaignControlEvent],
    event: campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::FrontierState> {
    let mut forged = events.to_vec();
    forged.push(event);
    campaign_core::reconstruct(&forged)
}

/// Publishes the planner's ranked selection for the current state.
pub(crate) fn select_and_publish(
    events: Events,
    state: &campaign_core::FrontierState,
) -> Result<
    (
        Events,
        campaign_core::FrontierState,
        campaign_core::SelectionDecision,
    ),
    String,
> {
    let selection = selected(&ranked(state)?)?;
    let (events, state) = crate::suite::selection::publish(events, selection.clone())?;
    Ok((events, state, selection))
}

/// Settles one outstanding selection with a result carrying one child and the given findings.
pub(crate) fn settle(
    mut events: Events,
    state: &campaign_core::FrontierState,
    selection: &campaign_core::SelectionDecision,
    labels: (&'static [u8], &'static [u8]),
    findings: Vec<campaign_core::FindingRef>,
) -> Result<History, String> {
    let mut observation =
        crate::suite::observation::result_for(state, selection, labels.0, labels.1)?;
    observation.findings = findings;
    events.push(observed(campaign_core::admit_result(state, observation))?);
    let next = observed(campaign_core::reconstruct(&events))?;
    Ok((events, next))
}

pub(crate) fn finding(label: &'static [u8]) -> campaign_core::FindingRef {
    campaign_core::FindingRef {
        finding_id: crate::suite::admission::xid(campaign_core::IdKind::Finding, label),
        priority_facts_id: crate::suite::admission::xid(campaign_core::IdKind::SourceFacts, label),
    }
}

/// Replaces public state fields and recomputes the identity, modelling a caller-built state.
pub(crate) fn rebuilt(
    mut state: campaign_core::FrontierState,
    edit: impl FnOnce(&mut campaign_core::FrontierState),
) -> Result<campaign_core::FrontierState, String> {
    edit(&mut state);
    observed(state.recompute_identity())?;
    Ok(state)
}

/// The planning suites' only direct candidate-intake call: admits explicit records in one batch.
pub(crate) fn batch(
    state: &campaign_core::FrontierState,
    records: Vec<campaign_core::CandidateRecord>,
) -> campaign_core::CampaignResult<campaign_core::CampaignControlEvent> {
    campaign_core::admit_candidates(
        state,
        records,
        campaign_core::AdmissionRule {
            duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
        },
        crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce"),
    )
}

/// Admits one candidate with explicit declared costs and returns its identity.
pub(crate) fn admit_costed(
    mut events: Events,
    state: &campaign_core::FrontierState,
    label: &'static [u8],
    costs: Vec<campaign_core::ResourceAmount>,
) -> Result<(Events, campaign_core::FrontierState, campaign_core::Id), String> {
    let mut record =
        crate::suite::candidate::record(state, label, crate::suite::constants::COST_ONE);
    record.declared_costs = costs;
    let record = observed(record.derive())?;
    let candidate_id = record.candidate_id;
    events.push(observed(batch(state, vec![record]))?);
    let next = observed(campaign_core::reconstruct(&events))?;
    assert!(next.candidate(candidate_id).is_ok());
    Ok((events, next, candidate_id))
}

pub(crate) const RANK_TOP: i64 = 20;
pub(crate) const RANK_BOTTOM: i64 = 10;

/// Two guided candidates: `top` ranked above `bottom`.
pub(crate) fn two_guided() -> Result<
    (
        Events,
        campaign_core::FrontierState,
        campaign_core::Id,
        campaign_core::Id,
    ),
    String,
> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, top) = guided(events, &state, b"top", RANK_TOP)?;
    let (events, state, bottom) = guided(events, &state, b"bottom", RANK_BOTTOM)?;
    assert_ne!(top, bottom);
    assert_eq!(state.candidates.len(), 2);
    Ok((events, state, top, bottom))
}
