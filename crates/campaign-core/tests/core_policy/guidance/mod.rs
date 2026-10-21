#[test]
// r[verify campaign.fd.dynamics.relax]
fn relaxation_requires_strict_improvement() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, candidate_id) = crate::suite::candidate::admit(
        events,
        &state,
        b"candidate",
        crate::suite::constants::COST_ONE,
    )?;
    let (_events, state, _guidance_id) = crate::suite::candidate::guide(
        events,
        &state,
        candidate_id,
        crate::suite::constants::RANK_HIGH,
        crate::suite::constants::COST_ONE,
    )?;
    assert!(
        state
            .candidates
            .iter()
            .filter_map(|entry| entry.latest_guidance.as_ref())
            .any(|record| record.ranks.as_slice() == [crate::suite::constants::RANK_HIGH])
    );
    let worse = campaign_core::GuidanceRecord {
        guidance_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        candidate_id,
        generation: crate::suite::constants::GENERATION_NEXT,
        ranks: vec![crate::suite::constants::RANK_LOW],
        source_facts_id: crate::suite::admission::xid(
            campaign_core::IdKind::SourceFacts,
            b"source-facts-two",
        ),
        ordering_profile_id: state.admission.guidance_profile_id,
        adapter_id: state.admission.adapter_id,
    };
    assert_eq!(
        (campaign_core::admit_guidance(&state, worse))
            .err()
            .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::NonImprovingRelaxation
    );
    Ok(())
}
#[test]
// r[verify campaign.entropy]
fn entropy_is_explicit_and_deterministic() -> Result<(), String> {
    let (events, state) = crate::suite::admission::initial_state()?;
    let (events, state, first) = crate::suite::candidate::admit(
        events,
        &state,
        b"first",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, second) = crate::suite::candidate::admit(
        events,
        &state,
        b"second",
        crate::suite::constants::COST_ONE,
    )?;
    let (events, state, _g) = crate::suite::candidate::guide(
        events,
        &state,
        first,
        crate::suite::constants::RANK_LOW,
        crate::suite::constants::COST_ONE,
    )?;
    let (_events, state, _g) = crate::suite::candidate::guide(
        events,
        &state,
        second,
        crate::suite::constants::RANK_OTHER,
        crate::suite::constants::COST_ONE,
    )?;
    let source = state.admission.entropy_profile_id;
    let profile = crate::suite::selection::profile(campaign_core::SelectionMode::Exploratory {
        threshold_parts_per_million: crate::suite::constants::ENTROPY_ALWAYS,
        entropy_source_id: source,
        exploration_order: vec![crate::suite::constants::FIRST_RANK_DIMENSION],
    })?;
    assert_eq!(
        (campaign_core::plan_next(
            &state,
            crate::suite::selection::request(&state, profile.clone(), None),
            &state.admission.projection
        ))
        .err()
        .ok_or_else(|| String::from("expected the exact typed denial"))?,
        campaign_core::CampaignError::InvalidEntropy
    );
    let entropy = ticket(&state, &profile, source)?;
    let left = (campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state, profile.clone(), Some(entropy.clone())),
        &state.admission.projection,
    ))
    .map_err(|error| format!("{error:?}"))?;
    let right = (campaign_core::plan_next(
        &state,
        crate::suite::selection::request(&state, profile, Some(entropy)),
        &state.admission.projection,
    ))
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(left, right);
    Ok(())
}

fn ticket(
    state: &campaign_core::FrontierState,
    profile: &campaign_core::SelectionProfile,
    source: campaign_core::Id,
) -> Result<campaign_core::EntropyTicket, String> {
    let mut bytes = [crate::suite::constants::FIRST_BYTE_INDEX_ZERO; campaign_core::DIGEST_LENGTH];
    bytes[0] = crate::suite::constants::FIRST_BYTE_INDEX_ONE;
    let entropy = campaign_core::EntropyTicket {
        entropy_id: campaign_core::Id::absent(),
        campaign_id: state.admission.campaign_id,
        state_id: state.state_id,
        profile_id: profile.profile_id,
        source_id: source,
        counter: crate::suite::constants::ENTROPY_COUNTER,
        bytes,
    }
    .derive()
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(entropy.state_id, state.state_id);
    assert_eq!(entropy.profile_id, profile.profile_id);
    Ok(entropy)
}
