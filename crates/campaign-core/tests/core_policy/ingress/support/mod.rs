//! Fixtures for candidate and guidance intake scenarios.

pub(crate) type Events = Vec<campaign_core::CampaignControlEvent>;

pub(crate) const REJECT: campaign_core::AdmissionRule = campaign_core::AdmissionRule {
    duplicate_candidate: campaign_core::DuplicateCandidateRule::Reject,
};
pub(crate) const COALESCE: campaign_core::AdmissionRule = campaign_core::AdmissionRule {
    duplicate_candidate: campaign_core::DuplicateCandidateRule::Coalesce,
};

pub(crate) fn text<T, E: std::fmt::Debug>(value: Result<T, E>) -> Result<T, String> {
    value.map_err(|error| format!("{error:?}"))
}

pub(crate) fn coalescing_policy() -> campaign_core::Id {
    crate::suite::admission::xid(campaign_core::IdKind::CoalescingPolicy, b"coalesce")
}

/// Admitted campaign whose bounds were adjusted by `configure`.
pub(crate) fn admitted_with(
    configure: impl FnOnce(&mut campaign_core::ExplorationBounds),
) -> Result<(Events, campaign_core::FrontierState), String> {
    let mut input = crate::suite::admission::default_input()?;
    configure(&mut input.bounds);
    input.bounds = text(input.bounds.derive())?;
    crate::suite::admission::admitted(input)
}

pub(crate) fn admit_batch(
    state: &campaign_core::FrontierState,
    candidates: Vec<campaign_core::CandidateRecord>,
    rule: campaign_core::AdmissionRule,
) -> campaign_core::CampaignResult<campaign_core::CampaignControlEvent> {
    campaign_core::admit_candidates(state, candidates, rule, coalescing_policy())
}

pub(crate) fn batch_of(
    event: &campaign_core::CampaignControlEvent,
) -> Result<&campaign_core::CandidateBatchEvent, String> {
    match event {
        campaign_core::CampaignControlEvent::CandidatesAdmitted(batch) => Ok(batch),
        other => Err(format!("expected a candidate batch, got {other:?}")),
    }
}

pub(crate) fn guidance_of(
    event: &campaign_core::CampaignControlEvent,
) -> Result<&campaign_core::GuidanceEvent, String> {
    match event {
        campaign_core::CampaignControlEvent::GuidanceAdmitted(guidance) => Ok(guidance),
        other => Err(format!("expected a guidance event, got {other:?}")),
    }
}

/// Appends one event and reconstructs; a denial leaves the prefix state untouched.
pub(crate) fn replay_with(
    events: &[campaign_core::CampaignControlEvent],
    event: campaign_core::CampaignControlEvent,
) -> campaign_core::CampaignResult<campaign_core::FrontierState> {
    let mut extended = events.to_vec();
    extended.push(event);
    campaign_core::reconstruct(&extended)
}

/// Asserts that replaying the prefix still yields the given state identity.
pub(crate) fn assert_unchanged(
    events: &[campaign_core::CampaignControlEvent],
    state: &campaign_core::FrontierState,
) -> Result<(), String> {
    let rebuilt = text(campaign_core::reconstruct(events))?;
    assert_eq!(rebuilt.state_id, state.state_id);
    Ok(())
}

/// Builds a well-formed batch event that bypasses intake; every record is marked admitted.
pub(crate) fn forged_batch(
    state: &campaign_core::FrontierState,
    mut candidates: Vec<campaign_core::CandidateRecord>,
    rule: campaign_core::AdmissionRule,
) -> Result<campaign_core::CampaignControlEvent, String> {
    candidates.sort_by_key(|candidate| candidate.candidate_id);
    let outcomes: Vec<campaign_core::AdmissionOutcome> = candidates
        .iter()
        .map(|candidate| campaign_core::AdmissionOutcome {
            candidate_id: candidate.candidate_id,
            outcome: campaign_core::AdmissionDecision::Admitted,
        })
        .collect();
    let mut batch = campaign_core::CandidateBatchEvent {
        event_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        candidates,
        admission_rule: rule,
        admission_policy_id: text(rule.admission_policy_id())?,
        coalescing_policy_id: coalescing_policy(),
        admission_outcomes_id: text(campaign_core::derive_admission_outcomes_id(&outcomes))?,
        outcomes,
    };
    batch.event_id = text(batch.expected_event_id())?;
    assert_eq!(batch.outcomes.len(), batch.candidates.len());
    assert_eq!(batch.event_id.kind, campaign_core::IdKind::Event);
    Ok(campaign_core::CampaignControlEvent::CandidatesAdmitted(
        batch,
    ))
}

/// Re-derives the outcome and event identities of an edited batch so only its content is forged.
pub(crate) fn resealed_batch(
    mut batch: campaign_core::CandidateBatchEvent,
) -> Result<campaign_core::CampaignControlEvent, String> {
    batch.admission_outcomes_id =
        text(campaign_core::derive_admission_outcomes_id(&batch.outcomes))?;
    batch.event_id = text(batch.expected_event_id())?;
    Ok(campaign_core::CampaignControlEvent::CandidatesAdmitted(
        batch,
    ))
}

/// Builds a well-formed guidance event with an arbitrary claimed dynamic, bypassing intake.
pub(crate) fn forged_guidance(
    state: &campaign_core::FrontierState,
    guidance: campaign_core::GuidanceRecord,
    dynamics: campaign_core::GuidanceDynamics,
) -> Result<campaign_core::CampaignControlEvent, String> {
    let mut event = campaign_core::GuidanceEvent {
        event_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_before_id: state.state_id,
        admission_outcomes_id: text(dynamics.outcomes_id(guidance.guidance_id))?,
        guidance,
        dynamics,
    };
    event.event_id = text(event.expected_event_id())?;
    Ok(campaign_core::CampaignControlEvent::GuidanceAdmitted(event))
}

/// Guidance record for `candidate_id` with explicit generation and ranks.
pub(crate) fn guidance(
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
        source_facts_id: crate::suite::admission::xid(
            campaign_core::IdKind::SourceFacts,
            b"intake-source-facts",
        ),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    }
}

/// Admits one event through intake and replay, returning the extended history and state.
pub(crate) fn accept(
    mut events: Events,
    event: campaign_core::CampaignControlEvent,
) -> Result<(Events, campaign_core::FrontierState), String> {
    events.push(event);
    let state = text(campaign_core::reconstruct(&events))?;
    Ok((events, state))
}

/// Admitted state with one guided candidate at `rank` (generation 1).
pub(crate) fn guided(
    rank: i64,
) -> Result<(Events, campaign_core::FrontierState, campaign_core::Id), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"guided",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        rank,
        crate::suite::constants::COST_ONE,
    )?;
    Ok((events, state, candidate_id))
}

/// Latest guidance record of a candidate.
pub(crate) fn latest(
    state: &campaign_core::FrontierState,
    candidate_id: campaign_core::Id,
) -> Result<campaign_core::GuidanceRecord, String> {
    text(state.candidate(candidate_id))?
        .latest_guidance
        .clone()
        .ok_or_else(|| String::from("candidate has no guidance"))
}
